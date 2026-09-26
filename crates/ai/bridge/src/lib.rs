//! The bridge: the port's far ends, served on a socket.
//!
//! A mind runs outside the game, on its own clock, in whatever language it likes. This
//! crate holds the brainstem's [`Board`] and [`Orders`] handles and answers for them on a
//! loopback TCP socket, one thread per connection, speaking the contract in
//! `proto/murabito.proto`: a `Request` in, framed as a four-byte big-endian length and
//! then the bytes; a `Snapshots` out, framed the same way, in answer to a request for
//! them. An order gets no reply; what became of it is the body's next
//! `previous`. Several clients may connect; a visualizer that only asks for
//! snapshots is a client like the mind.
//!
//! Nothing behind this crate knows protobuf exists: `wire` converts at the edge, and a
//! malformed order is refused there with a word on why. Always on, not a debug feature;
//! a game with no client connected has bodies that stand still. Design:
//! `docs/ai_readme.md`.

use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};

use bevy::prelude::*;
use murabito_brainstem::{Board, BrainstemPlugin, Orders, Port};
use prost::Message;

mod wire;

/// The contract, as `proto/murabito.proto` compiles to.
pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/murabito.ai.rs"));
}

/// The port the game listens on, beside the debug server's 15702.
pub const PORT: u16 = 15703;

/// A frame longer than this is a client gone wrong, not a request, and closes the
/// connection: a snapshots request is a few bytes, an order a few dozen.
const LONGEST_FRAME: u32 = 1 << 20;

/// Serves the port on the loopback address. Adds the brainstem if the app hasn't.
pub struct BridgePlugin {
    port: u16,
}

impl Default for BridgePlugin {
    fn default() -> Self {
        Self::on(PORT)
    }
}

impl BridgePlugin {
    /// On that port; 0 lets the system pick one, which [`Bridge`] then names.
    pub fn on(port: u16) -> Self {
        Self { port }
    }
}

impl Plugin for BridgePlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<BrainstemPlugin>() {
            app.add_plugins(BrainstemPlugin);
        }
        let port = app.world().resource::<Port>();
        let (board, orders) = (port.board(), port.orders());
        let listener = match TcpListener::bind(("127.0.0.1", self.port)) {
            Ok(listener) => listener,
            Err(cause) => {
                error!(
                    "the bridge can't listen on port {}: {cause}. No mind can reach this game.",
                    self.port
                );
                return;
            }
        };
        let address = listener
            .local_addr()
            .expect("a bound listener has an address");
        info!("the bridge listens on {address}");
        app.insert_resource(Bridge { address });
        std::thread::spawn(move || serve(listener, board, orders));
    }
}

/// Where the bridge listens. Absent if it couldn't.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bridge {
    pub address: SocketAddr,
}

/// Accepts connections forever, each on its own thread.
fn serve(listener: TcpListener, board: Board, orders: Orders) {
    for stream in listener.incoming() {
        let Ok(stream) = stream else {
            continue;
        };
        let (board, orders) = (board.clone(), orders.clone());
        std::thread::spawn(move || {
            if let Err(cause) = attend(stream, &board, &orders) {
                debug!("a client left: {cause}");
            }
        });
    }
}

/// Answers one client until it hangs up or sends something that isn't a request.
fn attend(mut stream: TcpStream, board: &Board, orders: &Orders) -> io::Result<()> {
    loop {
        let bytes = read_frame(&mut stream)?;
        let request = proto::Request::decode(bytes.as_slice())
            .map_err(|cause| io::Error::new(io::ErrorKind::InvalidData, cause))?;
        match request.kind {
            Some(proto::request::Kind::Snapshots(_)) => {
                let reply = wire::snapshots(board.read());
                write_frame(&mut stream, &reply.encode_to_vec())?;
            }
            Some(proto::request::Kind::Order(order)) => match wire::order_in(order) {
                Ok((id, intent)) => {
                    if orders.send(id, intent).is_err() {
                        return Err(io::Error::new(
                            io::ErrorKind::BrokenPipe,
                            "the world is gone",
                        ));
                    }
                }
                Err(cause) => warn!("an order was refused: {cause}"),
            },
            None => warn!("a request with nothing in it"),
        }
    }
}

/// One frame off the stream: a big-endian `u32` length, then that many bytes. An
/// orderly hang-up between frames is `UnexpectedEof`, like a cut mid-frame.
fn read_frame(stream: &mut impl Read) -> io::Result<Vec<u8>> {
    let mut length = [0u8; 4];
    stream.read_exact(&mut length)?;
    let length = u32::from_be_bytes(length);
    if length > LONGEST_FRAME {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("a frame of {length} bytes"),
        ));
    }
    let mut bytes = vec![0u8; length as usize];
    stream.read_exact(&mut bytes)?;
    Ok(bytes)
}

/// One frame onto the stream, the same way.
fn write_frame(stream: &mut impl Write, bytes: &[u8]) -> io::Result<()> {
    let length = u32::try_from(bytes.len())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "a frame too long to send"))?;
    stream.write_all(&length.to_be_bytes())?;
    stream.write_all(bytes)?;
    stream.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    use murabito_actions::{ActionQueue, ActionsPlugin};
    use murabito_brainstem::{Brainstem, Intent, Outcome, Short};
    use murabito_hexcoords::{Direction, VoxelCoord};
    use murabito_identity::{IdentityPlugin, Kind, NextThingId, ThingId};
    use murabito_movement::{Locomotion, MovementPlugin};
    use murabito_perception::PerceptionPlugin;
    use murabito_placement::{Facing, VoxelPosition};
    use murabito_progress::ProgressPlugin;
    use murabito_vision::{Band, Vision, VisionPlugin};

    const ALL_ROUND: Vision = Vision {
        arc: 360.0,
        bands: [
            Band {
                range: 8,
                sensitivity: 1.0,
            },
            Band {
                range: 16,
                sensitivity: 0.6,
            },
            Band {
                range: 24,
                sensitivity: 0.3,
            },
        ],
    };

    fn voxel(q: i32, r: i32) -> VoxelCoord {
        VoxelCoord::new(q, r, -q - r, 0).unwrap()
    }

    // -- the frames alone -----------------------------------------------------------

    #[test]
    fn a_frame_is_its_length_then_its_bytes_and_reads_back() {
        let mut wire = Vec::new();
        write_frame(&mut wire, b"hello").unwrap();
        assert_eq!(&wire[..4], &[0, 0, 0, 5]);
        assert_eq!(&wire[4..], b"hello");
        assert_eq!(read_frame(&mut wire.as_slice()).unwrap(), b"hello");
    }

    #[test]
    fn a_hang_up_between_frames_and_a_cut_mid_frame_both_read_as_eof() {
        let nothing: &[u8] = &[];
        assert_eq!(
            read_frame(&mut &*nothing).unwrap_err().kind(),
            io::ErrorKind::UnexpectedEof
        );
        let cut: &[u8] = &[0, 0, 0, 9, 1, 2];
        assert_eq!(
            read_frame(&mut &*cut).unwrap_err().kind(),
            io::ErrorKind::UnexpectedEof
        );
    }

    #[test]
    fn an_absurd_length_is_refused_before_anything_is_allocated() {
        let huge: &[u8] = &[0xff, 0xff, 0xff, 0xff];
        assert_eq!(
            read_frame(&mut &*huge).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }

    // -- over a socket --------------------------------------------------------------

    /// A ticking app with one labelled body at the origin facing east, and the bridge
    /// on a port the system picked.
    fn game() -> (App, Entity, ThingId) {
        use bevy::time::TimeUpdateStrategy;

        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            IdentityPlugin,
            ProgressPlugin,
            MovementPlugin,
            ActionsPlugin,
            PerceptionPlugin,
            VisionPlugin,
            BrainstemPlugin,
            BridgePlugin::on(0),
        ));
        app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
        app.update();
        let id = app.world_mut().resource_mut::<NextThingId>().mint();
        let body = app
            .world_mut()
            .spawn((
                id,
                Kind::at("test::bodies::body"),
                VoxelPosition(voxel(0, 0)),
                Facing(Direction::E),
                Locomotion {
                    speed: 4.0,
                    turn_speed: 180.0,
                },
                ALL_ROUND,
                ActionQueue::default(),
                Brainstem::default(),
            ))
            .id();
        (app, body, id)
    }

    fn tick(app: &mut App, times: usize) {
        for _ in 0..times {
            app.update();
        }
    }

    /// A client, as the Python side will be: connect, frame, wait for the answer.
    fn connect(app: &App) -> TcpStream {
        let address = app.world().resource::<Bridge>().address;
        let stream = TcpStream::connect(address).expect("the bridge is listening");
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        stream
    }

    fn ask(stream: &mut TcpStream, request: proto::Request) {
        write_frame(stream, &request.encode_to_vec()).unwrap();
    }

    fn snapshots_request() -> proto::Request {
        proto::Request {
            kind: Some(proto::request::Kind::Snapshots(proto::SnapshotsRequest {})),
        }
    }

    fn order(id: ThingId, intent: Intent) -> proto::Request {
        proto::Request {
            kind: Some(proto::request::Kind::Order(proto::Order {
                id: id.number(),
                intent: Some(intent.into()),
            })),
        }
    }

    fn answer(stream: &mut TcpStream) -> proto::Snapshots {
        let bytes = read_frame(stream).expect("an answer");
        proto::Snapshots::decode(bytes.as_slice()).expect("a snapshots message")
    }

    /// Orders cross a thread and a socket; the brainstem drains them on its next tick.
    /// Nothing in the test can see them land before then, so it waits a moment.
    fn let_the_order_arrive() {
        std::thread::sleep(Duration::from_millis(50));
    }

    #[test]
    fn the_bridge_names_where_it_listens_on_the_loopback_address() {
        let (app, _, _) = game();
        let bridge = app.world().resource::<Bridge>();
        assert!(bridge.address.ip().is_loopback());
        assert_ne!(bridge.address.port(), 0);
    }

    #[test]
    fn a_client_asks_for_snapshots_and_gets_every_body_as_of_the_last_tick() {
        let (mut app, _, id) = game();
        tick(&mut app, 3);

        let mut client = connect(&app);
        ask(&mut client, snapshots_request());
        let snapshots = answer(&mut client);

        assert_eq!(snapshots.bodies.len(), 1);
        let body = &snapshots.bodies[0];
        assert_eq!(body.id, id.number());
        assert_eq!(body.kind, "test::bodies::body");
        assert_eq!(body.tick, 3);
        assert_eq!(
            body.position,
            Some(proto::Voxel {
                q: 0,
                r: 0,
                layer: 0
            })
        );
        assert_eq!(body.facing, proto::Direction::E as i32);
        assert!(body.previous.is_none(), "nothing has been asked of it");
    }

    #[test]
    fn an_order_from_a_client_moves_the_body_and_the_next_snapshot_shows_it() {
        let (mut app, body, id) = game();
        tick(&mut app, 1);
        let mut client = connect(&app);

        ask(
            &mut client,
            order(id, Intent::Short(Short::Face(Direction::N))),
        );
        let_the_order_arrive();
        tick(&mut app, 1);
        assert_eq!(
            app.world()
                .get::<Brainstem>(body)
                .unwrap()
                .doing()
                .unwrap()
                .intent(),
            Intent::Short(Short::Face(Direction::N))
        );

        tick(&mut app, 32);
        assert_eq!(app.world().get::<Facing>(body).unwrap().0, Direction::N);
        assert_eq!(
            app.world()
                .get::<Brainstem>(body)
                .unwrap()
                .previous()
                .map(|p| p.outcome()),
            Some(Outcome::Done)
        );

        ask(&mut client, snapshots_request());
        let snapshots = answer(&mut client);
        assert_eq!(snapshots.bodies[0].facing, proto::Direction::N as i32);
        let previous = snapshots.bodies[0]
            .previous
            .as_ref()
            .expect("something ended");
        assert_eq!(
            previous.outcome.as_ref().unwrap().kind,
            Some(proto::outcome::Kind::Done(proto::Done {}))
        );
        assert_eq!(
            previous.intent.as_ref().unwrap().kind,
            Some(proto::intent::Kind::Short(proto::Short {
                kind: Some(proto::short::Kind::Face(proto::Direction::N as i32)),
            }))
        );
    }

    #[test]
    fn a_malformed_order_is_refused_and_the_client_stays_connected() {
        let (mut app, body, id) = game();
        tick(&mut app, 1);
        let mut client = connect(&app);

        let empty = proto::Request {
            kind: Some(proto::request::Kind::Order(proto::Order {
                id: id.number(),
                intent: None,
            })),
        };
        ask(&mut client, empty);
        let_the_order_arrive();
        tick(&mut app, 2);
        assert_eq!(app.world().get::<Brainstem>(body).unwrap().doing(), None);

        ask(&mut client, snapshots_request());
        assert_eq!(answer(&mut client).bodies.len(), 1, "still answered");
    }

    #[test]
    fn bytes_that_are_not_a_request_close_the_connection_and_the_game_ticks_on() {
        let (mut app, _, _) = game();
        tick(&mut app, 1);
        let mut client = connect(&app);

        write_frame(&mut client, &[0xff, 0xff, 0xff, 0xff, 0xff]).unwrap();
        let mut rest = Vec::new();
        assert_eq!(client.read_to_end(&mut rest).unwrap(), 0, "hung up on");
        tick(&mut app, 5);

        let mut again = connect(&app);
        ask(&mut again, snapshots_request());
        assert_eq!(answer(&mut again).bodies[0].tick, 6);
    }

    #[test]
    fn two_clients_are_answered_independently() {
        let (mut app, _, _) = game();
        tick(&mut app, 2);
        let mut one = connect(&app);
        let mut two = connect(&app);
        ask(&mut two, snapshots_request());
        ask(&mut one, snapshots_request());
        assert_eq!(answer(&mut one).bodies.len(), 1);
        assert_eq!(answer(&mut two).bodies.len(), 1);
    }
}

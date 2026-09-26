//! The port: where a mind meets a body, without either knowing what the other is.
//!
//! Two halves, in opposite directions, with opposite rules. Going up, the [`Board`]: one
//! [`Snapshot`] per body, rewritten every tick after the senses have run, latest wins,
//! read at any moment by whoever holds a handle to it. Going down, [`Orders`]: a channel
//! of intents addressed by `ThingId`, every one kept in order, drained once a tick and
//! handed to the body it names. The [`Port`] resource creates both and keeps the writing
//! half of the board and the receiving half of the channel; it hands out the far ends,
//! which are plain `Send` values, so a socket thread or a headless test can be a mind.
//!
//! A snapshot is flat facts with names, a picture of one tick, built for a scorer to
//! read: where the body is, what it sees and how well, what it is doing and for how long,
//! and how its last order ended.

use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use murabito_actions::{Action, ActionQueue};
use murabito_hexcoords::{Direction, Offset, VoxelCoord};
use murabito_identity::{Kind, ThingId};
use murabito_placement::{Facing, VoxelPosition};
use murabito_progress::Progress;
use murabito_vision::{Acuity, Seen};

use crate::{Brainstem, Doing, Intent, Previous, Tick};

/// One body's picture of one tick: everything a mind is told.
#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    pub id: ThingId,
    /// The body's place in the tree of kinds, its whole path.
    pub kind: Kind,
    /// Which tick this is a picture of.
    pub tick: u64,
    pub position: VoxelCoord,
    pub facing: Direction,
    /// Everything in view this tick, one entry per sighting.
    pub in_view: Vec<InView>,
    /// The current intent and the tick it began, or nothing.
    pub doing: Option<Doing>,
    /// Every action waiting on the queue, first to last.
    pub queue: Vec<Action>,
    /// How far along the action in flight is, 0 to 1, or nothing in flight.
    pub in_flight: Option<f32>,
    /// What the body did last and how it ended: the one before `doing`, not `doing`
    /// itself. Nothing until something has been asked.
    pub previous: Option<Previous>,
}

/// One thing in view, as the body sees it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InView {
    pub id: ThingId,
    /// Its place in the tree of kinds, or nothing if it carries no label.
    pub kind: Option<Kind>,
    /// Where it is relative to the body: `its voxel - mine`.
    pub offset: Offset,
    /// How many steps across faces away it is.
    pub distance: u32,
    pub acuity: Acuity,
}

/// One intent for one body, as a mind sends it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Order {
    pub id: ThingId,
    pub intent: Intent,
}

type Cards = Arc<Mutex<HashMap<ThingId, Snapshot>>>;

/// The reading end of the board: the latest snapshot of every body, at any moment.
#[derive(Clone)]
pub struct Board(Cards);

impl Board {
    /// Every body's latest snapshot, in no particular order.
    pub fn read(&self) -> Vec<Snapshot> {
        self.0
            .lock()
            .expect("the board")
            .values()
            .cloned()
            .collect()
    }

    pub fn get(&self, id: ThingId) -> Option<Snapshot> {
        self.0.lock().expect("the board").get(&id).cloned()
    }
}

/// The sending end of the orders channel. Clone it as often as wanted.
#[derive(Clone)]
pub struct Orders(Sender<Order>);

impl Orders {
    /// Drops an order in. It is taken up on the next tick; an order for a body no longer
    /// in the world is dropped there. Sending fails only once the world is gone.
    pub fn send(&self, id: ThingId, intent: Intent) -> Result<(), Order> {
        self.0.send(Order { id, intent }).map_err(|failed| failed.0)
    }
}

/// Both halves, owned by the world. Hands out the far ends.
#[derive(Resource)]
pub struct Port {
    cards: Cards,
    orders: Sender<Order>,
    inbox: Mutex<Receiver<Order>>,
}

impl Default for Port {
    fn default() -> Self {
        let (orders, inbox) = channel();
        Self {
            cards: Cards::default(),
            orders,
            inbox: Mutex::new(inbox),
        }
    }
}

impl Port {
    /// A handle that reads the board, for a mind.
    pub fn board(&self) -> Board {
        Board(Arc::clone(&self.cards))
    }

    /// A handle that sends orders, for a mind.
    pub fn orders(&self) -> Orders {
        Orders(self.orders.clone())
    }

    fn drain(&self) -> Vec<Order> {
        self.inbox.lock().expect("the inbox").try_iter().collect()
    }

    fn post(&self, snapshots: impl IntoIterator<Item = Snapshot>) {
        let mut cards = self.cards.lock().expect("the board");
        cards.clear();
        cards.extend(
            snapshots
                .into_iter()
                .map(|snapshot| (snapshot.id, snapshot)),
        );
    }
}

/// Hands every order that arrived since last tick to the body it names. Orders for
/// bodies not in the world are dropped: the sender learns nothing, and the board shows
/// no such body.
pub(crate) fn take_orders(port: Res<Port>, mut bodies: Query<(&ThingId, &mut Brainstem)>) {
    let orders = port.drain();
    if orders.is_empty() {
        return;
    }
    let mut by_id: HashMap<ThingId, Mut<Brainstem>> =
        bodies.iter_mut().map(|(id, body)| (*id, body)).collect();
    for order in orders {
        if let Some(body) = by_id.get_mut(&order.id) {
            body.order(order.intent);
        }
    }
}

/// One body's snapshot, from its parts. Pure, so what a mind sees is tested on values.
#[allow(clippy::too_many_arguments)]
fn snapshot(
    id: ThingId,
    kind: Kind,
    tick: u64,
    position: VoxelCoord,
    facing: Direction,
    seen: Option<&Seen>,
    kind_of: impl Fn(Entity) -> Option<Kind>,
    body: &Brainstem,
    queue: &ActionQueue,
    progress: &Progress,
) -> Snapshot {
    let in_view = seen
        .into_iter()
        .flat_map(Seen::iter)
        .map(|sighting| InView {
            id: sighting.id,
            kind: kind_of(sighting.entity),
            offset: sighting.offset,
            distance: sighting.offset.steps(),
            acuity: sighting.acuity,
        })
        .collect();
    Snapshot {
        id,
        kind,
        tick,
        position,
        facing,
        in_view,
        doing: body.doing(),
        queue: queue.iter().collect(),
        in_flight: progress.in_flight().then(|| progress.fraction()),
        previous: body.previous(),
    }
}

/// Everything of a body that goes into its snapshot.
type BodyParts<'a> = (
    &'a ThingId,
    &'a Kind,
    &'a VoxelPosition,
    &'a Facing,
    Option<&'a Seen>,
    &'a Brainstem,
    &'a ActionQueue,
    &'a Progress,
);

/// Rewrites the board with every body's picture of this tick. Runs after the senses, so
/// the picture is this tick's.
pub(crate) fn publish(
    tick: Res<Tick>,
    port: Res<Port>,
    kinds: Query<&Kind>,
    bodies: Query<BodyParts>,
) {
    let kind_of = |thing: Entity| kinds.get(thing).ok().copied();
    port.post(bodies.iter().map(
        |(id, kind, position, facing, seen, body, queue, progress)| {
            snapshot(
                *id, *kind, tick.0, position.0, facing.0, seen, kind_of, body, queue, progress,
            )
        },
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{
        ALL_ROUND, E, N, app, brainstem, facing, mint, order, position, previous_outcome, tick,
        voxel,
    };
    use crate::{Outcome, Short};
    use murabito_movement::Locomotion;

    const BODY: Kind = Kind::at("test::bodies::body");
    const THING: Kind = Kind::at("test::things::thing");

    /// A labelled body at the origin facing east, and its id.
    fn labelled_body() -> (App, Entity, ThingId) {
        let mut app = app();
        let id = mint(&mut app);
        let body = app
            .world_mut()
            .spawn((
                id,
                BODY,
                VoxelPosition(voxel(0, 0)),
                Facing(E),
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

    fn board(app: &App) -> Board {
        app.world().resource::<Port>().board()
    }

    fn orders(app: &App) -> Orders {
        app.world().resource::<Port>().orders()
    }

    #[test]
    fn the_board_is_empty_until_a_tick_has_run_and_then_holds_every_body() {
        let (mut app, _, id) = labelled_body();
        assert!(board(&app).read().is_empty());
        tick(&mut app, 1);
        let cards = board(&app).read();
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].id, id);
        assert_eq!(board(&app).get(id), Some(cards[0].clone()));
    }

    #[test]
    fn a_snapshot_is_the_bodys_picture_of_that_tick() {
        let (mut app, _, id) = labelled_body();
        let thing = mint(&mut app);
        app.world_mut()
            .spawn((thing, THING, VoxelPosition(voxel(2, -4))));
        let unlabelled = mint(&mut app);
        app.world_mut()
            .spawn((unlabelled, VoxelPosition(voxel(3, 0))));
        tick(&mut app, 1);

        let card = board(&app).get(id).expect("on the board");
        assert_eq!(card.kind, BODY);
        assert_eq!(card.tick, 1);
        assert_eq!(card.position, voxel(0, 0));
        assert_eq!(card.facing, E);
        assert_eq!(card.doing, None);
        assert_eq!(card.queue, Vec::<Action>::new());
        assert_eq!(card.in_flight, None);
        assert_eq!(card.previous, None);

        let mut in_view = card.in_view.clone();
        in_view.sort_by_key(|seen| seen.id);
        assert_eq!(
            in_view,
            [
                InView {
                    id: thing,
                    kind: Some(THING),
                    offset: voxel(2, -4) - voxel(0, 0),
                    distance: 4,
                    acuity: Acuity::Near,
                },
                InView {
                    id: unlabelled,
                    kind: None,
                    offset: voxel(3, 0) - voxel(0, 0),
                    distance: 3,
                    acuity: Acuity::Near,
                },
            ]
        );
    }

    #[test]
    fn a_body_at_work_shows_its_intent_its_queue_and_what_is_in_flight() {
        // A step north from a body facing east is a turn first: the turn is in flight
        // and the step waits behind it on the queue.
        let (mut app, body, id) = labelled_body();
        order(&mut app, body, Intent::Short(Short::Step(N)));
        tick(&mut app, 8);

        let card = board(&app).get(id).unwrap();
        assert_eq!(card.tick, 8);
        let doing = card.doing.expect("stepping north");
        assert_eq!(doing.intent(), Intent::Short(Short::Step(N)));
        assert_eq!(doing.since(), 1);
        assert_eq!(card.queue, [Action::Go(N)]);
        let fraction = card.in_flight.expect("a notch of turning in flight");
        assert!(
            (fraction - 0.75).abs() < 1e-3,
            "eight ticks of a 10.67-tick notch: {fraction}"
        );
        assert_eq!(card.previous, None);
    }

    #[test]
    fn a_body_that_leaves_the_world_leaves_the_board() {
        let (mut app, body, id) = labelled_body();
        tick(&mut app, 1);
        assert!(board(&app).get(id).is_some());
        app.world_mut().despawn(body);
        tick(&mut app, 1);
        assert_eq!(board(&app).get(id), None);
        assert!(board(&app).read().is_empty());
    }

    #[test]
    fn an_order_sent_through_the_port_is_taken_up_on_the_next_tick() {
        let (mut app, body, id) = labelled_body();
        orders(&app)
            .send(id, Intent::Short(Short::Face(N)))
            .expect("the world is here");
        tick(&mut app, 1);
        assert_eq!(
            brainstem(&app, body).doing().unwrap().intent(),
            Intent::Short(Short::Face(N))
        );
        tick(&mut app, 32);
        assert_eq!(facing(&app, body), N);
        assert_eq!(previous_outcome(&app, body), Some(Outcome::Done));
    }

    #[test]
    fn an_order_for_a_body_not_in_the_world_is_dropped_and_harms_nothing() {
        let (mut app, body, _) = labelled_body();
        let nobody = mint(&mut app);
        orders(&app)
            .send(nobody, Intent::Short(Short::Step(E)))
            .unwrap();
        tick(&mut app, 20);
        assert_eq!(position(&app, body), voxel(0, 0));
        assert_eq!(previous_outcome(&app, body), None);
    }

    #[test]
    fn the_far_ends_work_from_another_thread() {
        let (mut app, body, id) = labelled_body();
        tick(&mut app, 1);
        let far_board = board(&app);
        let far_orders = orders(&app);

        let mind = std::thread::spawn(move || {
            let card = far_board.get(id).expect("the body is on the board");
            assert_eq!(card.position, voxel(0, 0));
            far_orders
                .send(card.id, Intent::Short(Short::Step(E)))
                .expect("sent");
            card.tick
        });
        assert_eq!(mind.join().expect("the mind returns"), 1);

        tick(&mut app, 17);
        assert_eq!(position(&app, body), voxel(1, 0));
        assert_eq!(previous_outcome(&app, body), Some(Outcome::Done));
        assert_eq!(board(&app).get(id).unwrap().position, voxel(1, 0));
    }

    #[test]
    fn the_snapshot_of_a_thing_seen_nowhere_has_nothing_in_view() {
        let (mut app, _, id) = labelled_body();
        tick(&mut app, 1);
        assert!(board(&app).get(id).unwrap().in_view.is_empty());
    }
}

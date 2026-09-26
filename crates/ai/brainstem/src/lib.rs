//! The brainstem: in the driver's seat, not in charge.
//!
//! Every sentient body has one. It runs every tick, holds one intent at a time, and is
//! the only thing that pushes onto the body's `ActionQueue`. It is dumb on purpose: with
//! nothing to do it stands there, since a rest is an empty queue. What to do comes from
//! elsewhere, in the vocabulary this crate owns:
//!
//! - a [`Short`] finishes within a round of the slower mind: one turn, one step, a stop.
//!   A reflex may only ever say one of these.
//! - a `Sustained` outlives rounds, and drive re-aims it every step. (Next.)
//!
//! Anything may [`Brainstem::order`] an intent, and a reflex may [`Brainstem::preempt`]
//! one; the systems here carry the current intent out and record how the last one ended
//! in an [`Outcome`], which is how the mind that asked learns its order was dropped.
//!
//! The tick, inside `AskingSet`, in [`BrainstemSet`]'s order: **Orders** takes what
//! arrived; **Reflexes** is the slot the reflexes crate fills, so a fright beats an order
//! from the same tick; **Drive** clears the queue for a new intent and pushes the next
//! action only when the queue is empty, so a step already in flight lands before anything
//! new begins. Design: `docs/ai_readme.md`.

use bevy::prelude::*;
use murabito_actions::{Action, ActionQueue, AskingSet};
use murabito_hexcoords::Direction;
use murabito_progress::Progress;

pub struct BrainstemPlugin;

impl Plugin for BrainstemPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Tick>()
            .configure_sets(
                FixedUpdate,
                (
                    BrainstemSet::Orders,
                    BrainstemSet::Reflexes,
                    BrainstemSet::Drive,
                )
                    .chain()
                    .in_set(AskingSet),
            )
            .add_systems(
                FixedUpdate,
                (
                    count_tick.in_set(BrainstemSet::Orders),
                    drive.in_set(BrainstemSet::Drive),
                ),
            );
    }
}

/// The three steps of a tick, in order, all inside `AskingSet`.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BrainstemSet {
    /// What arrived since last tick is taken up.
    Orders,
    /// Where the reflexes crate runs: after orders, so a reflex firing this tick wins.
    Reflexes,
    /// The current intent is carried out.
    Drive,
}

/// Which tick this is, counted from the first. The unit of "since" and "how long".
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Tick(u64);

fn count_tick(mut tick: ResMut<Tick>) {
    tick.0 += 1;
}

/// What a body can be told that finishes within one round of the slower mind: a single
/// motion, or none. The whole of what a reflex may say.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Short {
    /// Drop whatever is being done and stand.
    Stop,
    /// Turn to face that way.
    Face(Direction),
    /// Walk one cell that way, turning first if need be.
    Step(Direction),
}

/// Everything a body can be told.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Intent {
    Short(Short),
}

/// How the last intent ended, for whoever gave it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Outcome {
    /// Nothing has been asked yet.
    #[default]
    Idle,
    /// It was carried out.
    Done,
    /// A stop replaced it.
    Stopped,
    /// A newer order replaced it.
    Superseded,
    /// A reflex, named, replaced it.
    Cancelled(&'static str),
}

/// What the body is doing now, and since which tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Doing {
    intent: Intent,
    since: u64,
}

impl Doing {
    pub fn intent(&self) -> Intent {
        self.intent
    }

    pub fn since(&self) -> u64 {
        self.since
    }
}

/// Who put an intent in: the mind, or a reflex by name. Decides the outcome of whatever
/// it replaced.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cause {
    Order,
    Reflex(&'static str),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Pending {
    intent: Intent,
    cause: Cause,
}

/// A body's driver: one intent at a time, and how the last one ended. Every sentient has
/// one; anything may order it, a reflex may preempt it, and only the systems here read
/// it out onto the queue.
#[derive(Component, Debug, Default, Clone, PartialEq, Eq)]
pub struct Brainstem {
    /// Given, not yet begun. Drive takes it at the start of the next tick; two in one
    /// tick and the later wins.
    pending: Option<Pending>,
    doing: Option<Doing>,
    outcome: Outcome,
    /// The current short's action is on the queue, or past it.
    pushed: bool,
}

impl Brainstem {
    /// Tell the body what to do. Takes effect on the next tick's drive: the queue is
    /// cleared and this begins. Whatever was being done ends as `Superseded`, or
    /// `Stopped` if this is a stop.
    pub fn order(&mut self, intent: Intent) {
        self.pending = Some(Pending {
            intent,
            cause: Cause::Order,
        });
    }

    /// A reflex takes the body. Like an order, but whatever was being done ends as
    /// `Cancelled` by the reflex's name, so the mind learns its plan was dropped.
    pub fn preempt(&mut self, short: Short, reflex: &'static str) {
        self.pending = Some(Pending {
            intent: Intent::Short(short),
            cause: Cause::Reflex(reflex),
        });
    }

    pub fn doing(&self) -> Option<Doing> {
        self.doing
    }

    pub fn outcome(&self) -> Outcome {
        self.outcome
    }

    /// Whether a new intent waits for drive to take it up.
    fn take_pending(&mut self) -> Option<Pending> {
        self.pending.take()
    }

    /// The pending intent becomes what is done, and what was done gets its outcome. Pure
    /// bookkeeping: the queue is drive's to clear.
    fn begin(&mut self, pending: Pending, now: u64) {
        let is_stop = pending.intent == Intent::Short(Short::Stop);
        if self.doing.is_some() {
            self.outcome = match (pending.cause, is_stop) {
                (Cause::Reflex(name), _) => Outcome::Cancelled(name),
                (Cause::Order, true) => Outcome::Stopped,
                (Cause::Order, false) => Outcome::Superseded,
            };
        }
        self.doing = (!is_stop).then_some(Doing {
            intent: pending.intent,
            since: now,
        });
        self.pushed = false;
    }

    fn finish(&mut self, outcome: Outcome) {
        self.doing = None;
        self.outcome = outcome;
        self.pushed = false;
    }
}

/// The one action a short motion is: a stop is none, and never reaches here.
fn action_for(short: Short) -> Option<Action> {
    match short {
        Short::Stop => None,
        Short::Face(direction) => Some(Action::Face(direction)),
        Short::Step(direction) => Some(Action::Go(direction)),
    }
}

/// Carries every body's current intent out, one push at a time. A new intent first
/// clears the queue; what is already in flight lands, since the mechanism holds that, and
/// the next action goes on only once the queue is empty. A short is done when its action
/// has been pushed, the queue is empty again and nothing is in flight.
fn drive(tick: Res<Tick>, mut bodies: Query<(&mut Brainstem, &mut ActionQueue, &Progress)>) {
    for (mut body, mut queue, progress) in &mut bodies {
        if let Some(pending) = body.take_pending() {
            queue.clear();
            body.begin(pending, tick.0);
        }
        let Some(doing) = body.doing else {
            continue;
        };
        match doing.intent {
            Intent::Short(short) => {
                if !body.pushed {
                    if let Some(action) = action_for(short) {
                        queue.push(action);
                    }
                    body.pushed = true;
                } else if queue.is_empty() && !progress.in_flight() {
                    body.finish(Outcome::Done);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use murabito_actions::ActionsPlugin;
    use murabito_hexcoords::VoxelCoord;
    use murabito_movement::{Locomotion, MovementPlugin};
    use murabito_placement::{Facing, VoxelPosition};
    use murabito_progress::ProgressPlugin;

    const E: Direction = Direction::E;
    const N: Direction = Direction::N;

    fn voxel(q: i32, r: i32) -> VoxelCoord {
        VoxelCoord::new(q, r, -q - r, 0).expect("on the plane")
    }

    /// A ticking app with one body at the origin, facing east, walking 4 shaku a second
    /// and turning 180 degrees a second: a face neighbour is 16 ticks, a quarter turn 32.
    fn body() -> (App, Entity) {
        use bevy::time::TimeUpdateStrategy;

        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            ProgressPlugin,
            MovementPlugin,
            ActionsPlugin,
            BrainstemPlugin,
        ));
        app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
        app.update();
        let body = app
            .world_mut()
            .spawn((
                VoxelPosition(voxel(0, 0)),
                Facing(E),
                Locomotion {
                    speed: 4.0,
                    turn_speed: 180.0,
                },
                ActionQueue::default(),
                Brainstem::default(),
            ))
            .id();
        (app, body)
    }

    fn tick(app: &mut App, times: usize) {
        for _ in 0..times {
            app.update();
        }
    }

    fn order(app: &mut App, body: Entity, intent: Intent) {
        app.world_mut()
            .get_mut::<Brainstem>(body)
            .unwrap()
            .order(intent);
    }

    fn brainstem(app: &App, body: Entity) -> Brainstem {
        app.world().get::<Brainstem>(body).unwrap().clone()
    }

    fn facing(app: &App, body: Entity) -> Direction {
        app.world().get::<Facing>(body).unwrap().0
    }

    fn position(app: &App, body: Entity) -> VoxelCoord {
        app.world().get::<VoxelPosition>(body).unwrap().0
    }

    fn queued(app: &App, body: Entity) -> usize {
        app.world().get::<ActionQueue>(body).unwrap().len()
    }

    // -- the component alone --------------------------------------------------------

    #[test]
    fn a_fresh_brainstem_does_nothing_and_has_no_outcome_yet() {
        let body = Brainstem::default();
        assert_eq!(body.doing(), None);
        assert_eq!(body.outcome(), Outcome::Idle);
    }

    #[test]
    fn an_order_begins_on_the_tick_drive_takes_it() {
        let mut body = Brainstem::default();
        body.order(Intent::Short(Short::Face(N)));
        assert_eq!(body.doing(), None, "not until drive takes it");
        let pending = body.take_pending().unwrap();
        body.begin(pending, 7);
        let doing = body.doing().unwrap();
        assert_eq!(doing.intent(), Intent::Short(Short::Face(N)));
        assert_eq!(doing.since(), 7);
        assert_eq!(body.outcome(), Outcome::Idle, "nothing ended");
    }

    #[test]
    fn a_newer_order_supersedes_and_a_stop_stops() {
        let mut body = Brainstem::default();
        body.order(Intent::Short(Short::Step(E)));
        let pending = body.take_pending().unwrap();
        body.begin(pending, 1);

        body.order(Intent::Short(Short::Face(N)));
        let pending = body.take_pending().unwrap();
        body.begin(pending, 2);
        assert_eq!(body.outcome(), Outcome::Superseded);
        assert_eq!(
            body.doing().unwrap().intent(),
            Intent::Short(Short::Face(N))
        );

        body.order(Intent::Short(Short::Stop));
        let pending = body.take_pending().unwrap();
        body.begin(pending, 3);
        assert_eq!(body.outcome(), Outcome::Stopped);
        assert_eq!(body.doing(), None);
    }

    #[test]
    fn a_reflex_cancels_by_name() {
        let mut body = Brainstem::default();
        body.order(Intent::Short(Short::Step(E)));
        let pending = body.take_pending().unwrap();
        body.begin(pending, 1);

        body.preempt(Short::Face(N), "startle_face_apparition");
        let pending = body.take_pending().unwrap();
        body.begin(pending, 2);
        assert_eq!(
            body.outcome(),
            Outcome::Cancelled("startle_face_apparition")
        );
        assert_eq!(
            body.doing().unwrap().intent(),
            Intent::Short(Short::Face(N))
        );
    }

    #[test]
    fn the_later_of_two_orders_in_one_tick_is_the_one_taken() {
        let mut body = Brainstem::default();
        body.order(Intent::Short(Short::Face(N)));
        body.order(Intent::Short(Short::Step(E)));
        let pending = body.take_pending().unwrap();
        assert_eq!(pending.intent, Intent::Short(Short::Step(E)));
        assert_eq!(body.take_pending(), None);
    }

    #[test]
    fn a_face_is_a_face_a_step_a_go_and_a_stop_nothing() {
        assert_eq!(action_for(Short::Face(N)), Some(Action::Face(N)));
        assert_eq!(action_for(Short::Step(E)), Some(Action::Go(E)));
        assert_eq!(action_for(Short::Stop), None);
    }

    // -- in the tick ----------------------------------------------------------------

    #[test]
    fn the_tick_is_counted() {
        let (mut app, _) = body();
        assert_eq!(
            app.world().resource::<Tick>().0,
            0,
            "the first update only starts the clock"
        );
        tick(&mut app, 3);
        assert_eq!(app.world().resource::<Tick>().0, 3);
    }

    #[test]
    fn with_nothing_ordered_a_body_stands_there() {
        let (mut app, body) = body();
        tick(&mut app, 10);
        assert_eq!(queued(&app, body), 0);
        assert_eq!(position(&app, body), voxel(0, 0));
        assert_eq!(brainstem(&app, body).outcome(), Outcome::Idle);
    }

    #[test]
    fn a_face_order_is_pushed_on_its_tick_lands_on_tick_thirty_two_and_is_done_on_the_next() {
        let (mut app, body) = body();
        order(&mut app, body, Intent::Short(Short::Face(N)));

        tick(&mut app, 1);
        let doing = brainstem(&app, body).doing().expect("begun");
        assert_eq!(doing.since(), 1);
        assert_eq!(
            queued(&app, body),
            0,
            "pushed in drive, issued the same tick"
        );
        assert_eq!(facing(&app, body), E);

        tick(&mut app, 31);
        assert_eq!(
            facing(&app, body),
            N,
            "a quarter turn at 180 degrees a second is 32 ticks"
        );
        assert!(
            brainstem(&app, body).doing().is_some(),
            "drive saw it still in flight"
        );

        tick(&mut app, 1);
        assert_eq!(brainstem(&app, body).doing(), None);
        assert_eq!(brainstem(&app, body).outcome(), Outcome::Done);
    }

    #[test]
    fn a_step_order_moves_the_body_one_cell_and_is_done() {
        let (mut app, body) = body();
        order(&mut app, body, Intent::Short(Short::Step(E)));
        tick(&mut app, 16);
        assert_eq!(
            position(&app, body),
            voxel(1, 0),
            "one shaku at four a second is 16 ticks"
        );
        tick(&mut app, 1);
        assert_eq!(brainstem(&app, body).outcome(), Outcome::Done);
        tick(&mut app, 10);
        assert_eq!(position(&app, body), voxel(1, 0), "and then nothing more");
    }

    #[test]
    fn a_newer_order_supersedes_but_the_step_in_flight_still_lands() {
        let (mut app, body) = body();
        order(&mut app, body, Intent::Short(Short::Step(E)));
        tick(&mut app, 1);
        order(&mut app, body, Intent::Short(Short::Face(N)));
        tick(&mut app, 1);
        assert_eq!(brainstem(&app, body).outcome(), Outcome::Superseded);
        assert_eq!(
            brainstem(&app, body).doing().unwrap().intent(),
            Intent::Short(Short::Face(N))
        );
        assert_eq!(
            queued(&app, body),
            1,
            "the face waits behind the step in flight"
        );

        tick(&mut app, 15);
        assert_eq!(position(&app, body), voxel(1, 0), "the step landed anyway");
        tick(&mut app, 33);
        assert_eq!(facing(&app, body), N);
        assert_eq!(brainstem(&app, body).outcome(), Outcome::Done);
    }

    #[test]
    fn a_stop_empties_the_body_and_says_so() {
        let (mut app, body) = body();
        order(&mut app, body, Intent::Short(Short::Step(E)));
        tick(&mut app, 1);
        order(&mut app, body, Intent::Short(Short::Stop));
        tick(&mut app, 1);
        assert_eq!(brainstem(&app, body).doing(), None);
        assert_eq!(brainstem(&app, body).outcome(), Outcome::Stopped);
        tick(&mut app, 40);
        assert_eq!(
            position(&app, body),
            voxel(1, 0),
            "what was in flight landed"
        );
        assert_eq!(
            brainstem(&app, body).outcome(),
            Outcome::Stopped,
            "and nothing else happened"
        );
    }
}

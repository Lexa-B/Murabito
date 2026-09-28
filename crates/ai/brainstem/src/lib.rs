//! The brainstem: in the driver's seat, not in charge.
//!
//! Every sentient body has one. It runs every tick, holds one intent at a time, and is
//! the only thing that pushes onto the body's `ActionQueue`. It is dumb on purpose: with
//! nothing to do it stands there, since a rest is an empty queue. What to do comes from
//! elsewhere, in the vocabulary this crate owns:
//!
//! - a [`Short`] finishes within a round of the slower mind: one turn, one step, a stop.
//!   A reflex may only ever say one of these.
//! - a [`Path`] outlives rounds: steps to take in order, queued as written. Drive plans
//!   nothing; whoever wrote the path re-plans as the world moves and amends it, keeping
//!   the steps still right and replacing the rest, and the step in flight is never cut
//!   by a path, only by a short.
//!
//! Anything may [`Brainstem::order`] an intent, and a reflex may [`Brainstem::preempt`]
//! one; the systems here carry the current intent out and record how the last one ended
//! in an [`Outcome`], which is how the mind that asked learns its order was dropped, or
//! that the thing it named was no longer in view.
//!
//! The tick, inside `AskingSet`, in [`BrainstemSet`]'s order: **Orders** takes what
//! arrived; **Reflexes** is the slot the reflexes crate fills, so a fright beats an order
//! from the same tick; **Drive** takes the body for a new intent, clearing the queue and
//! cutting short whatever is in flight, so the new one starts this tick, or amends the
//! path in hand; after that it pushes a short's one action only when the body is idle
//! and ends a path once its last step has landed. After the senses, **publish** writes every body's [`Snapshot`] to the
//! [`Port`]'s board, and orders arrive through the same port's channel: that is the
//! whole seam between a body and its mind. Design: `docs/ai_readme.md`.

use bevy::prelude::*;
use murabito_actions::{Action, ActionQueue, AskingSet, CutShort};
use murabito_hexcoords::Direction;
use murabito_identity::ThingId;
use murabito_perception::PerceptionSet;
use murabito_placement::{Facing, VoxelPosition};
use murabito_progress::Progress;
use murabito_vision::Seen;

mod port;

pub use port::{Board, InView, Order, Orders, Port, Snapshot};

pub struct BrainstemPlugin;

impl Plugin for BrainstemPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Tick>()
            .init_resource::<Port>()
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
                    (count_tick, port::take_orders)
                        .chain()
                        .in_set(BrainstemSet::Orders),
                    drive.in_set(BrainstemSet::Drive),
                    port::publish.after(PerceptionSet::Sense),
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
    /// Turn to face that thing, where it is seen now. `Lost` if it isn't in view.
    FaceThing(ThingId),
    /// One cell that way at walking pace, turning first if need be.
    Walk(Direction),
    /// A walk at twice the pace.
    Jog(Direction),
    /// A walk at three times the pace.
    Sprint(Direction),
    /// A walk at half the pace.
    Sneak(Direction),
    /// One cell sideways, without turning. `Refused` unless the way is lateral.
    Sidestep(Direction),
    /// One cell back, without turning. `Refused` unless the way is rear.
    Backstep(Direction),
    /// A backstep at a jog. `Refused` unless the way is rear.
    Recoil(Direction),
    /// Two cells forward at a sprint, in one go. `Refused` unless the way is forward.
    Lunge(Direction),
    /// Bite whatever is in the cell faced. Bites nothing yet.
    Bite,
}

/// What a body can be told that outlives rounds: steps to take in order, each a pace
/// word or a turn, walked as written. Drive plans none of it. A path arriving while the
/// body is on one amends it: the first `keep` actions still waiting on the queue stay,
/// the rest are dropped, and these steps go on after them; the step in flight is never
/// touched. Done when the last step has landed. A path carrying any other word is
/// refused whole.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Path {
    /// How many of the actions still waiting on the queue to keep ahead of `steps`.
    /// Means nothing unless the body is on a path already.
    pub keep: usize,
    pub steps: Vec<Action>,
}

impl Path {
    pub fn new(keep: usize, steps: Vec<Action>) -> Self {
        Self { keep, steps }
    }

    /// A path from a standstill: nothing kept.
    pub fn fresh(steps: Vec<Action>) -> Self {
        Self::new(0, steps)
    }

    /// Whether every step is a pace word or a turn; the reason if one is not.
    fn check(&self) -> Result<(), &'static str> {
        let is_step = |action: &Action| {
            matches!(
                action,
                Action::Walk(_)
                    | Action::Jog(_)
                    | Action::Sprint(_)
                    | Action::Sneak(_)
                    | Action::Face(_)
            )
        };
        if self.steps.iter().all(is_step) {
            Ok(())
        } else {
            Err("a path may only step and turn")
        }
    }
}

/// Everything a body can be told.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    Short(Short),
    Path(Path),
}

/// How an intent ended, for whoever gave it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// It was carried out.
    Done,
    /// A stop replaced it.
    Stopped,
    /// A newer order replaced it.
    Superseded,
    /// A reflex, named, replaced it.
    Cancelled(&'static str),
    /// The thing it named was not in view when the body went to act on it.
    Lost(ThingId),
    /// The body could not be asked it the way it faced, and the reason why.
    Refused(&'static str),
}

/// What the body did last, and how it ended: the record a mind reads to learn what
/// became of its order, and of what was done in its place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Previous {
    intent: Intent,
    outcome: Outcome,
}

impl Previous {
    pub fn new(intent: Intent, outcome: Outcome) -> Self {
        Self { intent, outcome }
    }

    pub fn intent(&self) -> Intent {
        self.intent.clone()
    }

    pub fn outcome(&self) -> Outcome {
        self.outcome
    }
}

/// What the body is doing now, and since which tick. On a path, the intent is the path
/// as last written: the amendment's `keep` and steps, not the whole walk.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Doing {
    intent: Intent,
    since: u64,
}

impl Doing {
    pub fn intent(&self) -> Intent {
        self.intent.clone()
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

#[derive(Clone, Debug, PartialEq, Eq)]
struct Pending {
    intent: Intent,
    cause: Cause,
}

/// A body's driver: one intent at a time, and what the last one was and how it ended.
/// Every sentient has one; anything may order it, a reflex may preempt it, and only the
/// systems here read it out onto the queue.
#[derive(Component, Debug, Default, Clone, PartialEq, Eq)]
pub struct Brainstem {
    /// Given, not yet begun. Drive takes it at the start of the next tick; two in one
    /// tick and the later wins.
    pending: Option<Pending>,
    doing: Option<Doing>,
    /// None until something has been asked.
    previous: Option<Previous>,
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
        self.doing.clone()
    }

    /// What the body did last and how it ended; nothing until something has been asked.
    pub fn previous(&self) -> Option<Previous> {
        self.previous.clone()
    }

    /// Whether a new intent waits for drive to take it up.
    fn take_pending(&mut self) -> Option<Pending> {
        self.pending.take()
    }

    /// Whether a pending intent amends the path in hand rather than replacing it: an
    /// ordered path, while a path is being done.
    fn amends(&self, pending: &Pending) -> bool {
        pending.cause == Cause::Order
            && matches!(pending.intent, Intent::Path(_))
            && matches!(
                self.doing,
                Some(Doing {
                    intent: Intent::Path(_),
                    ..
                })
            )
    }

    /// The pending intent becomes what is done, and what was done gets its outcome. Pure
    /// bookkeeping: the queue is drive's to clear.
    fn begin(&mut self, pending: Pending, now: u64) {
        let is_stop = pending.intent == Intent::Short(Short::Stop);
        if let Some(doing) = self.doing.take() {
            let outcome = match (pending.cause, is_stop) {
                (Cause::Reflex(name), _) => Outcome::Cancelled(name),
                (Cause::Order, true) => Outcome::Stopped,
                (Cause::Order, false) => Outcome::Superseded,
            };
            self.previous = Some(Previous::new(doing.intent, outcome));
        }
        self.doing = (!is_stop).then_some(Doing {
            intent: pending.intent,
            since: now,
        });
        self.pushed = false;
    }

    /// The path in hand is rewritten as the amendment says, and goes on since when it
    /// began: nothing ended.
    fn amend(&mut self, path: Path) {
        if let Some(doing) = self.doing.as_mut() {
            doing.intent = Intent::Path(path);
        }
    }

    fn finish(&mut self, ended: Outcome) {
        if let Some(doing) = self.doing.take() {
            self.previous = Some(Previous::new(doing.intent, ended));
        }
        self.doing = None;
        self.pushed = false;
    }
}

/// What a short motion comes to, once the body's view is consulted: an action to push,
/// or an end.
#[derive(Debug, PartialEq, Eq)]
enum Resolved {
    Push(Action),
    Finish(Outcome),
}

/// The one action a short is. A stop is none and never reaches here; facing a thing is a
/// turn toward where it is seen, done already if it shares the cell, lost if it is not
/// in view; a word the body cannot be asked the way it faces is refused, with the
/// reason, before it ever reaches the queue.
fn resolve_short(short: Short, facing: Direction, seen: Option<&Seen>) -> Resolved {
    let action = match short {
        Short::Stop => return Resolved::Finish(Outcome::Done),
        Short::Face(direction) => Action::Face(direction),
        Short::FaceThing(id) => {
            let sighting = seen.and_then(|seen| seen.iter().find(|sighting| sighting.id == id));
            return match sighting.map(|sighting| sighting.offset.bearing()) {
                None => Resolved::Finish(Outcome::Lost(id)),
                Some(None) => Resolved::Finish(Outcome::Done),
                Some(Some(direction)) => Resolved::Push(Action::Face(direction)),
            };
        }
        Short::Walk(direction) => Action::Walk(direction),
        Short::Jog(direction) => Action::Jog(direction),
        Short::Sprint(direction) => Action::Sprint(direction),
        Short::Sneak(direction) => Action::Sneak(direction),
        Short::Sidestep(direction) => Action::Sidestep(direction),
        Short::Backstep(direction) => Action::Backstep(direction),
        Short::Recoil(direction) => Action::Recoil(direction),
        Short::Lunge(direction) => Action::Lunge(direction),
        Short::Bite => Action::Bite,
    };
    match action.check(facing) {
        Ok(()) => Resolved::Push(action),
        Err(wrong) => Resolved::Finish(Outcome::Refused(wrong.reason())),
    }
}

/// Carries every body's current intent out. A new intent takes the body now: the queue
/// is cleared and whatever is in flight is cut short, so the new intent's first action
/// is issued this same tick; a path over a path instead amends the queue and cuts
/// nothing. After that a short's one action goes on only once the queue is empty and
/// the bar is idle, and the short is done when the body is idle again; a path's steps
/// are all queued as it begins, and it is done when the body is idle again.
/// Everything of a body that drive touches. The id and facing are only for the trace.
type Driven<'a> = (
    Entity,
    Option<&'a ThingId>,
    &'a mut Brainstem,
    &'a mut ActionQueue,
    &'a Progress,
    &'a VoxelPosition,
    &'a Facing,
    Option<&'a Seen>,
);

fn drive(tick: Res<Tick>, mut commands: Commands, mut bodies: Query<Driven>) {
    for (entity, id, mut body, mut queue, progress, position, facing, seen) in &mut bodies {
        let who = id.map_or_else(|| "a body".to_owned(), ThingId::to_string);
        if let Some(pending) = body.take_pending() {
            if body.amends(&pending) {
                let Intent::Path(path) = pending.intent else {
                    unreachable!("amends only a path");
                };
                debug!(
                    "tick {}: {who} at {:?} keeps {} of {} queued and takes {} more steps",
                    tick.0,
                    position.0,
                    path.keep.min(queue.len()),
                    queue.len(),
                    path.steps.len()
                );
                queue.truncate(path.keep);
                take_up(&mut body, &mut queue, path);
            } else {
                queue.clear();
                if progress.in_flight() {
                    commands.entity(entity).insert(CutShort);
                }
                let is_path = matches!(pending.intent, Intent::Path(_));
                body.begin(pending, tick.0);
                if is_path {
                    let Some(Intent::Path(path)) = body.doing().map(|doing| doing.intent()) else {
                        unreachable!("a path was begun");
                    };
                    debug!(
                        "tick {}: {who} at {:?} sets off on {} steps",
                        tick.0,
                        position.0,
                        path.steps.len()
                    );
                    take_up(&mut body, &mut queue, path);
                }
            }
        }
        let Some(doing) = body.doing() else {
            continue;
        };
        let idle = queue.is_empty() && !progress.in_flight();
        match doing.intent {
            Intent::Short(short) if !body.pushed => {
                match resolve_short(short, facing.0, seen) {
                    Resolved::Push(action) => {
                        debug!(
                            "tick {}: {who} facing {:?} at {:?} pushes {action:?} for {short:?}",
                            tick.0, facing.0, position.0
                        );
                        queue.push(action);
                    }
                    Resolved::Finish(ended) => {
                        debug!("tick {}: {who} ends {short:?} as {ended:?}", tick.0);
                        body.finish(ended);
                    }
                }
                body.pushed = true;
            }
            Intent::Short(_) if idle => body.finish(Outcome::Done),
            Intent::Short(_) => {}
            Intent::Path(_) if idle => body.finish(Outcome::Done),
            Intent::Path(_) => {}
        }
    }
}

/// A path's steps go on the queue, or the whole path is refused if one is not a step.
/// Amending and beginning alike: the path in hand becomes this one.
fn take_up(body: &mut Brainstem, queue: &mut ActionQueue, path: Path) {
    if let Err(why) = path.check() {
        queue.clear();
        body.amend(path);
        body.finish(Outcome::Refused(why));
        return;
    }
    for step in &path.steps {
        queue.push(*step);
    }
    body.amend(path);
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use murabito_actions::ActionsPlugin;
    use murabito_attacks::AttacksPlugin;
    use murabito_hexcoords::VoxelCoord;
    use murabito_identity::{IdentityPlugin, NextThingId};
    use murabito_movement::{Locomotion, MovementPlugin};
    use murabito_perception::PerceptionPlugin;
    use murabito_placement::Facing;
    use murabito_progress::ProgressPlugin;
    use murabito_vision::{Band, Vision, VisionPlugin};

    pub(crate) const E: Direction = Direction::E;
    pub(crate) const N: Direction = Direction::N;
    const ENE: Direction = Direction::ENE;

    /// Eyes all round, sharp to 8 cells, then 16, then 24.
    pub(crate) const ALL_ROUND: Vision = Vision {
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

    pub(crate) fn voxel(q: i32, r: i32) -> VoxelCoord {
        VoxelCoord::new(q, r, -q - r, 0).expect("on the plane")
    }

    fn short(short: Short) -> Intent {
        Intent::Short(short)
    }

    /// A straight walk from the origin to a cell, as the steps a mind would send.
    fn walk_to(q: i32, r: i32) -> Intent {
        path_at(Action::Walk, voxel(0, 0), voxel(q, r))
    }

    fn path_at(word: fn(Direction) -> Action, from: VoxelCoord, to: VoxelCoord) -> Intent {
        Intent::Path(Path::fresh(
            from.straight_to(to).into_iter().map(word).collect(),
        ))
    }

    /// A ticking app with everything a body needs to look, be told, and move.
    pub(crate) fn app() -> App {
        use bevy::time::TimeUpdateStrategy;

        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            IdentityPlugin,
            ProgressPlugin,
            MovementPlugin,
            AttacksPlugin,
            ActionsPlugin,
            PerceptionPlugin,
            VisionPlugin,
            BrainstemPlugin,
        ));
        app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
        app.update();
        app
    }

    pub(crate) fn mint(app: &mut App) -> ThingId {
        app.world_mut().resource_mut::<NextThingId>().mint()
    }

    /// A body at the origin facing east, walking 4 shaku a second and turning 180
    /// degrees a second: a face neighbour is 16 ticks, a corner one 28, a quarter turn 32.
    fn body() -> (App, Entity) {
        let mut app = app();
        let id = mint(&mut app);
        let body = app
            .world_mut()
            .spawn((
                id,
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
        (app, body)
    }

    /// Something to look at, standing still.
    fn thing_at(app: &mut App, cell: VoxelCoord) -> ThingId {
        let id = mint(app);
        app.world_mut().spawn((id, VoxelPosition(cell)));
        id
    }

    pub(crate) fn tick(app: &mut App, times: usize) {
        for _ in 0..times {
            app.update();
        }
    }

    pub(crate) fn order(app: &mut App, body: Entity, intent: Intent) {
        app.world_mut()
            .get_mut::<Brainstem>(body)
            .unwrap()
            .order(intent);
    }

    pub(crate) fn brainstem(app: &App, body: Entity) -> Brainstem {
        app.world().get::<Brainstem>(body).unwrap().clone()
    }

    /// How the body's last intent ended, if anything has been asked.
    pub(crate) fn previous_outcome(app: &App, body: Entity) -> Option<Outcome> {
        brainstem(app, body).previous().map(|p| p.outcome())
    }

    pub(crate) fn facing(app: &App, body: Entity) -> Direction {
        app.world().get::<Facing>(body).unwrap().0
    }

    pub(crate) fn position(app: &App, body: Entity) -> VoxelCoord {
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
        assert_eq!(body.previous(), None);
    }

    #[test]
    fn an_order_begins_on_the_tick_drive_takes_it() {
        let mut body = Brainstem::default();
        body.order(short(Short::Face(N)));
        assert_eq!(body.doing(), None, "not until drive takes it");
        let pending = body.take_pending().unwrap();
        body.begin(pending, 7);
        let doing = body.doing().unwrap();
        assert_eq!(doing.intent(), short(Short::Face(N)));
        assert_eq!(doing.since(), 7);
        assert_eq!(body.previous(), None, "nothing ended");
    }

    #[test]
    fn a_newer_order_supersedes_and_a_stop_stops() {
        let mut body = Brainstem::default();
        body.order(short(Short::Walk(E)));
        let pending = body.take_pending().unwrap();
        body.begin(pending, 1);

        body.order(short(Short::Face(N)));
        let pending = body.take_pending().unwrap();
        body.begin(pending, 2);
        let previous = body.previous().expect("something ended");
        assert_eq!(previous.outcome(), Outcome::Superseded);
        assert_eq!(previous.intent(), short(Short::Walk(E)), "and it says what");
        assert_eq!(body.doing().unwrap().intent(), short(Short::Face(N)));

        body.order(short(Short::Stop));
        let pending = body.take_pending().unwrap();
        body.begin(pending, 3);
        assert_eq!(body.previous().map(|p| p.outcome()), Some(Outcome::Stopped));
        assert_eq!(body.doing(), None);
    }

    #[test]
    fn a_reflex_cancels_by_name() {
        let mut body = Brainstem::default();
        body.order(short(Short::Walk(E)));
        let pending = body.take_pending().unwrap();
        body.begin(pending, 1);

        body.preempt(Short::Face(N), "startle_face_apparition");
        let pending = body.take_pending().unwrap();
        body.begin(pending, 2);
        assert_eq!(
            body.previous().map(|p| p.outcome()),
            Some(Outcome::Cancelled("startle_face_apparition"))
        );
        assert_eq!(body.doing().unwrap().intent(), short(Short::Face(N)));
    }

    #[test]
    fn the_later_of_two_orders_in_one_tick_is_the_one_taken() {
        let mut body = Brainstem::default();
        body.order(short(Short::Face(N)));
        body.order(short(Short::Walk(E)));
        let pending = body.take_pending().unwrap();
        assert_eq!(pending.intent, short(Short::Walk(E)));
        assert_eq!(body.take_pending(), None);
    }

    // -- the pure parts -------------------------------------------------------------

    #[test]
    fn each_word_is_its_action() {
        use Direction::W;
        let words = [
            (Short::Face(N), Action::Face(N)),
            (Short::Walk(E), Action::Walk(E)),
            (Short::Jog(W), Action::Jog(W)),
            (Short::Sprint(E), Action::Sprint(E)),
            (Short::Sneak(E), Action::Sneak(E)),
            (Short::Sidestep(N), Action::Sidestep(N)),
            (Short::Backstep(W), Action::Backstep(W)),
            (Short::Recoil(W), Action::Recoil(W)),
            (Short::Lunge(E), Action::Lunge(E)),
            (Short::Bite, Action::Bite),
        ];
        for (word, action) in words {
            assert_eq!(
                resolve_short(word, E, None),
                Resolved::Push(action),
                "{word:?}"
            );
        }
    }

    #[test]
    fn a_way_word_the_wrong_way_is_refused_before_it_reaches_the_queue() {
        use Direction::W;
        assert_eq!(
            resolve_short(Short::Sidestep(E), E, None),
            Resolved::Finish(Outcome::Refused("not lateral"))
        );
        assert_eq!(
            resolve_short(Short::Recoil(E), E, None),
            Resolved::Finish(Outcome::Refused("not rear"))
        );
        assert_eq!(
            resolve_short(Short::Lunge(W), E, None),
            Resolved::Finish(Outcome::Refused("not forward"))
        );
    }

    #[test]
    fn facing_a_thing_with_nothing_in_view_is_lost() {
        let id = NextThingId::default().mint();
        assert_eq!(
            resolve_short(Short::FaceThing(id), E, None),
            Resolved::Finish(Outcome::Lost(id))
        );
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
    fn a_refused_order_ends_on_its_tick_and_the_body_never_moves() {
        let (mut app, body) = body();
        order(&mut app, body, short(Short::Sidestep(E)));

        tick(&mut app, 1);
        assert_eq!(brainstem(&app, body).doing(), None);
        assert_eq!(
            previous_outcome(&app, body),
            Some(Outcome::Refused("not lateral"))
        );
        assert_eq!(queued(&app, body), 0);

        tick(&mut app, 30);
        assert_eq!(position(&app, body), voxel(0, 0));
    }

    #[test]
    fn a_recoil_order_lands_the_body_one_back_in_eight_ticks_still_facing_east() {
        use Direction::W;
        let (mut app, body) = body();
        order(&mut app, body, short(Short::Recoil(W)));

        tick(&mut app, 8);
        assert_eq!(position(&app, body), voxel(-1, 0));
        assert_eq!(facing(&app, body), E);
        tick(&mut app, 1);
        assert_eq!(previous_outcome(&app, body), Some(Outcome::Done));
    }

    #[test]
    fn with_nothing_ordered_a_body_stands_there() {
        let (mut app, body) = body();
        tick(&mut app, 10);
        assert_eq!(queued(&app, body), 0);
        assert_eq!(position(&app, body), voxel(0, 0));
        assert_eq!(previous_outcome(&app, body), None);
    }

    #[test]
    fn a_face_order_is_pushed_on_its_tick_lands_on_tick_thirty_two_and_is_done_on_the_next() {
        let (mut app, body) = body();
        order(&mut app, body, short(Short::Face(N)));

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
            "drive ran before the last notch landed"
        );

        tick(&mut app, 1);
        assert_eq!(brainstem(&app, body).doing(), None);
        assert_eq!(previous_outcome(&app, body), Some(Outcome::Done));
    }

    #[test]
    fn a_step_order_moves_the_body_one_cell_and_is_done() {
        let (mut app, body) = body();
        order(&mut app, body, short(Short::Walk(E)));
        tick(&mut app, 16);
        assert_eq!(
            position(&app, body),
            voxel(1, 0),
            "one shaku at four a second is 16 ticks"
        );
        tick(&mut app, 1);
        assert_eq!(previous_outcome(&app, body), Some(Outcome::Done));
        tick(&mut app, 10);
        assert_eq!(position(&app, body), voxel(1, 0), "and then nothing more");
    }

    #[test]
    fn a_newer_order_cuts_the_step_in_flight_and_begins_on_that_tick() {
        let (mut app, body) = body();
        order(&mut app, body, short(Short::Walk(E)));
        tick(&mut app, 1);
        order(&mut app, body, short(Short::Face(N)));
        tick(&mut app, 1);
        assert_eq!(previous_outcome(&app, body), Some(Outcome::Superseded));
        assert_eq!(
            brainstem(&app, body).doing().unwrap().intent(),
            short(Short::Face(N))
        );
        assert_eq!(queued(&app, body), 0, "the face was issued this tick");

        tick(&mut app, 31);
        assert_eq!(facing(&app, body), N, "32 ticks from the order");
        assert_eq!(position(&app, body), voxel(0, 0), "the step never landed");
        tick(&mut app, 1);
        assert_eq!(previous_outcome(&app, body), Some(Outcome::Done));
    }

    #[test]
    fn a_stop_empties_the_body_and_says_so() {
        let (mut app, body) = body();
        order(&mut app, body, short(Short::Walk(E)));
        tick(&mut app, 1);
        order(&mut app, body, short(Short::Stop));
        tick(&mut app, 1);
        assert_eq!(brainstem(&app, body).doing(), None);
        assert_eq!(previous_outcome(&app, body), Some(Outcome::Stopped));
        tick(&mut app, 40);
        assert_eq!(
            position(&app, body),
            voxel(0, 0),
            "stop means stop: the step was cut"
        );
        assert_eq!(
            previous_outcome(&app, body),
            Some(Outcome::Stopped),
            "and nothing else happened"
        );
    }

    #[test]
    fn facing_a_thing_turns_toward_where_it_is_seen() {
        let (mut app, body) = body();
        let thing = thing_at(&mut app, voxel(2, -4));
        tick(&mut app, 1); // a look, so the thing is in view
        order(&mut app, body, short(Short::FaceThing(thing)));
        tick(&mut app, 32);
        assert_eq!(facing(&app, body), N, "two cells north of the eye");
        tick(&mut app, 1);
        assert_eq!(previous_outcome(&app, body), Some(Outcome::Done));
    }

    #[test]
    fn facing_a_thing_that_is_not_in_view_is_lost_and_names_it() {
        let (mut app, body) = body();
        let unseen = thing_at(&mut app, voxel(30, 0));
        tick(&mut app, 1);
        order(&mut app, body, short(Short::FaceThing(unseen)));
        tick(&mut app, 1);
        assert_eq!(brainstem(&app, body).doing(), None);
        assert_eq!(previous_outcome(&app, body), Some(Outcome::Lost(unseen)));
        assert_eq!(facing(&app, body), E, "and the body did not move");
    }

    #[test]
    fn facing_a_thing_before_the_body_has_looked_is_lost_since_the_view_is_a_tick_old() {
        let (mut app, body) = body();
        let thing = thing_at(&mut app, voxel(2, -4));
        order(&mut app, body, short(Short::FaceThing(thing)));
        tick(&mut app, 1);
        assert_eq!(previous_outcome(&app, body), Some(Outcome::Lost(thing)));
    }

    #[test]
    fn going_to_a_cell_three_east_is_three_steps_and_done_on_arrival() {
        let (mut app, body) = body();
        order(&mut app, body, walk_to(3, 0));
        tick(&mut app, 16);
        assert_eq!(position(&app, body), voxel(1, 0));
        assert!(brainstem(&app, body).doing().is_some(), "not there yet");
        tick(&mut app, 32);
        assert_eq!(position(&app, body), voxel(3, 0), "three steps of 16 ticks");
        assert!(
            brainstem(&app, body).doing().is_some(),
            "drive ran before the last step landed"
        );
        tick(&mut app, 1);
        assert_eq!(previous_outcome(&app, body), Some(Outcome::Done));
        tick(&mut app, 10);
        assert_eq!(position(&app, body), voxel(3, 0), "and stays");
    }

    #[test]
    fn going_to_a_corner_neighbour_is_one_corner_step_of_twenty_eight_ticks() {
        let (mut app, body) = body();
        order(&mut app, body, walk_to(2, -1));
        tick(&mut app, 27);
        assert_eq!(
            position(&app, body),
            voxel(0, 0),
            "root three shaku at four a second is 27.7 ticks"
        );
        tick(&mut app, 1);
        assert_eq!(position(&app, body), voxel(2, -1));
        assert_eq!(facing(&app, body), ENE);
    }

    #[test]
    fn the_overshoot_of_one_pushed_step_carries_into_the_next() {
        // At five shaku a second a corner step is 22.17 ticks. Three of them, each pushed
        // by drive only once the last has landed, still carry their leftovers: the third
        // lands on tick 67, where three separate 23-tick steps would land on 69. The body
        // faces north first, so no turn is in the count.
        let (mut app, body) = body();
        app.world_mut().get_mut::<Locomotion>(body).unwrap().speed = 5.0;
        app.world_mut().get_mut::<Facing>(body).unwrap().0 = N;
        order(&mut app, body, walk_to(3, -6));
        tick(&mut app, 66);
        assert_eq!(
            position(&app, body),
            voxel(2, -4),
            "two corner steps north, so far"
        );
        tick(&mut app, 1);
        assert_eq!(
            position(&app, body),
            voxel(3, -6),
            "the third lands on tick 67"
        );
    }

    #[test]
    fn going_to_the_cell_the_body_stands_on_is_done_at_once() {
        let (mut app, body) = body();
        order(&mut app, body, walk_to(0, 0));
        tick(&mut app, 1);
        assert_eq!(previous_outcome(&app, body), Some(Outcome::Done));
        assert_eq!(queued(&app, body), 0);
    }

    #[test]
    fn a_path_may_only_step_and_turn_and_is_refused_whole_otherwise() {
        assert_eq!(
            Path::fresh(vec![Action::Sneak(E), Action::Face(N), Action::Jog(N)]).check(),
            Ok(())
        );
        assert_eq!(
            Path::fresh(vec![Action::Walk(E), Action::Bite]).check(),
            Err("a path may only step and turn")
        );
        let (mut app, body) = body();
        order(
            &mut app,
            body,
            Intent::Path(Path::fresh(vec![Action::Walk(E), Action::Lunge(E)])),
        );
        tick(&mut app, 1);
        assert_eq!(
            previous_outcome(&app, body),
            Some(Outcome::Refused("a path may only step and turn"))
        );
        assert_eq!(queued(&app, body), 0, "none of it was taken");
        tick(&mut app, 20);
        assert_eq!(position(&app, body), voxel(0, 0));
    }

    #[test]
    fn a_sneaking_path_creeps_there_at_half_pace_and_is_done() {
        // Two edge steps east at a sneak: 32 ticks each, 64 to stand there, done the tick after.
        let (mut app, body) = body();
        order(
            &mut app,
            body,
            path_at(Action::Sneak, voxel(0, 0), voxel(2, 0)),
        );

        tick(&mut app, 32);
        assert_eq!(
            position(&app, body),
            voxel(1, 0),
            "one sneaking step in 32 ticks"
        );
        tick(&mut app, 32);
        assert_eq!(position(&app, body), voxel(2, 0));
        tick(&mut app, 1);
        assert_eq!(previous_outcome(&app, body), Some(Outcome::Done));
    }

    #[test]
    fn a_path_amended_keeps_the_step_in_flight_and_goes_on_from_where_it_lands() {
        // Four steps east are ordered. With two landed and the third in flight, one step
        // waits on the queue. The amendment keeps none of it and adds a corner step, the
        // plan from where the third lands: that step is not cut, it lands on tick 48,
        // and the corner step follows from (3, 0) to (4, -1).
        let (mut app, body) = body();
        order(&mut app, body, walk_to(4, 0));
        tick(&mut app, 33);
        assert_eq!(position(&app, body), voxel(2, 0), "two steps of 16 ticks");
        assert_eq!(
            queued(&app, body),
            1,
            "the third is in flight, the fourth waits"
        );

        order(
            &mut app,
            body,
            Intent::Path(Path::new(0, vec![Action::Walk(Direction::NNE)])),
        );
        tick(&mut app, 15);
        assert_eq!(
            position(&app, body),
            voxel(3, 0),
            "the third step landed on tick 48"
        );
        let doing = brainstem(&app, body).doing().expect("still on the path");
        assert_eq!(doing.since(), 1, "the same path, since it began");
        assert_eq!(previous_outcome(&app, body), None, "nothing ended");
        tick(&mut app, 28);
        assert_eq!(
            position(&app, body),
            voxel(4, -1),
            "one corner step from there"
        );
        tick(&mut app, 1);
        assert_eq!(previous_outcome(&app, body), Some(Outcome::Done));
    }

    #[test]
    fn an_amendment_keeps_the_first_of_what_waits_and_drops_the_rest() {
        // Four steps east: with one landed and the second in flight, two wait. Keep one
        // and turn north after it: three steps east in all, then the turn.
        let (mut app, body) = body();
        order(&mut app, body, walk_to(4, 0));
        tick(&mut app, 17);
        assert_eq!(position(&app, body), voxel(1, 0));
        assert_eq!(queued(&app, body), 2);

        order(
            &mut app,
            body,
            Intent::Path(Path::new(1, vec![Action::Face(N)])),
        );
        tick(&mut app, 1);
        assert_eq!(queued(&app, body), 2, "one kept, one added");
        tick(&mut app, 31);
        assert_eq!(
            position(&app, body),
            voxel(3, 0),
            "the second landed on 32, the kept third on 48"
        );
        tick(&mut app, 32);
        assert_eq!(facing(&app, body), N, "and the turn after it");
        assert_eq!(position(&app, body), voxel(3, 0));
        tick(&mut app, 1);
        assert_eq!(previous_outcome(&app, body), Some(Outcome::Done));
    }

    #[test]
    fn a_fresh_path_over_a_short_cuts_it_and_supersedes_it() {
        let (mut app, body) = body();
        order(&mut app, body, short(Short::Face(N)));
        tick(&mut app, 8);
        order(&mut app, body, walk_to(1, 0));
        tick(&mut app, 1);
        assert_eq!(previous_outcome(&app, body), Some(Outcome::Superseded));
        tick(&mut app, 15);
        assert_eq!(
            position(&app, body),
            voxel(1, 0),
            "the step east from tick 9 lands on 24"
        );
        assert_eq!(
            facing(&app, body),
            E,
            "the turn north was cut: the body still faced east"
        );
    }

    #[test]
    fn a_short_over_a_path_cuts_the_step_in_flight_and_supersedes_the_path() {
        let (mut app, body) = body();
        order(&mut app, body, walk_to(3, 0));
        tick(&mut app, 20);
        order(&mut app, body, short(Short::Face(N)));
        tick(&mut app, 1);
        assert_eq!(previous_outcome(&app, body), Some(Outcome::Superseded));
        assert_eq!(queued(&app, body), 0, "the path's steps are gone");
        tick(&mut app, 40);
        assert_eq!(
            position(&app, body),
            voxel(1, 0),
            "the second step never landed"
        );
        assert_eq!(facing(&app, body), N);
    }
}

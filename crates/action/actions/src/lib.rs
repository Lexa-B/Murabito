//! The action queue: what a body has been asked to do, in what order. Anything may push
//! onto it, an instinct, a social pull, a player's command, a planner, and nothing in it
//! says who did or why. One system takes the action at the head when nothing is in
//! flight and issues the mechanism's intent for it. `docs/actions_readme.md` is the
//! design this implements.
//!
//! Sequencing lives here and nowhere else: a mechanism carries out one intent, and this
//! is what decides that a walk is a turn first and then a step. It is also the one place
//! that knows every mechanism, so cutting a body short, dropping whatever is in flight
//! so that something new can start this tick, lives here too: mark the body [`CutShort`].

use std::collections::VecDeque;

use bevy::prelude::*;
use murabito_attacks::Bite;
use murabito_hexcoords::Direction;
use murabito_movement::{Gait, Step, Turn, Way};
use murabito_placement::Facing;
use murabito_progress::{MechanismSet, Progress};

pub struct ActionsPlugin;

impl Plugin for ActionsPlugin {
    fn build(&self, app: &mut App) {
        // Asking comes before issuing, so an action pushed this tick is looked at this
        // tick; issuing before the mechanisms, so an intent issued this tick starts this
        // tick. Without the first, when a push and the issue land on the same tick
        // which runs first is the scheduler's whim, and a walk is a tick late or not.
        app.add_systems(
            FixedUpdate,
            (cut_short, issue)
                .chain()
                .after(AskingSet)
                .before(MechanismSet),
        );
    }
}

/// Put this on a body to drop whatever it is doing, now: the intent in flight is
/// removed, the bar is abandoned, and the head of the queue is issued on this same tick.
/// A cut step never happened, since a body's place changes only on landing; a cut turn
/// keeps the notches already made; a cut bite bit nothing. Removed as it is acted on.
#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CutShort;

/// Cuts short every body marked for it, before the head of its queue is issued.
fn cut_short(mut commands: Commands, mut bodies: Query<(Entity, &mut Progress), With<CutShort>>) {
    for (body, mut progress) in &mut bodies {
        progress.abandon();
        commands
            .entity(body)
            .remove::<(Step, Turn, Bite, CutShort)>();
    }
}

/// Where whatever pushes onto an `ActionQueue` runs: a scene's placeholder walk, later
/// the AI. In `FixedUpdate`, before the head of the queue is issued, so what is asked
/// on a tick is taken up on that tick.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AskingSet;

/// One thing a body can be asked to do: the vocabulary of everything above this crate,
/// which never sees a gait or a reach, only the word. A variant per word; the mechanism
/// that does it is below this crate, and the queue never changes when one is added. The
/// words that name a way, sidestep, backstep, recoil and lunge, are only that way: one
/// whose direction is not lateral, rear or forward of the body's facing is refused,
/// dropped with a warning, since whoever asked has it wrong and should hear so.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// One voxel that way at walking pace, turning first if need be.
    Walk(Direction),
    /// A walk at twice the pace.
    Jog(Direction),
    /// A walk at three times the pace.
    Sprint(Direction),
    /// A walk at half the pace.
    Sneak(Direction),
    /// Turn to face that way, without moving.
    Face(Direction),
    /// One voxel sideways at a walk, without turning; lands facing orthogonal to the way.
    Sidestep(Direction),
    /// One voxel back at a walk, without turning; lands facing away from the way.
    Backstep(Direction),
    /// A backstep at a jog.
    Recoil(Direction),
    /// Two voxels forward at a sprint, landed in one go.
    Lunge(Direction),
    /// Bite whatever is in the cell faced. Bites nothing yet.
    Bite,
}

impl Action {
    /// Whether the body, facing `facing`, may be asked this: `Err` says why not. The same
    /// rule `issue` refuses by, so whoever pushes can ask first and hear the answer
    /// rather than find a warning in the log.
    pub fn check(self, facing: Direction) -> Result<(), WrongWay> {
        next_intent(self, facing).map(|_| ())
    }
}

/// The actions a body has been asked to do, first to last.
#[derive(Component, Debug, Default)]
pub struct ActionQueue(VecDeque<Action>);

impl ActionQueue {
    pub fn push(&mut self, action: Action) {
        self.0.push_back(action);
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// What is queued, first to last, for whoever shows or reports it.
    pub fn iter(&self) -> impl Iterator<Item = Action> + '_ {
        self.0.iter().copied()
    }

    /// Drops everything queued. What is already in flight finishes, since the mechanism
    /// holds that, not the queue, unless the body is also marked [`CutShort`].
    pub fn clear(&mut self) {
        self.0.clear();
    }

    fn head(&self) -> Option<Action> {
        self.0.front().copied()
    }

    fn done_with_head(&mut self) {
        self.0.pop_front();
    }
}

/// Which intent the action at the head needs next, given the way the body faces, and
/// whether that intent is the action's last; or why the action is refused. Pure, so the
/// rule is testable on its own.
fn next_intent(action: Action, facing: Direction) -> Result<(Intent, bool), WrongWay> {
    let one = |direction, gait| Step {
        gait,
        ..Step::walk(direction)
    };
    match action {
        Action::Face(direction) => Ok((Intent::Turn(direction), true)),
        Action::Walk(direction) => forward(one(direction, Gait::Walk), facing),
        Action::Jog(direction) => forward(one(direction, Gait::Jog), facing),
        Action::Sprint(direction) => forward(one(direction, Gait::Sprint), facing),
        Action::Sneak(direction) => forward(one(direction, Gait::Sneak), facing),
        Action::Sidestep(direction) => only(Way::Lateral, one(direction, Gait::Walk), facing),
        Action::Backstep(direction) => only(Way::Rear, one(direction, Gait::Walk), facing),
        Action::Recoil(direction) => only(Way::Rear, one(direction, Gait::Jog), facing),
        Action::Lunge(direction) => {
            let step = Step {
                reach: 2,
                ..one(direction, Gait::Sprint)
            };
            only(Way::Forward, step, facing)
        }
        Action::Bite => Ok((Intent::Bite, true)),
    }
}

/// The step, and done, if the body already faces its way; otherwise a turn to one notch
/// short of it, with the action kept, and the step takes the last notch for free on
/// landing. That is the turn-then-step rule, in one place.
fn forward(step: Step, facing: Direction) -> Result<(Intent, bool), WrongWay> {
    if Way::of(facing, step.direction) == Way::Forward {
        return Ok((Intent::Step(step), true));
    }
    let notches = facing.notches_to(step.direction);
    Ok((
        Intent::Turn(facing.rotated(notches - notches.signum())),
        false,
    ))
}

/// The step, and done, if it goes the way the word needs; refused otherwise.
fn only(needs: Way, step: Step, facing: Direction) -> Result<(Intent, bool), WrongWay> {
    let way = Way::of(facing, step.direction);
    if way == needs {
        Ok((Intent::Step(step), true))
    } else {
        Err(WrongWay { needs, way })
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Intent {
    Step(Step),
    Turn(Direction),
    Bite,
}

/// Why an action is refused: its word is only for one way, and its direction is another.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WrongWay {
    pub needs: Way,
    pub way: Way,
}

impl WrongWay {
    /// The refusal in three words, for an outcome.
    pub fn reason(self) -> &'static str {
        match self.needs {
            Way::Forward => "not forward",
            Way::Lateral => "not lateral",
            Way::Rear => "not rear",
        }
    }
}

/// A body with no intent on it, so far as every mechanism knows.
type Idle = (Without<Step>, Without<Turn>, Without<Bite>);

/// Issues, for every body with nothing in flight, the intent its head action needs, and
/// drops the action once its last intent is out. An action refused is dropped with a
/// warning, and the next is looked at on the next tick.
fn issue(
    mut commands: Commands,
    mut bodies: Query<(Entity, &mut ActionQueue, &Facing, &Progress), Idle>,
) {
    for (body, mut queue, facing, progress) in &mut bodies {
        if progress.in_flight() {
            continue;
        }
        let Some(action) = queue.head() else {
            continue;
        };
        let (intent, last) = match next_intent(action, facing.0) {
            Ok(next) => next,
            Err(WrongWay { needs, way }) => {
                warn!(
                    "{body}: {action:?} while facing {:?} is {way:?}, and the word is only \
                     {needs:?}: refused",
                    facing.0
                );
                queue.done_with_head();
                continue;
            }
        };
        match intent {
            Intent::Step(step) => commands.entity(body).insert(step),
            Intent::Turn(direction) => commands.entity(body).insert(Turn(direction)),
            Intent::Bite => commands.entity(body).insert(Bite),
        };
        if last {
            queue.done_with_head();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_queue_reads_back_first_to_last() {
        let mut queue = ActionQueue::default();
        queue.push(Action::Walk(Direction::E));
        queue.push(Action::Face(Direction::N));
        assert_eq!(
            queue.iter().collect::<Vec<_>>(),
            [Action::Walk(Direction::E), Action::Face(Direction::N)]
        );
    }
    use murabito_attacks::AttacksPlugin;
    use murabito_hexcoords::VoxelCoord;
    use murabito_movement::{Locomotion, MovementPlugin};
    use murabito_placement::VoxelPosition;
    use murabito_progress::ProgressPlugin;

    fn voxel(q: i32, r: i32, layer: i32) -> VoxelCoord {
        VoxelCoord::new(q, r, -q - r, layer).expect("on the plane")
    }

    fn step(direction: Direction, gait: Gait, reach: u8) -> Step {
        Step {
            direction,
            gait,
            reach,
        }
    }

    #[test]
    fn facing_the_way_to_go_is_a_step_at_the_words_gait_and_done() {
        assert_eq!(
            next_intent(Action::Walk(Direction::E), Direction::E),
            Ok((Intent::Step(step(Direction::E, Gait::Walk, 1)), true))
        );
        assert_eq!(
            next_intent(Action::Sprint(Direction::E), Direction::E),
            Ok((Intent::Step(step(Direction::E, Gait::Sprint, 1)), true))
        );
        assert_eq!(
            next_intent(Action::Jog(Direction::E), Direction::E),
            Ok((Intent::Step(step(Direction::E, Gait::Jog, 1)), true))
        );
        assert_eq!(
            next_intent(Action::Sneak(Direction::E), Direction::E),
            Ok((Intent::Step(step(Direction::E, Gait::Sneak, 1)), true))
        );
    }

    #[test]
    fn every_pace_turns_first_when_it_is_not_forward() {
        // Exactly opposite is the tie that goes anticlockwise: one notch short is WNW.
        for word in [Action::Walk, Action::Jog, Action::Sprint, Action::Sneak] {
            assert_eq!(
                next_intent(word(Direction::W), Direction::E),
                Ok((Intent::Turn(Direction::WNW), false))
            );
        }
    }

    #[test]
    fn one_notch_off_is_still_a_step_the_landing_turns_for_free() {
        assert_eq!(
            next_intent(Action::Walk(Direction::E), Direction::ESE),
            Ok((Intent::Step(step(Direction::E, Gait::Walk, 1)), true))
        );
    }

    #[test]
    fn further_off_is_a_turn_to_one_notch_short_and_the_action_stays() {
        assert_eq!(
            next_intent(Action::Walk(Direction::E), Direction::NNW),
            Ok((Intent::Turn(Direction::ENE), false))
        );
        assert_eq!(
            next_intent(Action::Walk(Direction::E), Direction::W),
            Ok((Intent::Turn(Direction::ESE), false))
        );
    }

    #[test]
    fn facing_is_a_turn_all_the_way_and_done() {
        assert_eq!(
            next_intent(Action::Face(Direction::W), Direction::E),
            Ok((Intent::Turn(Direction::W), true))
        );
    }

    #[test]
    fn each_way_word_is_one_step_at_its_gait_and_reach() {
        use Direction::*;
        assert_eq!(
            next_intent(Action::Sidestep(N), E),
            Ok((Intent::Step(step(N, Gait::Walk, 1)), true))
        );
        assert_eq!(
            next_intent(Action::Backstep(W), E),
            Ok((Intent::Step(step(W, Gait::Walk, 1)), true))
        );
        assert_eq!(
            next_intent(Action::Recoil(WNW), E),
            Ok((Intent::Step(step(WNW, Gait::Jog, 1)), true))
        );
        assert_eq!(
            next_intent(Action::Lunge(ENE), E),
            Ok((Intent::Step(step(ENE, Gait::Sprint, 2)), true))
        );
    }

    #[test]
    fn a_bite_is_a_bite_and_done_whichever_way_the_body_faces() {
        assert_eq!(
            next_intent(Action::Bite, Direction::E),
            Ok((Intent::Bite, true))
        );
    }

    #[test]
    fn a_check_is_the_same_answer_issue_gives_without_issuing() {
        assert_eq!(Action::Sidestep(Direction::N).check(Direction::E), Ok(()));
        assert_eq!(Action::Walk(Direction::W).check(Direction::E), Ok(()));
        assert_eq!(
            Action::Sidestep(Direction::E)
                .check(Direction::E)
                .map_err(WrongWay::reason),
            Err("not lateral")
        );
        assert_eq!(
            Action::Lunge(Direction::W)
                .check(Direction::E)
                .map_err(WrongWay::reason),
            Err("not forward")
        );
    }

    #[test]
    fn a_way_word_the_wrong_way_is_refused() {
        use Direction::*;
        let wrong = |needs, way| Err(WrongWay { needs, way });
        assert_eq!(
            next_intent(Action::Sidestep(E), E),
            wrong(Way::Lateral, Way::Forward)
        );
        assert_eq!(
            next_intent(Action::Backstep(N), E),
            wrong(Way::Rear, Way::Lateral)
        );
        assert_eq!(
            next_intent(Action::Recoil(E), E),
            wrong(Way::Rear, Way::Forward)
        );
        assert_eq!(
            next_intent(Action::Lunge(W), E),
            wrong(Way::Forward, Way::Rear)
        );
    }

    #[test]
    fn a_queue_is_first_in_first_out_and_can_be_cleared() {
        let mut queue = ActionQueue::default();
        assert!(queue.is_empty());

        queue.push(Action::Walk(Direction::E));
        queue.push(Action::Face(Direction::N));
        assert_eq!(queue.len(), 2);
        assert_eq!(queue.head(), Some(Action::Walk(Direction::E)));

        queue.done_with_head();
        assert_eq!(queue.head(), Some(Action::Face(Direction::N)));

        queue.clear();
        assert!(queue.is_empty());
    }

    /// A headless app whose every `update` is exactly one 64 Hz tick, with one body in
    /// it: 4 shaku/s, 180 degrees/s. An app's first update only starts its clock, so it
    /// is spent here.
    fn body_facing(facing: Direction) -> (App, Entity) {
        use bevy::time::TimeUpdateStrategy;

        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            ProgressPlugin,
            MovementPlugin,
            AttacksPlugin,
            ActionsPlugin,
        ));
        app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
        app.update();
        let body = app
            .world_mut()
            .spawn((
                VoxelPosition(voxel(0, 0, 0)),
                Facing(facing),
                Locomotion {
                    speed: 4.0,
                    turn_speed: 180.0,
                },
                ActionQueue::default(),
            ))
            .id();
        (app, body)
    }

    fn push(app: &mut App, body: Entity, action: Action) {
        app.world_mut()
            .get_mut::<ActionQueue>(body)
            .expect("a queue")
            .push(action);
    }

    #[test]
    fn a_body_cut_short_mid_step_stays_where_it_was_and_starts_its_next_action_at_once() {
        let (mut app, body) = body_facing(Direction::E);
        push(&mut app, body, Action::Walk(Direction::E));
        for _ in 0..8 {
            app.update();
        }
        assert!(
            app.world().get::<Step>(body).is_some(),
            "half way through the step"
        );

        push(&mut app, body, Action::Face(Direction::N));
        app.world_mut().entity_mut(body).insert(CutShort);
        app.update();
        assert!(app.world().get::<Step>(body).is_none(), "the step is gone");
        assert!(
            app.world().get::<CutShort>(body).is_none(),
            "the mark is spent"
        );
        assert!(
            app.world().get::<Turn>(body).is_some(),
            "the turn began this tick"
        );
        assert_eq!(
            app.world().get::<VoxelPosition>(body).unwrap().0,
            voxel(0, 0, 0),
            "the body never arrived"
        );

        for _ in 0..31 {
            app.update();
        }
        assert_eq!(app.world().get::<Facing>(body).unwrap().0, Direction::N);
        for _ in 0..30 {
            app.update();
        }
        assert_eq!(
            app.world().get::<VoxelPosition>(body).unwrap().0,
            voxel(0, 0, 0),
            "and never does"
        );
    }

    #[test]
    fn a_turn_cut_short_keeps_the_notches_already_made() {
        let (mut app, body) = body_facing(Direction::E);
        push(&mut app, body, Action::Face(Direction::W));
        for _ in 0..25 {
            app.update();
        }
        let so_far = app.world().get::<Facing>(body).unwrap().0;
        assert_eq!(
            so_far,
            Direction::NNE,
            "two notches in 25 ticks at 180 degrees a second"
        );

        app.world_mut().entity_mut(body).insert(CutShort);
        app.update();
        assert!(app.world().get::<Turn>(body).is_none());
        assert_eq!(app.world().get::<Facing>(body).unwrap().0, so_far);
        assert!(!app.world().get::<Progress>(body).unwrap().in_flight());
    }

    fn position_of(app: &App, body: Entity) -> VoxelCoord {
        app.world()
            .get::<VoxelPosition>(body)
            .expect("a position")
            .0
    }

    fn facing_of(app: &App, body: Entity) -> Direction {
        app.world().get::<Facing>(body).expect("a facing").0
    }

    fn queued(app: &App, body: Entity) -> usize {
        app.world().get::<ActionQueue>(body).expect("a queue").len()
    }

    fn has<C: Component>(app: &App, body: Entity) -> bool {
        app.world().get::<C>(body).is_some()
    }

    /// Ticks until the queue is empty and nothing is in flight, giving up after too many.
    fn ticks_until_idle(app: &mut App, body: Entity) -> u32 {
        for tick in 1..=1000 {
            app.update();
            let idle = queued(app, body) == 0
                && !has::<Step>(app, body)
                && !has::<Turn>(app, body)
                && !has::<Bite>(app, body);
            if idle {
                return tick;
            }
        }
        panic!("never idle");
    }

    #[test]
    fn a_face_action_turns_the_body_and_leaves_the_queue_empty() {
        let (mut app, body) = body_facing(Direction::E);
        push(&mut app, body, Action::Face(Direction::W));

        ticks_until_idle(&mut app, body);

        assert_eq!(facing_of(&app, body), Direction::W);
        assert_eq!(position_of(&app, body), voxel(0, 0, 0));
    }

    #[test]
    fn a_go_action_already_faced_steps_on_the_tick_it_is_issued() {
        // Issued before the mechanisms run, the step walks its first tick's worth on the
        // same tick, so it lands on tick 16 as a bare step would.
        let (mut app, body) = body_facing(Direction::E);
        push(&mut app, body, Action::Walk(Direction::E));

        let ticks = ticks_until_idle(&mut app, body);

        assert_eq!(ticks, 16);
        assert_eq!(position_of(&app, body), voxel(1, 0, 0));
    }

    #[test]
    fn a_go_action_one_notch_off_never_turns_first() {
        let (mut app, body) = body_facing(Direction::ENE);
        push(&mut app, body, Action::Walk(Direction::E));

        let mut turned = false;
        for _ in 0..16 {
            app.update();
            turned |= has::<Turn>(&app, body);
        }

        assert!(!turned);
        assert_eq!(position_of(&app, body), voxel(1, 0, 0));
        assert_eq!(facing_of(&app, body), Direction::E);
    }

    #[test]
    fn a_go_action_far_off_turns_to_one_notch_short_then_steps() {
        // Facing W, going E: five notches of turning (the sixth is free on landing) at
        // 180 degrees/s is 53.3 ticks, so 54; the step is issued on tick 55 and lands 16
        // ticks later, on tick 70.
        let (mut app, body) = body_facing(Direction::W);
        push(&mut app, body, Action::Walk(Direction::E));

        app.update();
        assert_eq!(
            app.world().get::<Turn>(body).copied(),
            Some(Turn(Direction::ESE))
        );
        assert_eq!(
            queued(&app, body),
            1,
            "the action waits until its step is out"
        );

        let ticks = 1 + ticks_until_idle(&mut app, body);

        assert_eq!(ticks, 70);
        assert_eq!(position_of(&app, body), voxel(1, 0, 0));
        assert_eq!(facing_of(&app, body), Direction::E);
    }

    #[test]
    fn a_sidestep_never_turns_first_and_lands_facing_the_way_it_was() {
        let (mut app, body) = body_facing(Direction::E);
        push(&mut app, body, Action::Sidestep(Direction::N));

        for tick in 1..=28 {
            app.update();
            assert!(!has::<Turn>(&app, body), "turning on tick {tick}");
        }

        assert!(!has::<Step>(&app, body), "a corner step lands on tick 28");
        assert_eq!(position_of(&app, body), voxel(1, -2, 0));
        assert_eq!(facing_of(&app, body), Direction::E);
    }

    #[test]
    fn a_recoil_is_a_step_back_in_half_the_ticks_still_facing_forward() {
        let (mut app, body) = body_facing(Direction::E);
        push(&mut app, body, Action::Recoil(Direction::W));

        assert_eq!(ticks_until_idle(&mut app, body), 8);
        assert_eq!(position_of(&app, body), voxel(-1, 0, 0));
        assert_eq!(facing_of(&app, body), Direction::E);
    }

    #[test]
    fn a_lunge_lands_two_cells_on() {
        let (mut app, body) = body_facing(Direction::E);
        push(&mut app, body, Action::Lunge(Direction::E));

        assert_eq!(ticks_until_idle(&mut app, body), 11);
        assert_eq!(position_of(&app, body), voxel(2, 0, 0));
    }

    #[test]
    fn a_refused_action_is_dropped_and_the_one_after_it_is_done() {
        let (mut app, body) = body_facing(Direction::E);
        push(&mut app, body, Action::Sidestep(Direction::E));
        push(&mut app, body, Action::Walk(Direction::E));

        // Refused on tick 1, the walk issued on tick 2, landing 16 ticks later.
        assert_eq!(ticks_until_idle(&mut app, body), 17);
        assert_eq!(position_of(&app, body), voxel(1, 0, 0));
    }

    #[test]
    fn a_bite_holds_the_body_sixteen_ticks_and_moves_nothing() {
        let (mut app, body) = body_facing(Direction::E);
        push(&mut app, body, Action::Bite);

        assert_eq!(ticks_until_idle(&mut app, body), 16);
        assert_eq!(position_of(&app, body), voxel(0, 0, 0));
        assert_eq!(facing_of(&app, body), Direction::E);
    }

    #[test]
    fn a_bite_cut_short_is_gone_and_the_next_action_starts_at_once() {
        let (mut app, body) = body_facing(Direction::E);
        push(&mut app, body, Action::Bite);
        for _ in 0..8 {
            app.update();
        }
        assert!(has::<Bite>(&app, body));

        app.world_mut().entity_mut(body).insert(CutShort);
        push(&mut app, body, Action::Walk(Direction::E));
        app.update();

        assert!(!has::<Bite>(&app, body));
        assert!(has::<Step>(&app, body));
    }

    #[test]
    fn actions_are_done_in_the_order_they_were_pushed() {
        let (mut app, body) = body_facing(Direction::E);
        push(&mut app, body, Action::Walk(Direction::E));
        push(&mut app, body, Action::Walk(Direction::E));
        push(&mut app, body, Action::Face(Direction::N));

        ticks_until_idle(&mut app, body);

        assert_eq!(position_of(&app, body), voxel(2, 0, 0));
        assert_eq!(facing_of(&app, body), Direction::N);
    }

    #[test]
    fn nothing_is_issued_while_something_is_in_flight() {
        let (mut app, body) = body_facing(Direction::E);
        app.world_mut()
            .entity_mut(body)
            .insert(Step::walk(Direction::E));
        push(&mut app, body, Action::Face(Direction::W));

        app.update();

        assert!(!has::<Turn>(&app, body));
        assert_eq!(queued(&app, body), 1);
    }

    #[test]
    fn an_empty_queue_issues_nothing() {
        let (mut app, body) = body_facing(Direction::E);

        (0..8).for_each(|_| app.update());

        assert!(!has::<Step>(&app, body) && !has::<Turn>(&app, body));
        assert_eq!(position_of(&app, body), voxel(0, 0, 0));
    }
}

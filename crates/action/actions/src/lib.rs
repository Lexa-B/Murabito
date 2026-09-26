//! The action queue: what a body has been asked to do, in what order. Anything may push
//! onto it, an instinct, a social pull, a player's command, a planner, and nothing in it
//! says who did or why. One system takes the action at the head when nothing is in
//! flight and issues the mechanism's intent for it. `docs/actions_readme.md` is the
//! design this implements.
//!
//! Sequencing lives here and nowhere else: a mechanism carries out one intent, and this
//! is what decides that a walk is a turn first and then a step.

use std::collections::VecDeque;

use bevy::prelude::*;
use murabito_hexcoords::Direction;
use murabito_movement::{Step, Turn, can_step};
use murabito_placement::Facing;
use murabito_progress::{MechanismSet, Progress};

pub struct ActionsPlugin;

impl Plugin for ActionsPlugin {
    fn build(&self, app: &mut App) {
        // Asking comes before issuing, so an action pushed this tick is looked at this
        // tick; issuing before the mechanisms, so an intent issued this tick starts this
        // tick. Without the first, when a push and the issue land on the same tick
        // which runs first is the scheduler's whim, and a walk is a tick late or not.
        app.add_systems(FixedUpdate, issue.after(AskingSet).before(MechanismSet));
    }
}

/// Where whatever pushes onto an `ActionQueue` runs: a scene's placeholder walk, later
/// the AI. In `FixedUpdate`, before the head of the queue is issued, so what is asked
/// on a tick is taken up on that tick.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AskingSet;

/// One thing a body can be asked to do. A variant per kind; the mechanism that does it
/// is below this crate, and the queue never changes when one is added.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Walk one voxel that way, turning first if need be.
    Go(Direction),
    /// Turn to face that way, without moving.
    Face(Direction),
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

    /// Drops everything queued. What is already in flight finishes: the mechanism holds
    /// that, not the queue.
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
/// whether that intent is the action's last. Pure, so the rule is testable on its own.
fn next_intent(action: Action, facing: Direction) -> (Intent, bool) {
    match action {
        Action::Face(direction) => (Intent::Turn(direction), true),
        Action::Go(direction) if can_step(facing, direction) => (Intent::Step(direction), true),
        // Too far off to step: turn until one notch short, and let the step take the last
        // notch for free on landing.
        Action::Go(direction) => {
            let notches = facing.notches_to(direction);
            (
                Intent::Turn(facing.rotated(notches - notches.signum())),
                false,
            )
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Intent {
    Step(Direction),
    Turn(Direction),
}

/// A body doing nothing, so far as movement knows.
type Idle = (Without<Step>, Without<Turn>);

/// Issues, for every body with nothing in flight, the intent its head action needs, and
/// drops the action once its last intent is out.
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
        let (intent, last) = next_intent(action, facing.0);
        match intent {
            Intent::Step(direction) => commands.entity(body).insert(Step(direction)),
            Intent::Turn(direction) => commands.entity(body).insert(Turn(direction)),
        };
        if last {
            queue.done_with_head();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use murabito_hexcoords::VoxelCoord;
    use murabito_movement::{Locomotion, MovementPlugin};
    use murabito_placement::VoxelPosition;
    use murabito_progress::ProgressPlugin;

    fn voxel(q: i32, r: i32, layer: i32) -> VoxelCoord {
        VoxelCoord::new(q, r, -q - r, layer).expect("on the plane")
    }

    #[test]
    fn facing_the_way_to_go_is_a_step_and_done() {
        assert_eq!(
            next_intent(Action::Go(Direction::E), Direction::E),
            (Intent::Step(Direction::E), true)
        );
    }

    #[test]
    fn one_notch_off_is_still_a_step_the_landing_turns_for_free() {
        assert_eq!(
            next_intent(Action::Go(Direction::E), Direction::ESE),
            (Intent::Step(Direction::E), true)
        );
    }

    #[test]
    fn further_off_is_a_turn_to_one_notch_short_and_the_action_stays() {
        assert_eq!(
            next_intent(Action::Go(Direction::E), Direction::NNW),
            (Intent::Turn(Direction::ENE), false)
        );
        assert_eq!(
            next_intent(Action::Go(Direction::E), Direction::W),
            (Intent::Turn(Direction::ESE), false)
        );
    }

    #[test]
    fn facing_is_a_turn_all_the_way_and_done() {
        assert_eq!(
            next_intent(Action::Face(Direction::W), Direction::E),
            (Intent::Turn(Direction::W), true)
        );
    }

    #[test]
    fn a_queue_is_first_in_first_out_and_can_be_cleared() {
        let mut queue = ActionQueue::default();
        assert!(queue.is_empty());

        queue.push(Action::Go(Direction::E));
        queue.push(Action::Face(Direction::N));
        assert_eq!(queue.len(), 2);
        assert_eq!(queue.head(), Some(Action::Go(Direction::E)));

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
            let idle = queued(app, body) == 0 && !has::<Step>(app, body) && !has::<Turn>(app, body);
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
        push(&mut app, body, Action::Go(Direction::E));

        let ticks = ticks_until_idle(&mut app, body);

        assert_eq!(ticks, 16);
        assert_eq!(position_of(&app, body), voxel(1, 0, 0));
    }

    #[test]
    fn a_go_action_one_notch_off_never_turns_first() {
        let (mut app, body) = body_facing(Direction::ENE);
        push(&mut app, body, Action::Go(Direction::E));

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
        push(&mut app, body, Action::Go(Direction::E));

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
    fn actions_are_done_in_the_order_they_were_pushed() {
        let (mut app, body) = body_facing(Direction::E);
        push(&mut app, body, Action::Go(Direction::E));
        push(&mut app, body, Action::Go(Direction::E));
        push(&mut app, body, Action::Face(Direction::N));

        ticks_until_idle(&mut app, body);

        assert_eq!(position_of(&app, body), voxel(2, 0, 0));
        assert_eq!(facing_of(&app, body), Direction::N);
    }

    #[test]
    fn nothing_is_issued_while_something_is_in_flight() {
        let (mut app, body) = body_facing(Direction::E);
        app.world_mut().entity_mut(body).insert(Step(Direction::E));
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

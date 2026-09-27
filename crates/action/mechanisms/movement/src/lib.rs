//! How a body moves across the hex voxel grid: the mechanics of stepping and turning.
//! Nothing here decides *what* to do; that is the actions layer above, which puts a
//! `Step` or a `Turn` on an entity and this crate carries it out. Where the body is and
//! which way it faces are `murabito_placement`'s `VoxelPosition` and `Facing`, which
//! this is the one mechanism that writes to. `docs/movement_readme.md` is the design
//! this implements.
//!
//! Compass: **east is +X, north is −Z, up is +Y.** Facing is one of the twelve compass
//! directions, never up or down.
//!
//! Movement is by whole voxels: an entity is in exactly one cell, and a step moves it to
//! a neighbour in one go, once the body has walked the distance.

use bevy::prelude::*;
use murabito_hexcoords::Direction;
use murabito_placement::{Facing, VoxelPosition};
use murabito_progress::{MechanismSet, Progress};

/// The distance to a corner neighbour, in shaku, and so the cost of stepping to one.
const SQRT_3: f32 = 1.732_050_8;

/// One notch of the compass, in degrees, and so the cost of turning one.
const NOTCH: f32 = 30.0;

pub struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, (step, turn).in_set(MechanismSet));
    }
}

/// How a body moves: how fast it walks, in shaku per second, and how fast it turns, in
/// degrees per second. Requires a `Progress`, the bar its steps and turns fill.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
#[require(Progress)]
pub struct Locomotion {
    pub speed: f32,
    pub turn_speed: f32,
}

/// How fast a body goes on a step, as a multiple of its walking speed. A property of the
/// step, not of the body: a fox sneaks up and then sprints with nothing on it changing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Gait {
    Sneak,
    #[default]
    Walk,
    Jog,
    Sprint,
}

impl Gait {
    /// The multiple of `Locomotion::speed` this gait moves at.
    pub fn factor(self) -> f32 {
        match self {
            Gait::Sneak => 0.5,
            Gait::Walk => 1.0,
            Gait::Jog => 2.0,
            Gait::Sprint => 3.0,
        }
    }
}

/// The intent to step `reach` voxels in a direction, at a gait, landing in one go. Put it
/// on a body with a `Locomotion` and `step` carries it out over ticks, then removes it.
/// Any of the twelve directions is a legal step: which [`Way`] it goes relative to the
/// facing decides how the body faces on landing, and the landing turns it at most one
/// notch. A reach of two is a lunge: the body is never in the cell between. One at a
/// time: the actions layer never issues another while one is in flight.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Step {
    pub direction: Direction,
    pub gait: Gait,
    pub reach: u8,
}

impl Step {
    /// One voxel at walking pace.
    pub fn walk(direction: Direction) -> Self {
        Step {
            direction,
            gait: Gait::Walk,
            reach: 1,
        }
    }

    /// What the step costs, in shaku walked: the way's cost, `reach` times over.
    pub fn cost(self) -> f32 {
        cost(self.direction) * f32::from(self.reach)
    }
}

/// The intent to turn to face a direction. Put it on a body with a `Locomotion` and
/// `turn` carries it out a notch of 30° at a time, the short way round, passing through
/// every direction between, then removes it. One at a time, as for `Step`.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Turn(pub Direction);

/// What a step costs, in shaku walked: 1 to a face neighbour, √3 to a corner one.
pub fn cost(direction: Direction) -> f32 {
    if direction.is_edge() { 1.0 } else { SQRT_3 }
}

/// Which way a step goes, relative to the way the body faces. Forward is the way faced
/// or a notch either side; rear is the way behind or a notch either side; lateral is the
/// six between.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Way {
    Forward,
    Lateral,
    Rear,
}

impl Way {
    pub fn of(facing: Direction, direction: Direction) -> Self {
        match facing.notches_to(direction).abs() {
            0 | 1 => Way::Forward,
            2..=4 => Way::Lateral,
            _ => Way::Rear,
        }
    }

    /// How many notches the way gone is turned back toward the old facing on landing.
    fn notches_back(self) -> i8 {
        match self {
            Way::Forward => 0,
            Way::Lateral => 3,
            Way::Rear => 6,
        }
    }
}

/// The way a body facing `facing` faces after stepping `direction`: a forward step lands
/// facing the way it went, a lateral one orthogonal to it and a rear one opposite, each on
/// the side of the old facing. The landing never turns the body more than one notch, so
/// that one notch is the free adjustment every step gets.
pub fn landing(facing: Direction, direction: Direction) -> Direction {
    let notches = facing.notches_to(direction);
    direction.rotated(-Way::of(facing, direction).notches_back() * notches.signum())
}

/// Carries out every `Step` in flight, one tick's walk at a time, and lands it once the
/// distance is walked: the body is in the neighbour, facing as the [`landing`] rule says.
/// Inside `FixedUpdate`, `Time` is the fixed clock, so `delta_secs` is the tick.
fn step(
    time: Res<Time>,
    mut commands: Commands,
    mut bodies: Query<(
        Entity,
        &Step,
        &Locomotion,
        &mut VoxelPosition,
        &mut Facing,
        &mut Progress,
    )>,
) {
    for (body, step, locomotion, mut position, mut facing, mut progress) in &mut bodies {
        if !progress.in_flight() {
            progress.start::<Step>(step.cost());
        }
        let pace = locomotion.speed * step.gait.factor();
        if progress.advance(pace * time.delta_secs()) {
            for _ in 0..step.reach {
                position.0 = position.0.neighbour(step.direction);
            }
            facing.0 = landing(facing.0, step.direction);
            progress.finish();
            commands.entity(body).remove::<Step>();
        }
    }
}

/// Carries out every `Turn` in flight, one tick's turning at a time: each notch is its
/// own action on the bar, so a wide swing is a run of them and the leftover degrees carry
/// from one to the next. At most one notch is taken per tick, and the next begins on the
/// bar the moment one lands, so the bar reads in flight for the whole swing: whatever
/// asks `Progress` whether the body is busy gets the truth between notches too.
fn turn(
    time: Res<Time>,
    mut commands: Commands,
    mut bodies: Query<(Entity, &Turn, &Locomotion, &mut Facing, &mut Progress)>,
) {
    for (body, turn, locomotion, mut facing, mut progress) in &mut bodies {
        if facing.0 == turn.0 {
            commands.entity(body).remove::<Turn>();
            continue;
        }
        if !progress.in_flight() {
            progress.start::<Turn>(NOTCH);
        }
        if progress.advance(locomotion.turn_speed * time.delta_secs()) {
            facing.0 = facing.0.rotated(facing.0.notches_to(turn.0).signum());
            progress.finish();
            if facing.0 == turn.0 {
                commands.entity(body).remove::<Turn>();
            } else {
                progress.start::<Turn>(NOTCH);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use murabito_hexcoords::VoxelCoord;
    use murabito_progress::ProgressPlugin;

    fn voxel(q: i32, r: i32, layer: i32) -> VoxelCoord {
        VoxelCoord::new(q, r, -q - r, layer).expect("on the plane")
    }

    /// A headless app whose every `update` is exactly one 64 Hz tick, with one body in
    /// it. An app's first update only starts its clock, so it is spent here.
    fn body_at(position: VoxelCoord, facing: Direction, speed: f32) -> (App, Entity) {
        use bevy::time::TimeUpdateStrategy;

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, ProgressPlugin, MovementPlugin));
        app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
        app.update();
        let body = app
            .world_mut()
            .spawn((
                VoxelPosition(position),
                Facing(facing),
                Locomotion {
                    speed,
                    turn_speed: 180.0,
                },
            ))
            .id();
        (app, body)
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

    fn is_stepping(app: &App, body: Entity) -> bool {
        app.world().get::<Step>(body).is_some()
    }

    /// Issues a step and counts the ticks until it lands, giving up after too many.
    fn ticks_to_land(app: &mut App, body: Entity, step: Step) -> u32 {
        app.world_mut().entity_mut(body).insert(step);
        for tick in 1..=200 {
            app.update();
            if !is_stepping(app, body) {
                return tick;
            }
        }
        panic!("the step never landed");
    }

    /// Issues a one-voxel step at a gait and counts the ticks until it lands.
    fn ticks_to_step_at(app: &mut App, body: Entity, direction: Direction, gait: Gait) -> u32 {
        ticks_to_land(
            app,
            body,
            Step {
                gait,
                ..Step::walk(direction)
            },
        )
    }

    /// Issues a walking step and counts the ticks until it lands.
    fn ticks_to_step(app: &mut App, body: Entity, direction: Direction) -> u32 {
        ticks_to_step_at(app, body, direction, Gait::Walk)
    }

    /// Two voxels forward at sprint pace.
    fn lunge(direction: Direction) -> Step {
        Step {
            gait: Gait::Sprint,
            reach: 2,
            ..Step::walk(direction)
        }
    }

    fn is_turning(app: &App, body: Entity) -> bool {
        app.world().get::<Turn>(body).is_some()
    }

    /// Issues a turn and records the facing after each tick until it is done, giving up
    /// after too many. The first entry is the facing after the first tick.
    fn facings_while_turning(app: &mut App, body: Entity, direction: Direction) -> Vec<Direction> {
        app.world_mut().entity_mut(body).insert(Turn(direction));
        let mut seen = Vec::new();
        for _ in 1..=400 {
            app.update();
            seen.push(facing_of(app, body));
            if !is_turning(app, body) {
                return seen;
            }
        }
        panic!("the turn never finished");
    }

    #[test]
    fn a_face_neighbour_costs_one_shaku_and_a_corner_neighbour_root_three() {
        assert_eq!(cost(Direction::E), 1.0);
        assert!((cost(Direction::N) - 3f32.sqrt()).abs() < 1e-6);
    }

    #[test]
    fn three_forward_six_lateral_and_three_rear_make_the_twelve() {
        use Direction::*;
        let ways: Vec<_> = Direction::ALL
            .iter()
            .map(|&direction| Way::of(E, direction))
            .collect();
        assert_eq!(&ways[..2], [Way::Forward, Way::Forward]);
        assert_eq!(&ways[2..5], [Way::Lateral; 3]);
        assert_eq!(&ways[5..8], [Way::Rear; 3]);
        assert_eq!(&ways[8..11], [Way::Lateral; 3]);
        assert_eq!(ways[11], Way::Forward);
        assert_eq!(Way::of(N, NNE), Way::Forward);
        assert_eq!(Way::of(N, S), Way::Rear);
    }

    #[test]
    fn a_forward_step_lands_facing_the_way_it_went() {
        use Direction::*;
        assert_eq!(landing(E, E), E);
        assert_eq!(landing(E, ENE), ENE);
        assert_eq!(landing(E, ESE), ESE);
    }

    #[test]
    fn a_lateral_step_lands_orthogonal_to_the_way_it_went_on_the_side_faced() {
        use Direction::*;
        // Facing east, a step due north keeps the facing; two notches off turns the body
        // one notch toward the step, four notches off one notch away from it.
        assert_eq!(landing(E, N), E);
        assert_eq!(landing(E, NNE), ESE);
        assert_eq!(landing(E, NNW), ENE);
        assert_eq!(landing(E, S), E);
        assert_eq!(landing(E, SSE), ENE);
        assert_eq!(landing(E, SSW), ESE);
    }

    #[test]
    fn a_rear_step_lands_facing_opposite_the_way_it_went() {
        use Direction::*;
        assert_eq!(landing(E, W), E);
        assert_eq!(landing(E, WNW), ESE);
        assert_eq!(landing(E, WSW), ENE);
        assert_eq!(landing(N, S), N);
    }

    #[test]
    fn no_landing_turns_the_body_more_than_one_notch() {
        for &facing in &Direction::ALL {
            for &direction in &Direction::ALL {
                let landed = landing(facing, direction);
                assert!(
                    facing.notches_to(landed).abs() <= 1,
                    "facing {facing:?}, stepping {direction:?}, landed {landed:?}"
                );
            }
        }
    }

    #[test]
    fn an_edge_step_at_four_shaku_a_second_lands_on_tick_sixteen() {
        let (mut app, body) = body_at(voxel(0, 0, 0), Direction::E, 4.0);

        let ticks = ticks_to_step(&mut app, body, Direction::E);

        assert_eq!(ticks, 16);
        assert_eq!(position_of(&app, body), voxel(1, 0, 0));
    }

    #[test]
    fn a_corner_step_takes_root_three_times_as_long_rounded_up_to_the_tick() {
        let (mut app, body) = body_at(voxel(0, 0, 0), Direction::N, 4.0);

        let ticks = ticks_to_step(&mut app, body, Direction::N);

        assert_eq!(ticks, 28);
        assert_eq!(position_of(&app, body), voxel(1, -2, 0));
    }

    #[test]
    fn a_gait_is_a_multiple_of_the_walking_speed() {
        assert_eq!(Gait::Sneak.factor(), 0.5);
        assert_eq!(Gait::Walk.factor(), 1.0);
        assert_eq!(Gait::Jog.factor(), 2.0);
        assert_eq!(Gait::Sprint.factor(), 3.0);
        assert_eq!(Gait::default(), Gait::Walk);
    }

    #[test]
    fn a_walk_step_is_one_voxel_at_walking_pace() {
        assert_eq!(
            Step::walk(Direction::E),
            Step {
                direction: Direction::E,
                gait: Gait::Walk,
                reach: 1,
            }
        );
    }

    #[test]
    fn a_step_costs_its_way_times_its_reach() {
        assert_eq!(Step::walk(Direction::E).cost(), 1.0);
        assert_eq!(lunge(Direction::E).cost(), 2.0);
        assert!((lunge(Direction::N).cost() - 2.0 * SQRT_3).abs() < 1e-6);
    }

    #[test]
    fn a_lunge_lands_two_cells_on_in_one_go_at_sprint_pace() {
        // Two shaku at 12 shaku/s is 10.67 ticks, so 11; a corner lunge, 2√3, is 18.5.
        let (mut app, body) = body_at(voxel(0, 0, 0), Direction::E, 4.0);

        assert_eq!(ticks_to_land(&mut app, body, lunge(Direction::E)), 11);
        assert_eq!(position_of(&app, body), voxel(2, 0, 0));

        let (mut app, body) = body_at(voxel(0, 0, 0), Direction::N, 4.0);

        assert_eq!(ticks_to_land(&mut app, body, lunge(Direction::N)), 19);
        assert_eq!(position_of(&app, body), voxel(2, -4, 0));
    }

    #[test]
    fn a_lunge_mid_flight_is_still_in_the_cell_it_left_from() {
        let (mut app, body) = body_at(voxel(0, 0, 0), Direction::E, 4.0);
        app.world_mut().entity_mut(body).insert(lunge(Direction::E));

        (0..10).for_each(|_| app.update());

        assert!(is_stepping(&app, body));
        assert_eq!(position_of(&app, body), voxel(0, 0, 0));
    }

    #[test]
    fn the_gaits_take_the_ticks_their_multiple_says_for_an_edge_step() {
        // At 4 shaku/s an edge step walks in 16 ticks: a sneak is 32, a jog 8, a sprint
        // 5.33 and so 6.
        let expected = [
            (Gait::Sneak, 32),
            (Gait::Walk, 16),
            (Gait::Jog, 8),
            (Gait::Sprint, 6),
        ];
        for (gait, ticks) in expected {
            let (mut app, body) = body_at(voxel(0, 0, 0), Direction::E, 4.0);

            assert_eq!(
                ticks_to_step_at(&mut app, body, Direction::E, gait),
                ticks,
                "{gait:?}"
            );
            assert_eq!(position_of(&app, body), voxel(1, 0, 0));
        }
    }

    #[test]
    fn a_change_of_gait_keeps_the_head_start_since_the_bar_is_in_shaku() {
        // A sprinting edge step at 4 shaku/s passes 1 shaku on tick 6 with 0.125 to
        // spare; the walk after it starts that far in, and lands on tick 14 not 16.
        let (mut app, body) = body_at(voxel(0, 0, 0), Direction::E, 4.0);
        ticks_to_step_at(&mut app, body, Direction::E, Gait::Sprint);

        assert_eq!(ticks_to_step(&mut app, body, Direction::E), 14);
    }

    #[test]
    fn landing_faces_the_body_the_way_it_went() {
        let (mut app, body) = body_at(voxel(0, 0, 0), Direction::ENE, 4.0);

        ticks_to_step(&mut app, body, Direction::E);

        assert_eq!(facing_of(&app, body), Direction::E);
    }

    #[test]
    fn a_run_of_steps_lands_when_the_total_distance_says_not_each_step_rounded() {
        // At 3 shaku/s an edge is 21.3 ticks and a corner 36.9: rounded one by one,
        // edge + corner + edge is 22 + 37 + 22 = 81. The distance, 3.732 shaku, is 79.6.
        let (mut app, body) = body_at(voxel(0, 0, 0), Direction::E, 3.0);

        let ticks = ticks_to_step(&mut app, body, Direction::E)
            + ticks_to_step(&mut app, body, Direction::ENE)
            + ticks_to_step(&mut app, body, Direction::E);

        assert_eq!(ticks, 80);
    }

    #[test]
    fn a_tick_standing_still_forgets_the_head_start() {
        let (mut app, body) = body_at(voxel(0, 0, 0), Direction::E, 3.0);
        ticks_to_step(&mut app, body, Direction::E);

        app.update();

        assert_eq!(ticks_to_step(&mut app, body, Direction::E), 22);
    }

    #[test]
    fn a_step_straight_after_another_keeps_the_head_start() {
        let (mut app, body) = body_at(voxel(0, 0, 0), Direction::E, 3.0);
        ticks_to_step(&mut app, body, Direction::E);

        assert_eq!(ticks_to_step(&mut app, body, Direction::E), 21);
    }

    #[test]
    fn a_sidestep_moves_the_body_and_keeps_it_facing_the_way_it_was() {
        let (mut app, body) = body_at(voxel(0, 0, 0), Direction::E, 4.0);

        let ticks = ticks_to_step(&mut app, body, Direction::N);

        assert_eq!(ticks, 28);
        assert_eq!(position_of(&app, body), voxel(1, -2, 0));
        assert_eq!(facing_of(&app, body), Direction::E);
    }

    #[test]
    fn a_step_backward_lands_the_body_still_facing_forward() {
        let (mut app, body) = body_at(voxel(0, 0, 0), Direction::E, 4.0);

        let ticks = ticks_to_step(&mut app, body, Direction::W);

        assert_eq!(ticks, 16);
        assert_eq!(position_of(&app, body), voxel(-1, 0, 0));
        assert_eq!(facing_of(&app, body), Direction::E);
    }

    #[test]
    fn a_body_mid_step_is_still_in_the_voxel_it_left_from() {
        let (mut app, body) = body_at(voxel(0, 0, 0), Direction::E, 4.0);
        app.world_mut()
            .entity_mut(body)
            .insert(Step::walk(Direction::E));

        (0..15).for_each(|_| app.update());

        assert!(is_stepping(&app, body));
        assert_eq!(position_of(&app, body), voxel(0, 0, 0));
    }

    fn turner_facing(facing: Direction, turn_speed: f32) -> (App, Entity) {
        let (mut app, body) = body_at(voxel(0, 0, 0), facing, 4.0);
        app.world_mut()
            .get_mut::<Locomotion>(body)
            .expect("a locomotion")
            .turn_speed = turn_speed;
        (app, body)
    }

    #[test]
    fn a_half_turn_at_ninety_degrees_a_second_takes_two_seconds_of_ticks() {
        let (mut app, body) = turner_facing(Direction::E, 90.0);

        let seen = facings_while_turning(&mut app, body, Direction::W);

        assert_eq!(seen.len(), 128);
        assert_eq!(facing_of(&app, body), Direction::W);
    }

    #[test]
    fn the_bar_stays_in_flight_from_the_first_notch_of_a_swing_to_the_last() {
        let (mut app, body) = turner_facing(Direction::E, 90.0);
        app.world_mut().entity_mut(body).insert(Turn(Direction::W));
        let in_flight = |app: &App| app.world().get::<Progress>(body).unwrap().in_flight();

        for tick in 1..128 {
            app.update();
            assert!(in_flight(&app), "idle between notches on tick {tick}");
        }
        app.update();
        assert!(!in_flight(&app), "landed on tick 128");
        assert!(!is_turning(&app, body));
    }

    #[test]
    fn a_wide_swing_passes_through_every_direction_between() {
        let (mut app, body) = turner_facing(Direction::E, 90.0);

        let mut seen = facings_while_turning(&mut app, body, Direction::W);
        seen.dedup();

        let every_notch: Vec<_> = (0..=6).map(|n| Direction::E.rotated(n)).collect();
        assert_eq!(seen, every_notch);
    }

    #[test]
    fn a_turn_goes_the_short_way_round_either_way() {
        let (mut app, body) = turner_facing(Direction::E, 90.0);
        let mut clockwise = facings_while_turning(&mut app, body, Direction::SSE);
        clockwise.dedup();

        let (mut app, body) = turner_facing(Direction::E, 90.0);
        let mut anticlockwise = facings_while_turning(&mut app, body, Direction::NNE);
        anticlockwise.dedup();

        assert_eq!(clockwise, [Direction::E, Direction::ESE, Direction::SSE]);
        assert_eq!(
            anticlockwise,
            [Direction::E, Direction::ENE, Direction::NNE]
        );
    }

    #[test]
    fn turning_to_face_the_way_already_faced_is_done_on_the_first_tick() {
        let (mut app, body) = turner_facing(Direction::N, 90.0);

        let seen = facings_while_turning(&mut app, body, Direction::N);

        assert_eq!(seen, [Direction::N]);
    }

    #[test]
    fn a_turn_does_not_move_the_body() {
        let (mut app, body) = turner_facing(Direction::E, 90.0);

        facings_while_turning(&mut app, body, Direction::W);

        assert_eq!(position_of(&app, body), voxel(0, 0, 0));
    }

    #[test]
    fn leftover_degrees_carry_from_one_turn_straight_into_the_next() {
        // At 90 degrees/s a notch is 21.3 ticks: rounded one by one, two notches are 22 + 22.
        let (mut app, body) = turner_facing(Direction::E, 90.0);

        let first = facings_while_turning(&mut app, body, Direction::ENE).len();
        let second = facings_while_turning(&mut app, body, Direction::NNE).len();

        assert_eq!((first, second), (22, 21));
    }
}

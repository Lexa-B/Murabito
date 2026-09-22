//! How a body moves across the hex voxel grid: where it is, which way it faces, and the
//! mechanics of stepping and turning. Nothing here decides *what* to do; that is the
//! actions layer above, which puts a `Step` or a `Turn` on an entity and this crate
//! carries it out. `Docs/movement_readme.md` is the design this implements.
//!
//! Compass: **east is +X, north is −Z, up is +Y.** Facing is one of the twelve compass
//! directions, never up or down.
//!
//! Movement is by whole voxels: an entity is in exactly one cell, and a step moves it to
//! a neighbour in one go, once the body has walked the distance. `place` keeps the model
//! where the cell is.

use bevy::prelude::*;
use murabito_hexcoords::{Direction, VoxelCoord};
use murabito_progress::{MechanismSet, Progress};

/// The distance to a corner neighbour, in shaku, and so the cost of stepping to one.
const SQRT_3: f32 = 1.732_050_8;

/// One notch of the compass, in degrees, and so the cost of turning one.
const NOTCH: f32 = 30.0;

pub struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, (step, turn).in_set(MechanismSet))
            .add_systems(Update, place);
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

/// The intent to step one voxel in a direction. Put it on a body with a `Locomotion` and
/// `step` carries it out over ticks, then removes it. Refused, and removed with a
/// warning, unless the body already faces within one notch of the way it is to go: a
/// wider turn is its own action first. One at a time: the actions layer never issues
/// another while one is in flight.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Step(pub Direction);

/// The intent to turn to face a direction. Put it on a body with a `Locomotion` and
/// `turn` carries it out a notch of 30° at a time, the short way round, passing through
/// every direction between, then removes it. One at a time, as for `Step`.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Turn(pub Direction);

/// What a step costs, in shaku walked: 1 to a face neighbour, √3 to a corner one.
pub fn cost(direction: Direction) -> f32 {
    if direction.is_edge() { 1.0 } else { SQRT_3 }
}

/// Whether a body facing `facing` may step `direction` without turning first: the same
/// way, or one notch of 30° either side. The one-notch turn is free, taken on landing.
pub fn can_step(facing: Direction, direction: Direction) -> bool {
    facing.notches_to(direction).abs() <= 1
}

/// Carries out every `Step` in flight, one tick's walk at a time, and lands it once the
/// distance is walked: the body is in the neighbour, facing the way it went. Inside
/// `FixedUpdate`, `Time` is the fixed clock, so `delta_secs` is the tick.
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
            if !can_step(facing.0, step.0) {
                warn!(
                    "{body}: a step {:?} while facing {:?} needs a turn first",
                    step.0, facing.0
                );
                commands.entity(body).remove::<Step>();
                continue;
            }
            progress.start::<Step>(cost(step.0));
        }
        if progress.advance(locomotion.speed * time.delta_secs()) {
            position.0 = position.0.neighbour(step.0);
            facing.0 = step.0;
            progress.finish();
            commands.entity(body).remove::<Step>();
        }
    }
}

/// The voxel an entity is in. Integer: it changes only when a step lands.
///
/// Requires a `Transform`, so `place` has somewhere to put the model.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
#[require(Transform)]
pub struct VoxelPosition(pub VoxelCoord);

/// Which of the twelve compass directions an entity faces.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Facing(pub Direction);

/// An entity whose position or facing changed since `place` last ran.
type Moved = Or<(Changed<VoxelPosition>, Changed<Facing>)>;

/// Keeps a model where its cell is: at the centre of the voxel's bottom face, turned to
/// its heading. The only thing here that writes a `Transform`, and only when the
/// position or facing changed, so a standing entity is left alone.
fn place(mut placed: Query<(&VoxelPosition, &Facing, &mut Transform), Moved>) {
    for (position, facing, mut transform) in &mut placed {
        *transform =
            Transform::from_translation(position.0.to_world()).with_rotation(facing.0.heading());
    }
}

/// Carries out every `Turn` in flight, one tick's turning at a time: each notch is its
/// own action on the bar, so a wide swing is a run of them and the leftover degrees carry
/// from one to the next. At most one notch is taken per tick.
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
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn ticks_to_step(app: &mut App, body: Entity, direction: Direction) -> u32 {
        app.world_mut().entity_mut(body).insert(Step(direction));
        for tick in 1..=200 {
            app.update();
            if !is_stepping(app, body) {
                return tick;
            }
        }
        panic!("the step never landed");
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
    fn a_step_needs_the_body_facing_within_one_notch() {
        assert!(can_step(Direction::E, Direction::E));
        assert!(can_step(Direction::E, Direction::ENE));
        assert!(can_step(Direction::E, Direction::ESE));
        assert!(!can_step(Direction::E, Direction::NNE));
        assert!(!can_step(Direction::E, Direction::W));
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
    fn a_step_two_notches_off_the_facing_is_refused_and_the_body_stays_put() {
        let (mut app, body) = body_at(voxel(0, 0, 0), Direction::E, 4.0);
        app.world_mut()
            .entity_mut(body)
            .insert(Step(Direction::NNE));

        app.update();

        assert!(!is_stepping(&app, body));
        assert_eq!(position_of(&app, body), voxel(0, 0, 0));
        assert_eq!(facing_of(&app, body), Direction::E);
    }

    #[test]
    fn a_body_mid_step_is_still_in_the_voxel_it_left_from() {
        let (mut app, body) = body_at(voxel(0, 0, 0), Direction::E, 4.0);
        app.world_mut().entity_mut(body).insert(Step(Direction::E));

        (0..15).for_each(|_| app.update());

        assert!(is_stepping(&app, body));
        assert_eq!(position_of(&app, body), voxel(0, 0, 0));
    }

    /// A headless app: no window, no GPU. `MinimalPlugins` brings the schedules, and
    /// placing a model needs nothing else.
    fn app_with(position: VoxelCoord, facing: Direction) -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, MovementPlugin));
        let entity = app
            .world_mut()
            .spawn((VoxelPosition(position), Facing(facing)))
            .id();
        (app, entity)
    }

    fn transform_of(app: &App, entity: Entity) -> Transform {
        *app.world().get::<Transform>(entity).expect("a transform")
    }

    #[test]
    fn a_placed_entity_gets_a_transform_without_asking() {
        let (app, entity) = app_with(voxel(0, 0, 0), Direction::N);

        assert!(app.world().get::<Transform>(entity).is_some());
    }

    #[test]
    fn a_placed_entity_stands_at_its_voxels_centre_facing_its_heading() {
        let (mut app, entity) = app_with(voxel(1, -2, 1), Direction::N);

        app.update();

        let transform = transform_of(&app, entity);
        assert_eq!(transform.translation, voxel(1, -2, 1).to_world());
        assert!(transform.rotation.abs_diff_eq(Quat::IDENTITY, 1e-6));
    }

    #[test]
    fn turning_to_face_east_turns_the_model_to_face_east() {
        let (mut app, entity) = app_with(voxel(0, 0, 0), Direction::N);
        app.update();

        app.world_mut()
            .get_mut::<Facing>(entity)
            .expect("a facing")
            .0 = Direction::E;
        app.update();

        let faces = transform_of(&app, entity).forward();
        assert!(faces.abs_diff_eq(Vec3::X, 1e-5), "faces {faces}");
    }

    #[test]
    fn moving_to_another_voxel_moves_the_model_there() {
        let (mut app, entity) = app_with(voxel(0, 0, 0), Direction::N);
        app.update();

        app.world_mut()
            .get_mut::<VoxelPosition>(entity)
            .expect("a position")
            .0 = voxel(3, 0, 2);
        app.update();

        assert_eq!(
            transform_of(&app, entity).translation,
            voxel(3, 0, 2).to_world()
        );
    }

    #[test]
    fn a_standing_entity_is_left_alone() {
        let (mut app, entity) = app_with(voxel(0, 0, 0), Direction::N);
        app.update();
        let nudged = Transform::from_xyz(9.0, 9.0, 9.0);
        *app.world_mut()
            .get_mut::<Transform>(entity)
            .expect("a transform") = nudged;

        app.update();

        assert_eq!(transform_of(&app, entity), nudged);
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

//! The placeholder world: a ground, a sun and sky to see it by, a fox walking a
//! twelve-sided loop about the origin, and a hare walking a triangle beside it, so that
//! there is something to look at.
//!
//! Lengths are in shaku: one world unit is one 尺, about 30.3 cm. Metres appear only in
//! comments, to give a familiar sense of scale.
//!
//! This crate spawns no camera: seeing the world is another module's job.

use bevy::prelude::*;
use murabito_actions::{Action, ActionQueue, AskingSet};
use murabito_hexcoords::{Direction, VoxelCoord};
use murabito_kinds::{Fox, Hare};
use murabito_movement::{Facing, VoxelPosition};
use murabito_progress::Progress;
use std::time::Duration;

/// One 町 (cho): 360 shaku, about 109 m.
const GROUND_SIDE: f32 = 360.0;
const GROUND_COLOUR: Color = Color::srgb(0.35, 0.42, 0.30);

/// Where the sun shines from, toward the origin. Only the direction matters: a
/// directional light has no position, so every point is lit from the same angle.
const SUN_SHINES_FROM: Vec3 = Vec3::new(4.0, 8.0, 4.0);

/// What the camera paints where nothing else is drawn. Stands in for a sky.
const SKY_COLOUR: Color = Color::srgb(0.53, 0.81, 0.92);

/// Light that reaches everywhere equally, standing in for the sky's glow. Without it,
/// whatever faces away from the sun is pure black.
const SKY_GLOW: f32 = 200.0;

/// Which way the fox starts off facing: toward the camera and to its right, which shows
/// its face and its flank at once.
const FOX_FACES: Direction = Direction::ESE;

/// Where the hare starts: six shaku east of the fox, clear of its loop, facing east.
const HARE_STARTS_AT: (i32, i32, i32) = (6, 0, -6);
const HARE_FACES: Direction = Direction::E;

/// The hare walks a triangle: three edge steps a side, then a rest, then the next side,
/// which is a third of a turn on. The three directions sum to nothing, so the triangle
/// closes on the voxel it started from.
const HARE_TRIANGLE: [Direction; 3] = [Direction::E, Direction::NNW, Direction::SSW];
const HARE_SIDE_STEPS: usize = 3;
const HARE_REST: Duration = Duration::from_secs(1);

/// The progress bar drawn under a body with something in flight: on the ground just in
/// front of it, toward the camera, a shaku wide. Placeholder until a UI module owns it.
const BAR_WIDTH: f32 = 1.0;
const BAR_TOWARD_CAMERA: f32 = 0.8;
/// A hair above the ground, so it isn't lost in it.
const BAR_LIFT: f32 = 0.02;
/// In pixels: gizmo lines are drawn in screen space.
const BAR_THICKNESS: f32 = 6.0;
const BAR_EMPTY: Color = Color::srgb(0.15, 0.15, 0.15);
/// A desaturated mint.
const BAR_FULL: Color = Color::srgb(0.62, 0.85, 0.74);

pub struct ScenePlugin;

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(SKY_COLOUR))
            .insert_resource(GlobalAmbientLight {
                brightness: SKY_GLOW,
                ..default()
            })
            .add_systems(
                Startup,
                (spawn_ground, spawn_sun, spawn_fox, spawn_hare, thicken_bars),
            )
            .add_systems(FixedUpdate, (walk_the_fox, walk_the_hare).in_set(AskingSet))
            .add_systems(Update, draw_progress_bars);
    }
}

/// Marks the ground, so it can be found without guessing from its mesh.
#[derive(Component)]
struct Ground;

/// Marks the sun.
#[derive(Component)]
struct Sun;

fn spawn_ground(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let square = Plane3d::default().mesh().size(GROUND_SIDE, GROUND_SIDE);
    commands.spawn((
        Ground,
        Mesh3d(meshes.add(square)),
        MeshMaterial3d(materials.add(GROUND_COLOUR)),
    ));
}

fn spawn_sun(mut commands: Commands) {
    commands.spawn((
        Sun,
        DirectionalLight {
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_translation(SUN_SHINES_FROM).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// What a fox is, has and looks like is the kind's business; where this one stands and
/// which way it starts off facing is the scene's.
fn spawn_fox(mut commands: Commands) {
    let origin = VoxelCoord::new(0, 0, 0, 0).expect("the origin is on the plane");
    commands.spawn((Fox, VoxelPosition(origin), Facing(FOX_FACES)));
}

/// Which corner of its triangle the hare heads for next, and how long it has rested at
/// the last one. The scene's own bookkeeping for its placeholder walk.
#[derive(Component)]
struct TriangleWalk {
    corner: usize,
    rest: Timer,
}

impl Default for TriangleWalk {
    fn default() -> Self {
        Self {
            corner: 0,
            rest: Timer::new(HARE_REST, TimerMode::Once),
        }
    }
}

fn spawn_hare(mut commands: Commands) {
    let (q, r, s) = HARE_STARTS_AT;
    let start = VoxelCoord::new(q, r, s, 0).expect("the hare's start is on the plane");
    commands.spawn((
        Hare,
        VoxelPosition(start),
        Facing(HARE_FACES),
        TriangleWalk::default(),
    ));
}

/// Until something decides for it, the fox walks a loop: whenever its queue runs dry it
/// is asked to go each of the twelve directions in turn. Each is one notch on from the
/// last, so no step needs a turn first, and the twelve sum to nothing, so the loop
/// closes on the voxel it started from. The queue empties as the last step is issued,
/// while it is still in flight, so the refill never leaves a tick at rest.
fn walk_the_fox(mut foxes: Query<&mut ActionQueue, With<Fox>>) {
    for mut queue in &mut foxes {
        if queue.is_empty() {
            Direction::ALL
                .into_iter()
                .for_each(|direction| queue.push(Action::Go(direction)));
        }
    }
}

/// The hare walks a side, rests once the last step has landed, and is then asked to
/// walk the next side; the third of a turn between sides is `Go`'s own turn-then-step.
/// A rest is not an action: it is an AI choosing not to ask for anything, and this timer
/// stands in for that AI until there is one.
fn walk_the_hare(
    time: Res<Time>,
    mut hares: Query<(&mut ActionQueue, &Progress, &mut TriangleWalk), With<Hare>>,
) {
    for (mut queue, progress, mut walk) in &mut hares {
        if !queue.is_empty() || progress.in_flight() {
            continue;
        }
        walk.rest.tick(time.delta());
        if !walk.rest.is_finished() {
            continue;
        }
        let side = HARE_TRIANGLE[walk.corner];
        (0..HARE_SIDE_STEPS).for_each(|_| queue.push(Action::Go(side)));
        walk.corner = (walk.corner + 1) % HARE_TRIANGLE.len();
        walk.rest.reset();
    }
}

fn thicken_bars(mut config: ResMut<GizmoConfigStore>) {
    config.config_mut::<DefaultGizmoConfigGroup>().0.line.width = BAR_THICKNESS;
}

/// Until a UI module owns it: a bar under every body with something in flight, drawn
/// with gizmos, which are lines redrawn each frame and need no mesh. Two lines end to
/// end, the part done and the part still to do: they must not overlap, since gizmo
/// lines are depth-tested and one drawn over another at the same depth loses to it. It
/// reads only `Progress`, so it shows a step, a notch of a turn, or anything a later
/// mechanism does, without knowing which.
fn draw_progress_bars(mut gizmos: Gizmos, bodies: Query<(&Transform, &Progress)>) {
    for (transform, progress) in &bodies {
        if !progress.in_flight() {
            continue;
        }
        let left = transform.translation + Vec3::new(-BAR_WIDTH / 2.0, BAR_LIFT, BAR_TOWARD_CAMERA);
        let right = left + Vec3::X * BAR_WIDTH;
        let done_to = left + Vec3::X * BAR_WIDTH * progress.fraction();
        gizmos.line(left, done_to, BAR_FULL);
        gizmos.line(done_to, right, BAR_EMPTY);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A headless app, run for one frame: no window, no GPU. `MinimalPlugins` brings the
    /// schedules; the asset stores are what the spawn systems ask for, and in the real
    /// app they arrive with `DefaultPlugins`. No glTF loader is registered, so the fox's
    /// handle never resolves to a model, which no test here needs. Gizmos want their
    /// asset store and a config group registered, which `GizmoPlugin` would do.
    fn app_after_startup() -> App {
        use bevy::gizmos::AppGizmoBuilder;

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Mesh>()
            .init_asset::<StandardMaterial>()
            .init_asset::<WorldAsset>()
            .init_asset::<bevy::gizmos::GizmoAsset>()
            .init_gizmo_group::<DefaultGizmoConfigGroup>()
            .add_plugins(ScenePlugin);
        app.update();
        app
    }

    fn count<Marker: Component>(app: &mut App) -> usize {
        let world = app.world_mut();
        world
            .query_filtered::<(), With<Marker>>()
            .iter(world)
            .count()
    }

    #[test]
    fn the_scene_is_one_ground_and_one_sun() {
        let mut app = app_after_startup();

        assert_eq!(count::<Ground>(&mut app), 1);
        assert_eq!(count::<Sun>(&mut app), 1);
    }

    #[test]
    fn a_fox_stands_at_the_origin_facing_toward_the_camera_and_to_its_right() {
        let mut app = app_after_startup();
        let world = app.world_mut();

        let (position, facing) = world
            .query_filtered::<(&VoxelPosition, &Facing), With<Fox>>()
            .single(world)
            .expect("exactly one fox");

        assert_eq!(position.0.to_world(), Vec3::ZERO);
        assert_eq!(facing.0, Direction::ESE);
    }

    #[test]
    fn a_hare_starts_six_shaku_east_of_the_fox_facing_east() {
        let mut app = app_after_startup();
        let world = app.world_mut();

        let (position, facing) = world
            .query_filtered::<(&VoxelPosition, &Facing), With<Hare>>()
            .single(world)
            .expect("exactly one hare");

        assert_eq!(position.0.to_world(), Vec3::new(6.0, 0.0, 0.0));
        assert_eq!(facing.0, Direction::E);
    }

    #[test]
    fn the_sky_is_what_the_screen_is_cleared_to() {
        let app = app_after_startup();

        assert_eq!(app.world().resource::<ClearColor>().0, SKY_COLOUR);
    }

    #[test]
    fn the_ground_has_a_mesh_and_a_material() {
        let mut app = app_after_startup();
        let world = app.world_mut();

        let drawable = world
            .query_filtered::<(), (
                With<Ground>,
                With<Mesh3d>,
                With<MeshMaterial3d<StandardMaterial>>,
            )>()
            .iter(world)
            .count();

        assert_eq!(drawable, 1);
    }

    #[test]
    fn the_sun_shines_downward() {
        let mut app = app_after_startup();
        let world = app.world_mut();

        let sun = world
            .query_filtered::<&Transform, With<Sun>>()
            .single(world)
            .expect("exactly one sun");

        assert!(sun.forward().y < 0.0, "the sun points {:?}", sun.forward());
    }

    #[test]
    fn the_fox_walks_a_twelve_sided_loop_and_comes_home() {
        use bevy::gizmos::AppGizmoBuilder;
        use bevy::time::TimeUpdateStrategy;
        use murabito_actions::ActionsPlugin;
        use murabito_movement::MovementPlugin;
        use murabito_progress::ProgressPlugin;

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Mesh>()
            .init_asset::<StandardMaterial>()
            .init_asset::<WorldAsset>()
            .init_asset::<bevy::gizmos::GizmoAsset>()
            .init_gizmo_group::<DefaultGizmoConfigGroup>()
            .add_plugins((ProgressPlugin, MovementPlugin, ActionsPlugin, ScenePlugin))
            .insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
        app.update();
        let fox_at = |app: &mut App| {
            let world = app.world_mut();
            world
                .query_filtered::<&VoxelPosition, With<Fox>>()
                .single(world)
                .expect("exactly one fox")
                .0
        };
        let home = fox_at(&mut app);
        let mut visited = std::collections::HashSet::from([home]);
        let mut left_home = false;

        // Six edges and six corners at 4 shaku/s: 6 + 6√3 = 16.39 shaku, 262.3 ticks of
        // 1/16 shaku, so the last step lands on tick 263.
        let came_home_on = (1..=400).find(|_| {
            app.update();
            let here = fox_at(&mut app);
            visited.insert(here);
            left_home |= here != home;
            left_home && here == home
        });

        assert_eq!(came_home_on, Some(263));
        assert_eq!(visited.len(), 12, "the fox visited only {visited:?}");
    }

    #[test]
    fn the_hare_walks_a_triangle_resting_at_each_corner_and_comes_home() {
        use bevy::gizmos::AppGizmoBuilder;
        use bevy::time::TimeUpdateStrategy;
        use murabito_actions::ActionsPlugin;
        use murabito_movement::MovementPlugin;
        use murabito_progress::ProgressPlugin;

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Mesh>()
            .init_asset::<StandardMaterial>()
            .init_asset::<WorldAsset>()
            .init_asset::<bevy::gizmos::GizmoAsset>()
            .init_gizmo_group::<DefaultGizmoConfigGroup>()
            .add_plugins((ProgressPlugin, MovementPlugin, ActionsPlugin, ScenePlugin))
            .insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
        app.update();
        let hare = |app: &mut App| {
            let world = app.world_mut();
            let (position, facing) = world
                .query_filtered::<(&VoxelPosition, &Facing), With<Hare>>()
                .single(world)
                .expect("exactly one hare");
            (position.0, facing.0)
        };
        let (home, _) = hare(&mut app);
        let mut visited = std::collections::HashSet::from([home]);
        let mut ticks_at_rest_at_home = 0;
        let mut left_home = false;
        let mut triangle = std::collections::HashSet::new();
        let mut corner = home;
        for side in HARE_TRIANGLE {
            for _ in 0..HARE_SIDE_STEPS {
                corner = corner.neighbour(side);
                triangle.insert(corner);
            }
        }

        // A side is 3 shaku at 5 shaku/s, 38.4 ticks; a corner is a third of a turn at
        // 240°/s, 32 ticks; a rest is 64 ticks. A lap is well under 500 ticks, so by 600
        // the hare has been home, rested, and set off again.
        for _ in 1..=600 {
            app.update();
            let (here, _) = hare(&mut app);
            visited.insert(here);
            left_home |= here != home;
            if left_home && here == home {
                ticks_at_rest_at_home += 1;
            }
        }

        assert_eq!(visited, triangle, "three sides of three, and nowhere else");
        assert!(
            ticks_at_rest_at_home >= 64,
            "it came home and rested there a whole second, not {ticks_at_rest_at_home} ticks"
        );
    }
}

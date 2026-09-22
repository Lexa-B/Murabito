//! The placeholder world: a ground, a sun and sky to see it by, a fox walking a
//! twelve-sided loop west of the origin, a hare walking a triangle east of it, and a tree
//! between them, so that there is something to look at and something in the way.
//!
//! Lengths are in shaku: one world unit is one 尺, about 30.3 cm. Metres appear only in
//! comments, to give a familiar sense of scale.
//!
//! This crate spawns no camera: seeing the world is another module's job.

use bevy::prelude::*;
use murabito_actions::{Action, ActionQueue, AskingSet};
use murabito_hexcoords::{Direction, VoxelCoord};
use murabito_kinds::{Fox, Hare, Sugi};
use murabito_placement::{Facing, VoxelPosition};
use murabito_progress::Progress;
use murabito_vision::{Acuity, Seen, Vision};
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

/// Where the fox starts: eight cells west of the origin, so that its loop, which runs
/// five cells east and six north of its start, stays well west of the tree; and which
/// way it starts off facing: toward the camera and to its right, which shows its face
/// and its flank at once.
const FOX_STARTS_AT: (i32, i32, i32) = (-8, 0, 8);
const FOX_FACES: Direction = Direction::ESE;

/// Where the hare starts: six cells east of the origin, on the fox's line and clear of
/// both walks, facing east.
const HARE_STARTS_AT: (i32, i32, i32) = (6, 0, -6);
const HARE_FACES: Direction = Direction::E;

/// The hare walks a triangle: three edge steps a side, then a rest, then the next side,
/// which is a third of a turn on. The three directions sum to nothing, so the triangle
/// closes on the voxel it started from.
const HARE_TRIANGLE: [Direction; 3] = [Direction::E, Direction::NNW, Direction::SSW];
const HARE_SIDE_STEPS: usize = 3;
const HARE_REST: Duration = Duration::from_secs(1);

/// Where the tree stands: two corner steps north of the line from the fox's start to
/// the hare's, clear of both walks, and in the fox's cone from where it starts.
const TREE_AT: (i32, i32, i32) = (5, -4, -1);

/// Sightlines and cones are drawn per looker in its own colour: not the species' hue,
/// so the two stay tellable apart where they cross.
const FOX_SIGHT: Color = Color::srgb(1.0, 0.45, 0.1);
const HARE_SIGHT: Color = Color::srgb(0.35, 0.8, 1.0);
/// How solid a sightline is, by how well the thing was seen.
const SIGHT_ALPHA: [f32; 3] = [1.0, 0.55, 0.25];
const CONE_ALPHA: f32 = 0.35;
/// Sightlines run at about eye height, clear of the ground and the bar; the cone is
/// drawn on the ground, a hair above it.
const SIGHT_LIFT: f32 = 0.6;
const CONE_LIFT: f32 = 0.03;

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
                (
                    spawn_ground,
                    spawn_sun,
                    spawn_fox,
                    spawn_hare,
                    spawn_tree,
                    thicken_bars,
                ),
            )
            .add_systems(FixedUpdate, (walk_the_fox, walk_the_hare).in_set(AskingSet))
            .add_systems(Update, (draw_progress_bars, draw_cones, draw_sightings));
    }
}

/// Marks the ground, so it can be found without guessing from its mesh.
#[derive(Component)]
struct Ground;

/// Marks the sun.
#[derive(Component)]
struct Sun;

/// The colour a looker's sightlines and cone are drawn in. The scene's, not the
/// senses': what a body sees is the same whatever colour it is drawn in.
#[derive(Component, Clone, Copy)]
struct SightColour(Color);

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
    let (q, r, s) = FOX_STARTS_AT;
    let start = VoxelCoord::new(q, r, s, 0).expect("the fox's start is on the plane");
    commands.spawn((
        Fox,
        VoxelPosition(start),
        Facing(FOX_FACES),
        SightColour(FOX_SIGHT),
    ));
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
        SightColour(HARE_SIGHT),
        TriangleWalk::default(),
    ));
}

/// Something in the way. A tree is a thing with a place and nothing else: it neither
/// faces nor sees, and it stands where it is put.
fn spawn_tree(mut commands: Commands) {
    let (q, r, s) = TREE_AT;
    let at = VoxelCoord::new(q, r, s, 0).expect("the tree's place is on the plane");
    commands.spawn((Sugi, VoxelPosition(at)));
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

/// Until a debug module owns it: the bare cone of every sighted body, drawn on the
/// ground in its colour, three arcs at the bands' ranges and the two edges. What the
/// eye could see with nothing in the way; the sightlines say what it does see. Reads
/// only the public `Vision`, `Facing` and the body's `Transform`.
fn draw_cones(mut gizmos: Gizmos, lookers: Query<(&Transform, &Facing, &Vision, &SightColour)>) {
    for (transform, facing, vision, colour) in &lookers {
        if vision.is_blind() {
            continue;
        }
        let eye = transform.translation.with_y(CONE_LIFT);
        let colour = colour.0.with_alpha(CONE_ALPHA);
        // A direction's bearing is its index in turns of 30° from +X about Y, the same
        // sense an arc gizmo sweeps in from its own +X, so the arc starts at the cone's
        // clockwise edge and sweeps the cone's width anticlockwise.
        let bearing = f32::from(facing.0.index()) * 30.0;
        let half_arc = vision.arc * 0.5;
        let edge_from = Quat::from_rotation_y((bearing - half_arc).to_radians());
        let edge_to = Quat::from_rotation_y((bearing + half_arc).to_radians());
        for acuity in Acuity::ALL {
            let range = vision.bands[acuity as usize].range as f32;
            gizmos
                .arc_3d(
                    vision.arc.to_radians(),
                    range,
                    Isometry3d::new(eye, edge_from),
                    colour,
                )
                .resolution(64);
        }
        let reach = vision.far_range() as f32;
        gizmos.line(eye, eye + edge_from * Vec3::X * reach, colour);
        gizmos.line(eye, eye + edge_to * Vec3::X * reach, colour);
    }
}

/// Until a debug module owns it: a line from every sighted body to each thing it sees,
/// in the looker's colour, solid when seen sharply and fainter with distance. Reads
/// only the public `Seen`: what is drawn is exactly what the AI will be handed.
fn draw_sightings(
    mut gizmos: Gizmos,
    lookers: Query<(&Transform, &Seen, &SightColour)>,
    placed: Query<&Transform>,
) {
    for (transform, seen, colour) in &lookers {
        let from = transform.translation.with_y(SIGHT_LIFT);
        for sighting in seen.iter() {
            let Ok(target) = placed.get(sighting.entity) else {
                continue;
            };
            let to = target.translation.with_y(SIGHT_LIFT);
            let alpha = SIGHT_ALPHA[sighting.acuity as usize];
            gizmos.line(from, to, colour.0.with_alpha(alpha));
        }
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

    /// The tree stands where the scene says, and the hare's triangle never enters its
    /// cell nor the ring of cells around it, which is about where the canopy ends.
    #[test]
    fn a_tree_stands_north_of_the_hare_clear_of_its_triangle() {
        let mut app = app_after_startup();
        let world = app.world_mut();

        let tree = world
            .query_filtered::<&VoxelPosition, With<Sugi>>()
            .single(world)
            .expect("exactly one tree")
            .0;

        let (q, r, s) = TREE_AT;
        assert_eq!(tree, VoxelCoord::new(q, r, s, 0).expect("on the plane"));
        let (hq, hr, hs) = HARE_STARTS_AT;
        let mut here = VoxelCoord::new(hq, hr, hs, 0).expect("on the plane");
        for side in HARE_TRIANGLE {
            for _ in 0..HARE_SIDE_STEPS {
                here = here.neighbour(side);
                assert!(here.distance(tree) > 1, "the triangle passes {here:?}");
            }
        }
    }

    /// The fox's loop never enters the tree's cell, nor the ring of cells around it,
    /// which is about where the canopy ends.
    #[test]
    fn the_foxs_loop_keeps_clear_of_the_tree() {
        let (q, r, s) = TREE_AT;
        let tree = VoxelCoord::new(q, r, s, 0).expect("on the plane");
        let (fq, fr, fs) = FOX_STARTS_AT;
        let mut here = VoxelCoord::new(fq, fr, fs, 0).expect("on the plane");

        for direction in Direction::ALL {
            here = here.neighbour(direction);
            assert!(
                here.distance(tree) > 1,
                "the loop passes {here:?}, next to the tree"
            );
        }
    }

    /// The starting tableau, as a claim rather than a screenshot: the fox faces east
    /// across the ground and sees the tree, near, and the hare beyond it, less well; the
    /// hare faces away and sees neither.
    #[test]
    fn at_the_start_the_fox_sees_the_tree_near_and_the_hare_beyond_it() {
        use bevy::gizmos::AppGizmoBuilder;
        use bevy::time::TimeUpdateStrategy;
        use murabito_perception::PerceptionPlugin;
        use murabito_vision::VisionPlugin;

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Mesh>()
            .init_asset::<StandardMaterial>()
            .init_asset::<WorldAsset>()
            .init_asset::<bevy::gizmos::GizmoAsset>()
            .init_gizmo_group::<DefaultGizmoConfigGroup>()
            .add_plugins((PerceptionPlugin, VisionPlugin, ScenePlugin))
            .insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
        app.update();
        app.update();

        let world = app.world_mut();
        let fox = world
            .query_filtered::<Entity, With<Fox>>()
            .single(world)
            .expect("a fox");
        let hare = world
            .query_filtered::<Entity, With<Hare>>()
            .single(world)
            .expect("a hare");
        let tree = world
            .query_filtered::<Entity, With<Sugi>>()
            .single(world)
            .expect("a tree");
        let fox_sees = world.get::<Seen>(fox).expect("the fox has eyes");
        assert_eq!(
            fox_sees.sees(tree),
            Some(Acuity::Near),
            "the tree is eleven and a half shaku off"
        );
        assert_eq!(
            fox_sees.sees(hare),
            Some(Acuity::Mid),
            "the hare is fourteen shaku off"
        );
        let hare_sees = world.get::<Seen>(hare).expect("the hare has eyes");
        assert!(hare_sees.is_empty(), "the hare faces east, away from both");
    }

    #[test]
    fn the_scene_is_one_ground_and_one_sun() {
        let mut app = app_after_startup();

        assert_eq!(count::<Ground>(&mut app), 1);
        assert_eq!(count::<Sun>(&mut app), 1);
    }

    #[test]
    fn a_fox_stands_eight_cells_west_of_the_origin_facing_toward_the_camera_and_to_its_right() {
        let mut app = app_after_startup();
        let world = app.world_mut();

        let (position, facing) = world
            .query_filtered::<(&VoxelPosition, &Facing), With<Fox>>()
            .single(world)
            .expect("exactly one fox");

        assert_eq!(position.0.to_world(), Vec3::new(-8.0, 0.0, 0.0));
        assert_eq!(facing.0, Direction::ESE);
    }

    #[test]
    fn a_hare_starts_six_cells_east_of_the_origin_facing_east() {
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

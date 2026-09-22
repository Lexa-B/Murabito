//! The placeholder world: a ground, a sun and sky to see it by, and a fox standing at
//! the origin so that there is something to look at.
//!
//! Lengths are in shaku: one world unit is one 尺, about 30.3 cm. Metres appear only in
//! comments, to give a familiar sense of scale.
//!
//! This crate spawns no camera: seeing the world is another module's job.

use bevy::prelude::*;
use murabito_hexcoords::{Direction, VoxelCoord};
use murabito_movement::{Facing, Locomotion, Turn, VoxelPosition};
use murabito_progress::MechanismSet;

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

/// Relative to `assets/`. The model is in shaku, faces forward (-Z) and stands with
/// its soles at y = 0, so it needs no scaling or righting: see `art/ART-README.md`.
const FOX_MODEL: &str = "models/fox.glb";

/// Which way the fox starts off facing: toward the camera and to its right, which shows
/// its face and its flank at once.
const FOX_FACES: Direction = Direction::ESE;

/// How the fox moves, until something more considered decides: a brisk walk of 4 shaku
/// (about 1.2 m) a second, and half a turn a second.
const FOX_LOCOMOTION: Locomotion = Locomotion {
    speed: 4.0,
    turn_speed: 180.0,
};

pub struct ScenePlugin;

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(SKY_COLOUR))
            .insert_resource(GlobalAmbientLight {
                brightness: SKY_GLOW,
                ..default()
            })
            .add_systems(Startup, (spawn_ground, spawn_sun, spawn_fox))
            .add_systems(FixedUpdate, spin_the_fox.before(MechanismSet));
    }
}

/// Marks the ground, so it can be found without guessing from its mesh.
#[derive(Component)]
struct Ground;

/// Marks the sun.
#[derive(Component)]
struct Sun;

/// Marks the fox.
#[derive(Component)]
struct Fox;

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

fn spawn_fox(mut commands: Commands, assets: Res<AssetServer>) {
    let model = assets.load(GltfAssetLabel::Scene(0).from_asset(FOX_MODEL));
    let origin = VoxelCoord::new(0, 0, 0, 0).expect("the origin is on the plane");
    commands.spawn((
        Fox,
        WorldAssetRoot(model),
        VoxelPosition(origin),
        Facing(FOX_FACES),
        FOX_LOCOMOTION,
    ));
}

/// Until the actions layer exists, the fox spins on the spot: whenever it has no turn in
/// flight it is asked to face the opposite way, and a half turn always goes
/// anticlockwise, so it keeps going round. Runs before the mechanisms so the new turn
/// starts on the tick the last one ended, with no tick at rest between.
/// A fox with no turn in flight.
type IdleFox = (With<Fox>, Without<Turn>);

fn spin_the_fox(mut commands: Commands, foxes: Query<(Entity, &Facing), IdleFox>) {
    for (fox, facing) in &foxes {
        commands.entity(fox).insert(Turn(facing.0.rotated(6)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A headless app, run for one frame: no window, no GPU. `MinimalPlugins` brings the
    /// schedules; the asset stores are what the spawn systems ask for, and in the real
    /// app they arrive with `DefaultPlugins`. No glTF loader is registered, so the fox's
    /// handle never resolves to a model, which no test here needs.
    fn app_after_startup() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Mesh>()
            .init_asset::<StandardMaterial>()
            .init_asset::<WorldAsset>()
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

    /// The model is made by the art pipeline, not by this crate. A rename there would
    /// otherwise show up only as a fox quietly missing from the screen.
    #[test]
    fn the_fox_model_is_where_the_scene_looks_for_it() {
        let model = bevy::asset::io::file::FileAssetReader::get_base_path()
            .join("assets")
            .join(FOX_MODEL);

        assert!(model.is_file(), "no model at {}", model.display());
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
    fn the_fox_keeps_turning() {
        use bevy::time::TimeUpdateStrategy;
        use murabito_movement::MovementPlugin;
        use murabito_progress::ProgressPlugin;

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Mesh>()
            .init_asset::<StandardMaterial>()
            .init_asset::<WorldAsset>()
            .add_plugins((ProgressPlugin, MovementPlugin, ScenePlugin))
            .insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
        app.update();
        let mut seen = std::collections::HashSet::new();

        for _ in 0..512 {
            app.update();
            let world = app.world_mut();
            let facing = world
                .query_filtered::<&Facing, With<Fox>>()
                .single(world)
                .expect("exactly one fox");
            seen.insert(facing.0);
        }

        assert_eq!(seen.len(), 12, "the fox faced only {seen:?}");
    }
}

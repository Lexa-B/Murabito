//! The placeholder world: a ground to stand things on, and a sun to see them by.
//!
//! Lengths are in shaku: one world unit is one 尺, about 30.3 cm. Metres appear only in
//! comments, to give a familiar sense of scale.
//!
//! This crate spawns no camera. Without one nothing is drawn, so until a camera module
//! exists the window stays grey; the tests below are what show the scene is there.

use bevy::prelude::*;

/// One 町 (cho): 360 shaku, about 109 m.
const GROUND_SIDE: f32 = 360.0;
const GROUND_COLOUR: Color = Color::srgb(0.35, 0.42, 0.30);

/// Where the sun shines from, toward the origin. Only the direction matters: a
/// directional light has no position, so every point is lit from the same angle.
const SUN_SHINES_FROM: Vec3 = Vec3::new(4.0, 8.0, 4.0);

/// What the camera paints where nothing else is drawn. Stands in for a sky.
const SKY_COLOUR: Color = Color::srgb(0.53, 0.81, 0.92);

pub struct ScenePlugin;

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(SKY_COLOUR))
            .add_systems(Startup, (spawn_ground, spawn_sun));
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A headless app, run for one frame: no window, no GPU. `MinimalPlugins` brings the
    /// schedules; the asset stores are what `spawn_ground` asks for, and in the real app
    /// they arrive with `DefaultPlugins`.
    fn app_after_startup() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Mesh>()
            .init_asset::<StandardMaterial>()
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
}

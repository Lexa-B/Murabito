//! The starter scene: ground, one cube, and a sun. Placeholder content — the point of
//! it is to give the camera something to look at.

use bevy::prelude::*;

/// The colour the framebuffer is cleared to each frame: everything the camera doesn't
/// draw over. Stands in for a sky.
const SKY: Color = Color::srgb(0.53, 0.81, 0.92);

pub struct ScenePlugin;

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(SKY))
            .add_systems(Startup, spawn_scene)
            // Not gated on `AppState`: it stops because the clock stops, which is the
            // point of pausing `Time<Virtual>` rather than gating every system by hand.
            .add_systems(Update, spin);
    }
}

/// Marks something that turns slowly on the spot. Placeholder motion: without it the
/// scene is completely static and a pause would be impossible to see.
#[derive(Component)]
struct Spinner;

/// A quarter turn a second, in radians.
const SPIN_RATE: f32 = std::f32::consts::FRAC_PI_2;

/// Runs once, before the first frame. `Commands` queues entity spawns; the two
/// `ResMut<Assets<_>>` are the engine's mesh and material stores — `add` uploads an
/// asset and hands back a `Handle`, which is what an entity actually carries.
fn spawn_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Ground: 20 x 20 m, centred on the origin, facing up (Y is up in Bevy).
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(20.0, 20.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.35, 0.42, 0.30))),
    ));

    // A 1 m cube, lifted half its height so it sits on the ground rather than in it.
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.85, 0.45, 0.20))),
        Transform::from_xyz(0.0, 0.5, 0.0),
        Spinner,
    ));

    // Sun. A directional light has no position — only a direction, which is why this is
    // aimed with `looking_at` and the translation only serves to set that angle.
    commands.spawn((
        DirectionalLight {
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(-10.0, 14.0, -4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// Turns every `Spinner` on the spot. `time.delta_secs()` is zero while the game is
/// paused, so this keeps running and simply moves nothing.
fn spin(time: Res<Time>, mut spinners: Query<&mut Transform, With<Spinner>>) {
    for mut transform in &mut spinners {
        transform.rotate_y(SPIN_RATE * time.delta_secs());
    }
}

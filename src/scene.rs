//! The starter scene: ground, one cube, and a sun. Placeholder content — the point of
//! it is to give the camera something to look at.

use bevy::prelude::*;

use crate::諸法::種;

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
    // Ground: one cho square - 360 shaku, about 109 m - centred on the origin, Y up.
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(360.0, 360.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.35, 0.42, 0.30))),
    ));

    // A 3.3 shaku cube (1 m), lifted half its height so it sits on the ground.
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(3.3, 3.3, 3.3))),
        MeshMaterial3d(materials.add(Color::srgb(0.85, 0.45, 0.20))),
        Transform::from_xyz(0.0, 1.65, 0.0),
        Spinner,
    ));

    // Plants, so that sight has something to stop against. They carry a 種 and nothing
    // else: what they do to a sense is read off the taxonomy, which is why neither the
    // mesh nor any marker component says anything about blocking.
    //
    // Placed across the line from the fox to the rabbit, grass first and trees beyond
    // it, so that costing sight a band and stopping it outright show up in one picture.
    let trunk = meshes.add(Cuboid::new(0.9, 8.0, 0.9));
    let bark = materials.add(Color::srgb(0.24, 0.30, 0.18));
    for (x, z) in [(3.0, -2.6), (4.6, -3.8), (6.2, -1.8)] {
        commands.spawn((
            種::new("木"),
            Mesh3d(trunk.clone()),
            MeshMaterial3d(bark.clone()),
            Transform::from_xyz(x, 4.0, z),
        ));
    }

    let tuft = meshes.add(Cuboid::new(0.9, 1.6, 0.9));
    let blade = materials.add(Color::srgb(0.46, 0.60, 0.28));
    for (x, z) in [(-8.0, 4.6), (-7.0, 3.8), (-6.4, 5.2), (-8.8, 3.6)] {
        commands.spawn((
            種::new("草"),
            Mesh3d(tuft.clone()),
            MeshMaterial3d(blade.clone()),
            Transform::from_xyz(x, 0.8, z),
        ));
    }

    // Sun. A directional light has no position — only a direction, which is why this is
    // aimed with `looking_at` and the translation only serves to set that angle.
    commands.spawn((
        DirectionalLight {
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(-33.0, 46.0, -13.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// Turns every `Spinner` on the spot. `time.delta_secs()` is zero while the game is
/// paused, so this keeps running and simply moves nothing.
fn spin(time: Res<Time>, mut spinners: Query<&mut Transform, With<Spinner>>) {
    for mut transform in &mut spinners {
        transform.rotate_y(SPIN_RATE * time.delta_secs());
    }
}

//! The starter scene: ground, plants, and a sun.
//!
//! Nothing here moves. The spinning cube that used to stand in for motion is gone — it
//! existed so a pause was visible, and pausing is covered by `tests/pause.rs` rather
//! than by watching a box. Bite 0i puts motion back as an idle turn, which shows
//! something true about a being instead of standing in for it.

use bevy::prelude::*;

use crate::諸法::種;

/// Where the trees stand, in shaku. Across the line from the fox to the rabbit, so that
/// the one cannot simply look at the other — see `being`'s tableau tests, which assert
/// that arrangement rather than leaving it to a screenshot.
pub(crate) const TREES: [(f32, f32); 3] = [(3.0, -2.6), (4.6, -3.8), (6.2, -1.8)];

/// Where the grass grows. Nearer the fox than the trees are, so that costing sight a
/// band and stopping it outright are separable in one picture.
pub(crate) const GRASS: [(f32, f32); 4] = [(-8.0, 4.6), (-7.0, 3.8), (-6.4, 5.2), (-8.8, 3.6)];

/// The colour the framebuffer is cleared to each frame: everything the camera doesn't
/// draw over. Stands in for a sky.
const SKY: Color = Color::srgb(0.53, 0.81, 0.92);

pub struct ScenePlugin;

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(SKY))
            .add_systems(Startup, spawn_scene);
    }
}

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

    // Plants, so that sight has something to stop against. They carry a 種 and nothing
    // else: what they do to a sense is read off the taxonomy, which is why neither the
    // mesh nor any marker component says anything about blocking.
    //
    // Placed across the line from the fox to the rabbit, grass first and trees beyond
    // it, so that costing sight a band and stopping it outright show up in one picture.
    let trunk = meshes.add(Cuboid::new(0.9, 8.0, 0.9));
    let bark = materials.add(Color::srgb(0.24, 0.30, 0.18));
    for (x, z) in TREES {
        commands.spawn((
            種::new("木"),
            Mesh3d(trunk.clone()),
            MeshMaterial3d(bark.clone()),
            Transform::from_xyz(x, 4.0, z),
        ));
    }

    let tuft = meshes.add(Cuboid::new(0.9, 1.6, 0.9));
    let blade = materials.add(Color::srgb(0.46, 0.60, 0.28));
    for (x, z) in GRASS {
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

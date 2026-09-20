//! One entity per chunk on screen.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use hexworld::{mesh::MeshData, plane::cell_centre_m, ChunkKey, Level, WorldConfig};

use crate::axes::{mesh_position, to_bevy};

#[derive(Component, Clone, Copy, Debug)]
pub struct ChunkView(pub ChunkKey);

pub fn setup_material(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>) {
    let handle = materials.add(StandardMaterial {
        perceptual_roughness: 0.95,
        ..default()
    });
    commands.insert_resource(crate::GroundMaterialHandle(handle));
}

/// Build a Bevy mesh from the core's data, mapping the axes once.
pub fn to_bevy_mesh(data: &MeshData) -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        data.positions
            .iter()
            .map(|p| mesh_position(*p))
            .collect::<Vec<[f32; 3]>>(),
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_NORMAL,
        data.normals
            .iter()
            .map(|n| mesh_position(*n))
            .collect::<Vec<[f32; 3]>>(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, data.colours.clone());
    // Line data rides in the UVs, so no custom vertex shader is needed:
    // UV0 = (edge_w, border_level), UV1 = (cell_level, 0).
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_UV_0,
        data.line
            .iter()
            .map(|l| [l[0], l[1]])
            .collect::<Vec<[f32; 2]>>(),
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_UV_1,
        data.line
            .iter()
            .map(|l| [l[2], 0.0])
            .collect::<Vec<[f32; 2]>>(),
    );
    mesh.insert_indices(Indices::U32(data.indices.clone()));
    mesh
}

/// Where a chunk's mesh sits in the world: the centre of its own cell.
pub fn chunk_origin(key: ChunkKey) -> Vec3 {
    if key.level == Level::World {
        Vec3::ZERO
    } else {
        let (east, north) = cell_centre_m(key.cell, key.level);
        to_bevy(east, north, 0.0)
    }
}

pub fn spawn_chunk_entity(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    material: Handle<StandardMaterial>,
    key: ChunkKey,
    data: &MeshData,
    _cfg: &WorldConfig,
) -> Entity {
    let handle = meshes.add(to_bevy_mesh(data));
    commands
        .spawn((
            Mesh3d(handle),
            MeshMaterial3d(material),
            Transform::from_translation(chunk_origin(key)),
            ChunkView(key),
        ))
        .id()
}

pub fn despawn_unloaded(
    commands: &mut Commands,
    views: &Query<(Entity, &ChunkView)>,
    unloaded: &[ChunkKey],
) {
    for (entity, view) in views.iter() {
        if unloaded.contains(&view.0) {
            commands.entity(entity).despawn();
        }
    }
}

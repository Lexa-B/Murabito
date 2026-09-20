//! Generation and meshing on Bevy's background task pool. Chunks are pure functions of
//! the config and the key, so a job needs nothing from the main thread and can finish in
//! any order.

use std::collections::HashSet;

use bevy::prelude::*;
use bevy::tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task};
use hexworld::{mesh::MeshData, Chunk, ChunkKey, WorldConfig};

use crate::{entities, HexWorld};

pub struct JobOutput {
    pub key: ChunkKey,
    pub chunk: Chunk,
    pub mesh: MeshData,
}

#[derive(Component)]
pub struct ChunkJob(pub Task<JobOutput>);

/// Start generating the chunks the store asked for, up to the in-flight cap.
pub fn spawn_jobs(commands: &mut Commands, world: &mut HexWorld, to_load: Vec<ChunkKey>) {
    let cap = world.store().settings().max_in_flight;
    let running = world.store().in_flight_count();
    let pool = AsyncComputeTaskPool::get();
    for key in to_load.into_iter().take(cap.saturating_sub(running)) {
        let cfg: WorldConfig = *world.store().config();
        world.store_mut().begin_load(key);
        let task = pool.spawn(async move {
            let chunk = hexworld::chunk::generate(&cfg, key);
            let mesh = hexworld::mesh::mesh_chunk(&cfg, &chunk, &HashSet::new());
            JobOutput { key, chunk, mesh }
        });
        commands.spawn(ChunkJob(task));
    }
}

/// Take finished jobs, store the chunk, and put it on screen.
pub fn collect_finished_jobs(
    mut commands: Commands,
    time: Res<Time>,
    mut world: ResMut<HexWorld>,
    mut meshes: ResMut<Assets<Mesh>>,
    material: Option<Res<crate::GroundMaterialHandle>>,
    mut jobs: Query<(Entity, &mut ChunkJob)>,
) {
    let now = time.elapsed_secs_f64();
    for (entity, mut job) in &mut jobs {
        let Some(output) = block_on(future::poll_once(&mut job.0)) else {
            continue;
        };
        commands.entity(entity).despawn();
        if !world.store_mut().insert(output.chunk, now) {
            continue; // nobody wants it any more
        }
        if let Some(material) = material.as_ref() {
            entities::spawn_chunk_entity(
                &mut commands,
                &mut meshes,
                material.0.clone(),
                output.key,
                &output.mesh,
                world.store().config(),
            );
        }
    }
}

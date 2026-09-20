//! Generation and meshing on Bevy's background task pool. Chunks are pure functions of
//! the config and the key, so a job needs nothing from the main thread and can finish in
//! any order.

use bevy::prelude::*;
use bevy::tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task};
use hexworld::{mesh::MeshData, Chunk, ChunkKey, WorldConfig};

use crate::{entities, entities::Shown, HexWorld};

pub struct JobOutput {
    pub key: ChunkKey,
    pub chunk: Chunk,
    pub mesh: MeshData,
}

#[derive(Component)]
pub struct ChunkJob(pub Task<JobOutput>);

/// Start generating the chunks the store asked for, up to the in-flight cap. Each job
/// meshes with the chunk's *current* omissions, so a chunk that arrives while its
/// children (or a shown sibling that owns one of its border cells) are already on screen
/// does not draw over them from the first frame it appears.
pub fn spawn_jobs(
    commands: &mut Commands,
    world: &mut HexWorld,
    shown: &Shown,
    to_load: Vec<ChunkKey>,
) {
    let cap = world.store().settings().max_in_flight;
    let running = world.store().in_flight_count();
    let pool = AsyncComputeTaskPool::get();
    for key in to_load.into_iter().take(cap.saturating_sub(running)) {
        let cfg: WorldConfig = *world.store().config();
        let omitted = shown.omitted_for(key).clone();
        world.store_mut().begin_load(key);
        let task = pool.spawn(async move {
            let chunk = hexworld::chunk::generate(&cfg, key);
            let mesh = hexworld::mesh::mesh_chunk(&cfg, &chunk, &omitted);
            JobOutput { key, chunk, mesh }
        });
        commands.spawn(ChunkJob(task));
    }
}

/// Take finished jobs, store the chunk, and put it on screen — along with, in the very
/// same system call, every handover its arrival triggers: the parent (if shown) omitting
/// this chunk's cell, an already-shown sibling handing over a shared guest cell, and this
/// chunk itself absorbing any child that happened to arrive first. Handling all of it here
/// rather than across frames is what keeps the swap to a single frame: nothing else reads
/// `Shown` or an entity's mesh between this chunk's entity appearing and its neighbours'
/// meshes being corrected.
pub fn collect_finished_jobs(
    mut commands: Commands,
    time: Res<Time>,
    mut world: ResMut<HexWorld>,
    mut shown: ResMut<Shown>,
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
        let Some(material) = material.as_ref() else {
            continue;
        };
        let key = output.key;
        let chunk_entity = entities::spawn_chunk_entity(
            &mut commands,
            &mut meshes,
            material.0.clone(),
            key,
            &output.mesh,
            world.store().config(),
        );
        shown.insert_entity(key, chunk_entity);

        // This chunk may itself need to omit cells: a child that arrived first (rare —
        // see `absorb_already_shown_children`), or a guest cell whose true owner is
        // already shown.
        let mut self_dirty = entities::absorb_already_shown_children(&mut shown, key);

        // Its parent, if shown, must omit this chunk's own cell from now on.
        if let Some(parent) = key.parent_key() {
            if shown.entity(parent).is_some() {
                shown.omit(parent, key.cell);
                entities::remesh_shown(&mut commands, &mut meshes, &world, &shown, parent);
            }
        }

        let cfg = *world.store().config();
        for changed in entities::sync_guests_on_load(&cfg, &mut shown, key) {
            if changed == key {
                self_dirty = true;
            } else {
                entities::remesh_shown(&mut commands, &mut meshes, &world, &shown, changed);
            }
        }

        if self_dirty {
            entities::remesh_shown(&mut commands, &mut meshes, &world, &shown, key);
        }
    }
}

//! Loader entities: anything with a `Transform` and a `Loader` pulls the world in around it.

use bevy::prelude::*;
use hexworld::{plane::round_at, Level, Rings};

use crate::axes::from_bevy;
use crate::entities::Shown;
use crate::tasks::{spawn_jobs, Handovers};
use crate::HexWorld;

/// Put this on any entity with a transform to make it a loader.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct Loader {
    pub rings: Rings,
}

pub fn drive_store(
    mut commands: Commands,
    time: Res<Time>,
    mut world: ResMut<HexWorld>,
    shown: Res<Shown>,
    mut handovers: ResMut<Handovers>,
    loaders: Query<(&GlobalTransform, &Loader)>,
) {
    let core_loaders: Vec<hexworld::Loader> = loaders
        .iter()
        .map(|(transform, loader)| {
            let (east, north, _) = from_bevy(transform.translation());
            hexworld::Loader {
                focus: round_at(east, north, Level::Shaku),
                rings: loader.rings,
            }
        })
        .collect();

    let now = time.elapsed_secs_f64();
    let update = world.store_mut().update(&core_loaders, now);

    // Every unload whose chunk is actually shown is queued for `tasks::process_handovers`,
    // which works out what its departure requires (the parent restoring its cell, a
    // sibling taking back a guest cell), re-meshes each of them off the main thread, and
    // only then despawns this chunk's entity — all in the same frame those re-meshes land,
    // so there is never a frame with a hole where this chunk used to be. A key with no
    // entity here was never shown at all (its own load handover is still pending, or was
    // abandoned before it ever got one) — nothing to despawn.
    for key in &update.to_unload {
        if let Some(entity) = shown.entity(*key) {
            handovers.queue_unload(*key, entity);
        }
    }

    spawn_jobs(&mut commands, &mut world, update.to_load);
}

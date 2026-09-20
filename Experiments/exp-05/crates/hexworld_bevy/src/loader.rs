//! Loader entities: anything with a `Transform` and a `Loader` pulls the world in around it.

use bevy::prelude::*;
use hexworld::{plane::round_at, Level, Rings};

use crate::axes::from_bevy;
use crate::tasks::spawn_jobs;
use crate::{entities, HexWorld};

/// Put this on any entity with a transform to make it a loader.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct Loader {
    pub rings: Rings,
}

pub fn drive_store(
    mut commands: Commands,
    time: Res<Time>,
    mut world: ResMut<HexWorld>,
    loaders: Query<(&GlobalTransform, &Loader)>,
    views: Query<(Entity, &crate::ChunkView)>,
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
    entities::despawn_unloaded(&mut commands, &views, &update.to_unload);
    spawn_jobs(&mut commands, &mut world, update.to_load);
}

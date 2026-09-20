//! Loader entities: anything with a `Transform` and a `Loader` pulls the world in around it.

use bevy::prelude::*;
use hexworld::{plane::round_at, Level, Rings};

use crate::axes::from_bevy;
use crate::entities::Shown;
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
    mut shown: ResMut<Shown>,
    mut meshes: ResMut<Assets<Mesh>>,
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

    // Every unload's handover lands in this same system call: the parent (if shown)
    // restores the cell, any sibling that had handed a guest cell over to this chunk
    // takes it back, and only then does this chunk's own entity despawn — so there is
    // never a frame with a hole where this chunk used to be.
    for key in &update.to_unload {
        if let Some(parent) = key.parent_key() {
            if shown.entity(parent).is_some() {
                shown.restore(parent, key.cell);
                entities::remesh_shown(&mut commands, &mut meshes, &world, &shown, parent);
            }
        }
        for neighbour in entities::release_guests_on_unload(&mut shown, *key) {
            entities::remesh_shown(&mut commands, &mut meshes, &world, &shown, neighbour);
        }
        if let Some(entity) = shown.remove(*key) {
            commands.entity(entity).despawn();
        }
    }

    spawn_jobs(&mut commands, &mut world, &shown, update.to_load);
}

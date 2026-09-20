//! Bevy glue for `hexworld`. The core does the thinking; this crate turns loader entities
//! into requests, runs generation and meshing on background tasks, and keeps one entity
//! per chunk on screen.

pub mod axes;
pub mod entities;
pub mod loader;
pub mod tasks;

use bevy::prelude::*;
use hexworld::{ChunkStore, StoreSettings, WorldConfig};

pub use entities::ChunkView;
pub use loader::Loader;

/// Everything this plugin does, in one set, so a viewer can order against it.
#[derive(SystemSet, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct HexWorldSet;

#[derive(Resource)]
pub struct HexWorld {
    store: ChunkStore,
}

impl HexWorld {
    pub fn store(&self) -> &ChunkStore {
        &self.store
    }

    pub fn store_mut(&mut self) -> &mut ChunkStore {
        &mut self.store
    }
}

#[derive(Resource, Clone)]
pub struct GroundMaterialHandle(pub Handle<StandardMaterial>);

#[derive(Default)]
pub struct HexWorldPlugin {
    pub config: WorldConfig,
    pub settings: StoreSettings,
}

impl Plugin for HexWorldPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(HexWorld {
            store: ChunkStore::new(self.config, self.settings),
        })
        .add_systems(Startup, entities::setup_material)
        .add_systems(
            Update,
            (loader::drive_store, tasks::collect_finished_jobs)
                .chain()
                .in_set(HexWorldSet),
        );
    }
}

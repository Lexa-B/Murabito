//! Bevy glue for `hexworld`. The core does the thinking; this crate turns loader entities
//! into requests, runs generation and meshing on background tasks, and keeps one entity
//! per chunk on screen.

pub mod axes;
pub mod entities;
pub mod loader;
pub mod material;
pub mod tasks;

use bevy::pbr::MaterialPlugin;
use bevy::prelude::*;
use hexworld::{ChunkStore, StoreSettings, WorldConfig};

pub use entities::{ChunkView, Shown};
pub use loader::Loader;
pub use material::{GroundExtension, GroundMaterial, LineMode, TintByLevel};
pub use tasks::Handovers;

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
pub struct GroundMaterialHandle(pub Handle<GroundMaterial>);

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
        .init_resource::<entities::Shown>()
        .init_resource::<tasks::Handovers>()
        .init_resource::<LineMode>()
        .init_resource::<TintByLevel>()
        .add_plugins(MaterialPlugin::<GroundMaterial>::default())
        .add_systems(Startup, entities::setup_material)
        .add_systems(
            Update,
            (
                loader::drive_store,
                tasks::collect_finished_jobs,
                tasks::process_handovers,
            )
                .chain()
                .in_set(HexWorldSet),
        )
        .add_systems(Update, material::sync_ground_material);
    }
}

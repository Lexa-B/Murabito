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
    /// Bumped every time the world is replaced wholesale (see `set_config`), so chunk
    /// entities and bookkeeping from the old world can be told apart from the new one.
    generation: u64,
}

impl HexWorld {
    pub fn store(&self) -> &ChunkStore {
        &self.store
    }

    pub fn store_mut(&mut self) -> &mut ChunkStore {
        &mut self.store
    }

    /// Replace the world's settings and drop everything generated from the old ones. The
    /// store's tuning (`unload_delay_s`, `max_in_flight`) is carried over — only what a
    /// chunk *is* changes, not how eagerly it loads.
    pub fn set_config(&mut self, config: WorldConfig) {
        let settings = *self.store.settings();
        self.store = ChunkStore::new(config, settings);
        self.generation += 1;
    }

    /// Bumped whenever the world is replaced, so chunk entities from the old one can go.
    pub fn generation(&self) -> u64 {
        self.generation
    }
}

#[derive(Resource, Clone)]
pub struct GroundMaterialHandle(pub Handle<GroundMaterial>);

/// Despawn every chunk entity left over from a previous world. Runs whenever
/// `HexWorld::generation()` has moved on since the last time this system looked, which
/// only happens right after `set_config` replaces the store wholesale — an ordinary frame
/// is a no-op single comparison.
pub fn despawn_stale_chunks(
    mut commands: Commands,
    world: Res<HexWorld>,
    mut shown: ResMut<entities::Shown>,
    views: Query<(Entity, &ChunkView)>,
    mut last_generation: Local<u64>,
) {
    if world.generation() == *last_generation {
        return;
    }
    *last_generation = world.generation();
    for (entity, _) in &views {
        commands.entity(entity).despawn();
    }
    *shown = entities::Shown::default();
}

#[derive(Default)]
pub struct HexWorldPlugin {
    pub config: WorldConfig,
    pub settings: StoreSettings,
}

impl Plugin for HexWorldPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(HexWorld {
            store: ChunkStore::new(self.config, self.settings),
            generation: 0,
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
                despawn_stale_chunks,
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

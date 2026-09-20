//! `HexWorld::set_config` and the `despawn_stale_chunks` system it needs: changing the
//! seed or the layer thickness must drop the old world's chunks — both the store's data
//! and every entity drawing it — rather than leaving them to mix with the new one.

use std::collections::HashSet;

use bevy::prelude::*;
use hexworld::{Level, Rings, StoreSettings, WorldConfig};
use hexworld_bevy::{ChunkView, HexWorld, Loader};

#[path = "common/mod.rs"]
mod common;
use common::{headless_app, run_until_fully_settled};

/// A brittleness this test used to have, recorded rather than reintroduced: asserting
/// `loaded_keys().is_empty()` and "zero `ChunkView` entities" straight after a *single*
/// `app.update()` following `set_config` is racy in a release build. `despawn_stale_chunks`
/// runs first in the plugin's chain, so it always clears every *old* entity that frame —
/// but `drive_store`, chained right after it in the very same frame (and visible to it
/// through Bevy's automatic `ApplyDeferred` sync points), immediately starts loading the
/// new, empty store's window, and nothing stops one of those jobs — `ChunkKey::WORLD` is
/// essentially free to generate and mesh — from finishing and being committed before the
/// frame ends too. That is a *new* chunk, correctly loaded under the *new* config, not a
/// leftover from the old one, so asserting the screen is bare that same frame was checking
/// the wrong thing; it failed intermittently (~1 run in 20, observed directly) whenever the
/// scheduler happened to let that job complete same-frame. The checks below assert what
/// actually has to hold instead: the store is empty *synchronously*, the moment
/// `set_config` returns (no frame needed), and after settling again nothing on screen
/// belongs to a key the new store doesn't have loaded.
#[test]
fn changing_the_seed_clears_the_world() {
    let mut app = headless_app(StoreSettings::default());
    app.world_mut()
        .spawn((Transform::default(), Loader::default()));
    // 76: the default window's full total (37 ken + 37 cho + 1 ri + 1 world — see
    // `store::tests::default_rings_request_the_expected_chunks`), not just the 37 ken
    // chunks the later assertion checks. `run_until_fully_settled` only guarantees *at
    // least* its target loaded, so passing 37 there settled with ken short of a full
    // window on a fast run, which the "37 ken chunks" assertion below then failed on for
    // a reason that had nothing to do with `set_config`.
    run_until_fully_settled(&mut app, 76);
    assert!(!app
        .world()
        .resource::<HexWorld>()
        .store()
        .loaded_keys()
        .is_empty());
    let generation_before = app.world().resource::<HexWorld>().generation();

    app.world_mut()
        .resource_mut::<HexWorld>()
        .set_config(WorldConfig {
            seed: 7,
            ..WorldConfig::default()
        });

    // `set_config` replaces the `ChunkStore` outright — true the instant it returns, no
    // frame required.
    assert!(
        app.world()
            .resource::<HexWorld>()
            .store()
            .loaded_keys()
            .is_empty(),
        "set_config should drop the old world's chunks immediately"
    );
    assert_eq!(
        app.world().resource::<HexWorld>().generation(),
        generation_before + 1
    );

    // One frame is enough for `despawn_stale_chunks` to despawn every entity left over
    // from the old world — see the doc comment above for why this test does not also
    // assert the screen is bare at this exact point.
    app.update();

    // The world reloads cleanly under the new seed, exactly as a fresh store would.
    run_until_fully_settled(&mut app, 76);
    let world = app.world().resource::<HexWorld>();
    assert_eq!(world.store().config().seed, 7);
    // `loaded_keys()`, not `stats()`: `Stats` is only refreshed once per frame, inside
    // `ChunkStore::update` (called from `drive_store`, ahead of that same frame's
    // `process_handovers` commits) — so on the exact frame a settle first completes,
    // `stats()` can still be one commit behind `loaded_keys()`. `loaded_keys()` reads
    // `self.loaded` directly and has no such lag.
    let loaded = world.store().loaded_keys();
    let ken_chunks = loaded.iter().filter(|key| key.level == Level::Ken).count();
    assert_eq!(ken_chunks, 37);

    // And nothing survived from the old generation: every `ChunkView` entity on screen
    // names a key the new store still has loaded.
    let loaded: HashSet<_> = loaded.into_iter().collect();
    let mut query = app.world_mut().query::<&ChunkView>();
    for view in query.iter(app.world()) {
        assert!(
            loaded.contains(&view.0),
            "{:?} is drawn but is not loaded in the new world",
            view.0
        );
    }
}

#[test]
fn ring_changes_reach_the_store() {
    let mut app = headless_app(StoreSettings::default());
    let entity = app
        .world_mut()
        .spawn((Transform::default(), Loader::default()))
        .id();
    run_until_fully_settled(&mut app, 37);
    let before = app
        .world()
        .resource::<HexWorld>()
        .store()
        .loaded_keys()
        .len();

    app.world_mut().entity_mut(entity).insert(Loader {
        rings: Rings {
            shaku: 6,
            ken: 3,
            cho: 3,
        },
    });
    run_until_fully_settled(&mut app, before + 1);

    let after = app
        .world()
        .resource::<HexWorld>()
        .store()
        .loaded_keys()
        .len();
    assert!(
        after > before,
        "widening the shaku window should load more: {before} -> {after}"
    );
}

#[test]
fn generation_bumps_exactly_once_per_set_config() {
    let mut app = headless_app(StoreSettings::default());
    let world = app.world().resource::<HexWorld>();
    assert_eq!(world.generation(), 0);

    app.world_mut()
        .resource_mut::<HexWorld>()
        .set_config(WorldConfig {
            layer_thickness_sun: 10.0,
            ..WorldConfig::default()
        });
    assert_eq!(app.world().resource::<HexWorld>().generation(), 1);

    app.world_mut()
        .resource_mut::<HexWorld>()
        .set_config(WorldConfig::default());
    assert_eq!(app.world().resource::<HexWorld>().generation(), 2);
}

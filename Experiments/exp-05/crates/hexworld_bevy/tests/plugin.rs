use bevy::prelude::*;
use hexworld::{ChunkKey, Hex, Level, Rings, StoreSettings, WorldConfig};
use hexworld_bevy::{HexWorld, Loader};

#[path = "common/mod.rs"]
mod common;
use common::{headless_app, run_until_fully_settled, run_until_settled};

#[test]
fn a_loader_entity_drives_loading() {
    let mut app = headless_app(StoreSettings::default());
    app.world_mut().spawn((
        Transform::default(),
        Loader {
            rings: Rings::default(),
        },
    ));
    run_until_settled(&mut app);
    let world = app.world().resource::<HexWorld>();
    assert!(world.store().is_loaded(ChunkKey::WORLD));
    assert!(world
        .store()
        .is_loaded(ChunkKey::new(Level::Ken, Hex::ZERO)));
}

#[test]
fn chunk_entities_appear_for_loaded_chunks() {
    let mut app = headless_app(StoreSettings::default());
    app.world_mut().spawn((
        Transform::default(),
        Loader {
            rings: Rings::default(),
        },
    ));
    run_until_settled(&mut app);
    let mut query = app.world_mut().query::<&hexworld_bevy::ChunkView>();
    let views: Vec<ChunkKey> = query.iter(app.world()).map(|v| v.0).collect();
    assert!(
        views.contains(&ChunkKey::new(Level::Ken, Hex::ZERO)),
        "{views:?}"
    );
    let world = app.world().resource::<HexWorld>();
    for key in &views {
        assert!(
            world.store().is_loaded(*key),
            "{key:?} has an entity but is not loaded"
        );
    }
}

#[test]
fn a_second_loader_adds_its_windows_with_no_other_changes() {
    let mut app = headless_app(StoreSettings::default());
    app.world_mut().spawn((
        Transform::default(),
        Loader {
            rings: Rings::default(),
        },
    ));
    // 76 is not a guess: Task 7's `default_rings_request_the_expected_chunks` pins it as
    // the exact request count for default rings at the origin (37 ken + 37 cho + 1 ri +
    // 1 world). Waiting for a genuine settle against that known total, rather than just
    // "ken reached 37", is what makes `before` a fact instead of an observation.
    run_until_fully_settled(&mut app, 76);
    let before = app
        .world()
        .resource::<HexWorld>()
        .store()
        .loaded_keys()
        .len();
    assert_eq!(before, 76, "a single loader's whole window");

    // One more entity with a Loader: that is the whole API.
    let far_cell = Hex::new(400, 200, -600).unwrap();
    let cfg = WorldConfig::default();
    assert!(
        hexworld::cell_in_world(far_cell, Level::Shaku, &cfg),
        "the second loader's focus must be inside the default (single-ri) world, or its \
         window is hollow and this test proves nothing"
    );
    app.world_mut().spawn((
        Transform::from_translation(hexworld::cell_centre(far_cell, Level::Shaku)),
        Loader {
            rings: Rings::default(),
        },
    ));

    // The exact union both loaders should settle to, computed the same way
    // `ChunkStore::update` computes it internally (union `requests()` per loader) — this
    // is what "genuinely settled with two loaders" means, not just "grew by one".
    let loader1 = hexworld::Loader {
        focus: Hex::ZERO,
        rings: Rings::default(),
    };
    let loader2 = hexworld::Loader {
        focus: far_cell,
        rings: Rings::default(),
    };
    let mut expected: Vec<ChunkKey> = hexworld::requests(&cfg, &loader1);
    expected.extend(hexworld::requests(&cfg, &loader2));
    expected.sort();
    expected.dedup();
    let expected_total = expected.len();

    run_until_fully_settled(&mut app, expected_total);
    let after = app
        .world()
        .resource::<HexWorld>()
        .store()
        .loaded_keys()
        .len();
    assert_eq!(
        after, expected_total,
        "two loaders' unioned window: before={before}"
    );
    assert!(
        after >= before + 30,
        "expected roughly a whole ken window more (~37 chunks), got {before} -> {after}"
    );
}

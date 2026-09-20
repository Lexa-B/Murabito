use bevy::prelude::*;
use hexworld_bevy::axes::{from_bevy, mesh_position, to_bevy};

#[test]
fn north_maps_to_negative_z() {
    // Bevy is right-handed with +Y up, so north must be -Z or the world comes out mirrored.
    let v = to_bevy(3.0, 5.0, 7.0);
    assert_eq!(v, Vec3::new(3.0, 7.0, -5.0));
}

#[test]
fn the_mapping_round_trips() {
    let (e, n, h) = (12.5, -4.25, 100.0);
    let (e2, n2, h2) = from_bevy(to_bevy(e, n, h));
    assert!((e - e2).abs() < 1e-9 && (n - n2).abs() < 1e-9 && (h - h2).abs() < 1e-9);
}

#[test]
fn the_mapping_keeps_its_handedness() {
    // east cross north must point up, in both frames: otherwise triangles wind backwards
    // and every hexagon is a mirror image of the core's.
    let east = to_bevy(1.0, 0.0, 0.0);
    let north = to_bevy(0.0, 1.0, 0.0);
    let up = to_bevy(0.0, 0.0, 1.0);
    assert!(
        east.cross(north).dot(up) > 0.0,
        "the axis mapping mirrors the world"
    );
}

#[test]
fn mesh_positions_use_the_same_mapping() {
    assert_eq!(mesh_position([3.0, 5.0, 7.0]), [3.0, 7.0, -5.0]);
}

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
    let far_cell = Hex::new(400, 200);
    let cfg = WorldConfig::default();
    assert!(
        hexworld::cell_in_world(far_cell, Level::Shaku, &cfg),
        "the second loader's focus must be inside the default (single-ri) world, or its \
         window is hollow and this test proves nothing"
    );
    let far = hexworld::plane::cell_centre_m(far_cell, Level::Shaku);
    app.world_mut().spawn((
        Transform::from_translation(hexworld_bevy::axes::to_bevy(far.0, far.1, 0.0)),
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

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

use std::time::{Duration, Instant};

use hexworld::{ChunkKey, Hex, Level, Rings, WorldConfig};
use hexworld_bevy::{HexWorld, HexWorldPlugin, Loader};

/// How long a `run_until` loop is allowed to spend waiting for background work, and how
/// many frames it is allowed to spend doing it, whichever comes first. Frames are not a
/// reliable unit here: a headless app with no render loop burns through hundreds of them
/// in a few milliseconds, while real chunk generation is genuinely CPU-bound (tens of
/// milliseconds per cho chunk in the dev profile), so waiting is bound by wall-clock time
/// with the frame count only as a backstop against a truly stuck test.
const SETTLE_TIMEOUT: Duration = Duration::from_secs(30);
const SETTLE_FRAME_CAP: usize = 20_000;

/// A headless app: no window, no renderer, just the schedule.
///
/// `TransformPlugin` is included even though nothing else here renders, because
/// `drive_store` reads loaders' `GlobalTransform`, and that is only ever kept in sync
/// with `Transform` by `TransformPlugin`'s propagation systems (`MinimalPlugins` does
/// not include them). Propagation runs in `PostUpdate`, so a loader's `GlobalTransform`
/// is still identity on the very first frame it is spawned; a settle loop runs many
/// frames, so this does not affect these tests.
fn headless_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::transform::TransformPlugin)
        .add_plugins(bevy::asset::AssetPlugin::default())
        .init_asset::<Mesh>()
        .init_asset::<StandardMaterial>()
        .add_plugins(HexWorldPlugin::default());
    app
}

/// Run frames until `condition` is true, or the settle timeout/frame cap is hit,
/// whichever comes first. Returns the number of frames it took. `describe` builds the
/// panic message on timeout, so a genuine failure is diagnosable rather than just
/// "never settled".
fn run_until(
    app: &mut App,
    mut condition: impl FnMut(&HexWorld) -> bool,
    describe: impl Fn(&HexWorld) -> String,
) -> usize {
    let started = Instant::now();
    for frame in 0..SETTLE_FRAME_CAP {
        app.update();
        let world = app.world().resource::<HexWorld>();
        if condition(world) {
            return frame;
        }
        if started.elapsed() >= SETTLE_TIMEOUT {
            panic!(
                "timed out after {:?} and {} frames: {}",
                started.elapsed(),
                frame + 1,
                describe(world)
            );
        }
    }
    let world = app.world().resource::<HexWorld>();
    panic!(
        "hit the {SETTLE_FRAME_CAP}-frame cap after {:?}: {}",
        started.elapsed(),
        describe(world)
    );
}

/// Run frames until the store reports at least 37 loaded ken chunks (the default
/// window's full shaku-detail ring), or the settle timeout/frame cap is hit.
///
/// This is *not* a guarantee that every requested chunk is loaded: `max_in_flight` is
/// shared across levels, and `load_order` only decides which chunk starts next, not
/// when it finishes — once the last cho/ri/world chunk has merely started, freed
/// capacity immediately pulls in cheaper ken chunks, so ken can reach 37 while some of
/// the caller's own cho chunks are still in flight. Good enough for tests that only
/// check specific keys or that loaded entities match store state; not good enough for
/// anything that needs an exact count (see `run_until_fully_settled`).
fn run_until_settled(app: &mut App) -> usize {
    run_until(
        app,
        |world| world.store().stats().per_level[Level::Ken as usize].chunks >= 37,
        |world| {
            format!(
                "{} ken chunks loaded",
                world.store().stats().per_level[Level::Ken as usize].chunks
            )
        },
    )
}

/// Run frames until the store has *genuinely* settled: nothing outstanding
/// (`in_flight_count() == 0`) and at least `expected` chunks loaded. Both conditions are
/// needed together: `in_flight_count()` can read 0 for a single frame in the middle of a
/// settle too, if every currently in-flight job happens to finish in the same frame
/// while more requested chunks are still waiting for a free slot (they only get picked
/// up on the *next* frame's `drive_store`) — pairing it with a known target count closes
/// that gap, the same way `in_flight` alone would not.
fn run_until_fully_settled(app: &mut App, expected: usize) -> usize {
    run_until(
        app,
        move |world| {
            world.store().in_flight_count() == 0 && world.store().loaded_keys().len() >= expected
        },
        move |world| {
            format!(
                "in_flight={} loaded={} (want in_flight == 0 and loaded >= {expected})",
                world.store().in_flight_count(),
                world.store().loaded_keys().len()
            )
        },
    )
}

#[test]
fn a_loader_entity_drives_loading() {
    let mut app = headless_app();
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
    let mut app = headless_app();
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
    let mut app = headless_app();
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

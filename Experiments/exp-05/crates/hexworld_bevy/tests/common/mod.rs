//! Shared test scaffolding: a headless app and wall-clock-bounded "settle" helpers.
//!
//! Real chunk generation is genuinely CPU-bound (tens of milliseconds per cho chunk in the
//! dev profile), so waiting for the store to settle is bound by wall-clock time, with a
//! frame count only as a backstop against a truly stuck test. A fixed frame budget was
//! already found, in Task 11, to make a test pass for the wrong reason.

#![allow(dead_code)] // not every test file uses every helper here.

use std::time::{Duration, Instant};

use bevy::prelude::*;
use hexworld::{Level, StoreSettings};
use hexworld_bevy::{Handovers, HexWorld, HexWorldPlugin};

pub const SETTLE_TIMEOUT: Duration = Duration::from_secs(30);
pub const SETTLE_FRAME_CAP: usize = 20_000;

/// A headless app: no window, no renderer, just the schedule.
///
/// `TransformPlugin` is included even though nothing else here renders, because
/// `drive_store` reads loaders' `GlobalTransform`, and that is only ever kept in sync
/// with `Transform` by `TransformPlugin`'s propagation systems (`MinimalPlugins` does
/// not include them). Propagation runs in `PostUpdate`, so a loader's `GlobalTransform`
/// is still identity on the very first frame it is spawned; a settle loop runs many
/// frames, so this does not affect these tests.
pub fn headless_app(settings: StoreSettings) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::transform::TransformPlugin)
        .add_plugins(bevy::asset::AssetPlugin::default())
        .init_asset::<Mesh>()
        .init_asset::<StandardMaterial>()
        .add_plugins(HexWorldPlugin {
            settings,
            ..default()
        });
    app
}

/// Run frames until `condition` is true, or the settle timeout/frame cap is hit,
/// whichever comes first. Returns the number of frames it took. `describe` builds the
/// panic message on timeout, so a genuine failure is diagnosable rather than just
/// "never settled".
pub fn run_until(
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

/// Like `run_until`, but the condition (and `describe`) see the whole `World` rather than
/// just `HexWorld` — for a settle check that also needs another resource, such as
/// `Handovers` (see `run_until_fully_settled`).
pub fn run_until_world(
    app: &mut App,
    mut condition: impl FnMut(&World) -> bool,
    describe: impl Fn(&World) -> String,
) -> usize {
    let started = Instant::now();
    for frame in 0..SETTLE_FRAME_CAP {
        app.update();
        let world = app.world();
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
    let world = app.world();
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
pub fn run_until_settled(app: &mut App) -> usize {
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
/// (`in_flight_count() == 0`), at least `expected` chunks loaded, and no handover left
/// queued or mid-re-mesh (`Handovers::is_idle`). All three are needed together:
/// `in_flight_count()` can read 0 for a single frame in the middle of a settle too, if
/// every currently in-flight job happens to finish in the same frame while more requested
/// chunks are still waiting for a free slot (they only get picked up on the *next*
/// frame's `drive_store`) — pairing it with a known target count closes that gap, the
/// same way `in_flight` alone would not. And since the store considers a load "loaded"
/// (or an unload already gone from `loaded`) well before that chunk's handover — if it
/// needs one — actually finishes applying, `in_flight`/`loaded_keys` reaching their target
/// numbers does not by itself mean nothing is still queued in `Handovers`; without this
/// third check, a test racing a reload against a still-queued unload handover (see
/// `tests/handover.rs::growing_the_window_back_after_a_shrink_leaves_no_duplicate_or_orphaned_entities`)
/// could read the world as settled while a stale unload was still waiting its turn,
/// making the test's own eventual failure look identical to the real defect it exists to
/// catch (fix round 2, task-12b-report.md).
pub fn run_until_fully_settled(app: &mut App, expected: usize) -> usize {
    run_until_world(
        app,
        move |world| {
            let store = world.resource::<HexWorld>();
            let handovers = world.resource::<Handovers>();
            store.store().in_flight_count() == 0
                && store.store().loaded_keys().len() >= expected
                && handovers.is_idle()
        },
        move |world| {
            let store = world.resource::<HexWorld>();
            let handovers = world.resource::<Handovers>();
            format!(
                "in_flight={} loaded={} pending_loads={} pending_unloads={} (want in_flight \
                 == 0, loaded >= {expected}, and no pending handovers)",
                store.store().in_flight_count(),
                store.store().loaded_keys().len(),
                handovers.pending_load_keys().len(),
                handovers.pending_unload_keys().len(),
            )
        },
    )
}

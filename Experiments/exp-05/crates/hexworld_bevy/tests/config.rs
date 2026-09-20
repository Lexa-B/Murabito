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

    // Snapshot every entity on screen before the swap. The final check below does not
    // lean on "no key in the new set was loaded before" — the loader never moves in this
    // test, so the reloaded key set is identical to the old one, and a
    // `despawn_stale_chunks` that silently became a no-op (combined with a handover
    // pipeline that just re-meshed the same old entities in place under the new content)
    // would pass a key-only check without ever having dropped anything. Entity identity
    // is the one thing that can only survive if the entity itself was never despawned —
    // recycled Bevy `Entity` slots get a bumped internal version, so a genuinely new
    // entity can never equal an old one even at the same index — so it is what actually
    // gets tested.
    let old_entities: HashSet<Entity> = {
        let mut query = app.world_mut().query_filtered::<Entity, With<ChunkView>>();
        query.iter(app.world()).collect()
    };
    assert!(!old_entities.is_empty());
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

    // None of the old generation's entities are still on screen — the strong check; see
    // the snapshot comment above for why a key-only check cannot catch a
    // `despawn_stale_chunks` that quietly stopped despawning anything.
    let new_entities: HashSet<Entity> = {
        let mut query = app.world_mut().query_filtered::<Entity, With<ChunkView>>();
        query.iter(app.world()).collect()
    };
    let survivors: Vec<Entity> = old_entities.intersection(&new_entities).copied().collect();
    assert!(
        survivors.is_empty(),
        "{} entities from the old generation are still on screen: {survivors:?}",
        survivors.len()
    );

    // And every entity on screen genuinely draws a key the new store has loaded — catches
    // a different failure mode (an orphaned entity for a key nothing loaded any more).
    let loaded_set: HashSet<_> = loaded.into_iter().collect();
    let mut query = app.world_mut().query::<&ChunkView>();
    for view in query.iter(app.world()) {
        assert!(
            loaded_set.contains(&view.0),
            "{:?} is drawn but is not loaded in the new world",
            view.0
        );
    }
}

/// The race `set_config` itself could not close on its own (review finding, Important 1):
/// a generation job started under the *old* config, still running in the background when
/// `set_config` replaces the store, finishes afterwards while its key is — because the
/// loader never moves — still spatially requested under the *new* store too.
/// `ChunkStore::is_requested` only ever asks "is this key wanted right now", which knows
/// nothing about which config produced the data in hand, so without `JobOutput::generation`
/// and the check in `tasks::step`, that stale chunk would be accepted straight into the new
/// world.
///
/// A first version of this test just settled normally after `set_config` and compared
/// loaded content against the new config. It passed even with the generation check
/// disabled: `load_order` is purely geometric (independent of the config), so the very
/// keys with an old job in flight are also the keys `drive_store` immediately starts a
/// *new*, correctly-tagged duplicate job for — and since `ChunkStore::insert` overwrites
/// `loaded` unconditionally, whichever of the two duplicates commits *last* wins,
/// self-correcting almost every time by accident rather than by anything that guarantees
/// it. That accident is exactly what `ChunkStore::update`'s own `to_load` filter
/// (`!self.loaded.contains_key(k)`) turns into a *permanent* stale key the moment the old
/// job wins the race instead: once any chunk is `loaded`, the store never asks for it
/// again. So the real bug is a race whose outcome — not whose existence — is
/// timing-dependent, which is a fault this test needs to isolate deterministically rather
/// than hope to observe.
///
/// Fixed by starving new loads (`max_in_flight = 0`) the instant `set_config` returns, so
/// no same-key duplicate can ever be spawned to paper over a still-running old job — only
/// the old, already-in-flight jobs can land. Whatever they produce is then checked against
/// the new config directly.
#[test]
fn a_job_in_flight_during_set_config_does_not_leak_into_the_new_world() {
    let mut app = headless_app(StoreSettings::default());
    app.world_mut()
        .spawn((Transform::default(), Loader::default()));

    // One frame is enough to put several real jobs in flight (chunk generation costs
    // real wall-clock time — see `tasks::spawn_remesh`'s own comment on cho re-mesh cost
    // — so nothing spawned this frame can have finished by the time `app.update()`
    // returns), and deliberately not enough to settle.
    app.update();
    let in_flight_before = app.world().resource::<HexWorld>().store().in_flight_count();
    assert!(
        in_flight_before > 0,
        "need at least one job genuinely in flight to provoke the race"
    );

    let old_cfg = *app.world().resource::<HexWorld>().store().config();
    let new_cfg = WorldConfig {
        seed: 7,
        ..WorldConfig::default()
    };
    {
        let mut world = app.world_mut().resource_mut::<HexWorld>();
        world.set_config(new_cfg);
        // No new job may start until the still-running old-generation jobs have landed —
        // otherwise a same-key duplicate can win the race and mask the bug, as explained
        // above.
        world.store_mut().settings_mut().max_in_flight = 0;
    }

    // Wait for every old-generation `ChunkJob` to finish (none can be new: `max_in_flight`
    // is 0) *and* for the handovers they produced to be fully resolved — committed or
    // abandoned, never left queued. `in_flight_count()` cannot be used here: it is 0 from
    // the moment `set_config` returns regardless of what the old jobs eventually do,
    // because they were never marked in-flight on the *new* store to begin with.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        app.update();
        let no_jobs_left = {
            let mut query = app.world_mut().query::<&hexworld_bevy::tasks::ChunkJob>();
            query.iter(app.world()).next().is_none()
        };
        let idle = app.world().resource::<hexworld_bevy::Handovers>().is_idle();
        if no_jobs_left && idle {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the old in-flight jobs never finished settling"
        );
    }

    // Whatever landed while starved (this is where stale data would show up if the fix
    // were broken) must match the *new* config, never the old one. With the fix working,
    // this is typically empty — every old-generation job gets rejected outright rather
    // than committed — which is itself the point, not a reason to skip the check.
    {
        let world = app.world().resource::<HexWorld>();
        assert_eq!(world.store().config().seed, 7);
        for key in world.store().loaded_keys() {
            let actual = world
                .store()
                .chunk(key)
                .expect("just reported as loaded")
                .clone();
            let expected_under_new = hexworld::chunk::generate(&new_cfg, key);
            assert_eq!(
                actual, expected_under_new,
                "{key:?} does not match what the new config generates — an in-flight job \
                 from the old world leaked in"
            );
        }
    }

    // Lift the starvation and let the world settle for real, the way an interactive
    // session would after the tuning slider it came from stops being dragged.
    app.world_mut()
        .resource_mut::<HexWorld>()
        .store_mut()
        .settings_mut()
        .max_in_flight = StoreSettings::default().max_in_flight;
    run_until_fully_settled(&mut app, 76);

    let world = app.world().resource::<HexWorld>();
    let landed = world.store().loaded_keys();
    assert_eq!(
        landed.len(),
        76,
        "the world should settle fully once un-starved"
    );

    let mut saw_a_difference = false;
    for key in landed {
        let actual = world
            .store()
            .chunk(key)
            .expect("just reported as loaded")
            .clone();
        let expected_under_new = hexworld::chunk::generate(&new_cfg, key);
        let would_be_under_old = hexworld::chunk::generate(&old_cfg, key);
        assert_eq!(
            actual, expected_under_new,
            "{key:?} does not match what the new config generates — an in-flight job from \
             the old world leaked in and never self-corrected"
        );
        if would_be_under_old != expected_under_new {
            saw_a_difference = true;
        }
    }
    assert!(
        saw_a_difference,
        "the old and new configs generated identical content everywhere loaded — this test \
         would pass even with the bug present"
    );
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

use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use hexworld::{ChunkKey, Hex, Level, Rings, StoreSettings, WorldConfig};
use hexworld_bevy::{ChunkView, Handovers, HexWorld, Loader, Shown};

#[path = "common/mod.rs"]
mod common;
use common::{headless_app, run_until_fully_settled};

// The default rings settle to 76 chunks (Task 7's known total for default rings at the
// origin: 37 ken + 37 cho + 1 ri + 1 world) — see `plugin.rs`'s
// `a_second_loader_adds_its_windows_with_no_other_changes` for where that number comes
// from. Waiting for a genuine settle, rather than a fixed frame budget, is what makes the
// state these tests inspect a fact instead of an observation caught mid-flight.
const DEFAULT_WINDOW_TOTAL: usize = 76;

#[test]
fn a_shown_child_is_left_out_of_its_parents_mesh() {
    let mut app = headless_app(StoreSettings::default());
    app.world_mut().spawn((
        Transform::default(),
        Loader {
            rings: Rings::default(),
        },
    ));
    run_until_fully_settled(&mut app, DEFAULT_WINDOW_TOTAL);
    let shown = app.world().resource::<Shown>();
    let cho_key = ChunkKey::new(Level::Cho, Hex::ZERO);
    let omitted = shown.omitted_for(cho_key);
    // Every ken chunk on screen must be omitted from the cho chunk that would draw it.
    for key in shown.keys() {
        if key.level == Level::Ken && key.parent_key() == Some(cho_key) {
            assert!(omitted.contains(&key.cell), "{key:?} is drawn twice");
        }
    }
    assert!(!omitted.is_empty(), "nothing was handed over");
    assert_invariants(app.world());
}

#[test]
fn every_shown_chunk_has_exactly_one_entity() {
    let mut app = headless_app(StoreSettings::default());
    app.world_mut().spawn((
        Transform::default(),
        Loader {
            rings: Rings::default(),
        },
    ));
    run_until_fully_settled(&mut app, DEFAULT_WINDOW_TOTAL);
    let mut query = app.world_mut().query::<&ChunkView>();
    let mut seen: HashSet<ChunkKey> = HashSet::new();
    for view in query.iter(app.world()) {
        assert!(seen.insert(view.0), "{:?} has two entities", view.0);
    }
    let shown = app.world().resource::<Shown>();
    assert_eq!(seen.len(), shown.keys().len());
    assert_invariants(app.world());
}

#[test]
fn unloading_puts_the_cell_back_into_its_parent() {
    // Ruling 3: a short unload delay, or the test would wait out the real 5 s default.
    let mut app = headless_app(StoreSettings {
        unload_delay_s: 0.05,
        ..StoreSettings::default()
    });
    let entity = app
        .world_mut()
        .spawn((
            Transform::default(),
            Loader {
                rings: Rings::default(),
            },
        ))
        .id();
    run_until_fully_settled(&mut app, DEFAULT_WINDOW_TOTAL);
    let cho_key = ChunkKey::new(Level::Cho, Hex::ZERO);
    assert!(!app
        .world()
        .resource::<Shown>()
        .omitted_for(cho_key)
        .is_empty());

    // Shrink the loader's shaku window to nothing and wait out the unload delay.
    app.world_mut().entity_mut(entity).insert(Loader {
        rings: Rings {
            shaku: 0,
            ken: 3,
            cho: 3,
        },
    });
    // The new window (shaku=0) is a strict subset of the old one: it never grows, so a
    // genuine settle is simply in_flight==0. There is no larger "expected" total to wait
    // for the way there is on first load.
    common::run_until(
        &mut app,
        |world| world.store().in_flight_count() == 0,
        |world| format!("in_flight={}", world.store().in_flight_count()),
    );
    let shown = app.world().resource::<Shown>();
    let still_shown: Vec<ChunkKey> = shown
        .keys()
        .into_iter()
        .filter(|k| k.level == Level::Ken && k.parent_key() == Some(cho_key))
        .collect();
    // `cho_key`'s omissions are a mix of two different mechanisms (Ruling 2), and this
    // loop must tell them apart rather than assume every omitted cell is a ken child:
    //   - a cell `cho_key` itself owns (Ruling 1's single-cell handover): omitted only
    //     because that exact ken chunk is shown, so it must still be in `still_shown`.
    //   - a guest cell `cho_key` does not own (Ruling 2's sibling dedup): omitted
    //     because the *neighbouring* cho chunk that owns it is shown — that neighbour
    //     draws it at cho-level detail instead, with no ken chunk involved at all. The
    //     cho window (`rings.ken`) was never shrunk, so that neighbour is still shown
    //     and the cell must stay omitted.
    for cell in shown.omitted_for(cho_key) {
        let owner_cho = hexworld::owner::parent_of(*cell, Level::Ken);
        if owner_cho == cho_key.cell {
            assert!(
                still_shown.iter().any(|k| k.cell == *cell),
                "{cell:?} is owned by {cho_key:?} and omitted from it, but no ken chunk draws it"
            );
        } else {
            let owning_cho = ChunkKey::new(Level::Cho, owner_cho);
            assert!(
                shown.entity(owning_cho).is_some(),
                "{cell:?} is a guest cell omitted from {cho_key:?}, but its true owner \
                 {owning_cho:?} is not shown to draw it instead"
            );
        }
    }

    // Both invariants describe a *settled* world, and the wait above only asked the store;
    // the store calls an unload gone from `loaded` well before that unload's handover has
    // actually applied. Wait for the handovers too before checking them.
    common::run_until_world(
        &mut app,
        |world| {
            world.resource::<HexWorld>().store().in_flight_count() == 0
                && world.resource::<Handovers>().is_idle()
        },
        |world| {
            format!(
                "in_flight={} pending_loads={} pending_unloads={}",
                world.resource::<HexWorld>().store().in_flight_count(),
                world.resource::<Handovers>().pending_load_keys().len(),
                world.resource::<Handovers>().pending_unload_keys().len(),
            )
        },
    );
    assert_invariants(app.world());
}

/// The one-frame property, made testable: check the bookkeeping invariant after *every*
/// single frame, not just once at the end after many frames have run. `Shown.omitted` and
/// a chunk's entity are both updated inside the same system call in `collect_finished_jobs`
/// (see `entities.rs`), so there must never be a frame where a child's entity exists but
/// its parent's omission has not yet caught up — that lag is exactly what an async or
/// next-frame remesh (the route Ruling 1 forbids) would produce: a doubled surface for one
/// frame while the parent's stale mesh is still on screen next to the new child.
///
/// This only checks `Shown`'s bookkeeping, not the mesh actually on screen — a bug that
/// updated `Shown.omitted` correctly but forgot to call `remesh_shown` would pass this
/// test. `the_parents_mesh_drops_the_vertex_the_same_frame_the_childs_entity_appears`
/// below checks the mesh geometry itself for exactly that reason.
#[test]
fn the_parents_omission_never_lags_a_shown_childs_entity_by_even_one_frame() {
    let mut app = headless_app(StoreSettings::default());
    app.world_mut().spawn((
        Transform::default(),
        Loader {
            rings: Rings::default(),
        },
    ));
    let cho_key = ChunkKey::new(Level::Cho, Hex::ZERO);
    let mut saw_a_child_shown = false;

    // Wall-clock bounded like `run_until` (Ruling 4): real generation is tens of
    // milliseconds per cho chunk, so a fixed frame budget is not a reliable unit, only a
    // backstop against a truly stuck test. Unlike `run_until`, the check that matters
    // here runs on *every* frame along the way, not just once at the end — that is what
    // makes this test able to catch a swap that lands a frame late.
    let started = std::time::Instant::now();
    for frame in 0..common::SETTLE_FRAME_CAP {
        app.update();
        let world = app.world();
        let shown = world.resource::<Shown>();
        for key in shown.keys() {
            if key.level == Level::Ken && key.parent_key() == Some(cho_key) {
                saw_a_child_shown = true;
                assert!(
                    shown.omitted_for(cho_key).contains(&key.cell),
                    "frame {frame}: {key:?} has an entity but {cho_key:?} has not omitted \
                     its cell yet — the swap did not land in one frame"
                );
            }
        }
        if world_is_fully_settled(world, DEFAULT_WINDOW_TOTAL) {
            break;
        }
        assert!(
            started.elapsed() < common::SETTLE_TIMEOUT,
            "timed out after {:?} and {} frames waiting to settle",
            started.elapsed(),
            frame + 1
        );
    }
    assert!(
        saw_a_child_shown,
        "the loop never saw a ken child of {cho_key:?} shown; the test proves nothing"
    );
    assert_invariants(app.world());
}

/// The mesh-geometry half of the one-frame property. Every frame a new ken child of
/// `cho_key` appears, this checks the *actual vertex data* Bevy has on screen that same
/// frame, using the same position-matching technique
/// `hexworld/tests/handover.rs::a_parent_and_its_children_cover_the_ground_exactly_once`
/// uses at the core level: a top-face fan puts a vertex exactly at a drawn cell's own
/// local centre, and only there, so a cell's absence from the vertex list means the mesh
/// did not draw it.
///
/// A positive control makes this more than "the vertex isn't there, trivially": the
/// PREVIOUS frame's mesh (still on screen up to that point) must still have had the
/// vertex, so the loop is genuinely observing a real swap, not a coincidence of a mesh
/// that never drew the cell.
#[test]
fn the_parents_mesh_drops_the_vertex_the_same_frame_the_childs_entity_appears() {
    let mut app = headless_app(StoreSettings::default());
    app.world_mut().spawn((
        Transform::default(),
        Loader {
            rings: Rings::default(),
        },
    ));
    let cho_key = ChunkKey::new(Level::Cho, Hex::ZERO);
    let origin = hexworld::plane::cell_centre_m(cho_key.cell, Level::Cho);

    let mut prev_children: HashSet<Hex> = HashSet::new();
    let mut prev_positions: Option<Vec<[f32; 3]>> = None;
    let mut checked_at_least_one = false;

    let started = std::time::Instant::now();
    for frame in 0..common::SETTLE_FRAME_CAP {
        app.update();

        let (current_children, current_positions) = {
            let world = app.world();
            let shown = world.resource::<Shown>();
            let children: HashSet<Hex> = shown
                .keys()
                .into_iter()
                .filter(|k| k.level == Level::Ken && k.parent_key() == Some(cho_key))
                .map(|k| k.cell)
                .collect();
            let positions = shown.entity(cho_key).and_then(|entity| {
                let handle = &world.get::<Mesh3d>(entity)?.0;
                let mesh = world.resource::<Assets<Mesh>>().get(handle)?;
                let values = mesh.attribute(Mesh::ATTRIBUTE_POSITION)?.as_float3()?;
                Some(values.to_vec())
            });
            (children, positions)
        };

        for new_cell in current_children.difference(&prev_children) {
            checked_at_least_one = true;
            let (ce, cn) = hexworld::plane::cell_centre_m(*new_cell, Level::Ken);
            let target = hexworld_bevy::axes::mesh_position([
                (ce - origin.0) as f32,
                (cn - origin.1) as f32,
                0.0,
            ]);
            let has_vertex_at_target = |positions: &[[f32; 3]]| {
                positions
                    .iter()
                    .any(|v| (v[0] - target[0]).abs() < 1e-3 && (v[2] - target[2]).abs() < 1e-3)
            };

            if let Some(prev) = &prev_positions {
                assert!(
                    has_vertex_at_target(prev),
                    "frame {frame}: positive control failed — {cho_key:?}'s mesh the frame \
                     *before* {new_cell:?} appeared should already have had a vertex at its \
                     centre, or this test cannot tell a real swap from a coincidence"
                );
            }
            let current = current_positions
                .as_deref()
                .expect("the parent must have a mesh once a child of it has an entity");
            assert!(
                !has_vertex_at_target(current),
                "frame {frame}: {new_cell:?}'s entity appeared this frame, but {cho_key:?}'s \
                 mesh — read this same frame — still has a vertex at its centre: the swap did \
                 not land in one frame"
            );
        }

        prev_children = current_children;
        prev_positions = current_positions;

        if world_is_fully_settled(app.world(), DEFAULT_WINDOW_TOTAL) {
            break;
        }
        assert!(
            started.elapsed() < common::SETTLE_TIMEOUT,
            "timed out after {:?} and {} frames waiting to settle",
            started.elapsed(),
            frame + 1
        );
    }
    assert!(
        checked_at_least_one,
        "the loop never saw a new ken child of {cho_key:?} appear; the test proves nothing"
    );
    assert_invariants(app.world());
}

/// The mesh-geometry check for Ruling 2's guest-cell dedup, in the same shape as
/// `the_parents_mesh_drops_the_vertex_the_same_frame_the_childs_entity_appears` above, but
/// for two same-level neighbours sharing a guest (tie) cell rather than a parent and its
/// child. `entities::tests::a_guest_chunk_omits_once_its_owner_is_shown` already proves
/// the `Shown` bookkeeping is right; this proves the actual mesh follows it, in the frame
/// the second of the pair is shown — not merely eventually — the same gap the parent/child
/// sabotage exercise above found for bookkeeping-only checks.
#[test]
fn the_guest_sides_mesh_drops_the_shared_vertex_the_frame_both_neighbours_are_shown() {
    let mut app = headless_app(StoreSettings::default());
    app.world_mut().spawn((
        Transform::default(),
        Loader {
            rings: Rings::default(),
        },
    ));

    // Same pair `entities.rs`'s own unit test uses: known, by construction, to share at
    // least one guest cell (see `hexworld`'s `shared_cells_are_exactly_guests`), and both
    // are well within the default window (rings.shaku = 3) around the origin.
    let a = ChunkKey::new(Level::Ken, Hex::ZERO);
    let b = ChunkKey::new(Level::Ken, Hex::new(1, 0));
    let cfg = WorldConfig::default();

    // Which of a/b owns the shared tie cell is fixed by geometry alone, independent of
    // load order — work it out once, up front, the pure-data way.
    let cells_a: HashSet<Hex> = hexworld::mesh::drawn_cells(&cfg, a).into_iter().collect();
    let cells_b: HashSet<Hex> = hexworld::mesh::drawn_cells(&cfg, b).into_iter().collect();
    let mut shared: Vec<Hex> = cells_a.intersection(&cells_b).copied().collect();
    shared.sort_by_key(|h| (h.q, h.r));
    let cell = *shared
        .first()
        .expect("a and b are known neighbours and must share at least one guest cell");
    let owner_cell = hexworld::owner::parent_of(cell, Level::Shaku);
    let (owner_key, guest_key) = if owner_cell == a.cell { (a, b) } else { (b, a) };

    // Each chunk's mesh is relative to its own centre (`chunk_origin`/`cell_centre_m`), so
    // the shared cell's local target position differs between the two meshes — converted
    // through the same public `axes::mesh_position` helper the production code uses, not
    // reimplemented.
    let local_target = |key: ChunkKey| -> [f32; 2] {
        let origin = hexworld::plane::cell_centre_m(key.cell, Level::Ken);
        let (ce, cn) = hexworld::plane::cell_centre_m(cell, Level::Shaku);
        let p = hexworld_bevy::axes::mesh_position([
            (ce - origin.0) as f32,
            (cn - origin.1) as f32,
            0.0,
        ]);
        [p[0], p[2]]
    };
    let owner_target = local_target(owner_key);
    let guest_target = local_target(guest_key);

    let mesh_positions = |world: &World, key: ChunkKey| -> Option<Vec<[f32; 3]>> {
        let shown = world.resource::<Shown>();
        let entity = shown.entity(key)?;
        let handle = &world.get::<Mesh3d>(entity)?.0;
        let mesh = world.resource::<Assets<Mesh>>().get(handle)?;
        let values = mesh.attribute(Mesh::ATTRIBUTE_POSITION)?.as_float3()?;
        Some(values.to_vec())
    };
    let has_vertex_at = |positions: &[[f32; 3]], target: [f32; 2]| {
        positions
            .iter()
            .any(|v| (v[0] - target[0]).abs() < 1e-3 && (v[2] - target[1]).abs() < 1e-3)
    };

    let mut both_were_shown = false;
    let mut checked = false;

    let started = std::time::Instant::now();
    for frame in 0..common::SETTLE_FRAME_CAP {
        app.update();

        let world = app.world();
        let both_shown_now = {
            let shown = world.resource::<Shown>();
            shown.entity(a).is_some() && shown.entity(b).is_some()
        };

        if both_shown_now && !both_were_shown {
            checked = true;
            let owner_positions = mesh_positions(world, owner_key)
                .expect("the owner must have a mesh once both chunks are shown");
            let guest_positions = mesh_positions(world, guest_key)
                .expect("the guest must have a mesh once both chunks are shown");
            assert!(
                has_vertex_at(&owner_positions, owner_target),
                "frame {frame}: {owner_key:?} owns {cell:?} and must still draw it"
            );
            assert!(
                !has_vertex_at(&guest_positions, guest_target),
                "frame {frame}: {guest_key:?} and {owner_key:?} are both shown this frame, \
                 but {guest_key:?}'s mesh — read this same frame — still has a vertex at the \
                 shared cell {cell:?}: the guest-side handover did not land in one frame"
            );
        }
        both_were_shown = both_shown_now;

        if world_is_fully_settled(world, DEFAULT_WINDOW_TOTAL) {
            break;
        }
        assert!(
            started.elapsed() < common::SETTLE_TIMEOUT,
            "timed out after {:?} and {} frames waiting to settle",
            started.elapsed(),
            frame + 1
        );
    }
    assert!(
        checked,
        "never observed both {a:?} and {b:?} shown together; the test proves nothing"
    );
    assert_invariants(app.world());
}

/// Task 12b: the async-specific half of the one-frame property. The tests above check
/// that the swap lands in one frame — a fact a synchronous implementation would also
/// satisfy. This one checks the hold-back mechanism the async route added: `Handovers`
/// tracks a child chunk as "pending" for as long as its handover has not yet fully
/// applied, and asserts a key is never *both* pending and shown in the same frame.
///
/// What this actually has teeth against (confirmed by falsification; see
/// `task-12b-report.md`): a bug that spawns the chunk's entity while `Handovers` still
/// separately considers its load pending — in the current design, `commit` running without
/// the handover leaving the queue. It does **not** catch a bug where the apply step runs
/// early (committing without every required mesh being ready): that would show the chunk
/// *and* drop the handover in the same system call, so "pending" and "shown" never overlap
/// at a frame boundary for this test to observe either state change without the other. That failure mode is instead what
/// `the_parents_mesh_drops_the_vertex_the_same_frame_the_childs_entity_appears` and
/// `the_parents_omission_never_lags_a_shown_childs_entity_by_even_one_frame` are for: they
/// check the parent's actual mesh/bookkeeping, which an early apply would still get wrong.
#[test]
fn a_child_is_never_shown_while_its_load_handover_is_pending() {
    let mut app = headless_app(StoreSettings::default());
    app.world_mut().spawn((
        Transform::default(),
        Loader {
            rings: Rings::default(),
        },
    ));

    let mut ever_pending: HashSet<ChunkKey> = HashSet::new();
    let mut witnessed_a_deferred_reveal = false;

    let started = std::time::Instant::now();
    for frame in 0..common::SETTLE_FRAME_CAP {
        app.update();
        let world = app.world();
        let shown = world.resource::<Shown>();
        let handovers = world.resource::<Handovers>();

        for key in shown.keys() {
            assert!(
                !handovers.is_pending_load(key),
                "frame {frame}: {key:?} has an entity in Shown but Handovers still \
                 considers its load pending — it was shown before its handover landed"
            );
            if ever_pending.contains(&key) {
                // Seen pending on some earlier frame, and only now shown: proof the hold-
                // back genuinely spanned more than an instant, not just this frame's check
                // happening to run either side of an atomic same-frame swap.
                witnessed_a_deferred_reveal = true;
            }
        }
        ever_pending.extend(handovers.pending_load_keys());

        if world_is_fully_settled(world, DEFAULT_WINDOW_TOTAL) {
            break;
        }
        assert!(
            started.elapsed() < common::SETTLE_TIMEOUT,
            "timed out after {:?} and {} frames waiting to settle",
            started.elapsed(),
            frame + 1
        );
    }
    assert!(
        witnessed_a_deferred_reveal,
        "never saw a chunk held pending on one frame and only shown on a later one; the \
         test proves nothing about the async hold-back actually spanning frames"
    );
    assert_invariants(app.world());
}

/// Fix round 1 (task-12b-report.md): a reload racing back in while a chunk's unload
/// handover is still queued must not orphan that handover's stale entity. Shrinks the
/// window to nothing (queuing a large backlog of unload handovers — comfortably more than
/// `MAX_ACTIVE_HANDOVERS` can drain in one frame, so most of it is still genuinely queued,
/// not just promoted-and-waiting), then grows it straight back to the original window
/// *while that backlog is still draining* — racing a reload against each pending unload's
/// handover, the exact sequence Critical 2 described: chunk `K` unloads as entity `E1`,
/// queues; before its handover is promoted, `K` is requested again, regenerates, and is
/// shown as a fresh `E2`; when `(K, E1)` finally reaches the front of the queue, `E1` must
/// be despawned rather than dropped on the floor with its `ChunkView`/`Mesh3d` still live
/// (which `every_shown_chunk_has_exactly_one_entity`'s query below would catch as two
/// entities for the same key). The same race exercises Critical 1's fix too: any of these
/// reloads that also needs a load handover (a guest cell ceded back by a neighbour, say)
/// must not have `shown.omit`/`restore` applied against a chunk that turned out to have
/// unloaded in the meantime.
#[test]
fn growing_the_window_back_after_a_shrink_leaves_no_duplicate_or_orphaned_entities() {
    let mut app = headless_app(StoreSettings {
        unload_delay_s: 0.05,
        ..StoreSettings::default()
    });
    let entity = app
        .world_mut()
        .spawn((
            Transform::default(),
            Loader {
                rings: Rings::default(),
            },
        ))
        .id();
    run_until_fully_settled(&mut app, DEFAULT_WINDOW_TOTAL);

    // Shrink to nothing: everything starts lingering at once, so once the (short) unload
    // delay elapses, a large batch becomes `to_unload` in a single `drive_store` call —
    // comfortably more than `MAX_ACTIVE_HANDOVERS` (3) can promote per frame.
    app.world_mut().entity_mut(entity).insert(Loader {
        rings: Rings {
            shaku: 0,
            ken: 0,
            cho: 0,
        },
    });
    let started = std::time::Instant::now();
    loop {
        app.update();
        let backlog = app
            .world()
            .resource::<Handovers>()
            .pending_unload_keys()
            .len();
        if backlog >= 8 {
            break;
        }
        assert!(
            started.elapsed() < common::SETTLE_TIMEOUT,
            "timed out after {:?} waiting for an unload backlog to build up (got {backlog})",
            started.elapsed(),
        );
    }

    // Grow the window straight back to the original default while that backlog is still
    // draining.
    app.world_mut().entity_mut(entity).insert(Loader {
        rings: Rings::default(),
    });
    run_until_fully_settled(&mut app, DEFAULT_WINDOW_TOTAL);

    // No duplicate entity for any chunk (Critical 2's failure mode: an orphaned stale
    // entity left behind by a dropped unload, alongside the reload's fresh one)...
    let mut query = app.world_mut().query::<&ChunkView>();
    let mut seen: HashSet<ChunkKey> = HashSet::new();
    for view in query.iter(app.world()) {
        assert!(seen.insert(view.0), "{:?} has two entities", view.0);
    }
    // ...and `Shown` and the ECS still agree on exactly what is shown.
    let shown = app.world().resource::<Shown>();
    assert_eq!(seen.len(), shown.keys().len());
    assert_eq!(seen.len(), DEFAULT_WINDOW_TOTAL);

    // ...and neither the bookkeeping nor the meshes came out of the churn disagreeing with
    // what the shown set says they must be. This is the test that exercises the reload-races-
    // an-unload sequences, so it is where a stale re-mesh applied against the wrong omission
    // set (Task 12c's defect 1) would show up as a mesh that no `Shown` check could fault.
    assert_invariants(app.world());
}

/// Same three-part condition as `common::run_until_fully_settled` (see its doc comment):
/// nothing in flight, the expected count loaded, and — added in fix round 2
/// (task-12b-report.md) — no handover left queued or mid-re-mesh either, or a settle can
/// read true while a stale handover is still waiting its turn.
fn world_is_fully_settled(world: &World, expected: usize) -> bool {
    let hex_world = world.resource::<HexWorld>();
    let handovers = world.resource::<Handovers>();
    hex_world.store().in_flight_count() == 0
        && hex_world.store().loaded_keys().len() >= expected
        && handovers.is_idle()
}

/// Task 12c: the reachable half of the desired-set staleness check, made deterministic.
///
/// A handover's required omission sets are recomputed from the live shown set every frame,
/// and a finished re-mesh is applied only if the set it was computed against still equals
/// the set now being recorded. For a *partner* chunk that equality is hard to break from
/// outside — the frame's exclusive, ordered claims get in the way. For the **arriving chunk
/// itself** it is wide open: nothing claims a chunk that is not shown yet, so its own desired
/// set can change between its re-mesh being spawned and that re-mesh being applied.
///
/// The window has to be hit on purpose, not hoped for. A chunk that is merely *pending* may
/// still be sitting in the queue with no re-mesh started, and injecting into one of those
/// proves nothing: by the time it is promoted, the change is already part of the set its
/// re-mesh is spawned against. (Written that way first, this test passed against a build
/// with the equality check removed — six runs out of six.) So it uses
/// `Handovers::remeshes_in_flight()` intersected with `pending_load_keys()`, which names
/// exactly the chunks whose *own* re-mesh is on the pool right now, and injects into those.
///
/// Injecting = making one more of the chunk's ken children appear on screen behind the
/// plugin's back. That changes the pending chunk's desired set — and, by how the child is
/// chosen, nothing else's. Exactly one injection per chunk: one is enough once the window is
/// hit precisely, and repeating it every frame would livelock the *correct* implementation,
/// which would re-spawn against a new set forever and never commit.
///
/// With the equality check in place the handover notices on its next look, re-spawns against
/// the new set, and reveals a mesh that omits the injected child. Remove the check — keep
/// whatever re-mesh happens to be running or to have landed — and the reveal pairs a
/// pre-drift mesh with a post-drift omission set. That is defect 1's exact signature behind
/// perfectly healthy bookkeeping, and only a mesh-content check sees it. It has to be taken
/// in the frame of the reveal, too: a cho chunk is re-meshed again every time one of its ken
/// children arrives, so a stale mesh is quietly corrected a few frames later and an
/// end-of-test check cannot tell it from one that was right all along.
#[test]
fn a_pending_chunks_own_desired_set_can_drift_and_the_mesh_still_matches_it() {
    /// Perturb this many different chunks before stopping. One would do; a handful keeps the
    /// test from resting on a single handover happening to reach its reveal.
    const WANTED: usize = 6;

    let mut app = headless_app(StoreSettings::default());
    app.world_mut().spawn((
        Transform::default(),
        Loader {
            rings: Rings::default(),
        },
    ));
    let cfg = WorldConfig::default();
    let mut drifted: HashMap<ChunkKey, ChunkKey> = HashMap::new();
    let mut revealed: Vec<ChunkKey> = Vec::new();

    let started = std::time::Instant::now();
    for _ in 0..common::SETTLE_FRAME_CAP {
        app.update();

        if drifted.len() < WANTED {
            // Cho chunks only: their children are ken chunks, which are cheap to generate
            // and mesh here on the main thread, and — unlike a ken chunk, whose "children"
            // are shaku cells, never a chunk level at all — they are a level `Shown` can
            // legitimately hold.
            let handovers = app.world().resource::<Handovers>();
            let in_flight: HashSet<ChunkKey> = handovers.remeshes_in_flight().into_iter().collect();
            let mut targets: Vec<ChunkKey> = handovers
                .pending_load_keys()
                .into_iter()
                .filter(|key| {
                    key.level == Level::Cho && in_flight.contains(key) && !drifted.contains_key(key)
                })
                .collect();
            targets.sort();
            for key in targets {
                if drifted.len() >= WANTED {
                    break;
                }
                let Some(child) = injectable_child(app.world(), &cfg, key) else {
                    continue;
                };
                inject_shown_chunk(&mut app, &cfg, child);
                drifted.insert(key, child);
            }
        }

        // The moment of truth: each perturbed chunk, in the one frame it first appears.
        let shown_keys: HashSet<ChunkKey> =
            app.world().resource::<Shown>().keys().into_iter().collect();
        for key in drifted.keys() {
            if shown_keys.contains(key) && !revealed.contains(key) {
                revealed.push(*key);
                assert_chunk_mesh_matches(
                    app.world(),
                    &shown_keys,
                    *key,
                    "the frame a chunk whose desired set drifted mid-re-mesh was revealed",
                );
            }
        }

        if world_is_fully_settled(app.world(), DEFAULT_WINDOW_TOTAL) {
            break;
        }
        assert!(
            started.elapsed() < common::SETTLE_TIMEOUT,
            "timed out after {:?} waiting to settle",
            started.elapsed(),
        );
    }

    assert!(
        !drifted.is_empty(),
        "never caught a chunk with its own re-mesh in flight, so no re-mesh was ever spawned \
         against a set that then changed under it; the test proves nothing"
    );
    assert!(
        !revealed.is_empty(),
        "perturbed {} chunk(s) ({drifted:?}) but none was ever revealed, so no possibly-stale \
         mesh was ever applied and looked at; the test proves nothing",
        drifted.len(),
    );

    let shown = app.world().resource::<Shown>();
    for child in drifted.values() {
        assert!(shown.entity(*child).is_some(), "{child:?} vanished");
    }
    assert_invariants(app.world());
}

/// A child of `pending` that can be put on screen without changing the desired set of any
/// *already-shown* chunk — so injecting it perturbs exactly one thing, the pending chunk's
/// own desired set.
///
/// The conditions, all read straight off `Shown`: the child is not shown, its parent
/// (`pending`) is not shown, and none of its same-level neighbours are shown. Those are the
/// only chunks whose omissions can mention it. It must also be inside the world, or nothing
/// draws its cell and omitting it would change no mesh.
fn injectable_child(world: &World, cfg: &WorldConfig, pending: ChunkKey) -> Option<ChunkKey> {
    let shown = world.resource::<Shown>();
    if shown.entity(pending).is_some() {
        return None;
    }
    let child_level = pending.child_level();
    hexworld::owner::children(pending.cell, pending.level)
        .into_iter()
        .map(|cell| ChunkKey::new(child_level, cell))
        .find(|child| {
            hexworld::cell_in_world(child.cell, child_level, cfg)
                && shown.entity(*child).is_none()
                && hexworld::hex::neighbours(child.cell)
                    .into_iter()
                    .all(|n| shown.entity(ChunkKey::new(child_level, n)).is_none())
        })
}

/// Put `key` on screen directly, with the mesh a chunk omitting nothing would have — which
/// is the correct mesh for it, since the caller has checked nothing shown can owe it an
/// omission. Deliberately bypasses `Handovers`: the point is to move the shown set under a
/// handover that is already mid-flight, which nothing in the public API does on purpose.
fn inject_shown_chunk(app: &mut App, cfg: &WorldConfig, key: ChunkKey) {
    let data =
        hexworld::mesh::mesh_chunk(cfg, &hexworld::chunk::generate(cfg, key), &HashSet::new());
    let material = app
        .world()
        .resource::<hexworld_bevy::GroundMaterialHandle>()
        .0
        .clone();
    let handle = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(hexworld_bevy::entities::to_bevy_mesh(&data));
    let entity = app
        .world_mut()
        .spawn((
            Mesh3d(handle),
            MeshMaterial3d(material),
            Transform::from_translation(hexworld_bevy::entities::chunk_origin(key)),
            ChunkView(key),
        ))
        .id();
    app.world_mut()
        .resource_mut::<Shown>()
        .insert_entity(key, entity);
}

/// The omission set every shown chunk must have: the union of (a) the cells of its own
/// currently-shown children, and (b) the guest cells it draws whose true same-level owner is
/// currently shown instead.
///
/// **Deliberately a second implementation of `entities::desired_omissions`.** The production
/// function is the one the plugin actually runs on; this one is written straight off
/// `hexworld`'s public geometry (`mesh::drawn_cells`, `owner::parent_of`,
/// `ChunkKey::parent_key`) with none of the production function's level-uniform offset table,
/// so the assertions below are an independent oracle rather than a check that the
/// implementation agrees with itself. Keep the duplication: calling
/// `entities::desired_omissions` from here would turn every test in this file into a
/// tautology.
fn expected_omissions(
    cfg: &WorldConfig,
    shown_keys: &HashSet<ChunkKey>,
    key: ChunkKey,
) -> HashSet<Hex> {
    let child_level = key.child_level();
    let mut expected: HashSet<Hex> = HashSet::new();

    // (a) cells of currently-shown children.
    for &candidate in shown_keys {
        if candidate.level == child_level && candidate.parent_key() == Some(key) {
            expected.insert(candidate.cell);
        }
    }

    // (b) guest cells whose same-level owner is currently shown.
    for cell in hexworld::mesh::drawn_cells(cfg, key) {
        let owner_cell = hexworld::owner::parent_of(cell, child_level);
        if owner_cell == key.cell {
            continue; // key owns this cell outright, never a guest of itself
        }
        if shown_keys.contains(&ChunkKey::new(key.level, owner_cell)) {
            expected.insert(cell);
        }
    }

    expected
}

/// Both invariants a settled world must satisfy, run at the end of every test in this file.
///
/// Neither subsumes the other. The bookkeeping check cannot see a mesh that disagrees with
/// correct bookkeeping — the shape of Task 12c's defect 1, where two concurrent re-meshes of
/// one chunk raced and the later apply overwrote the earlier one's omission, leaving a
/// doubled surface behind a perfectly consistent `Shown`. The mesh check cannot see a
/// phantom omission recorded against a chunk that has no entity at all, because there is no
/// mesh to look at.
fn assert_invariants(world: &World) {
    assert_omission_invariant(world);
    assert_mesh_content_invariant(world);
}

/// The bookkeeping invariant, over *every* shown chunk at once: its recorded omission set is
/// exactly `expected_omissions` — no more (a phantom omission, drawing nothing extra but
/// sitting there ready to corrupt the next regeneration) and no less (a doubled surface, or,
/// read from the owner's side, a hole).
///
/// Includes a presence check, because iterating shown keys alone never looks at a key that
/// has omissions but no entity — exactly where a phantom omission hides.
fn assert_omission_invariant(world: &World) {
    let hex_world = world.resource::<HexWorld>();
    let shown = world.resource::<Shown>();
    let cfg = *hex_world.store().config();
    let shown_keys: HashSet<ChunkKey> = shown.keys().into_iter().collect();

    for key in shown.keys_with_omissions() {
        assert!(
            shown_keys.contains(&key),
            "{key:?} omits cells but has no entity — a phantom omission left over from a \
             handover applied against a chunk that had already unloaded"
        );
    }

    for &key in &shown_keys {
        let expected = expected_omissions(&cfg, &shown_keys, key);
        let actual = shown.omitted_for(key).clone();
        assert_eq!(
            actual,
            expected,
            "{key:?}'s omission set does not match the content invariant — extra (phantom \
             or stale) cells: {:?}; missing cells: {:?}",
            actual.difference(&expected).collect::<Vec<_>>(),
            expected.difference(&actual).collect::<Vec<_>>(),
        );
    }
}

/// The mesh-content invariant: for every shown chunk, the vertex data actually on its entity
/// must be what `mesh_chunk` produces from that chunk and the omission set the invariant
/// says it should have.
///
/// This is the check no `Shown`-based assertion can make. A chunk's geometry is a pure
/// function of `(key, omissions)`, so comparing the real mesh against
/// `mesh_chunk(cfg, chunk, expected_omissions(...))` catches a mesh that was computed
/// against some *other* omission set and applied anyway — a doubled surface or a hole on
/// screen, with the bookkeeping reading perfectly healthy.
fn assert_mesh_content_invariant(world: &World) {
    let shown_keys: HashSet<ChunkKey> = world.resource::<Shown>().keys().into_iter().collect();
    for &key in &shown_keys {
        assert_chunk_mesh_matches(world, &shown_keys, key, "in the settled world");
    }
}

/// One chunk's half of the mesh-content invariant, so a test can also ask it about a single
/// chunk in a single frame.
///
/// That single-frame form is not a convenience. A cho chunk is re-meshed again every time
/// one of its ken children arrives, so a stale mesh applied at its reveal is quietly
/// corrected a few frames later and an end-of-test check cannot tell a mesh that was right
/// all along from one that was wrong and got fixed. See
/// `a_pending_chunks_own_desired_set_can_drift_and_the_mesh_still_matches_it`.
fn assert_chunk_mesh_matches(
    world: &World,
    shown_keys: &HashSet<ChunkKey>,
    key: ChunkKey,
    when: &str,
) {
    let hex_world = world.resource::<HexWorld>();
    let shown = world.resource::<Shown>();
    let cfg = *hex_world.store().config();

    let omissions = expected_omissions(&cfg, shown_keys, key);
    // A shown chunk whose store data has already been dropped (its unload is under way) is
    // regenerated: generation is pure, so this is the same chunk either way.
    let chunk = hex_world
        .store()
        .chunk(key)
        .cloned()
        .unwrap_or_else(|| hexworld::chunk::generate(&cfg, key));
    let want = hexworld::mesh::mesh_chunk(&cfg, &chunk, &omissions);
    let want_positions: Vec<[f32; 3]> = want
        .positions
        .iter()
        .map(|p| hexworld_bevy::axes::mesh_position(*p))
        .collect();

    let entity = shown
        .entity(key)
        .expect("the caller checked this key is shown");
    let handle = &world
        .get::<Mesh3d>(entity)
        .expect("a shown chunk always has a mesh")
        .0;
    let mesh = world
        .resource::<Assets<Mesh>>()
        .get(handle)
        .expect("the mesh asset is still alive");
    let got = mesh
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .expect("positions")
        .as_float3()
        .expect("positions are f32x3");

    assert_eq!(
        got.len(),
        want_positions.len(),
        "{when}: {key:?}'s mesh has {} vertices, but meshing it with the {} cells the \
         invariant says it must omit gives {} — its mesh was built against a different \
         omission set",
        got.len(),
        omissions.len(),
        want_positions.len(),
    );
    if let Some((i, (a, b))) = got
        .iter()
        .zip(want_positions.iter())
        .enumerate()
        .find(|(_, (a, b))| a != b)
    {
        panic!(
            "{when}: {key:?}'s mesh differs from its expected mesh at vertex {i}: {a:?} vs {b:?}"
        );
    }
}

/// The content invariant, checked on the simplest possible settle: a single loader, no
/// window changes at all. Establishes the baseline the shrink/grow test below also checks
/// under churn.
#[test]
fn a_settled_worlds_omissions_match_the_content_invariant() {
    let mut app = headless_app(StoreSettings::default());
    app.world_mut().spawn((
        Transform::default(),
        Loader {
            rings: Rings::default(),
        },
    ));
    run_until_fully_settled(&mut app, DEFAULT_WINDOW_TOTAL);
    assert_invariants(app.world());
}

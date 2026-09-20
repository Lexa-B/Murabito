use std::collections::HashSet;

use bevy::prelude::*;
use hexworld::{ChunkKey, Hex, Level, Rings, StoreSettings};
use hexworld_bevy::{ChunkView, HexWorld, Loader, Shown};

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
}

fn world_is_fully_settled(world: &World, expected: usize) -> bool {
    let hex_world = world.resource::<HexWorld>();
    hex_world.store().in_flight_count() == 0 && hex_world.store().loaded_keys().len() >= expected
}

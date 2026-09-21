//! The handover properties the spec's "no holes and no cracks" criterion rests on.

use std::collections::HashSet;

use hexworld::chunk::{generate, ChunkKey};
use hexworld::level::Level;
use hexworld::mesh::{drawn_cells, mesh_chunk, MeshData};
use hexworld::plane::{cell_centre_m, corners_m};
use hexworld::{Hex, WorldConfig};

#[test]
fn a_parent_and_its_children_cover_the_ground_exactly_once() {
    // Every ken a cho chunk draws is drawn by that cho chunk, or by the ken's own chunk
    // when it is loaded — never by both, never by neither.
    let cfg = WorldConfig::default();
    let cho_key = ChunkKey::new(Level::Cho, Hex::ZERO);
    let cho_chunk = generate(&cfg, cho_key);
    let all_kens = drawn_cells(&cfg, cho_key);
    let shown_ken = all_kens[17];

    // The child covers its parent's whole territory: every shaku the shown ken owns is
    // among the shaku its own chunk draws. All integer arithmetic — no `round_at`, whose
    // float tie-break disagrees with `owner()`'s exact one even on cells a ken owns
    // outright (that was this test's original, wrong form: see owner.rs's own doc comment
    // on `drawn_offsets` for why deriving membership from `round_at` does not work).
    let ken_drawn: HashSet<Hex> = drawn_cells(&cfg, ChunkKey::new(Level::Ken, shown_ken))
        .into_iter()
        .collect();
    for shaku in hexworld::owner::children(shown_ken, Level::Ken) {
        assert!(
            ken_drawn.contains(&shaku),
            "{shaku:?}, owned by {shown_ken:?}, is a hole in its own chunk"
        );
    }

    // A top-face fan puts a vertex exactly at a drawn cell's own centre, and only there
    // (side-wall vertices sit at corners, offset from centre) — so counting cells whose
    // exact local centre has a vertex counts how many of the cho's declared cells the mesh
    // actually drew.
    let origin = cell_centre_m(cho_key.cell, Level::Cho);
    let local = |cell: Hex| -> (f32, f32) {
        let (e, n) = cell_centre_m(cell, Level::Ken);
        ((e - origin.0) as f32, (n - origin.1) as f32)
    };
    let count_drawn = |mesh: &MeshData| {
        all_kens
            .iter()
            .filter(|c| {
                let (px, py) = local(**c);
                mesh.positions
                    .iter()
                    .any(|v| (v[0] - px).abs() < 1e-4 && (v[1] - py).abs() < 1e-4)
            })
            .count()
    };

    let full_mesh = mesh_chunk(&cfg, &cho_chunk, &HashSet::new());
    assert_eq!(
        count_drawn(&full_mesh),
        all_kens.len(),
        "every declared cell should be drawn when nothing is omitted"
    );

    let mut omitted = HashSet::new();
    omitted.insert(shown_ken);
    let parent_mesh = mesh_chunk(&cfg, &cho_chunk, &omitted);
    assert_eq!(
        count_drawn(&parent_mesh),
        all_kens.len() - 1,
        "omitting one ken should drop the drawn count by exactly one"
    );

    let child_mesh = mesh_chunk(
        &cfg,
        &generate(&cfg, ChunkKey::new(Level::Ken, shown_ken)),
        &HashSet::new(),
    );
    assert!(parent_mesh.triangle_count() > 0);
    assert!(child_mesh.triangle_count() > 0);
}

#[test]
fn shared_cells_are_exactly_guests() {
    // Two neighbouring ken chunks deliberately draw some cells in common: the guest cells
    // sitting on the tie between them. That overlap is what stops a half-cell gap opening
    // along the handover; it is not a bug. Every shared cell must be a genuine tie — equal
    // d2 to both centres — and owned by exactly one of the two chunks.
    let cfg = WorldConfig::default();
    let a = ChunkKey::new(Level::Ken, Hex::ZERO);
    let b = ChunkKey::new(Level::Ken, Hex::new(1, 0));
    let cells_a: HashSet<Hex> = drawn_cells(&cfg, a).into_iter().collect();
    let cells_b: HashSet<Hex> = drawn_cells(&cfg, b).into_iter().collect();
    let shared: Vec<Hex> = cells_a.intersection(&cells_b).copied().collect();
    assert!(
        !shared.is_empty(),
        "neighbouring chunks should share guest cells"
    );

    let centre_a = hexworld::owner::centre_shaku(a.cell, Level::Ken);
    let centre_b = hexworld::owner::centre_shaku(b.cell, Level::Ken);
    for cell in shared {
        let da = hexworld::d2(cell - centre_a);
        let db = hexworld::d2(cell - centre_b);
        assert_eq!(
            da, db,
            "{cell:?} is shared but not equidistant from {a:?} and {b:?}"
        );

        let owned_by_a = hexworld::owner::parent_of(cell, Level::Shaku) == a.cell;
        let owned_by_b = hexworld::owner::parent_of(cell, Level::Shaku) == b.cell;
        assert!(
            owned_by_a ^ owned_by_b,
            "{cell:?} must be owned by exactly one of the two neighbours, got a={owned_by_a} b={owned_by_b}"
        );
    }
}

#[test]
fn every_cell_is_drawn_by_at_least_one_chunk() {
    // Over a patch of shaku cells, each is drawn by its owning ken's chunk — and possibly
    // also by a neighbouring ken as a guest — so nothing is missed.
    let cfg = WorldConfig::default();
    let mut drawn: HashSet<Hex> = HashSet::new();
    for ken in hexworld::hex::range(Hex::ZERO, 2) {
        for cell in drawn_cells(&cfg, ChunkKey::new(Level::Ken, ken)) {
            drawn.insert(cell);
        }
    }
    for ken in hexworld::hex::range(Hex::ZERO, 1) {
        for shaku in hexworld::owner::children(ken, Level::Ken) {
            assert!(
                drawn.contains(&shaku),
                "{shaku:?}, owned by {ken:?}, is a hole"
            );
        }
    }
}

#[test]
fn a_patch_of_neighbouring_chunks_leaves_no_hole() {
    // Take a ken and its six neighbours; every shaku owned by the middle ken is drawn by
    // something in the patch. A guest shaku may legitimately be drawn by more than one of
    // them, so the property is "at least one", not "exactly one".
    let cfg = WorldConfig::default();
    let centre = Hex::ZERO;
    let keys: Vec<ChunkKey> = hexworld::hex::range(centre, 1)
        .into_iter()
        .map(|c| ChunkKey::new(Level::Ken, c))
        .collect();
    let mut drawn: HashSet<Hex> = HashSet::new();
    for key in &keys {
        for cell in drawn_cells(&cfg, *key) {
            drawn.insert(cell);
        }
    }
    // Every shaku owned by the centre ken is drawn by something in the patch.
    for shaku in hexworld::owner::children(centre, Level::Ken) {
        assert!(drawn.contains(&shaku), "{shaku:?} is a hole");
    }
}

#[test]
fn outer_walls_reach_the_world_bottom() {
    let cfg = WorldConfig::default();
    let chunk = generate(&cfg, ChunkKey::new(Level::Ken, Hex::ZERO));
    let mesh = mesh_chunk(&cfg, &chunk, &HashSet::new());
    let lowest = mesh.positions.iter().map(|p| p[2]).fold(f32::MAX, f32::min);
    let expected = cfg.layer_bottom_m(cfg.bottom_layer) as f32;
    assert!((lowest - expected).abs() < 1e-3, "{lowest} vs {expected}");
}

#[test]
fn an_omitted_cells_neighbours_grow_full_depth_walls() {
    // When a child chunk takes over a cell, the cells around the hole must wall down to
    // the bottom, so a height difference between detail levels cannot show as a crack.
    // A full-depth wall is exactly one quad per (cell, direction): 2 vertices at the world
    // bottom per walled edge. A cell only grows a wall facing `target` if that cell is
    // itself part of this chunk's own drawn set (only cells the chunk actually visits get
    // to emit geometry) — so the walled-edge count is the number of `target`'s neighbours
    // that are themselves in `drawn_cells(cfg, key)`. For `target` = offset 50 in the cho's
    // drawn ken cells, that is 3 (this cell sits near the edge of the cho's own drawn
    // hexagon, so three of its neighbours belong to a sibling cho instead), giving 6
    // bottom vertices — 2 per walled edge.
    //
    // A position-only radius filter is not enough any more: since the mesher fix, a
    // chunk's whole perimeter walls down to the bottom too (that was the production bug
    // this task found), so cells elsewhere on the cho's own perimeter can also contribute
    // bottom vertices within an easy radius of `target`. Matching each wall's exact outward
    // normal as well as its position — computed the same way `mesh_chunk` computes it, from
    // the drawing neighbour's own local position and its own direction index — isolates
    // exactly the 3 walls facing `target`, the same technique Task 9 used for the
    // equivalent single-wall case in `mesh.rs`.
    let cfg = WorldConfig::default();
    let key = ChunkKey::new(Level::Cho, Hex::ZERO);
    let chunk = generate(&cfg, key);
    let all_drawn_list = drawn_cells(&cfg, key);
    let all_drawn: HashSet<Hex> = all_drawn_list.iter().copied().collect();
    let target = all_drawn_list[50];
    let mut omitted = HashSet::new();
    omitted.insert(target);
    let mesh = mesh_chunk(&cfg, &chunk, &omitted);

    let walled: Vec<usize> = (0..6)
        .filter(|k| all_drawn.contains(&(target + hexworld::DIRECTIONS[*k])))
        .collect();
    assert_eq!(
        walled.len(),
        3,
        "expected 3 of target's neighbours to be in this chunk's own drawn set"
    );

    let bottom = cfg.layer_bottom_m(cfg.bottom_layer) as f32;
    let origin = cell_centre_m(key.cell, Level::Cho);
    let corners = corners_m(Level::Ken); // the cho's child level
    let mut expected: Vec<((f32, f32), (f32, f32))> = Vec::new();
    for k in walled {
        let neighbour = target + hexworld::DIRECTIONS[k];
        let (ne, nn) = cell_centre_m(neighbour, Level::Ken);
        let (nx, ny) = ((ne - origin.0) as f32, (nn - origin.1) as f32);
        // The wall is drawn by `neighbour`, at the direction index facing back at target.
        let j = (k + 3) % 6;
        let (ax, ay) = corners[(j + 5) % 6];
        let (bx, by) = corners[j];
        let (mx, my) = ((ax + bx) / 2.0, (ay + by) / 2.0);
        let len = (mx * mx + my * my).sqrt();
        let normal = ((mx / len) as f32, (my / len) as f32);
        expected.push(((nx + ax as f32, ny + ay as f32), normal));
        expected.push(((nx + bx as f32, ny + by as f32), normal));
    }

    let near_hole = mesh
        .positions
        .iter()
        .enumerate()
        .filter(|(_, p)| (p[2] - bottom).abs() < 1e-3)
        .filter(|(i, p)| {
            let n = mesh.normals[*i];
            expected.iter().any(|((ex, ey), (enx, eny))| {
                (p[0] - ex).abs() < 1e-3
                    && (p[1] - ey).abs() < 1e-3
                    && (n[0] - enx).abs() < 1e-4
                    && (n[1] - eny).abs() < 1e-4
            })
        })
        .count();
    assert_eq!(
        near_hole, 6,
        "expected one full-depth quad (2 bottom vertices) for each of the 3 walled edges"
    );
}

#[test]
fn the_world_ends_in_a_wall() {
    // Walk outward along ken level until we find a ken chunk that genuinely straddles the
    // world boundary: it draws at least one cell, and at least one of those cells has a
    // neighbour outside the world. That is the chunk whose wall this test is actually
    // about — not one (like the brief's original formula) that lies entirely outside the
    // world and so never exercises anything.
    let cfg = WorldConfig::default();
    let mut edge_ken = None;
    for q in 0..20_000 {
        let ken = Hex::new(q, 0);
        let cells = drawn_cells(&cfg, ChunkKey::new(Level::Ken, ken));
        if cells.is_empty() {
            continue;
        }
        let straddles = cells.iter().any(|c| {
            hexworld::DIRECTIONS
                .iter()
                .any(|d| !hexworld::cell_in_world(*c + *d, Level::Shaku, &cfg))
        });
        if straddles {
            edge_ken = Some(ken);
            break;
        }
    }
    let edge_ken = edge_ken.expect("no ken chunk straddling the world boundary was found");

    let key = ChunkKey::new(Level::Ken, edge_ken);
    let cells = drawn_cells(&cfg, key);
    assert!(!cells.is_empty());
    for cell in &cells {
        assert!(
            hexworld::cell_in_world(*cell, Level::Shaku, &cfg),
            "{cell:?} is outside"
        );
    }

    // And the chunk actually walls that boundary down to the world bottom.
    let chunk = generate(&cfg, key);
    let mesh = mesh_chunk(&cfg, &chunk, &HashSet::new());
    let lowest = mesh.positions.iter().map(|p| p[2]).fold(f32::MAX, f32::min);
    let expected = cfg.layer_bottom_m(cfg.bottom_layer) as f32;
    assert!((lowest - expected).abs() < 1e-3, "{lowest} vs {expected}");
}

#[test]
#[ignore = "measurement, not a check: cargo test -p hexworld --test handover -- --ignored --nocapture"]
fn measure_triangle_counts() {
    let cfg = WorldConfig::default();
    for (level, chunks) in [(Level::Ken, 37), (Level::Cho, 37), (Level::Ri, 1)] {
        let chunk = generate(&cfg, ChunkKey::new(level, Hex::ZERO));
        let started = std::time::Instant::now();
        let mesh = mesh_chunk(&cfg, &chunk, &HashSet::new());
        let elapsed = started.elapsed();
        println!(
            "{:>5} chunk: {:>8} triangles, {:>8} vertices, {:?} to mesh; window of {chunks}: {} triangles",
            level.name(),
            mesh.triangle_count(),
            mesh.positions.len(),
            elapsed,
            mesh.triangle_count() * chunks
        );
    }
}

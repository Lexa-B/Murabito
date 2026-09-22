//! Turning chunks into triangles. Pure geometry: Bevy's math types only, no ECS, no GPU.
//!
//! Positions are Bevy-axis `[x, y, z]` in shaku, relative to the chunk's own centre.
//! A chunk draws every child cell whose centre lies in its parent's ideal hexagon
//! (`drawn_offsets`), minus the cells whose own chunk is on screen.

use std::collections::{HashMap, HashSet};

use bevy::math::Vec3;

use crate::chunk::{Chunk, ChunkKey};
use crate::column::Column;
use crate::config::WorldConfig;
use crate::hex::{Hex, DIRECTIONS};
use crate::layer::{layer_bottom, layer_top};
use crate::level::Level;
use crate::owner::{centre_child, drawn_offsets};
use crate::placeholder_terrain::column_at;
use crate::plane::{cell_centre, corners};
use crate::world::{cell_in_world, world_ri};

#[derive(Clone, Default, Debug)]
pub struct MeshData {
    /// Bevy-axis `[x, y, z]` in shaku, relative to the chunk's centre.
    pub positions: Vec<[f32; 3]>,
    /// Bevy-axis, like the positions: top faces are `[0, 1, 0]`.
    pub normals: Vec<[f32; 3]>,
    pub colours: Vec<[f32; 4]>,
    /// (edge_w, border_level, cell_level, 0): see `border_level`.
    pub line: Vec<[f32; 4]>,
    pub indices: Vec<u32>,
}

impl MeshData {
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    fn push_vertex(
        &mut self,
        pos: [f32; 3],
        normal: [f32; 3],
        colour: [f32; 4],
        line: [f32; 4],
    ) -> u32 {
        self.positions.push(pos);
        self.normals.push(normal);
        self.colours.push(colour);
        self.line.push(line);
        (self.positions.len() - 1) as u32
    }

    fn push_triangle(&mut self, a: u32, b: u32, c: u32) {
        self.indices.extend_from_slice(&[a, b, c]);
    }
}

/// The cells a chunk draws, in world coordinates at the chunk's child level.
pub fn drawn_cells(cfg: &WorldConfig, key: ChunkKey) -> Vec<Hex> {
    if key.level == Level::World {
        return world_ri(cfg);
    }
    let centre = centre_child(key.cell, key.level);
    let child = key.child_level();
    drawn_offsets(key.level)
        .iter()
        .map(|o| centre + *o)
        .filter(|c| cell_in_world(*c, child, cfg))
        .collect()
}

/// The highest level at which two neighbouring cells belong to different parents.
/// Their own level if they only differ there; this is what the hex lines are styled by.
fn border_level(a: Hex, b: Hex, child: Level) -> Level {
    let mut result = child;
    let mut level = child;
    while let Some(up_level) = level.parent() {
        if up_level == Level::World {
            break;
        }
        if crate::owner::up(a, child, up_level) != crate::owner::up(b, child, up_level) {
            result = up_level;
        }
        level = up_level;
    }
    result
}

/// Mesh one chunk. `omitted` holds the child cells whose own chunk is on screen.
pub fn mesh_chunk(cfg: &WorldConfig, chunk: &Chunk, omitted: &HashSet<Hex>) -> MeshData {
    let key = chunk.key;
    let child = key.child_level();
    let origin = if key.level == Level::World {
        Vec3::ZERO
    } else {
        cell_centre(key.cell, key.level)
    };

    // The chunk's own columns, plus any guest or neighbour column generated on demand.
    let mut columns: HashMap<Hex, Column> = HashMap::with_capacity(chunk.len() * 2);
    for (cell, column) in chunk.cells.iter().zip(chunk.columns.iter()) {
        columns.insert(*cell, column.clone());
    }
    let column_for = |cell: Hex, columns: &mut HashMap<Hex, Column>| -> Column {
        if let Some(c) = columns.get(&cell) {
            return c.clone();
        }
        let generated = if cell_in_world(cell, child, cfg) {
            column_at(cfg, cell, child)
        } else {
            Column::default() // outside the world: air, so the world ends in a wall
        };
        columns.insert(cell, generated.clone());
        generated
    };

    let corners = corners(child);
    let mut mesh = MeshData::default();
    let cell_level = child as u32 as f32;
    let bottom = layer_bottom(cfg.bottom_layer);

    // This chunk's own drawn set, built once: full depth is for edges facing a cell this
    // chunk does not draw, not merely a cell some other chunk might. A neighbour outside
    // this set is only ever culled against a same-level column generated on demand for a
    // *different* chunk's territory — a coarser or absent neighbour there can sit at a
    // different height, and only a full-depth wall is guaranteed not to crack against it.
    let drawn_list = drawn_cells(cfg, key);
    let drawn: HashSet<Hex> = drawn_list.iter().copied().collect();

    for cell in &drawn_list {
        let cell = *cell;
        if omitted.contains(&cell) {
            continue;
        }
        let column = column_for(cell, &mut columns);
        if column.runs.is_empty() {
            continue;
        }
        let c = cell_centre(cell, child) - origin;

        // Neighbours, for face culling and for the border levels of the lines.
        let neighbour_cells: Vec<Hex> = DIRECTIONS.iter().map(|d| cell + *d).collect();
        let neighbour_columns: Vec<Column> = neighbour_cells
            .iter()
            .map(|n| column_for(*n, &mut columns))
            .collect();
        let neighbour_drawn: Vec<bool> = neighbour_cells
            .iter()
            .map(|n| drawn.contains(n) && !omitted.contains(n))
            .collect();

        // --- top faces: one fan per run whose layer above is air ---
        for run in &column.runs {
            if column.is_solid(run.top + 1) {
                continue;
            }
            let top = layer_top(run.top);
            let colour = shaded(run.material, cell);
            let centre_v = mesh.push_vertex(
                (c + Vec3::Y * top).to_array(),
                [0.0, 1.0, 0.0],
                colour,
                [0.0, 0.0, cell_level, 0.0],
            );
            for k in 0..6 {
                let border = border_level(cell, neighbour_cells[k], child) as u32 as f32;
                let a = corners[(k + 5) % 6];
                let b = corners[k];
                let va = mesh.push_vertex(
                    (c + a + Vec3::Y * top).to_array(),
                    [0.0, 1.0, 0.0],
                    colour,
                    [1.0, border, cell_level, 0.0],
                );
                let vb = mesh.push_vertex(
                    (c + b + Vec3::Y * top).to_array(),
                    [0.0, 1.0, 0.0],
                    colour,
                    [1.0, border, cell_level, 0.0],
                );
                mesh.push_triangle(centre_v, va, vb);
            }
        }

        // --- side faces: where the neighbour is air over those layers ---
        for (k, neighbour) in neighbour_columns.iter().enumerate() {
            let a = corners[(k + 5) % 6];
            let b = corners[k];
            // Outward normal: the edge's midpoint direction.
            let normal = ((a + b) / 2.0).normalize_or_zero().to_array();
            let line = [0.0, 0.0, cell_level, 0.0];
            let full_depth = !neighbour_drawn[k];
            if full_depth {
                // A full-depth wall is one silhouette quad for the whole column, from the
                // world bottom to the topmost run's top — not one quad per run, which
                // would overlap (and z-fight) across any air gap between runs.
                let topmost = column.runs.last().expect("checked non-empty above");
                let top = layer_top(topmost.top);
                let colour = shaded(topmost.material, cell);
                let v0 =
                    mesh.push_vertex((c + a + Vec3::Y * bottom).to_array(), normal, colour, line);
                let v1 =
                    mesh.push_vertex((c + b + Vec3::Y * bottom).to_array(), normal, colour, line);
                let v2 = mesh.push_vertex((c + b + Vec3::Y * top).to_array(), normal, colour, line);
                let v3 = mesh.push_vertex((c + a + Vec3::Y * top).to_array(), normal, colour, line);
                // Seen from outside the cell, anticlockwise.
                mesh.push_triangle(v0, v1, v2);
                mesh.push_triangle(v0, v2, v3);
                continue;
            }
            for run in &column.runs {
                let mut layer = run.bottom;
                while layer <= run.top {
                    if neighbour.is_solid(layer) {
                        layer += 1;
                        continue;
                    }
                    // Extend while the wall continues.
                    let start = layer;
                    while layer <= run.top && !neighbour.is_solid(layer) {
                        layer += 1;
                    }
                    let top = layer_top(layer - 1);
                    let bottom_of_wall = layer_bottom(start);
                    let colour = shaded(run.material, cell);
                    let v0 = mesh.push_vertex(
                        (c + a + Vec3::Y * bottom_of_wall).to_array(),
                        normal,
                        colour,
                        line,
                    );
                    let v1 = mesh.push_vertex(
                        (c + b + Vec3::Y * bottom_of_wall).to_array(),
                        normal,
                        colour,
                        line,
                    );
                    let v2 =
                        mesh.push_vertex((c + b + Vec3::Y * top).to_array(), normal, colour, line);
                    let v3 =
                        mesh.push_vertex((c + a + Vec3::Y * top).to_array(), normal, colour, line);
                    // Seen from outside the cell, anticlockwise.
                    mesh.push_triangle(v0, v1, v2);
                    mesh.push_triangle(v0, v2, v3);
                }
            }
        }
    }
    mesh
}

/// A material's colour, shaded slightly per cell so neighbouring cells read apart.
fn shaded(material: crate::column::Material, cell: Hex) -> [f32; 4] {
    let base = material.colour();
    let hash = crate::noise::gradient_2d(0x5EED, cell.q() as f32 * 0.37, cell.r() as f32 * 0.51);
    let k = 1.0 + 0.06 * hash;
    [base[0] * k, base[1] * k, base[2] * k, 1.0]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn ken_chunk() -> (WorldConfig, crate::chunk::Chunk) {
        let cfg = WorldConfig::default();
        let chunk = crate::chunk::generate(&cfg, ChunkKey::new(Level::Ken, Hex::ZERO));
        (cfg, chunk)
    }

    #[test]
    fn draws_the_ideal_hexagons_worth_of_cells() {
        let cfg = WorldConfig::default();
        let cells = drawn_cells(&cfg, ChunkKey::new(Level::Ken, Hex::ZERO));
        assert_eq!(cells.len(), crate::owner::drawn_offsets(Level::Ken).len());
    }

    #[test]
    fn produces_consistent_buffers() {
        let (cfg, chunk) = ken_chunk();
        let mesh = mesh_chunk(&cfg, &chunk, &HashSet::new());
        assert_eq!(mesh.positions.len(), mesh.normals.len());
        assert_eq!(mesh.positions.len(), mesh.colours.len());
        assert_eq!(mesh.positions.len(), mesh.line.len());
        assert_eq!(mesh.indices.len() % 3, 0);
        assert!(mesh
            .indices
            .iter()
            .all(|i| (*i as usize) < mesh.positions.len()));
        assert!(mesh.triangle_count() > 0);
    }

    #[test]
    fn top_faces_point_up_and_wind_anticlockwise() {
        // Anticlockwise seen from above (+Y), with Bevy's front faces anticlockwise:
        // (b - a) × (c - a) points up.
        let (cfg, chunk) = ken_chunk();
        let mesh = mesh_chunk(&cfg, &chunk, &HashSet::new());
        let mut tops = 0;
        for tri in mesh.indices.chunks(3) {
            let p: Vec<Vec3> = tri
                .iter()
                .map(|i| Vec3::from(mesh.positions[*i as usize]))
                .collect();
            let n = Vec3::from(mesh.normals[tri[0] as usize]);
            if n == Vec3::Y {
                tops += 1;
                assert!((p[1] - p[0]).cross(p[2] - p[0]).y > 0.0, "{p:?}");
            }
        }
        assert!(tops > 0);
    }

    #[test]
    fn side_walls_face_outward() {
        // A wall's winding must agree with its outward normal, so its front face is the
        // one seen from outside the cell.
        let (cfg, chunk) = ken_chunk();
        let mesh = mesh_chunk(&cfg, &chunk, &HashSet::new());
        let mut walls = 0;
        for tri in mesh.indices.chunks(3) {
            let p: Vec<Vec3> = tri
                .iter()
                .map(|i| Vec3::from(mesh.positions[*i as usize]))
                .collect();
            let n = Vec3::from(mesh.normals[tri[0] as usize]);
            if n.y == 0.0 {
                walls += 1;
                assert!((p[1] - p[0]).cross(p[2] - p[0]).dot(n) > 0.0, "{p:?} {n}");
            }
        }
        assert!(walls > 0);
    }

    #[test]
    fn a_flat_columns_top_sits_at_its_layer_top() {
        let cfg = WorldConfig::default();
        let chunk = crate::chunk::generate(&cfg, ChunkKey::new(Level::Ken, Hex::ZERO));
        let mesh = mesh_chunk(&cfg, &chunk, &HashSet::new());
        let centre_column = chunk.column(Hex::ZERO).expect("the centre shaku");
        let expected = centre_column.surface_height();
        // The centre cell's top face is at the chunk's own centre, at (0, 0) horizontally.
        let at_centre = mesh
            .positions
            .iter()
            .filter(|p| p[0].abs() < 1e-4 && p[2].abs() < 1e-4)
            .map(|p| p[1])
            .fold(f32::MIN, f32::max);
        assert!(
            (at_centre - expected).abs() < 1e-3,
            "{at_centre} vs {expected}"
        );
    }

    #[test]
    fn an_omitted_cell_is_gone_and_its_hole_is_walled() {
        let (cfg, chunk) = ken_chunk();
        let all = mesh_chunk(&cfg, &chunk, &HashSet::new());
        let mut omitted = HashSet::new();
        omitted.insert(Hex::ZERO);
        let fewer = mesh_chunk(&cfg, &chunk, &omitted);

        // The omitted cell's own geometry is gone. Only a cell's own top-face fan puts a
        // vertex exactly at its centre, so no vertex there means the cell is not drawn.
        assert!(!fewer
            .positions
            .iter()
            .any(|p| p[0].abs() < 1e-4 && p[2].abs() < 1e-4));
        assert!(all
            .positions
            .iter()
            .any(|p| p[0].abs() < 1e-4 && p[2].abs() < 1e-4));

        // Omitting a cell opens a boundary between detail levels, so its neighbours wall the
        // hole down to the world bottom: that is what stops a height step becoming a crack.
        // We deliberately do NOT assert on the raw triangle count here: whether omitting an
        // interior cell nets more or fewer triangles overall depends on meshing details
        // (how many material runs a column has, how many full-depth walls happen to merge)
        // that have nothing to do with the behaviour under test. Measured for this chunk and
        // config: 354 triangles all-drawn vs 352 with the centre omitted, and 0 vs 12 vertices
        // at the world bottom near the hole — fewer total triangles, but still walled.
        let bottom = layer_bottom(cfg.bottom_layer);
        let walls_at_bottom = |m: &MeshData| {
            m.positions
                .iter()
                .filter(|p| (p[1] - bottom).abs() < 1e-3)
                .filter(|p| (p[0] * p[0] + p[2] * p[2]).sqrt() < Level::Shaku.width() * 1.2)
                .count()
        };
        // Exactly one full-depth quad per walled edge: 6 edges × 2 bottom vertices = 12.
        // An inequality would not discriminate — emitting one quad per run (the bug this
        // replaced) gives 36 and would still satisfy `> 0`.
        assert_eq!(walls_at_bottom(&all), 0);
        assert_eq!(walls_at_bottom(&fewer), 12);
    }

    #[test]
    fn a_full_depth_wall_is_one_quad_even_when_the_column_has_a_gap() {
        use crate::column::{Material, Run};

        let cfg = WorldConfig::default();
        let key = ChunkKey::new(Level::Ken, Hex::ZERO);
        let mut chunk = crate::chunk::generate(&cfg, key);
        // A column with an air gap: rock low down, grass higher up, nothing between.
        let gapped = Column {
            runs: vec![
                Run {
                    bottom: cfg.bottom_layer,
                    top: -8,
                    material: Material::Rock,
                },
                Run {
                    bottom: -3,
                    top: -1,
                    material: Material::Grass,
                },
            ],
        };
        // Put it on any owned cell, and force one of its outward faces to be full depth by
        // omitting one neighbour — with the default world (radius 0) a `Ken` chunk near the
        // origin sits nowhere near the world's actual edge, so "the chunk's own border" does
        // not by itself make a neighbour full-depth; omitting one does, regardless of chunk
        // or world geometry.
        let target = chunk.cells[0];
        let index = chunk.cells.iter().position(|c| *c == target).unwrap();
        chunk.columns[index] = gapped;
        let mut omitted = HashSet::new();
        omitted.insert(target + DIRECTIONS[0]);

        let mesh = mesh_chunk(&cfg, &chunk, &omitted);

        // Vertices at the world bottom, facing exactly direction 0 (the omitted neighbour's
        // direction) and close to target. Filtering on the outward normal as well as
        // position is what makes this robust: `target` sits on this chunk's own perimeter
        // (`chunk.cells[0]`, `(-4, 2, 2)` for this chunk), so most of its edges are
        // full depth already (this chunk's own border walls down to the world bottom all the
        // way around, per this round's fix), and several *other* cells elsewhere on the same
        // perimeter also happen to face direction 0. Those are far away (the nearest is
        // more than a chunk-width away) and have a different outward normal from any cell
        // whose edge faces a *different* direction, including target's own other four
        // full-depth edges and the one drawn neighbour that shares a corner with this edge
        // (its wall faces direction 5, not 0) — so normal plus a modest radius isolates
        // target's own direction-0 quad exactly, regardless of how the rest of the chunk's
        // perimeter behaves. One quad is 2 vertices; the old per-run code gave 4 for this
        // two-run column (one quad per run, each starting at the world bottom).
        let bottom = layer_bottom(cfg.bottom_layer);
        let t = cell_centre(target, Level::Shaku);
        let corners = corners(Level::Shaku);
        let dir0 = ((corners[5] + corners[0]) / 2.0).normalize();
        let walls_at_bottom = mesh
            .positions
            .iter()
            .enumerate()
            .filter(|(_, p)| (p[1] - bottom).abs() < 1e-3)
            .filter(|(_, p)| {
                let (dx, dz) = (p[0] - t.x, p[2] - t.z);
                (dx * dx + dz * dz).sqrt() < Level::Shaku.width()
            })
            .filter(|(i, _)| {
                let n = mesh.normals[*i];
                (n[0] - dir0.x).abs() < 1e-4 && (n[2] - dir0.z).abs() < 1e-4
            })
            .count();
        assert_eq!(walls_at_bottom, 2);

        // No two triangles may share all three vertex positions: overlapping full-depth
        // walls would produce exactly that.
        let mut seen: Vec<[[f32; 3]; 3]> = Vec::new();
        for tri in mesh.indices.chunks(3) {
            let mut v = [
                mesh.positions[tri[0] as usize],
                mesh.positions[tri[1] as usize],
                mesh.positions[tri[2] as usize],
            ];
            v.sort_by(|a, b| a.partial_cmp(b).unwrap());
            assert!(!seen.contains(&v), "duplicate triangle: {v:?}");
            seen.push(v);
        }
    }

    #[test]
    fn a_chunks_perimeter_walls_down_even_with_nothing_omitted() {
        let (cfg, chunk) = ken_chunk();
        // `chunk.cells[0]`, `(-4, 2, 2)`, is a real perimeter cell of this chunk: its
        // neighbours in directions 2 through 5 belong to sibling chunks, not this one.
        // Its direction-2 neighbour is in-world and not omitted — the *only* reason this
        // edge is full depth is that the neighbour is outside this chunk's own drawn set.
        // Without the fix (testing membership rather than `cell_in_world`), this edge would
        // be culled as an ordinary, same-level neighbour instead.
        let target = chunk.cells[0];
        let mesh = mesh_chunk(&cfg, &chunk, &HashSet::new());

        let bottom = layer_bottom(cfg.bottom_layer);
        let t = cell_centre(target, Level::Shaku);
        let corners = corners(Level::Shaku);
        // The mesher pairs direction k with the cell edge between corners (k+5)%6 and k.
        let dir2 = ((corners[1] + corners[2]) / 2.0).normalize();
        let walls_at_bottom = mesh
            .positions
            .iter()
            .enumerate()
            .filter(|(_, p)| (p[1] - bottom).abs() < 1e-3)
            .filter(|(_, p)| {
                let (dx, dz) = (p[0] - t.x, p[2] - t.z);
                (dx * dx + dz * dz).sqrt() < Level::Shaku.width()
            })
            .filter(|(i, _)| {
                let n = mesh.normals[*i];
                (n[0] - dir2.x).abs() < 1e-4 && (n[2] - dir2.z).abs() < 1e-4
            })
            .count();
        assert_eq!(walls_at_bottom, 2);
    }

    #[test]
    fn meshing_is_deterministic() {
        let (cfg, chunk) = ken_chunk();
        let a = mesh_chunk(&cfg, &chunk, &HashSet::new());
        let b = mesh_chunk(&cfg, &chunk, &HashSet::new());
        assert_eq!(a.indices, b.indices);
        assert_eq!(a.positions, b.positions);
    }

    #[test]
    fn every_level_meshes() {
        let cfg = WorldConfig::default();
        for level in [Level::Ken, Level::Cho, Level::Ri, Level::World] {
            let chunk = crate::chunk::generate(&cfg, ChunkKey::new(level, Hex::ZERO));
            let mesh = mesh_chunk(&cfg, &chunk, &HashSet::new());
            assert!(mesh.triangle_count() > 0, "{level:?} produced nothing");
        }
    }
}

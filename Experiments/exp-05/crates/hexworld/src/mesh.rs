//! Turning chunks into triangles. Pure geometry: no engine types, no GPU.
//!
//! Positions are (east, north, height) in metres, relative to the chunk's own centre.
//! A chunk draws every child cell whose centre lies in its parent's ideal hexagon
//! (`drawn_offsets`), minus the cells whose own chunk is on screen.

use std::collections::{HashMap, HashSet};

use crate::chunk::{Chunk, ChunkKey};
use crate::column::Column;
use crate::config::WorldConfig;
use crate::hex::{Hex, DIRECTIONS};
use crate::level::Level;
use crate::owner::{centre_child, drawn_offsets};
use crate::placeholder_terrain::column_at;
use crate::plane::{cell_centre_m, corners_m};
use crate::world::{cell_in_world, world_ri};

#[derive(Clone, Default, Debug)]
pub struct MeshData {
    /// (east, north, height) in metres, relative to the chunk's centre.
    pub positions: Vec<[f32; 3]>,
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
    let (origin_e, origin_n) = if key.level == Level::World {
        (0.0, 0.0)
    } else {
        cell_centre_m(key.cell, key.level)
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

    let corners = corners_m(child);
    let mut mesh = MeshData::default();
    let cell_level = child as u32 as f32;
    let bottom_m = cfg.layer_bottom_m(cfg.bottom_layer) as f32;

    for cell in drawn_cells(cfg, key) {
        if omitted.contains(&cell) {
            continue;
        }
        let column = column_for(cell, &mut columns);
        if column.runs.is_empty() {
            continue;
        }
        let (ce, cn) = cell_centre_m(cell, child);
        let (cx, cy) = ((ce - origin_e) as f32, (cn - origin_n) as f32);

        // Neighbours, for face culling and for the border levels of the lines.
        let neighbour_cells: Vec<Hex> = DIRECTIONS.iter().map(|d| cell + *d).collect();
        let neighbour_columns: Vec<Column> = neighbour_cells
            .iter()
            .map(|n| column_for(*n, &mut columns))
            .collect();
        let neighbour_drawn: Vec<bool> = neighbour_cells
            .iter()
            .map(|n| !omitted.contains(n) && cell_in_world(*n, child, cfg))
            .collect();

        // --- top faces: one fan per run whose layer above is air ---
        for run in &column.runs {
            if column.is_solid(run.top + 1) {
                continue;
            }
            let top_m = cfg.layer_top_m(run.top) as f32;
            let colour = shaded(run.material, cell);
            let centre_v = mesh.push_vertex(
                [cx, cy, top_m],
                [0.0, 0.0, 1.0],
                colour,
                [0.0, 0.0, cell_level, 0.0],
            );
            for k in 0..6 {
                let border = border_level(cell, neighbour_cells[k], child) as u32 as f32;
                let (ax, ay) = corners[(k + 5) % 6];
                let (bx, by) = corners[k];
                let va = mesh.push_vertex(
                    [cx + ax as f32, cy + ay as f32, top_m],
                    [0.0, 0.0, 1.0],
                    colour,
                    [1.0, border, cell_level, 0.0],
                );
                let vb = mesh.push_vertex(
                    [cx + bx as f32, cy + by as f32, top_m],
                    [0.0, 0.0, 1.0],
                    colour,
                    [1.0, border, cell_level, 0.0],
                );
                mesh.push_triangle(centre_v, va, vb);
            }
        }

        // --- side faces: where the neighbour is air over those layers ---
        for (k, neighbour) in neighbour_columns.iter().enumerate() {
            let (ax, ay) = corners[(k + 5) % 6];
            let (bx, by) = corners[k];
            // Outward normal: the edge's midpoint direction.
            let (nx, ny) = {
                let (mx, my) = ((ax + bx) / 2.0, (ay + by) / 2.0);
                let len = (mx * mx + my * my).sqrt().max(1e-9);
                ((mx / len) as f32, (my / len) as f32)
            };
            let normal = [nx, ny, 0.0];
            let line = [0.0, 0.0, cell_level, 0.0];
            let full_depth = !neighbour_drawn[k];
            if full_depth {
                // A full-depth wall is one silhouette quad for the whole column, from the
                // world bottom to the topmost run's top — not one quad per run, which
                // would overlap (and z-fight) across any air gap between runs.
                let topmost = column.runs.last().expect("checked non-empty above");
                let top_m = cfg.layer_top_m(topmost.top) as f32;
                let colour = shaded(topmost.material, cell);
                let v0 = mesh.push_vertex(
                    [cx + ax as f32, cy + ay as f32, bottom_m],
                    normal,
                    colour,
                    line,
                );
                let v1 = mesh.push_vertex(
                    [cx + bx as f32, cy + by as f32, bottom_m],
                    normal,
                    colour,
                    line,
                );
                let v2 = mesh.push_vertex(
                    [cx + bx as f32, cy + by as f32, top_m],
                    normal,
                    colour,
                    line,
                );
                let v3 = mesh.push_vertex(
                    [cx + ax as f32, cy + ay as f32, top_m],
                    normal,
                    colour,
                    line,
                );
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
                    let top_m = cfg.layer_top_m(layer - 1) as f32;
                    let bottom_of_wall = cfg.layer_bottom_m(start) as f32;
                    let colour = shaded(run.material, cell);
                    let v0 = mesh.push_vertex(
                        [cx + ax as f32, cy + ay as f32, bottom_of_wall],
                        normal,
                        colour,
                        line,
                    );
                    let v1 = mesh.push_vertex(
                        [cx + bx as f32, cy + by as f32, bottom_of_wall],
                        normal,
                        colour,
                        line,
                    );
                    let v2 = mesh.push_vertex(
                        [cx + bx as f32, cy + by as f32, top_m],
                        normal,
                        colour,
                        line,
                    );
                    let v3 = mesh.push_vertex(
                        [cx + ax as f32, cy + ay as f32, top_m],
                        normal,
                        colour,
                        line,
                    );
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
    let hash = crate::noise::gradient_2d(0x5EED, cell.q as f64 * 0.37, cell.r as f64 * 0.51);
    let k = 1.0 + 0.06 * hash as f32;
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
        let (cfg, chunk) = ken_chunk();
        let mesh = mesh_chunk(&cfg, &chunk, &HashSet::new());
        let mut top_faces = 0;
        for tri in mesh.indices.chunks(3) {
            let p: Vec<[f32; 3]> = tri.iter().map(|i| mesh.positions[*i as usize]).collect();
            let up = mesh.normals[tri[0] as usize][2] > 0.9; // height is the third component
            if !up {
                continue;
            }
            top_faces += 1;
            // Cross product of the edges, height component, must be positive: anticlockwise
            // seen from above, which is Bevy's front face after the axis mapping.
            let (a, b, c) = (p[0], p[1], p[2]);
            let (ux, uy) = (b[0] - a[0], b[1] - a[1]);
            let (vx, vy) = (c[0] - a[0], c[1] - a[1]);
            assert!(ux * vy - uy * vx > 0.0, "clockwise top face: {p:?}");
        }
        assert!(top_faces > 0, "a chunk should have top faces");
    }

    #[test]
    fn a_flat_columns_top_sits_at_its_layer_top() {
        let cfg = WorldConfig::default();
        let chunk = crate::chunk::generate(&cfg, ChunkKey::new(Level::Ken, Hex::ZERO));
        let mesh = mesh_chunk(&cfg, &chunk, &HashSet::new());
        let centre_column = chunk.column(Hex::ZERO).expect("the centre shaku");
        let expected = centre_column.surface_height_m(&cfg) as f32;
        // The centre cell's top face is at the chunk's own centre, at (0, 0) horizontally.
        let at_centre = mesh
            .positions
            .iter()
            .filter(|p| p[0].abs() < 1e-4 && p[1].abs() < 1e-4)
            .map(|p| p[2])
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
            .any(|p| p[0].abs() < 1e-4 && p[1].abs() < 1e-4));
        assert!(all
            .positions
            .iter()
            .any(|p| p[0].abs() < 1e-4 && p[1].abs() < 1e-4));

        // Omitting a cell opens a boundary between detail levels, so its neighbours wall the
        // hole down to the world bottom: that is what stops a height step becoming a crack.
        // We deliberately do NOT assert on the raw triangle count here: whether omitting an
        // interior cell nets more or fewer triangles overall depends on meshing details
        // (how many material runs a column has, how many full-depth walls happen to merge)
        // that have nothing to do with the behaviour under test. Measured for this chunk and
        // config: 354 triangles all-drawn vs 352 with the centre omitted, and 0 vs 12 vertices
        // at the world bottom near the hole — fewer total triangles, but still walled.
        let bottom = cfg.layer_bottom_m(cfg.bottom_layer) as f32;
        let walls_at_bottom = |m: &MeshData| {
            m.positions
                .iter()
                .filter(|p| (p[2] - bottom).abs() < 1e-3)
                .filter(|p| {
                    (p[0] * p[0] + p[1] * p[1]).sqrt() < Level::Shaku.width_m() as f32 * 1.2
                })
                .count()
        };
        assert!(
            walls_at_bottom(&fewer) > walls_at_bottom(&all),
            "the hole should be ringed by full-depth walls"
        );
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

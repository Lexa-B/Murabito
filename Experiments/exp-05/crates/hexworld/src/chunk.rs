//! A chunk is one parent cell's owned children: the unit of generation, loading and
//! unloading. Its key names the parent's level and cell; its columns are at the level below.

use crate::column::Column;
use crate::config::WorldConfig;
use crate::hex::Hex;
use crate::level::Level;
use crate::owner::children;
use crate::placeholder_terrain::column_at;
use crate::world::world_ri;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct ChunkKey {
    /// The parent's level. The chunk holds columns for cells at `level.child()`.
    pub level: Level,
    pub cell: Hex,
}

impl ChunkKey {
    pub const WORLD: ChunkKey = ChunkKey {
        level: Level::World,
        cell: Hex::ZERO,
    };

    pub const fn new(level: Level, cell: Hex) -> Self {
        ChunkKey { level, cell }
    }

    pub fn child_level(self) -> Level {
        self.level
            .child()
            .expect("a chunk's level always has a child")
    }

    /// The chunk that holds this chunk's own cell as one of its columns.
    pub fn parent_key(self) -> Option<ChunkKey> {
        let parent_level = self.level.parent()?;
        Some(ChunkKey::new(
            parent_level,
            crate::owner::parent_of(self.cell, self.level),
        ))
    }

    /// Every chunk above this one, nearest first.
    pub fn ancestors(self) -> Vec<ChunkKey> {
        let mut out = Vec::new();
        let mut key = self;
        while let Some(parent) = key.parent_key() {
            out.push(parent);
            key = parent;
        }
        out
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Chunk {
    pub key: ChunkKey,
    /// The cells this chunk holds, in a fixed order.
    pub cells: Vec<Hex>,
    /// One column per cell, in the same order.
    pub columns: Vec<Column>,
}

impl Chunk {
    pub fn column(&self, cell: Hex) -> Option<&Column> {
        self.cells
            .iter()
            .position(|c| *c == cell)
            .map(|i| &self.columns[i])
    }

    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }
}

/// The cells a chunk holds: its parent's owned children, or every ri for the world chunk.
pub fn chunk_cells(cfg: &WorldConfig, key: ChunkKey) -> Vec<Hex> {
    if key.level == Level::World {
        world_ri(cfg)
    } else {
        children(key.cell, key.level)
    }
}

/// Generate a chunk. A pure function of the config and the key.
pub fn generate(cfg: &WorldConfig, key: ChunkKey) -> Chunk {
    let cells = chunk_cells(cfg, key);
    let child_level = key.child_level();
    let columns = cells
        .iter()
        .map(|c| column_at(cfg, *c, child_level))
        .collect();
    Chunk {
        key,
        cells,
        columns,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_sizes_match_the_hierarchy() {
        let cfg = WorldConfig::default();
        assert_eq!(
            generate(&cfg, ChunkKey::new(Level::Ken, Hex::ZERO)).len(),
            36
        );
        assert_eq!(
            generate(&cfg, ChunkKey::new(Level::Cho, Hex::ZERO)).len(),
            3_600
        );
        assert_eq!(
            generate(&cfg, ChunkKey::new(Level::Ri, Hex::ZERO)).len(),
            1_296
        );
        assert_eq!(
            generate(&cfg, ChunkKey::WORLD).len(),
            1,
            "one ri in the default world"
        );
    }

    #[test]
    fn the_world_chunk_grows_with_the_radius() {
        let cfg = WorldConfig {
            world_radius_ri: 12,
            ..WorldConfig::default()
        };
        assert_eq!(generate(&cfg, ChunkKey::WORLD).len(), 469);
    }

    #[test]
    fn generation_is_pure() {
        let cfg = WorldConfig::default();
        let key = ChunkKey::new(Level::Ken, Hex::new(2, -3));
        assert_eq!(generate(&cfg, key), generate(&cfg, key));
    }

    #[test]
    fn a_column_can_be_looked_up_by_cell() {
        let cfg = WorldConfig::default();
        let key = ChunkKey::new(Level::Ken, Hex::new(2, -3));
        let chunk = generate(&cfg, key);
        let cell = chunk.cells[7];
        assert_eq!(
            chunk.column(cell),
            Some(&crate::placeholder_terrain::column_at(
                &cfg,
                cell,
                Level::Shaku
            ))
        );
        assert_eq!(chunk.column(Hex::new(99_999, 0)), None);
    }

    #[test]
    fn ancestors_run_up_to_the_world() {
        let key = ChunkKey::new(Level::Ken, Hex::new(2, -3));
        let ancestors = key.ancestors();
        assert_eq!(ancestors.len(), 3);
        assert_eq!(ancestors[0].level, Level::Cho);
        assert_eq!(ancestors[1].level, Level::Ri);
        assert_eq!(ancestors[2], ChunkKey::WORLD);
        assert!(ChunkKey::WORLD.ancestors().is_empty());
    }

    #[test]
    fn a_chunks_cells_are_its_parents_owned_children() {
        let key = ChunkKey::new(Level::Cho, Hex::new(1, 1));
        let cfg = WorldConfig::default();
        let cells = chunk_cells(&cfg, key);
        assert_eq!(cells.len(), 3_600);
        for c in &cells {
            assert_eq!(crate::owner::parent_of(*c, Level::Ken), Hex::new(1, 1));
        }
    }
}

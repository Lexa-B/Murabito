//! Loaders, their windows, and the store that loads and unloads chunks.
//!
//! The store has no threads, no clock and no engine in it: the caller passes the time,
//! generates the chunks it is told to, and hands them back.

use std::collections::HashSet;

use crate::chunk::ChunkKey;
use crate::config::WorldConfig;
use crate::hex::{range, Hex};
use crate::level::Level;
use crate::owner::up;
use crate::plane::cell_centre_m;
use crate::world::cell_in_world;

/// How many rings of neighbouring cells a loader wants at each detail level.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Rings {
    /// Rings of ken around the loader's ken, loaded at shaku detail.
    pub shaku: i32,
    /// Rings of cho around the loader's cho, loaded at ken detail.
    pub ken: i32,
    /// Rings of ri around the loader's ri, loaded at cho detail.
    pub cho: i32,
}

impl Default for Rings {
    fn default() -> Self {
        Rings {
            shaku: 3,
            ken: 3,
            cho: 3,
        }
    }
}

/// Anything that wants the world loaded around it: the camera, an NPC, a building site.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Loader {
    /// Where it is, as a global shaku.
    pub focus: Hex,
    pub rings: Rings,
}

/// Coarse levels sort first. The world chunk is coarsest of all.
pub fn coarseness_rank(level: Level) -> u8 {
    match level {
        Level::World => 0,
        Level::Ri => 1,
        Level::Cho => 2,
        Level::Ken => 3,
        Level::Shaku => 4,
    }
}

/// Every chunk a loader wants, with ancestors, clipped to the world.
pub fn requests(cfg: &WorldConfig, loader: &Loader) -> Vec<ChunkKey> {
    let mut out: HashSet<ChunkKey> = HashSet::new();
    let windows = [
        (Level::Ken, loader.rings.shaku),
        (Level::Cho, loader.rings.ken),
        (Level::Ri, loader.rings.cho),
    ];
    for (level, rings) in windows {
        let centre = up(loader.focus, Level::Shaku, level);
        for cell in range(centre, rings) {
            if cell_in_world(cell, level, cfg) {
                out.insert(ChunkKey::new(level, cell));
            }
        }
    }
    out.insert(ChunkKey::WORLD);
    // Ancestors, so a loaded chunk's parent column always exists whatever the rings are.
    for key in out.clone() {
        for ancestor in key.ancestors() {
            out.insert(ancestor);
        }
    }
    let mut keys: Vec<ChunkKey> = out.into_iter().collect();
    keys.sort();
    keys
}

/// Sort chunks into load order: coarsest level first, then nearest to any loader,
/// with the key as a tie-break so the order never depends on the input order.
pub fn load_order(cfg: &WorldConfig, loaders: &[Loader], keys: &mut [ChunkKey]) {
    let _ = cfg;
    let distance2 = |key: &ChunkKey| -> f64 {
        if key.level == Level::World {
            return 0.0;
        }
        let (ce, cn) = cell_centre_m(key.cell, key.level);
        loaders
            .iter()
            .map(|l| {
                let (fe, fn_) = cell_centre_m(l.focus, Level::Shaku);
                (ce - fe).powi(2) + (cn - fn_).powi(2)
            })
            .fold(f64::MAX, f64::min)
    };
    keys.sort_by(|a, b| {
        coarseness_rank(a.level)
            .cmp(&coarseness_rank(b.level))
            .then(
                distance2(a)
                    .partial_cmp(&distance2(b))
                    .expect("finite distances"),
            )
            .then(a.cmp(b))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at_origin() -> Loader {
        Loader {
            focus: Hex::ZERO,
            rings: Rings::default(),
        }
    }

    #[test]
    fn default_rings_request_the_expected_chunks() {
        let cfg = WorldConfig::default();
        let keys = requests(&cfg, &at_origin());
        let count = |level| keys.iter().filter(|k| k.level == level).count();
        assert_eq!(
            count(Level::Ken),
            37,
            "shaku detail: the focus ken plus 3 rings"
        );
        assert_eq!(
            count(Level::Cho),
            37,
            "ken detail: the focus cho plus 3 rings"
        );
        assert_eq!(count(Level::Ri), 1, "cho detail, clipped to a one-ri world");
        assert_eq!(count(Level::World), 1);
        assert_eq!(keys.len(), 76);
    }

    #[test]
    fn the_shaku_window_holds_1332_columns() {
        // The success criterion in the spec: 1,332 shaku columns in 37 ken chunks.
        let cfg = WorldConfig::default();
        let keys = requests(&cfg, &at_origin());
        let columns: usize = keys
            .iter()
            .filter(|k| k.level == Level::Ken)
            .map(|k| crate::chunk::chunk_cells(&cfg, *k).len())
            .sum();
        assert_eq!(columns, 1_332);
    }

    #[test]
    fn requests_always_include_ancestors() {
        let cfg = WorldConfig::default();
        // Odd rings: a wide shaku window with no ken or cho window at all.
        let loader = Loader {
            focus: Hex::new(500, -200),
            rings: Rings {
                shaku: 5,
                ken: 0,
                cho: 0,
            },
        };
        let keys = requests(&cfg, &loader);
        for key in &keys {
            for ancestor in key.ancestors() {
                assert!(
                    keys.contains(&ancestor),
                    "{key:?} is missing ancestor {ancestor:?}"
                );
            }
        }
    }

    #[test]
    fn windows_are_clipped_to_the_world() {
        let cfg = WorldConfig::default();
        // A focus near the ri's edge: some of its cho window falls in neighbouring ri.
        let edge = Hex::new(Level::Ri.scale_shaku() / 2, 0);
        let keys = requests(
            &cfg,
            &Loader {
                focus: edge,
                rings: Rings::default(),
            },
        );
        for key in &keys {
            assert!(
                crate::world::cell_in_world(key.cell, key.level, &cfg),
                "{key:?} is outside"
            );
        }
        assert!(
            keys.iter().filter(|k| k.level == Level::Cho).count() < 37,
            "should be clipped"
        );
    }

    #[test]
    fn two_loaders_union_their_requests() {
        let cfg = WorldConfig::default();
        let a = at_origin();
        let b = Loader {
            focus: Hex::new(600, 600),
            rings: Rings::default(),
        };
        let ka = requests(&cfg, &a);
        let kb = requests(&cfg, &b);
        let mut both: Vec<ChunkKey> = ka.iter().chain(kb.iter()).copied().collect();
        both.sort();
        both.dedup();
        assert!(both.len() > ka.len(), "the second loader should add chunks");
        for key in ka.iter().chain(kb.iter()) {
            assert!(both.contains(key));
        }
    }

    #[test]
    fn load_order_is_coarsest_first_then_nearest() {
        let cfg = WorldConfig::default();
        let loaders = [at_origin()];
        let mut keys = requests(&cfg, &loaders[0]);
        keys.reverse();
        load_order(&cfg, &loaders, &mut keys);
        assert_eq!(keys[0], ChunkKey::WORLD, "the world chunk loads first");
        let ranks: Vec<u8> = keys.iter().map(|k| coarseness_rank(k.level)).collect();
        assert!(
            ranks.windows(2).all(|w| w[0] <= w[1]),
            "coarse levels must come first"
        );
        // Within the ken level, the focus's own chunk comes before the far edge of the window.
        let kens: Vec<ChunkKey> = keys
            .iter()
            .filter(|k| k.level == Level::Ken)
            .copied()
            .collect();
        assert_eq!(kens[0].cell, Hex::ZERO);
    }

    #[test]
    fn load_order_is_deterministic() {
        let cfg = WorldConfig::default();
        let loaders = [at_origin()];
        let mut a = requests(&cfg, &loaders[0]);
        let mut b = a.clone();
        b.reverse();
        load_order(&cfg, &loaders, &mut a);
        load_order(&cfg, &loaders, &mut b);
        assert_eq!(a, b, "order must not depend on the input order");
    }
}

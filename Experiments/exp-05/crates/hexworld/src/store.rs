//! Loaders, their windows, and the store that loads and unloads chunks.
//!
//! The store has no threads, no clock and no engine in it: the caller passes the time,
//! generates the chunks it is told to, and hands them back.

use std::collections::{HashMap, HashSet};

use crate::chunk::{Chunk, ChunkKey};
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

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct StoreSettings {
    /// How long a chunk stays loaded after nothing wants it. 0 unloads at once.
    pub unload_delay_s: f64,
    /// How many chunks the caller should generate at once. The store only reports it.
    pub max_in_flight: usize,
}

impl Default for StoreSettings {
    fn default() -> Self {
        StoreSettings {
            unload_delay_s: 5.0,
            max_in_flight: 8,
        }
    }
}

#[derive(Clone, Default, Debug)]
pub struct StoreUpdate {
    /// In load order: generate these and hand them back with `insert`.
    pub to_load: Vec<ChunkKey>,
    /// Already removed from the store; drop whatever the caller built from them.
    pub to_unload: Vec<ChunkKey>,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct LevelStats {
    pub chunks: usize,
    pub columns: usize,
    pub lingering: usize,
    pub loads_last_second: usize,
    pub unloads_last_second: usize,
}

#[derive(Clone, Default, Debug)]
pub struct Stats {
    /// Indexed by `Level as usize`.
    pub per_level: [LevelStats; 5],
}

pub struct ChunkStore {
    cfg: WorldConfig,
    settings: StoreSettings,
    loaded: HashMap<ChunkKey, Chunk>,
    /// Loaded but no longer requested, with the time it fell out of every window.
    lingering: HashMap<ChunkKey, f64>,
    /// Handed out by `update` and not yet returned by `insert`.
    in_flight: HashSet<ChunkKey>,
    requested: HashSet<ChunkKey>,
    /// (time, level, was_load) for the last second, for the stats.
    events: Vec<(f64, Level, bool)>,
    stats: Stats,
}

impl ChunkStore {
    pub fn new(cfg: WorldConfig, settings: StoreSettings) -> Self {
        ChunkStore {
            cfg,
            settings,
            loaded: HashMap::new(),
            lingering: HashMap::new(),
            in_flight: HashSet::new(),
            requested: HashSet::new(),
            events: Vec::new(),
            stats: Stats::default(),
        }
    }

    pub fn config(&self) -> &WorldConfig {
        &self.cfg
    }

    pub fn settings(&self) -> &StoreSettings {
        &self.settings
    }

    pub fn settings_mut(&mut self) -> &mut StoreSettings {
        &mut self.settings
    }

    pub fn is_loaded(&self, key: ChunkKey) -> bool {
        self.loaded.contains_key(&key)
    }

    pub fn is_requested(&self, key: ChunkKey) -> bool {
        self.requested.contains(&key)
    }

    pub fn is_lingering(&self, key: ChunkKey) -> bool {
        self.lingering.contains_key(&key)
    }

    pub fn chunk(&self, key: ChunkKey) -> Option<&Chunk> {
        self.loaded.get(&key)
    }

    pub fn loaded_keys(&self) -> Vec<ChunkKey> {
        self.loaded.keys().copied().collect()
    }

    pub fn in_flight_count(&self) -> usize {
        self.in_flight.len()
    }

    /// Everything the loaders want, then what to load and what has just been dropped.
    pub fn update(&mut self, loaders: &[Loader], now: f64) -> StoreUpdate {
        self.requested.clear();
        for loader in loaders {
            for key in requests(&self.cfg, loader) {
                self.requested.insert(key);
            }
        }

        // Anything requested and not already here or on its way.
        let mut to_load: Vec<ChunkKey> = self
            .requested
            .iter()
            .copied()
            .filter(|k| !self.loaded.contains_key(k) && !self.in_flight.contains(k))
            .collect();
        load_order(&self.cfg, loaders, &mut to_load);

        // Lingering: requested again clears the stamp; newly unwanted gets one.
        for key in self.loaded.keys().copied().collect::<Vec<_>>() {
            if self.requested.contains(&key) {
                self.lingering.remove(&key);
            } else {
                self.lingering.entry(key).or_insert(now);
            }
        }

        // Expired, unless something still loaded needs them as an ancestor.
        let expired: HashSet<ChunkKey> = self
            .lingering
            .iter()
            .filter(|(_, since)| now - **since >= self.settings.unload_delay_s)
            .map(|(k, _)| *k)
            .collect();
        let mut keep: HashSet<ChunkKey> = HashSet::new();
        for key in self.loaded.keys() {
            if !expired.contains(key) {
                for ancestor in key.ancestors() {
                    keep.insert(ancestor);
                }
            }
        }
        let mut to_unload: Vec<ChunkKey> = expired.difference(&keep).copied().collect();
        to_unload.sort();
        for key in &to_unload {
            self.loaded.remove(key);
            self.lingering.remove(key);
            self.events.push((now, key.level, false));
        }

        self.refresh_stats(now);
        StoreUpdate { to_load, to_unload }
    }

    /// Tell the store a chunk is being generated, so it is not handed out again.
    pub fn begin_load(&mut self, key: ChunkKey) {
        self.in_flight.insert(key);
    }

    /// Store a finished chunk. Returns false if nothing wants it any more.
    ///
    /// The caller supplies the time, the same way it does for `update`: the store has no
    /// clock of its own.
    pub fn insert(&mut self, chunk: Chunk, now: f64) -> bool {
        let key = chunk.key;
        self.in_flight.remove(&key);
        if !self.requested.contains(&key) {
            return false;
        }
        self.events.push((now, key.level, true));
        self.loaded.insert(key, chunk);
        self.lingering.remove(&key);
        true
    }

    pub fn clear(&mut self) {
        self.loaded.clear();
        self.lingering.clear();
        self.in_flight.clear();
        self.requested.clear();
        self.events.clear();
        self.stats = Stats::default();
    }

    /// The column for a cell at a level, if its chunk is loaded.
    pub fn column(&self, level: Level, cell: Hex) -> Option<&crate::column::Column> {
        let parent_level = level.parent()?;
        let key = if parent_level == Level::World {
            ChunkKey::WORLD
        } else {
            ChunkKey::new(parent_level, crate::owner::parent_of(cell, level))
        };
        self.loaded.get(&key)?.column(cell)
    }

    /// The finest level loaded at a shaku, if any.
    pub fn finest_at(&self, shaku: Hex) -> Option<Level> {
        for level in crate::level::CELL_LEVELS {
            let cell = up(shaku, Level::Shaku, level);
            if self.column(level, cell).is_some() {
                return Some(level);
            }
        }
        None
    }

    /// The height of the ground at a point, at the finest detail loaded there.
    pub fn surface_height_m(&self, east: f64, north: f64) -> Option<f64> {
        for level in crate::level::CELL_LEVELS {
            let cell = crate::plane::round_at(east, north, level);
            if let Some(column) = self.column(level, cell) {
                return Some(column.surface_height_m(&self.cfg));
            }
        }
        None
    }

    pub fn stats(&self) -> &Stats {
        &self.stats
    }

    fn refresh_stats(&mut self, now: f64) {
        self.events.retain(|(t, _, _)| now - *t < 1.0);
        let mut stats = Stats::default();
        for (key, chunk) in &self.loaded {
            let slot = &mut stats.per_level[key.level as usize];
            slot.chunks += 1;
            slot.columns += chunk.len();
            if self.lingering.contains_key(key) {
                slot.lingering += 1;
            }
        }
        for (_, level, was_load) in &self.events {
            let slot = &mut stats.per_level[*level as usize];
            if *was_load {
                slot.loads_last_second += 1;
            } else {
                slot.unloads_last_second += 1;
            }
        }
        self.stats = stats;
    }
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

#[cfg(test)]
mod store_tests {
    use super::*;

    fn store() -> ChunkStore {
        ChunkStore::new(WorldConfig::default(), StoreSettings::default())
    }

    /// Drain the queue: generate and insert everything the store asks for.
    fn settle(store: &mut ChunkStore, loaders: &[Loader], now: f64) {
        loop {
            let update = store.update(loaders, now);
            if update.to_load.is_empty() {
                break;
            }
            for key in update.to_load {
                store.begin_load(key);
                let chunk = crate::chunk::generate(store.config(), key);
                store.insert(chunk, now);
            }
        }
    }

    #[test]
    fn settles_with_every_requested_chunk_loaded() {
        let mut s = store();
        let loaders = [Loader {
            focus: Hex::ZERO,
            rings: Rings::default(),
        }];
        settle(&mut s, &loaders, 0.0);
        for key in requests(&WorldConfig::default(), &loaders[0]) {
            assert!(s.is_loaded(key), "{key:?} should be loaded");
        }
        assert_eq!(s.stats().per_level[Level::Ken as usize].chunks, 37);
        assert_eq!(s.stats().per_level[Level::Ken as usize].columns, 1_332);
    }

    #[test]
    fn parents_load_before_their_children() {
        let mut s = store();
        let loaders = [Loader {
            focus: Hex::ZERO,
            rings: Rings::default(),
        }];
        let mut loaded: Vec<ChunkKey> = Vec::new();
        loop {
            let update = s.update(&loaders, 0.0);
            if update.to_load.is_empty() {
                break;
            }
            for key in update.to_load {
                for ancestor in key.ancestors() {
                    assert!(
                        loaded.contains(&ancestor),
                        "{key:?} before its ancestor {ancestor:?}"
                    );
                }
                s.begin_load(key);
                s.insert(crate::chunk::generate(s.config(), key), 0.0);
                loaded.push(key);
            }
        }
    }

    #[test]
    fn a_chunk_out_of_range_lingers_then_unloads() {
        let mut s = store();
        let here = [Loader {
            focus: Hex::ZERO,
            rings: Rings::default(),
        }];
        settle(&mut s, &here, 0.0);
        let far_key = ChunkKey::new(Level::Ken, up(Hex::new(18, 0), Level::Shaku, Level::Ken));
        assert!(s.is_loaded(far_key), "should be in the starting window");

        // Move the loader away. The chunk is no longer requested, but it stays loaded.
        let away = [Loader {
            focus: Hex::new(600, 0),
            rings: Rings::default(),
        }];
        let update = s.update(&away, 1.0);
        assert!(
            update.to_unload.is_empty(),
            "nothing unloads before the delay"
        );
        assert!(s.is_loaded(far_key) && s.is_lingering(far_key));

        // Still inside the 5 s delay.
        let update = s.update(&away, 4.0);
        assert!(update.to_unload.is_empty());
        assert!(s.is_loaded(far_key));

        // Past the delay.
        let update = s.update(&away, 6.5);
        assert!(
            update.to_unload.contains(&far_key),
            "should unload after the delay"
        );
        assert!(!s.is_loaded(far_key));
    }

    #[test]
    fn a_chunk_requested_again_in_time_is_kept_and_not_reloaded() {
        let mut s = store();
        let here = [Loader {
            focus: Hex::ZERO,
            rings: Rings::default(),
        }];
        settle(&mut s, &here, 0.0);
        let away = [Loader {
            focus: Hex::new(600, 0),
            rings: Rings::default(),
        }];
        s.update(&away, 1.0);
        // Back again before the delay runs out.
        let update = s.update(&here, 3.0);
        assert!(
            update.to_load.is_empty(),
            "nothing needs reloading: {:?}",
            update.to_load
        );
        assert!(update.to_unload.is_empty());
        // And it no longer lingers, so it will not expire later.
        let update = s.update(&here, 99.0);
        assert!(update.to_unload.is_empty());
    }

    #[test]
    fn a_zero_delay_unloads_at_once() {
        let mut s = ChunkStore::new(
            WorldConfig::default(),
            StoreSettings {
                unload_delay_s: 0.0,
                ..StoreSettings::default()
            },
        );
        let here = [Loader {
            focus: Hex::ZERO,
            rings: Rings::default(),
        }];
        settle(&mut s, &here, 0.0);
        let away = [Loader {
            focus: Hex::new(600, 0),
            rings: Rings::default(),
        }];
        let update = s.update(&away, 0.0);
        assert!(!update.to_unload.is_empty(), "immediate unloading");
    }

    #[test]
    fn a_lingering_ancestor_is_kept_while_a_descendant_stays() {
        // Rings that keep a ken chunk but drop its cho: the cho must not unload.
        let mut s = store();
        let wide = [Loader {
            focus: Hex::ZERO,
            rings: Rings {
                shaku: 3,
                ken: 3,
                cho: 3,
            },
        }];
        settle(&mut s, &wide, 0.0);
        let narrow = [Loader {
            focus: Hex::ZERO,
            rings: Rings {
                shaku: 3,
                ken: 0,
                cho: 0,
            },
        }];
        s.update(&narrow, 1.0);
        let update = s.update(&narrow, 20.0);
        assert!(
            !update.to_unload.is_empty(),
            "the narrow window should have let something expire"
        );
        for key in &update.to_unload {
            for other in s.loaded_keys() {
                assert!(
                    !other.ancestors().contains(key),
                    "{key:?} is still an ancestor of {other:?}"
                );
            }
        }
        // The global invariant: every loaded chunk's whole ancestor chain is loaded too.
        for key in s.loaded_keys() {
            for ancestor in key.ancestors() {
                assert!(
                    s.is_loaded(ancestor),
                    "{key:?} is loaded but its ancestor {ancestor:?} is not"
                );
            }
        }
        // The ken chunks themselves are still requested, so their cho parents survive.
        assert!(s.is_loaded(ChunkKey::new(Level::Cho, Hex::ZERO)));

        // Still holds well past the delay, once everything settles.
        s.update(&narrow, 100.0);
        for key in s.loaded_keys() {
            for ancestor in key.ancestors() {
                assert!(
                    s.is_loaded(ancestor),
                    "{key:?} is loaded but its ancestor {ancestor:?} is not"
                );
            }
        }
    }

    #[test]
    fn insert_drops_a_chunk_nobody_wants_any_more() {
        let mut s = store();
        let here = [Loader {
            focus: Hex::ZERO,
            rings: Rings::default(),
        }];
        let update = s.update(&here, 0.0);
        let key = *update.to_load.last().expect("something to load");
        s.begin_load(key);
        // The loader leaves before the chunk arrives.
        let away = [Loader {
            focus: Hex::new(50_000, 0),
            rings: Rings::default(),
        }];
        s.update(&away, 0.0);
        let accepted = s.insert(crate::chunk::generate(s.config(), key), 0.0);
        assert!(!accepted, "a chunk nobody wants is dropped");
        assert!(!s.is_loaded(key));
    }

    #[test]
    fn lookups_report_the_finest_detail_loaded() {
        let mut s = store();
        let here = [Loader {
            focus: Hex::ZERO,
            rings: Rings::default(),
        }];
        settle(&mut s, &here, 0.0);
        assert_eq!(s.finest_at(Hex::ZERO), Some(Level::Shaku));
        // Far away, inside the ri, only coarse detail is loaded.
        let far = Hex::new(3_000, 0);
        let level = s
            .finest_at(far)
            .expect("something is loaded everywhere in the ri");
        assert!(level > Level::Shaku, "{level:?}");
        assert!(s.column(Level::Shaku, Hex::ZERO).is_some());
        assert!(s.surface_height_m(0.0, 0.0).is_some());
    }

    #[test]
    fn stats_count_loads_and_unloads_in_the_last_second() {
        let mut s = store();
        let here = [Loader {
            focus: Hex::ZERO,
            rings: Rings::default(),
        }];
        settle(&mut s, &here, 0.0);
        assert!(s.stats().per_level[Level::Ken as usize].loads_last_second > 0);
        // Two seconds later, with nothing happening, the counts fall back to zero.
        s.update(&here, 2.0);
        assert_eq!(
            s.stats().per_level[Level::Ken as usize].loads_last_second,
            0
        );

        // Move away and let the delay expire everything: unloads are counted too.
        let away = [Loader {
            focus: Hex::new(600, 0),
            rings: Rings::default(),
        }];
        s.update(&away, 2.5);
        let update = s.update(&away, 8.0);
        assert!(!update.to_unload.is_empty(), "should have expired by now");
        assert!(s.stats().per_level[Level::Ken as usize].unloads_last_second > 0);

        // A cold start at a non-zero time must not lose its load events to a stale stamp:
        // this is the case that would have caught the old `insert`, which stamped events
        // with the time of the *previous* event rather than the caller's `now`.
        let mut late = store();
        settle(&mut late, &here, 10.0);
        let update = late.update(&here, 10.5);
        assert!(update.to_load.is_empty(), "nothing left to load");
        assert!(late.stats().per_level[Level::Ken as usize].loads_last_second > 0);
    }

    #[test]
    fn in_flight_counts_chunks_being_generated() {
        let mut s = store();
        let here = [Loader {
            focus: Hex::ZERO,
            rings: Rings::default(),
        }];
        let update = s.update(&here, 0.0);
        let key = update.to_load[0];
        assert_eq!(s.in_flight_count(), 0);
        s.begin_load(key);
        assert_eq!(s.in_flight_count(), 1);
        let chunk = crate::chunk::generate(s.config(), key);
        s.insert(chunk, 0.0);
        assert_eq!(s.in_flight_count(), 0);
    }

    #[test]
    fn settings_are_readable_and_tunable() {
        let mut s = store();
        assert_eq!(s.settings().unload_delay_s, 5.0);
        s.settings_mut().unload_delay_s = 0.5;
        assert_eq!(s.settings().unload_delay_s, 0.5);
    }
}

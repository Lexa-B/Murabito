//! PLACEHOLDER terrain. This is not the real generator: it exists so that chunks carry
//! something real while the coordinate system and the chunk machinery are built. It is
//! point-evaluable, so any chunk at any level can be generated on its own, and a coarse
//! sample is exactly the fine sample minus the octaves that are too fine for the level.

use crate::column::{Column, Material, Run};
use crate::config::WorldConfig;
use crate::hex::Hex;
use crate::level::Level;
use crate::noise::gradient_2d;
use crate::plane::cell_centre_m;

/// Number of octaves. The first is 8 km across (about twice the ri width), the last about 0.5 m.
pub const OCTAVES: usize = 15;
const BASE_WAVELENGTH_M: f64 = 8_192.0;
const BASE_AMPLITUDE_M: f64 = 10.0;
const GAIN: f64 = 0.75;

fn wavelength(octave: usize) -> f64 {
    BASE_WAVELENGTH_M / (1 << octave) as f64
}

fn amplitude(octave: usize) -> f64 {
    BASE_AMPLITUDE_M * GAIN.powi(octave as i32)
}

/// Whether a level keeps an octave: only those at least twice its cell width.
pub fn octave_kept(octave: usize, level: Level) -> bool {
    if level == Level::World {
        return false;
    }
    wavelength(octave) >= 2.0 * level.width_m()
}

/// One octave's contribution at a point.
pub fn octave_value(cfg: &WorldConfig, east: f64, north: f64, octave: usize) -> f64 {
    let w = wavelength(octave);
    let seed = cfg.seed ^ (octave as u64).wrapping_mul(0x9E37_79B9);
    gradient_2d(seed, east / w, north / w) * amplitude(octave)
}

/// Terrain height in metres at a point, at a level's detail.
pub fn height_m(cfg: &WorldConfig, east: f64, north: f64, level: Level) -> f64 {
    (0..OCTAVES)
        .filter(|k| octave_kept(*k, level))
        .map(|k| octave_value(cfg, east, north, k))
        .sum()
}

/// The column at a cell, sampled at that cell's centre.
pub fn column_at(cfg: &WorldConfig, cell: Hex, level: Level) -> Column {
    let (east, north) = cell_centre_m(cell, level);
    let top = cfg.layer_of_height(height_m(cfg, east, north, level));
    let bottom = cfg.bottom_layer;
    if top < bottom {
        return Column::default();
    }
    let mut runs = Vec::with_capacity(3);
    // One layer of grass on top, two of dirt below it, rock all the way down.
    let dirt_bottom = (top - 2).max(bottom);
    let rock_top = dirt_bottom - 1;
    if rock_top >= bottom {
        runs.push(Run {
            bottom,
            top: rock_top,
            material: Material::Rock,
        });
    }
    if dirt_bottom < top {
        runs.push(Run {
            bottom: dirt_bottom,
            top: top - 1,
            material: Material::Dirt,
        });
    }
    runs.push(Run {
        bottom: top,
        top,
        material: Material::Grass,
    });
    Column { runs }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hex::Hex;

    #[test]
    fn height_is_deterministic() {
        let cfg = WorldConfig::default();
        let a = height_m(&cfg, 123.0, -45.0, Level::Shaku);
        let b = height_m(&cfg, 123.0, -45.0, Level::Shaku);
        assert_eq!(a, b);
    }

    #[test]
    fn a_coarse_sample_is_the_fine_sample_minus_the_dropped_octaves() {
        let cfg = WorldConfig::default();
        let (e, n) = (321.0, 654.0);
        let fine = height_m(&cfg, e, n, Level::Shaku);
        let coarse = height_m(&cfg, e, n, Level::Cho);
        let dropped: f64 = (0..OCTAVES)
            .filter(|k| !octave_kept(*k, Level::Cho))
            .map(|k| octave_value(&cfg, e, n, k))
            .sum();
        assert!(
            (fine - coarse - dropped).abs() < 1e-9,
            "{fine} {coarse} {dropped}"
        );
    }

    #[test]
    fn coarse_levels_keep_fewer_octaves() {
        let kept = |level| (0..OCTAVES).filter(|k| octave_kept(*k, level)).count();
        assert!(kept(Level::Shaku) > kept(Level::Ken));
        assert!(kept(Level::Ken) > kept(Level::Cho));
        assert!(kept(Level::Cho) >= kept(Level::Ri));
    }

    #[test]
    fn relief_is_in_the_expected_range() {
        // Placeholder country: tens of metres of relief across a ri, not hundreds.
        let cfg = WorldConfig::default();
        let mut min = f64::MAX;
        let mut max = f64::MIN;
        for i in 0..200 {
            let e = -1_900.0 + i as f64 * 19.0;
            let h = height_m(&cfg, e, 0.0, Level::Shaku);
            min = min.min(h);
            max = max.max(h);
        }
        assert!(max - min > 5.0, "too flat: {min}..{max}");
        assert!(max - min < 120.0, "too wild: {min}..{max}");
    }

    #[test]
    fn a_column_is_rock_then_dirt_then_grass() {
        let cfg = WorldConfig::default();
        let col = column_at(&cfg, Hex::new(3, -2), Level::Shaku);
        assert_eq!(col.runs.first().unwrap().material, Material::Rock);
        assert_eq!(col.runs.first().unwrap().bottom, cfg.bottom_layer);
        assert_eq!(col.runs.last().unwrap().material, Material::Grass);
        // Runs are sorted, touching and non-overlapping.
        for pair in col.runs.windows(2) {
            assert_eq!(pair[1].bottom, pair[0].top + 1, "{:?}", col.runs);
        }
    }

    #[test]
    fn a_columns_top_follows_the_height_function() {
        let cfg = WorldConfig::default();
        let cell = Hex::new(11, 5);
        let (e, n) = crate::plane::cell_centre_m(cell, Level::Shaku);
        let expected = cfg.layer_of_height(height_m(&cfg, e, n, Level::Shaku));
        assert_eq!(
            column_at(&cfg, cell, Level::Shaku).top_layer(),
            Some(expected)
        );
    }

    #[test]
    fn columns_are_seeded() {
        let a = WorldConfig::default();
        let b = WorldConfig { seed: 99, ..a };
        let cell = Hex::new(4, 4);
        assert_ne!(
            column_at(&a, cell, Level::Shaku),
            column_at(&b, cell, Level::Shaku)
        );
    }
}

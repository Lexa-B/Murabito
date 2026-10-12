//! PLACEHOLDER terrain. This is not the real generator: it exists so that chunks carry
//! something real while the coordinate system and the chunk machinery are built. It is
//! point-evaluable, so any chunk at any level can be generated on its own, and a coarse
//! sample is exactly the fine sample minus the octaves that are too fine for the level.

use crate::column::{Column, Material, Run};
use crate::config::WorldConfig;
use crate::hex::Hex;
use crate::layer::layer_of_height;
use crate::level::Level;
use crate::noise::gradient_2d;
use crate::plane::cell_centre;

/// Number of octaves. The first is about 27,000 shaku across (about twice the ri width),
/// the last about 1.65 shaku.
pub const OCTAVES: usize = 15;
const BASE_WAVELENGTH: f32 = 27_033.6; // 8,192 m
const BASE_AMPLITUDE: f32 = 33.0; // 10 m
const GAIN: f32 = 0.75;

fn wavelength(octave: usize) -> f32 {
    BASE_WAVELENGTH / (1 << octave) as f32
}

fn amplitude(octave: usize) -> f32 {
    BASE_AMPLITUDE * GAIN.powi(octave as i32)
}

/// Whether a level keeps an octave: only those at least twice its cell width.
pub fn octave_kept(octave: usize, level: Level) -> bool {
    if level == Level::World {
        return false;
    }
    wavelength(octave) >= 2.0 * level.width()
}

/// One octave's contribution at a point. The noise is sampled at (x, −z), so north
/// is up in noise space, as it was when this worked in (east, north) metres: the
/// landscape is the same one.
pub fn octave_value(cfg: &WorldConfig, x: f32, z: f32, octave: usize) -> f32 {
    let w = wavelength(octave);
    let seed = cfg.seed ^ (octave as u64).wrapping_mul(0x9E37_79B9);
    gradient_2d(seed, x / w, -z / w) * amplitude(octave)
}

/// Terrain height in shaku at a point, at a level's detail.
pub fn height(cfg: &WorldConfig, x: f32, z: f32, level: Level) -> f32 {
    (0..OCTAVES)
        .filter(|k| octave_kept(*k, level))
        .map(|k| octave_value(cfg, x, z, k))
        .sum()
}

/// The column at a cell, sampled at that cell's centre.
pub fn column_at(cfg: &WorldConfig, cell: Hex, level: Level) -> Column {
    let c = cell_centre(cell, level);
    let h = height(cfg, c.x, c.z, level);
    let top = layer_of_height(h).max(cfg.bottom_layer);
    let bottom = cfg.bottom_layer;
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
        let a = height(&cfg, 405.9, 148.5, Level::Shaku);
        let b = height(&cfg, 405.9, 148.5, Level::Shaku);
        assert_eq!(a, b);
    }

    #[test]
    fn a_coarse_sample_is_the_fine_sample_minus_the_dropped_octaves() {
        let cfg = WorldConfig::default();

        // Test at a lattice-aligned point
        let (x, z) = (1059.3, -2158.2);
        let fine = height(&cfg, x, z, Level::Shaku);
        let coarse = height(&cfg, x, z, Level::Cho);
        let dropped: f32 = (0..OCTAVES)
            .filter(|k| octave_kept(*k, Level::Shaku) && !octave_kept(*k, Level::Cho))
            .map(|k| octave_value(&cfg, x, z, k))
            .sum();
        assert!(
            (fine - coarse - dropped).abs() < 1e-3,
            "lattice-aligned: {fine} {coarse} {dropped}"
        );

        // Test at a non-lattice-aligned point to ensure the identity has teeth
        let (x, z) = (1060.29, -2160.51);
        let fine = height(&cfg, x, z, Level::Shaku);
        let coarse = height(&cfg, x, z, Level::Cho);
        let dropped: f32 = (0..OCTAVES)
            .filter(|k| octave_kept(*k, Level::Shaku) && !octave_kept(*k, Level::Cho))
            .map(|k| octave_value(&cfg, x, z, k))
            .sum();
        assert!(
            (fine - coarse - dropped).abs() < 1e-3,
            "non-aligned: {fine} {coarse} {dropped}"
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
    fn each_level_keeps_the_octaves_it_kept_in_metres() {
        // The metre-era rule: octave k is kept at a level iff 8192 m / 2^k is at least
        // twice the cell width in metres. Converting both sides by ×3.3 keeps the same set.
        for level in crate::level::CELL_LEVELS {
            for k in 0..OCTAVES {
                let width_m = level.scale_shaku() as f64 * 10.0 / 33.0;
                let kept_in_metres = 8_192.0 / (1u64 << k) as f64 >= 2.0 * width_m;
                assert_eq!(
                    octave_kept(k, level),
                    kept_in_metres,
                    "{level:?} octave {k}"
                );
            }
        }
        for k in 0..OCTAVES {
            assert!(!octave_kept(k, Level::World));
        }
    }

    #[test]
    fn relief_is_in_the_expected_range() {
        // Placeholder country: tens of metres of relief across a ri, not hundreds.
        // In shaku: more than 16.5 (5 m) and less than 396 (120 m).
        let cfg = WorldConfig::default();
        let mut min = f32::MAX;
        let mut max = f32::MIN;
        for i in 0..200 {
            let x = -6_270.0 + i as f32 * 62.7;
            let h = height(&cfg, x, 0.0, Level::Shaku);
            min = min.min(h);
            max = max.max(h);
        }
        assert!(max - min > 16.5, "too flat: {min}..{max}");
        assert!(max - min < 396.0, "too wild: {min}..{max}");
    }

    #[test]
    fn a_column_is_rock_then_dirt_then_grass() {
        let cfg = WorldConfig::default();
        let col = column_at(&cfg, Hex::axial(3, -2), Level::Shaku);
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
        let cell = Hex::axial(11, 5);
        let c = cell_centre(cell, Level::Shaku);
        let expected = layer_of_height(height(&cfg, c.x, c.z, Level::Shaku));
        assert_eq!(
            column_at(&cfg, cell, Level::Shaku).top_layer(),
            Some(expected)
        );
    }

    #[test]
    fn columns_are_seeded() {
        let a = WorldConfig::default();
        let b = WorldConfig { seed: 99, ..a };
        // Two seeds can land on the same layer at a cell by coincidence; at (6, 8) the
        // two tops are a comfortable margin (3 layers) apart.
        let cell = Hex::axial(6, 8);
        assert_ne!(
            column_at(&a, cell, Level::Shaku),
            column_at(&b, cell, Level::Shaku)
        );
    }

    #[test]
    fn a_degenerate_column_below_bottom_layer_has_at_least_one_run() {
        // Create a config with a deliberately high bottom_layer so the surface lands below it
        let cfg = WorldConfig {
            bottom_layer: 100,
            ..WorldConfig::default()
        };
        // This will produce a very negative height, landing well below bottom_layer = 100
        let col = column_at(&cfg, Hex::axial(0, 0), Level::Shaku);

        // The column must have at least one run (the grass at bottom_layer)
        assert!(!col.runs.is_empty(), "column has no runs");

        // Runs must be sorted and non-overlapping, starting at bottom_layer
        assert_eq!(
            col.runs[0].bottom, cfg.bottom_layer,
            "first run doesn't start at bottom_layer"
        );
        for (i, run) in col.runs.iter().enumerate() {
            assert!(run.bottom <= run.top, "run has inverted bounds: {:?}", run);
            if i > 0 {
                let prev = col.runs[i - 1];
                assert_eq!(
                    run.bottom,
                    prev.top + 1,
                    "runs are not contiguous: run {}: {:?} after {:?}",
                    i,
                    run,
                    prev
                );
            }
        }

        // Top run must be grass
        assert_eq!(
            col.runs.last().unwrap().material,
            Material::Grass,
            "top run is not grass"
        );
    }

    #[test]
    fn no_column_is_empty_across_a_wide_scan() {
        // Scan several thousand cells at Shaku level with default config
        let cfg = WorldConfig::default();
        let mut empty_count = 0;
        let mut total_count = 0;

        for q in -50..=50 {
            for r in -50..=50 {
                let cell = Hex::axial(q, r);
                let col = column_at(&cfg, cell, Level::Shaku);
                total_count += 1;

                if col.runs.is_empty() {
                    empty_count += 1;
                }

                // Invariant: every column has at least one run
                assert!(!col.runs.is_empty(), "empty column at ({}, {})", q, r);
            }
        }

        // Sanity check: we scanned a reasonable number of cells
        assert!(
            total_count > 1000,
            "scan covered only {total_count} cells; expand the range"
        );

        // Record for evidence: no empty columns found
        assert_eq!(
            empty_count, 0,
            "found {empty_count} empty columns out of {total_count} cells"
        );
    }
}

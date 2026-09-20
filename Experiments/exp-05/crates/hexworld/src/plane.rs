//! The hex plane in metres. Positions are `(east, north)`; height is separate.
//! Pointy-top cells: the position of axial (q, r), in units of the cell's flat-to-flat
//! width, is (q + r/2, r·√3/2).

use crate::hex::Hex;
use crate::level::Level;

const SQRT3: f64 = 1.732_050_807_568_877_2;

/// The centre of a cell, in metres east and north of the world centre.
pub fn cell_centre_m(cell: Hex, level: Level) -> (f64, f64) {
    let w = level.width_m();
    let q = cell.q as f64;
    let r = cell.r as f64;
    ((q + r / 2.0) * w, r * SQRT3 / 2.0 * w)
}

/// Fractional axial coordinates of a point at a level.
pub fn metres_to_axial(east: f64, north: f64, level: Level) -> (f64, f64) {
    let w = level.width_m();
    let r = north / (SQRT3 / 2.0 * w);
    (east / w - r / 2.0, r)
}

/// Round fractional axial coordinates to the nearest cell.
pub fn hex_round(fq: f64, fr: f64) -> Hex {
    let fs = -fq - fr;
    let (mut q, mut r, s) = (fq.round(), fr.round(), fs.round());
    let (dq, dr, ds) = ((q - fq).abs(), (r - fr).abs(), (s - fs).abs());
    if dq > dr && dq > ds {
        q = -r - s;
    } else if dr > ds {
        r = -q - s;
    }
    Hex::new(q as i32, r as i32)
}

/// The cell whose ideal hexagon contains the point, at the given level.
pub fn round_at(east: f64, north: f64, level: Level) -> Hex {
    let (fq, fr) = metres_to_axial(east, north, level);
    hex_round(fq, fr)
}

/// The six corners of a cell at this level, relative to its centre, anticlockwise.
/// Pointy top: a corner points north.
pub fn corners_m(level: Level) -> [(f64, f64); 6] {
    let w = level.width_m();
    let radius = w / SQRT3; // centre to corner
    let mut out = [(0.0, 0.0); 6];
    for (i, slot) in out.iter_mut().enumerate() {
        let angle = std::f64::consts::FRAC_PI_6 + i as f64 * std::f64::consts::FRAC_PI_3;
        *slot = (radius * angle.cos(), radius * angle.sin());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_origin_is_the_centre_of_every_level() {
        for level in crate::level::CELL_LEVELS {
            let (e, n) = cell_centre_m(Hex::ZERO, level);
            assert!(e.abs() < 1e-12 && n.abs() < 1e-12, "{level:?}");
        }
    }

    #[test]
    fn a_step_east_is_one_width() {
        let (e, n) = cell_centre_m(Hex::new(1, 0), Level::Shaku);
        assert!((e - Level::Shaku.width_m()).abs() < 1e-12);
        assert!(n.abs() < 1e-12);
    }

    #[test]
    fn metres_and_axial_round_trip() {
        for level in crate::level::CELL_LEVELS {
            for cell in [Hex::new(0, 0), Hex::new(5, -3), Hex::new(-17, 41)] {
                let (e, n) = cell_centre_m(cell, level);
                assert_eq!(round_at(e, n, level), cell, "{level:?} {cell:?}");
            }
        }
    }

    #[test]
    fn rounding_picks_the_nearest_centre() {
        // A point a tenth of a width east of a centre still belongs to that cell.
        let (e, n) = cell_centre_m(Hex::new(3, 2), Level::Ken);
        let nudged = (e + 0.1 * Level::Ken.width_m(), n);
        assert_eq!(round_at(nudged.0, nudged.1, Level::Ken), Hex::new(3, 2));
    }

    #[test]
    fn corners_are_a_pointy_top_hexagon() {
        let corners = corners_m(Level::Shaku);
        let w = Level::Shaku.width_m();
        // Pointy top: the tallest corner is at half the corner-to-corner height,
        // which is the width / sqrt(3).
        let max_north = corners.iter().map(|c| c.1).fold(f64::MIN, f64::max);
        assert!((max_north - w / 3f64.sqrt()).abs() < 1e-12, "{max_north}");
        let max_east = corners.iter().map(|c| c.0).fold(f64::MIN, f64::max);
        assert!((max_east - w / 2.0).abs() < 1e-12, "{max_east}");
    }
}

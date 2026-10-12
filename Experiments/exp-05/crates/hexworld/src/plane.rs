//! The hex plane in world space: Bevy's `Vec3`, in shaku, with east +X and north −Z, as
//! in `murabito_hexcoords`. Pointy-top cells: the position of axial (q, r), in units of
//! the cell's flat-to-flat width, is x = q + r/2, z = r·√3/2, so +r is SSE.

use std::f32::consts::{FRAC_PI_3, FRAC_PI_6};

use bevy::math::Vec3;
use murabito_hexcoords::VoxelspacePos;

use crate::hex::Hex;
use crate::level::Level;

const ROW_SPACING: f32 = 0.866_025_4; // √3 / 2

/// The centre of a cell on the ground plane (y = 0).
pub fn cell_centre(cell: Hex, level: Level) -> Vec3 {
    let w = level.width();
    let (q, r) = (cell.q() as f32, cell.r() as f32);
    Vec3::new((q + r / 2.0) * w, 0.0, r * ROW_SPACING * w)
}

/// The cell whose ideal hexagon contains the point, at the given level. Height is
/// ignored. Rounding is `murabito_hexcoords`'s, on the point scaled to the level's cells.
pub fn round_at(pos: Vec3, level: Level) -> Hex {
    let w = level.width();
    let voxel = VoxelspacePos::from(Vec3::new(pos.x / w, 0.0, pos.z / w)).round();
    Hex::axial(voxel.q(), voxel.r())
}

/// The six corners of a cell at this level, relative to its centre, anticlockwise on
/// screen from 30° (east and a little north). Corner k-1 to corner k is the edge facing
/// `hex::EDGES[k]`.
pub fn corners(level: Level) -> [Vec3; 6] {
    let radius = level.width() / 3f32.sqrt(); // centre to corner
    std::array::from_fn(|i| {
        let angle = FRAC_PI_6 + i as f32 * FRAC_PI_3;
        Vec3::new(radius * angle.cos(), 0.0, -radius * angle.sin())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use murabito_hexcoords::VoxelCoord;

    #[test]
    fn the_origin_is_the_centre_of_every_level() {
        for level in crate::level::CELL_LEVELS {
            assert_eq!(cell_centre(Hex::ZERO, level), Vec3::ZERO, "{level:?}");
        }
    }

    #[test]
    fn a_step_east_is_one_width_along_x() {
        let p = cell_centre(Hex::axial(1, 0), Level::Ken);
        assert!((p - Vec3::new(6.0, 0.0, 0.0)).length() < 1e-5, "{p}");
    }

    #[test]
    fn a_plus_r_step_is_half_a_cell_east_and_a_row_down_the_screen() {
        // As murabito_hexcoords' `one_step_along_r_is_half_a_cell_right_and_a_row_down_the_screen`.
        let p = cell_centre(Hex::axial(0, 1), Level::Shaku);
        assert!(
            (p - Vec3::new(0.5, 0.0, 3f32.sqrt() / 2.0)).length() < 1e-6,
            "{p}"
        );
    }

    #[test]
    fn shaku_centres_agree_with_voxel_coords() {
        for (q, r) in [(0, 0), (5, -3), (-17, 41), (4_211, -9_003)] {
            let voxel = VoxelCoord::new(q, r, -q - r, 0).unwrap();
            let p = cell_centre(Hex::axial(q, r), Level::Shaku);
            assert!((p - voxel.to_world()).length() < 1e-3, "{q},{r}");
        }
    }

    #[test]
    fn rounding_inverts_the_centre_at_every_level() {
        for level in crate::level::CELL_LEVELS {
            for cell in [Hex::ZERO, Hex::axial(5, -3), Hex::axial(-17, 41)] {
                assert_eq!(
                    round_at(cell_centre(cell, level), level),
                    cell,
                    "{level:?} {cell:?}"
                );
            }
        }
    }

    #[test]
    fn rounding_inverts_the_centre_at_a_far_shaku() {
        // A shaku near the edge of a 12-ri world (Ri.width() = 12_960, so 12 ri out is
        // about 155_520 shaku): rounding must still recover the exact cell even where an
        // f32 world position is coarse enough to matter.
        let cell = Hex::axial(150_000, -75_000);
        assert_eq!(
            round_at(cell_centre(cell, Level::Shaku), Level::Shaku),
            cell
        );
    }

    #[test]
    fn rounding_picks_the_nearest_centre_and_ignores_height() {
        let c = cell_centre(Hex::axial(3, 2), Level::Ken);
        let nudged = c + Vec3::new(0.1 * Level::Ken.width(), 40.0, 0.0);
        assert_eq!(round_at(nudged, Level::Ken), Hex::axial(3, 2));
    }

    #[test]
    fn corners_are_a_pointy_top_hexagon_with_north_at_minus_z() {
        let corners = corners(Level::Shaku);
        let w = Level::Shaku.width();
        // Pointy top: the northernmost corner (most negative z) is width/√3 out.
        let min_z = corners.iter().map(|c| c.z).fold(f32::MAX, f32::min);
        assert!((min_z + w / 3f32.sqrt()).abs() < 1e-6, "{min_z}");
        let max_x = corners.iter().map(|c| c.x).fold(f32::MIN, f32::max);
        assert!((max_x - w / 2.0).abs() < 1e-6, "{max_x}");
        assert!(corners.iter().all(|c| c.y == 0.0));
        // The first corner is at 30°, anticlockwise on screen: east and a little north.
        assert!(corners[0].x > 0.0 && corners[0].z < 0.0);
    }
}

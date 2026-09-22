//! Hex voxel coordinates: where a cell is, not what is in it.
//!
//! The world is hexagonal prisms, pointy-top, stacked in layers. A cell is addressed in
//! cube coordinates `(q, r, s)`, which always satisfy `q + r + s = 0`, plus an integer
//! layer. `Docs/hex_units.md` is the design this implements.
//!
//! World space is Bevy's: `f32` shaku, Y up. A voxel is one shaku flat to flat and half a
//! shaku (5 sun) tall, and its world position is the centre of its bottom face.

use std::fmt;

use bevy::math::Vec3;

/// A voxel's height, in shaku: 5 sun.
const LAYER_HEIGHT: f32 = 0.5;

/// How far apart rows of pointy-top hexes are, in cell widths: √3 / 2.
const ROW_SPACING: f32 = SQRT_3 / 2.0;

const SQRT_3: f32 = 1.732_050_8;

/// The address of one voxel: a cell of the hex plane and a layer.
///
/// Speaks cube `(q, r, s)` and stores axial `(q, r)`: `s` is never stored, so the sum
/// can never drift out of true. The fields are private, so [`VoxelCoord::new`] is the
/// only way to make one, and every `VoxelCoord` that exists is on the plane.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VoxelCoord {
    q: i32,
    r: i32,
    layer: i32,
}

/// Cube coordinates that are not on the hex plane: their sum is not zero. Carries the
/// rejected values, so a message can show them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NotOnHexPlane {
    pub q: i32,
    pub r: i32,
    pub s: i32,
}

impl fmt::Display for NotOnHexPlane {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { q, r, s } = self;
        write!(
            f,
            "({q}, {r}, {s}) is not on the hex plane: q + r + s must be 0"
        )
    }
}

impl std::error::Error for NotOnHexPlane {}

impl VoxelCoord {
    /// A voxel from cube coordinates and a layer, if the coordinates are on the plane.
    pub fn new(q: i32, r: i32, s: i32, layer: i32) -> Result<Self, NotOnHexPlane> {
        // Summed in i64: three i32s can add up past what an i32 holds, and a debug build
        // would panic there, where an Err is the answer.
        if i64::from(q) + i64::from(r) + i64::from(s) == 0 {
            Ok(Self { q, r, layer })
        } else {
            Err(NotOnHexPlane { q, r, s })
        }
    }

    pub fn q(self) -> i32 {
        self.q
    }

    pub fn r(self) -> i32 {
        self.r
    }

    /// `-q - r`, reconstructed. The answer always fits an i32, because `new` checked the
    /// sum; the way there needn't. `-q - r` overflows at `q == i32::MIN`, and `-(q + r)`
    /// at `q + r > i32::MAX`. Wrapping arithmetic passes through both and, since the
    /// true value is in range, lands on it.
    pub fn s(self) -> i32 {
        self.q.wrapping_add(self.r).wrapping_neg()
    }

    pub fn layer(self) -> i32 {
        self.layer
    }

    /// The centre of this voxel's bottom face, in world space.
    pub fn to_world(self) -> Vec3 {
        let q = self.q as f32;
        let r = self.r as f32;
        Vec3::new(
            q + r / 2.0,
            self.layer as f32 * LAYER_HEIGHT,
            r * ROW_SPACING,
        )
    }

    /// The voxel a point of world space is in. Every point is in exactly one voxel, so
    /// this can't fail: a point on the plane between two cells goes to whichever the
    /// rounding favours, and a point on a layer's bottom face belongs to that layer.
    pub fn from_world(point: Vec3) -> Self {
        let r = point.z / ROW_SPACING;
        let q = point.x - r / 2.0;
        let (q, r) = cube_round(q, r, -q - r);
        let layer = (point.y / LAYER_HEIGHT).floor() as i32;
        Self { q, r, layer }
    }
}

/// The nearest cell to fractional cube coordinates, as axial `(q, r)`.
///
/// Rounding each coordinate on its own can break the sum: `(0.4, 0.4, -0.8)` rounds to
/// `(0, 0, -1)`. So all three are rounded, and the one that moved furthest from its
/// fractional value is recomputed from the other two, which puts the sum back to zero.
fn cube_round(q: f32, r: f32, s: f32) -> (i32, i32) {
    let (rq, rr, rs) = (q.round(), r.round(), s.round());
    let (dq, dr, ds) = ((rq - q).abs(), (rr - r).abs(), (rs - s).abs());
    if dq > dr && dq > ds {
        ((-rr - rs) as i32, rr as i32)
    } else if dr > ds {
        (rq as i32, (-rq - rs) as i32)
    } else {
        (rq as i32, rr as i32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn a_point_on_the_plane_is_accepted_and_read_back() {
        let voxel = VoxelCoord::new(2, -5, 3, 7).expect("2 - 5 + 3 = 0");

        assert_eq!(
            (voxel.q(), voxel.r(), voxel.s(), voxel.layer()),
            (2, -5, 3, 7)
        );
    }

    #[test]
    fn a_point_off_the_plane_is_refused_with_what_was_asked_for() {
        let refused = VoxelCoord::new(1, 1, 1, 0);

        assert_eq!(refused, Err(NotOnHexPlane { q: 1, r: 1, s: 1 }));
    }

    #[test]
    fn the_refusal_says_why() {
        let message = NotOnHexPlane { q: 1, r: 1, s: 1 }.to_string();

        assert_eq!(
            message,
            "(1, 1, 1) is not on the hex plane: q + r + s must be 0"
        );
    }

    #[test]
    fn s_is_what_makes_the_sum_zero() {
        let voxel = VoxelCoord::new(4, -9, 5, 0).expect("on the plane");

        assert_eq!(voxel.q() + voxel.r() + voxel.s(), 0);
    }

    #[test]
    fn a_sum_too_big_for_an_i32_is_refused_rather_than_a_crash() {
        let refused = VoxelCoord::new(i32::MAX, i32::MAX, i32::MAX, 0);

        assert!(refused.is_err());
    }

    /// The two coordinates at which the naive spellings of `s` overflow: `-q - r` when
    /// `q` is `i32::MIN`, and `-(q + r)` when `q + r` passes `i32::MAX`. Both are valid
    /// voxels, so `s()` must give the right answer for both.
    #[test]
    fn s_is_right_at_the_edges_of_i32() {
        let q_at_min = VoxelCoord::new(i32::MIN, 1, i32::MAX, 0).expect("on the plane");
        let q_plus_r_past_max =
            VoxelCoord::new(1 << 30, 1 << 30, i32::MIN, 0).expect("on the plane");

        assert_eq!(q_at_min.s(), i32::MAX);
        assert_eq!(q_plus_r_past_max.s(), i32::MIN);
    }

    #[test]
    fn a_voxel_can_key_a_map() {
        let here = VoxelCoord::new(1, -1, 0, 2).expect("on the plane");
        let same_place = VoxelCoord::new(1, -1, 0, 2).expect("on the plane");
        let mut contents = HashMap::new();

        contents.insert(here, "a stone");

        assert_eq!(contents.get(&same_place), Some(&"a stone"));
    }

    #[test]
    fn layers_tell_voxels_apart() {
        let ground = VoxelCoord::new(0, 0, 0, 0).expect("on the plane");
        let above = VoxelCoord::new(0, 0, 0, 1).expect("on the plane");

        assert_ne!(ground, above);
    }

    fn voxel(q: i32, r: i32, layer: i32) -> VoxelCoord {
        VoxelCoord::new(q, r, -q - r, layer).expect("axial input is always on the plane")
    }

    fn axial_and_layer(voxel: VoxelCoord) -> (i32, i32, i32) {
        (voxel.q(), voxel.r(), voxel.layer())
    }

    #[test]
    fn the_origin_voxel_sits_at_the_world_origin() {
        assert_eq!(voxel(0, 0, 0).to_world(), Vec3::ZERO);
    }

    #[test]
    fn one_step_along_q_is_one_shaku_along_x() {
        assert_eq!(voxel(1, 0, 0).to_world(), Vec3::new(1.0, 0.0, 0.0));
    }

    #[test]
    fn one_step_along_r_is_half_a_cell_right_and_a_row_down_the_screen() {
        assert_eq!(voxel(0, 1, 0).to_world(), Vec3::new(0.5, 0.0, ROW_SPACING));
    }

    #[test]
    fn two_rows_down_lands_in_the_same_column_root_three_away() {
        let two_down = voxel(-1, 2, 0).to_world();

        assert_eq!(two_down.x, 0.0);
        assert!((two_down.z - SQRT_3).abs() < 1e-6, "z is {}", two_down.z);
    }

    #[test]
    fn a_layer_is_five_sun_up() {
        assert_eq!(voxel(0, 0, 3).to_world().y, 1.5);
    }

    #[test]
    fn every_voxel_on_a_patch_round_trips_through_world_space() {
        for q in -12..=12 {
            for r in -12..=12 {
                for layer in [-3, 0, 41] {
                    let there = voxel(q, r, layer);

                    let back = VoxelCoord::from_world(there.to_world());

                    assert_eq!(back, there);
                }
            }
        }
    }

    #[test]
    fn every_point_inside_a_cell_belongs_to_it() {
        let cell = voxel(3, -2, 0);
        let centre = cell.to_world();
        // Inside the cell's inscribed circle, radius 0.5, in all directions.
        for step in 0..24 {
            let angle = step as f32 * std::f32::consts::TAU / 24.0;
            let inside = centre + Vec3::new(angle.cos(), 0.0, angle.sin()) * 0.45;

            assert_eq!(VoxelCoord::from_world(inside), cell, "at {inside}");
        }
    }

    #[test]
    fn a_point_across_the_edge_between_two_cells_belongs_to_the_other_one() {
        let just_left_of_the_edge = Vec3::new(0.45, 0.0, 0.0);
        let just_right_of_the_edge = Vec3::new(0.55, 0.0, 0.0);

        assert_eq!(
            axial_and_layer(VoxelCoord::from_world(just_left_of_the_edge)),
            (0, 0, 0)
        );
        assert_eq!(
            axial_and_layer(VoxelCoord::from_world(just_right_of_the_edge)),
            (1, 0, 0)
        );
    }

    #[test]
    fn a_point_just_past_a_corner_belongs_to_the_cell_beyond_it() {
        // The corner shared by (0, 0), (1, 0) and (0, 1) is at (0.5, √3/6); this point
        // is a little past it toward (0, 1).
        let past_the_corner = Vec3::new(0.5, 0.0, 0.35);

        assert_eq!(
            axial_and_layer(VoxelCoord::from_world(past_the_corner)),
            (0, 1, 0)
        );
    }

    #[test]
    fn a_layer_owns_everything_from_its_bottom_face_up_to_the_next() {
        let at = |y| VoxelCoord::from_world(Vec3::new(0.0, y, 0.0)).layer();

        assert_eq!(at(0.0), 0);
        assert_eq!(at(0.49), 0);
        assert_eq!(at(0.5), 1);
        assert_eq!(at(-0.01), -1);
    }

    #[test]
    fn cube_round_keeps_the_sum_at_zero() {
        // Rounded one by one these give (0, 0, -1), which is off the plane. q moved the
        // furthest, so it is the one recomputed: (1, 0, -1).
        let (q, r) = cube_round(0.4, 0.3, -0.7);

        assert_eq!((q, r), (1, 0));
    }
}

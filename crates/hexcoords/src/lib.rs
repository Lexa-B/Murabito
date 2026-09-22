//! Hex voxel coordinates: where a cell is, not what is in it.
//!
//! The world is hexagonal prisms, pointy-top, stacked in layers. A cell is addressed in
//! cube coordinates `(q, r, s)`, which always satisfy `q + r + s = 0`, plus an integer
//! layer. `Docs/hex_units.md` is the design this implements.

use std::fmt;

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
}

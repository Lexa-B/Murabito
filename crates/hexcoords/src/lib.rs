//! Hex voxel coordinates: where a cell is, not what is in it.
//!
//! The world is hexagonal prisms, pointy-top, stacked in layers. A cell is addressed in
//! cube coordinates `(q, r, s)`, which always satisfy `q + r + s = 0`, plus an integer
//! layer. `Docs/hex_units_readme.md` is the design this implements.
//!
//! World space is Bevy's: `f32` shaku, Y up. A voxel is one shaku flat to flat and half a
//! shaku (5 sun) tall, and its world position is the centre of its bottom face.
//!
//! Compass: **east is +X, north is −Z, up is +Y.** Directions across the plane are named
//! by compass point, never up or down, which are for gravity.
//!
//! Two types share the same axes. [`VoxelCoord`] is integer: the address of one voxel.
//! [`VoxelspacePos`] is `f32`: any point, in voxel units, such as where a moving entity is
//! between two cells. World space and `VoxelspacePos` convert exactly, both ways, through
//! `From`; a `VoxelCoord` is what a `VoxelspacePos` rounds to.

use std::fmt;
use std::ops::{Add, Sub};

use bevy::math::{Quat, Vec3};

/// A voxel's height, in shaku: 5 sun.
const LAYER_HEIGHT: f32 = 0.5;

/// Centre to corner of a cell, in shaku. A cell is one shaku flat to flat, and a regular
/// hexagon's corners sit 1/√3 of its width from the centre.
const CORNER_RADIUS: f32 = 1.0 / SQRT_3;

/// How far apart rows of pointy-top hexes are, in cell widths: √3 / 2.
const ROW_SPACING: f32 = SQRT_3 / 2.0;

const SQRT_3: f32 = 1.732_050_8;

/// How far from zero `q + r + s` may be for fractional coordinates to count as on the
/// plane. Real arithmetic lands near zero, not on it.
pub const ON_PLANE_TOLERANCE: f32 = 1e-4;

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

/// A point in voxel space: cube `(q, r, s)` and a layer, all fractional. Where something
/// is, rather than which voxel it is in. Stored axial, like [`VoxelCoord`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VoxelspacePos {
    q: f32,
    r: f32,
    layer: f32,
}

/// Fractional cube coordinates that are not on the hex plane: their sum is further from
/// zero than [`ON_PLANE_TOLERANCE`]. Carries the values and the sum they gave.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NotNearHexPlane {
    pub q: f32,
    pub r: f32,
    pub s: f32,
    pub sum: f32,
}

impl fmt::Display for NotNearHexPlane {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { q, r, s, sum } = self;
        write!(
            f,
            "({q}, {r}, {s}) is not on the hex plane: q + r + s is {sum}, not 0"
        )
    }
}

impl std::error::Error for NotNearHexPlane {}

impl VoxelspacePos {
    /// A point from cube coordinates and a layer, if the coordinates are on the plane to
    /// within [`ON_PLANE_TOLERANCE`].
    pub fn new(q: f32, r: f32, s: f32, layer: f32) -> Result<Self, NotNearHexPlane> {
        let sum = q + r + s;
        if sum.abs() < ON_PLANE_TOLERANCE {
            Ok(Self { q, r, layer })
        } else {
            Err(NotNearHexPlane { q, r, s, sum })
        }
    }

    pub fn q(self) -> f32 {
        self.q
    }

    pub fn r(self) -> f32 {
        self.r
    }

    pub fn s(self) -> f32 {
        -self.q - self.r
    }

    pub fn layer(self) -> f32 {
        self.layer
    }

    /// The voxel this point is in. Every point is in exactly one, so this can't fail: a
    /// point on the plane between two cells goes to whichever the rounding favours, and a
    /// point on a layer's bottom face belongs to that layer.
    pub fn round(self) -> VoxelCoord {
        let (q, r) = cube_round(self.q, self.r, self.s());
        let layer = self.layer.floor() as i32;
        VoxelCoord { q, r, layer }
    }
}

/// World space to voxel space: exact, since nothing is rounded.
impl From<Vec3> for VoxelspacePos {
    fn from(world: Vec3) -> Self {
        let r = world.z / ROW_SPACING;
        Self {
            q: world.x - r / 2.0,
            r,
            layer: world.y / LAYER_HEIGHT,
        }
    }
}

/// Voxel space to world space: exact.
impl From<VoxelspacePos> for Vec3 {
    fn from(pos: VoxelspacePos) -> Self {
        Vec3::new(
            pos.q + pos.r / 2.0,
            pos.layer * LAYER_HEIGHT,
            pos.r * ROW_SPACING,
        )
    }
}

/// A voxel's address as a point: its centre, at its bottom face. Exact, so `From`.
impl From<VoxelCoord> for VoxelspacePos {
    fn from(voxel: VoxelCoord) -> Self {
        Self {
            q: voxel.q as f32,
            r: voxel.r as f32,
            layer: voxel.layer as f32,
        }
    }
}

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
        VoxelspacePos::from(self).into()
    }

    /// The voxel a point of world space is in. Rounds, so it is a named method rather
    /// than a `From`; see [`VoxelspacePos::round`] for what the rounding does.
    pub fn from_world(world: Vec3) -> Self {
        VoxelspacePos::from(world).round()
    }

    /// The voxel one step away in a direction, on the same layer.
    ///
    /// Plain addition: a world two billion shaku across would overflow it, and no world
    /// here is within a thousandth of that.
    pub fn neighbour(self, direction: Direction) -> Self {
        let (dq, dr) = direction.axial_offset();
        Self {
            q: self.q + dq,
            r: self.r + dr,
            layer: self.layer,
        }
    }

    /// All twelve neighbours on this layer, in [`Direction::ALL`]'s order.
    pub fn neighbours(self) -> [Self; 12] {
        Direction::ALL.map(|direction| self.neighbour(direction))
    }

    /// How many steps across faces from this voxel to another, on the plane: the layer
    /// is not counted. In steps, not shaku: a corner neighbour is two steps and √3
    /// shaku away, and the two disagree everywhere off the six edge directions.
    pub fn distance(self, other: Self) -> u32 {
        (other - self).steps()
    }

    /// Every voxel exactly `radius` steps from this one, on its layer: `6 × radius` of
    /// them, anticlockwise from due east, and radius 0 is this voxel alone. A scan that
    /// walks rings outward sees every cell once, nearest first.
    pub fn ring(self, radius: u32) -> std::vec::IntoIter<Self> {
        if radius == 0 {
            return vec![self].into_iter();
        }
        let mut cells = Vec::with_capacity(6 * radius as usize);
        let mut here = (0..radius).fold(self, |cell, _| cell.neighbour(Direction::E));
        // From due east, each side runs `radius` steps along the next edge direction
        // round, starting two notches past north-north-east so the walk is anticlockwise.
        for side in 0..6 {
            let along = Direction::ALL[(4 + 2 * side) % 12];
            for _ in 0..radius {
                cells.push(here);
                here = here.neighbour(along);
            }
        }
        cells.into_iter()
    }

    /// The six corners of this voxel's bottom face, in world space, anticlockwise from
    /// the corner 30° round from east. Pointy-top: corners 1 and 4 point due north and
    /// due south.
    pub fn corners(self) -> [Vec3; 6] {
        let centre = self.to_world();
        std::array::from_fn(|i| {
            let angle = (30.0 + 60.0 * i as f32).to_radians();
            // A positive angle about Y turns +X toward −Z, as `Direction`'s index does.
            centre + Vec3::new(angle.cos(), 0.0, -angle.sin()) * CORNER_RADIUS
        })
    }
}

/// How many rings out a scan must go to be sure of every cell whose centre is within
/// `shaku` of the eye. A step gains a whole shaku along an edge direction but only √3/2
/// along a corner one, so a scan by ring has to go `shaku / (√3/2)` rings out, rounded
/// up; stopping at `shaku` rings misses cells, and only on the diagonals, which reads
/// as a rendering fault rather than the logic error it is.
pub fn rings_covering(shaku: f32) -> u32 {
    (shaku / ROW_SPACING).ceil() as u32
}

/// One voxel relative to another: what `b - a` is, and what `a + offset` takes.
///
/// Cube `(dq, dr, ds)` on the plane plus a layer difference, stored axial like a
/// [`VoxelCoord`] and held to the plane the same way. A separate type from the
/// coordinate so that an address is never mistaken for a displacement: "three cells
/// that way" and "the cell three east of the origin" are different things.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Offset {
    dq: i32,
    dr: i32,
    dlayer: i32,
}

impl Offset {
    /// No displacement at all: the offset from a voxel to itself.
    pub const ZERO: Self = Self {
        dq: 0,
        dr: 0,
        dlayer: 0,
    };

    /// An offset from cube components and a layer difference, if the components are on
    /// the plane. The same check and the same error as [`VoxelCoord::new`].
    pub fn new(dq: i32, dr: i32, ds: i32, dlayer: i32) -> Result<Self, NotOnHexPlane> {
        if i64::from(dq) + i64::from(dr) + i64::from(ds) == 0 {
            Ok(Self { dq, dr, dlayer })
        } else {
            Err(NotOnHexPlane {
                q: dq,
                r: dr,
                s: ds,
            })
        }
    }

    pub fn dq(self) -> i32 {
        self.dq
    }

    pub fn dr(self) -> i32 {
        self.dr
    }

    /// `-dq - dr`, reconstructed the way [`VoxelCoord::s`] is.
    pub fn ds(self) -> i32 {
        self.dq.wrapping_add(self.dr).wrapping_neg()
    }

    pub fn dlayer(self) -> i32 {
        self.dlayer
    }

    /// How many steps across faces this offset spans on the plane: the largest of the
    /// three cube components' magnitudes. The layer is not counted.
    pub fn steps(self) -> u32 {
        self.dq
            .unsigned_abs()
            .max(self.dr.unsigned_abs())
            .max(self.ds().unsigned_abs())
    }
}

impl Sub for VoxelCoord {
    type Output = Offset;

    /// The offset from `rhs` to `self`: `b - a` is what takes `a` to `b`.
    fn sub(self, rhs: Self) -> Offset {
        Offset {
            dq: self.q - rhs.q,
            dr: self.r - rhs.r,
            dlayer: self.layer - rhs.layer,
        }
    }
}

impl Add<Offset> for VoxelCoord {
    type Output = Self;

    fn add(self, offset: Offset) -> Self {
        Self {
            q: self.q + offset.dq,
            r: self.r + offset.dr,
            layer: self.layer + offset.dlayer,
        }
    }
}

/// One of the twelve directions across the hex plane, every 30°, anticlockwise from
/// east. Even indices are edge neighbours, one shaku away across a face; odd indices are
/// corner neighbours, √3 shaku away through a corner. A corner direction is the sum of
/// the two edge directions flanking it.
///
/// Direction `k` is `Quat::from_rotation_y(k · 30°)` applied to +X, so the index matches
/// Bevy's rotation sense and a heading needs no conversion. Bevy's forward, −Z, is north.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Direction {
    E = 0,
    ENE = 1,
    NNE = 2,
    N = 3,
    NNW = 4,
    WNW = 5,
    W = 6,
    WSW = 7,
    SSW = 8,
    S = 9,
    SSE = 10,
    ESE = 11,
}

impl Direction {
    /// Every direction, in index order: anticlockwise from east.
    pub const ALL: [Self; 12] = [
        Self::E,
        Self::ENE,
        Self::NNE,
        Self::N,
        Self::NNW,
        Self::WNW,
        Self::W,
        Self::WSW,
        Self::SSW,
        Self::S,
        Self::SSE,
        Self::ESE,
    ];

    /// 0 to 11, anticlockwise from east: the number of 30° turns from +X.
    pub fn index(self) -> u8 {
        self as u8
    }

    /// Whether the neighbour this way shares a face: one shaku away, and one step.
    pub fn is_edge(self) -> bool {
        self.index().is_multiple_of(2)
    }

    /// Whether the neighbour this way touches at a corner: √3 shaku away, between the
    /// two edge neighbours that flank it.
    pub fn is_corner(self) -> bool {
        !self.is_edge()
    }

    /// For a corner direction, the two edge directions on either side of it, which are
    /// the faces a move this way passes between. `None` for an edge direction.
    pub fn flanks(self) -> Option<(Self, Self)> {
        if self.is_edge() {
            return None;
        }
        let k = usize::from(self.index());
        Some((Self::ALL[(k + 11) % 12], Self::ALL[(k + 1) % 12]))
    }

    /// The direction `notches` turns of 30° anticlockwise from this one; negative turns
    /// clockwise. Wraps, so any count is fine.
    pub fn rotated(self, notches: i8) -> Self {
        let k = i16::from(self.index()) + i16::from(notches);
        Self::ALL[usize::try_from(k.rem_euclid(12)).expect("0 to 11")]
    }

    /// The shortest turn from this direction to `other`, in notches of 30°: positive is
    /// anticlockwise, in `-5..=6`. Exactly opposite is the one tie, and goes anticlockwise.
    pub fn notches_to(self, other: Self) -> i8 {
        let anticlockwise = (i16::from(other.index()) - i16::from(self.index())).rem_euclid(12);
        let notches = if anticlockwise > 6 {
            anticlockwise - 12
        } else {
            anticlockwise
        };
        i8::try_from(notches).expect("-5 to 6")
    }

    /// The rotation that turns something built facing Bevy's forward, -Z, to face this
    /// way. -Z is north, direction 3, so heading `k` is `(k - 3)` turns of 30° about Y.
    pub fn heading(self) -> Quat {
        let turns = f32::from(self.index()) - 3.0;
        Quat::from_rotation_y((turns * 30.0).to_radians())
    }

    /// The axial step this direction is, `(dq, dr)`.
    fn axial_offset(self) -> (i32, i32) {
        match self {
            Self::E => (1, 0),
            Self::ENE => (2, -1),
            Self::NNE => (1, -1),
            Self::N => (1, -2),
            Self::NNW => (0, -1),
            Self::WNW => (-1, -1),
            Self::W => (-1, 0),
            Self::WSW => (-2, 1),
            Self::SSW => (-1, 1),
            Self::S => (-1, 2),
            Self::SSE => (0, 1),
            Self::ESE => (1, 1),
        }
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

    fn pos(q: f32, r: f32, layer: f32) -> VoxelspacePos {
        VoxelspacePos::new(q, r, -q - r, layer).expect("axial input is always on the plane")
    }

    #[test]
    fn a_point_near_enough_the_plane_is_accepted() {
        let nearly = VoxelspacePos::new(0.3, 0.3, -0.6 + 1e-6, 0.0);

        assert!(nearly.is_ok());
    }

    #[test]
    fn a_point_too_far_off_the_plane_is_refused_with_the_sum_it_gave() {
        let refused = VoxelspacePos::new(0.5, 0.5, 0.0, 0.0);

        assert_eq!(
            refused,
            Err(NotNearHexPlane {
                q: 0.5,
                r: 0.5,
                s: 0.0,
                sum: 1.0
            })
        );
    }

    #[test]
    fn world_and_voxel_space_convert_exactly_both_ways() {
        for (x, y, z) in [(0.0, 0.0, 0.0), (0.37, 1.25, -2.9), (-41.5, 0.1, 17.3)] {
            let world = Vec3::new(x, y, z);

            let back = Vec3::from(VoxelspacePos::from(world));

            assert!(back.abs_diff_eq(world, 1e-5), "{world} came back as {back}");
        }
    }

    #[test]
    fn halfway_between_two_cells_is_halfway_in_world_space_too() {
        let halfway = pos(0.5, 0.0, 0.0);

        assert_eq!(Vec3::from(halfway), Vec3::new(0.5, 0.0, 0.0));
    }

    #[test]
    fn half_a_layer_up_is_a_quarter_shaku_up() {
        assert_eq!(Vec3::from(pos(0.0, 0.0, 0.5)).y, 0.25);
    }

    #[test]
    fn a_voxel_as_a_point_is_its_centre_at_its_bottom_face() {
        let cell = voxel(3, -2, 4);

        let as_point = VoxelspacePos::from(cell);

        assert_eq!(
            (as_point.q(), as_point.r(), as_point.layer()),
            (3.0, -2.0, 4.0)
        );
        assert_eq!(as_point.round(), cell);
    }

    #[test]
    fn a_point_rounds_to_the_voxel_it_is_in() {
        let just_inside_the_next_cell = pos(0.55, 0.0, 1.9);

        assert_eq!(just_inside_the_next_cell.round(), voxel(1, 0, 1));
    }

    #[test]
    fn into_works_where_the_target_type_is_known() {
        let world: Vec3 = pos(1.0, 0.0, 0.0).into();

        assert_eq!(world, Vec3::new(1.0, 0.0, 0.0));
    }

    #[test]
    fn the_twelve_directions_run_anticlockwise_from_east_every_thirty_degrees() {
        use bevy::math::Quat;

        for direction in Direction::ALL {
            let turns = f32::from(direction.index());
            let expected = Quat::from_rotation_y(turns.to_radians() * 30.0) * Vec3::X;
            let step = voxel(0, 0, 0).neighbour(direction).to_world();

            assert!(
                step.normalize().abs_diff_eq(expected, 1e-5),
                "{direction:?} steps {step}, expected along {expected}"
            );
        }
    }

    #[test]
    fn edge_neighbours_are_one_shaku_away_and_corner_neighbours_root_three() {
        for direction in Direction::ALL {
            let distance = voxel(0, 0, 0).neighbour(direction).to_world().length();
            let expected = if direction.is_edge() { 1.0 } else { SQRT_3 };

            assert!(
                (distance - expected).abs() < 1e-5,
                "{direction:?} is {distance} away"
            );
        }
    }

    #[test]
    fn north_is_minus_z_and_east_is_plus_x() {
        let north = voxel(0, 0, 0).neighbour(Direction::N).to_world();
        let east = voxel(0, 0, 0).neighbour(Direction::E).to_world();

        assert!(north.z < 0.0 && north.x == 0.0, "north is {north}");
        assert_eq!(east, Vec3::X);
    }

    #[test]
    fn edges_are_the_even_directions_and_corners_the_odd() {
        for direction in Direction::ALL {
            assert_eq!(direction.is_edge(), direction.index() % 2 == 0);
            assert_ne!(direction.is_edge(), direction.is_corner());
        }
    }

    #[test]
    fn a_corner_direction_is_the_sum_of_the_edges_that_flank_it() {
        for direction in Direction::ALL.into_iter().filter(|d| d.is_corner()) {
            let (before, after) = direction.flanks().expect("a corner has flanks");
            let via_flanks = voxel(0, 0, 0).neighbour(before).neighbour(after);

            assert_eq!(
                voxel(0, 0, 0).neighbour(direction),
                via_flanks,
                "{direction:?}"
            );
            assert!(before.is_edge() && after.is_edge());
        }
    }

    #[test]
    fn an_edge_direction_has_no_flanks() {
        assert_eq!(Direction::E.flanks(), None);
    }

    #[test]
    fn north_is_flanked_by_north_north_east_and_north_north_west() {
        assert_eq!(
            Direction::N.flanks(),
            Some((Direction::NNE, Direction::NNW))
        );
    }

    #[test]
    fn stepping_out_and_back_returns_home() {
        let home = voxel(5, -3, 2);
        for direction in Direction::ALL {
            let opposite = Direction::ALL[(usize::from(direction.index()) + 6) % 12];

            assert_eq!(home.neighbour(direction).neighbour(opposite), home);
        }
    }

    #[test]
    fn neighbours_stay_on_the_plane_and_on_the_layer() {
        for neighbour in voxel(7, 4, -1).neighbours() {
            assert_eq!(neighbour.q() + neighbour.r() + neighbour.s(), 0);
            assert_eq!(neighbour.layer(), -1);
        }
    }

    #[test]
    fn the_twelve_neighbours_are_all_different() {
        let neighbours = voxel(0, 0, 0).neighbours();
        let distinct: std::collections::HashSet<_> = neighbours.into_iter().collect();

        assert_eq!(distinct.len(), 12);
    }

    #[test]
    fn rotating_a_direction_wraps_around_the_compass() {
        assert_eq!(Direction::E.rotated(1), Direction::ENE);
        assert_eq!(Direction::E.rotated(-1), Direction::ESE);
        assert_eq!(Direction::ESE.rotated(1), Direction::E);
        assert_eq!(Direction::N.rotated(12), Direction::N);
        assert_eq!(Direction::N.rotated(-25), Direction::NNE);
    }

    #[test]
    fn the_shortest_turn_goes_whichever_way_is_nearer() {
        assert_eq!(Direction::E.notches_to(Direction::E), 0);
        assert_eq!(Direction::E.notches_to(Direction::NNE), 2);
        assert_eq!(Direction::E.notches_to(Direction::ESE), -1);
        assert_eq!(Direction::ESE.notches_to(Direction::E), 1);
        assert_eq!(Direction::N.notches_to(Direction::SSE), -5);
    }

    #[test]
    fn a_turn_to_the_opposite_direction_goes_anticlockwise() {
        for direction in Direction::ALL {
            let opposite = direction.rotated(6);
            assert_eq!(direction.notches_to(opposite), 6, "{direction:?}");
        }
    }

    #[test]
    fn rotating_by_the_shortest_turn_arrives() {
        for from in Direction::ALL {
            for to in Direction::ALL {
                assert_eq!(from.rotated(from.notches_to(to)), to, "{from:?} to {to:?}");
            }
        }
    }

    #[test]
    fn a_heading_turns_bevys_forward_to_point_the_direction_it_names() {
        for direction in Direction::ALL {
            let faces = direction.heading() * Vec3::NEG_Z;
            let step = voxel(0, 0, 0).neighbour(direction).to_world().normalize();

            assert!(
                faces.abs_diff_eq(step, 1e-5),
                "heading {direction:?} faces {faces}, the step goes {step}"
            );
        }
    }

    #[test]
    fn facing_north_is_no_turn_at_all() {
        assert!(Direction::N.heading().abs_diff_eq(Quat::IDENTITY, 1e-6));
    }

    #[test]
    fn an_offset_round_trips() {
        let a = voxel(2, -5, 1);
        let b = voxel(-3, 4, 0);

        assert_eq!(a + (b - a), b);
        assert_eq!(b - b, Offset::ZERO);
    }

    #[test]
    fn an_offset_reads_back_in_cube_form() {
        let offset = voxel(3, -1, 2) - voxel(1, 1, 0);

        assert_eq!(
            (offset.dq(), offset.dr(), offset.ds(), offset.dlayer()),
            (2, -2, 0, 2)
        );
    }

    #[test]
    fn an_offset_off_the_plane_is_refused() {
        assert_eq!(
            Offset::new(1, 1, 1, 0),
            Err(NotOnHexPlane { q: 1, r: 1, s: 1 })
        );
        assert!(Offset::new(1, -1, 0, 3).is_ok());
    }

    #[test]
    fn an_edge_neighbour_is_one_step_and_a_corner_neighbour_two() {
        let home = voxel(0, 0, 0);
        for direction in Direction::ALL {
            let steps = home.distance(home.neighbour(direction));
            let expected = if direction.is_edge() { 1 } else { 2 };
            assert_eq!(steps, expected, "{direction:?}");
        }
    }

    #[test]
    fn distance_ignores_the_layer() {
        assert_eq!(voxel(0, 0, 0).distance(voxel(0, 0, 7)), 0);
    }

    /// Every voxel within `steps` of `home`, by brute force over a box, so the ring walk
    /// has something independent to be checked against.
    fn within(home: VoxelCoord, steps: u32) -> std::collections::HashSet<VoxelCoord> {
        let reach = steps as i32;
        let mut cells = std::collections::HashSet::new();
        for dq in -reach..=reach {
            for dr in -reach..=reach {
                let cell = voxel(home.q() + dq, home.r() + dr, home.layer());
                if home.distance(cell) <= steps {
                    cells.insert(cell);
                }
            }
        }
        cells
    }

    #[test]
    fn ring_zero_is_the_voxel_itself() {
        assert_eq!(voxel(2, 3, 1).ring(0).collect::<Vec<_>>(), [voxel(2, 3, 1)]);
    }

    #[test]
    fn a_ring_has_six_times_its_radius_cells_all_exactly_that_far_and_none_twice() {
        let home = voxel(1, -4, 2);
        for radius in 1..=5 {
            let ring: Vec<_> = home.ring(radius).collect();
            let distinct: std::collections::HashSet<_> = ring.iter().copied().collect();

            assert_eq!(ring.len(), 6 * radius as usize);
            assert_eq!(distinct.len(), ring.len());
            assert!(ring.iter().all(|cell| home.distance(*cell) == radius));
            assert!(ring.iter().all(|cell| cell.layer() == home.layer()));
        }
    }

    #[test]
    fn rings_out_to_a_radius_cover_exactly_the_cells_within_it() {
        let home = voxel(-2, 1, 0);
        let rings: std::collections::HashSet<_> = (0..=4).flat_map(|r| home.ring(r)).collect();

        assert_eq!(rings, within(home, 4));
    }

    #[test]
    fn a_ring_starts_due_east_and_runs_anticlockwise() {
        let home = voxel(0, 0, 0);
        let ring: Vec<_> = home.ring(1).collect();

        let edge_neighbours: Vec<_> = Direction::ALL
            .into_iter()
            .filter(|d| d.is_edge())
            .map(|d| home.neighbour(d))
            .collect();
        assert_eq!(ring, edge_neighbours);
    }

    #[test]
    fn rings_covering_a_radius_in_shaku_reach_every_cell_within_it() {
        let home = voxel(0, 0, 0);
        for shaku in [0.5, 1.0, 1.75, 3.0, 4.6, 10.0, 48.0] {
            let rings = rings_covering(shaku);
            let reached = within(home, rings);
            let generous = within(home, rings + 3);

            let missed = generous
                .iter()
                .filter(|cell| (cell.to_world() - home.to_world()).length() <= shaku)
                .find(|cell| !reached.contains(cell));
            assert_eq!(missed, None, "at {shaku} shaku, {rings} rings");
            assert!(
                f64::from(rings) <= f64::from(shaku) / 0.866 + 1.0,
                "{rings} rings for {shaku}"
            );
        }
    }

    #[test]
    fn one_shaku_needs_two_rings_because_of_the_diagonals() {
        assert_eq!(rings_covering(1.0), 2);
    }

    #[test]
    fn corners_sit_one_over_root_three_from_the_centre_and_point_north_and_south() {
        let cell = voxel(3, -1, 1);
        let centre = cell.to_world();
        let corners = cell.corners();

        for corner in corners {
            let radius = (corner - centre).length();
            assert!((radius - 1.0 / SQRT_3).abs() < 1e-6, "radius {radius}");
            assert_eq!(corner.y, centre.y);
        }
        assert!((corners[1] - centre).abs_diff_eq(Vec3::new(0.0, 0.0, -1.0 / SQRT_3), 1e-6));
        assert!((corners[4] - centre).abs_diff_eq(Vec3::new(0.0, 0.0, 1.0 / SQRT_3), 1e-6));
    }

    #[test]
    fn a_cells_sides_are_half_a_shaku_from_its_centre_and_shared_with_its_neighbour() {
        let cell = voxel(0, 0, 0);
        let corners = cell.corners();
        let east_side = (corners[0] + corners[5]) / 2.0;

        assert!(
            east_side.abs_diff_eq(Vec3::new(0.5, 0.0, 0.0), 1e-6),
            "{east_side}"
        );
        let east = cell.neighbour(Direction::E).corners();
        assert!(east.iter().any(|c| c.abs_diff_eq(corners[0], 1e-6)));
        assert!(east.iter().any(|c| c.abs_diff_eq(corners[5], 1e-6)));
    }
}

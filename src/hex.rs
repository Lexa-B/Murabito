//! The hex grid the senses are computed on.
//!
//! Pointy-up cells, one shaku flat to flat, in axial coordinates. The world is
//! continuous — beings stand wherever they like — and this grid exists underneath so
//! that occlusion and propagation have somewhere cheap to run. Movement fidelity and
//! sense cost are therefore tuned separately.
//!
//! **Provisional.** exp-05 is building the real ri/cho/ken/shaku address system, and this
//! is a stand-in until that lands: written to be replaced, not extended.
//!
//! Ground plane throughout: `Vec2` is world XZ, with `y` the world's Z.
//!
//! ```text
//!        __            q increases east; r increases south-east.
//!     __/  \__         A cell's six neighbours are all exactly one shaku away,
//!    /  \__/  \        so a hex distance in cells is a distance in shaku.
//!    \__/  \__/
//!       \__/
//! ```

use bevy::prelude::*;

use crate::camera::CameraRig;

/// Flat to flat, in shaku. One, by definition — this is what makes a cell count and a
/// shaku count the same number.
pub const CELL_WIDTH: f32 = 1.0;

const SQRT_3: f32 = 1.732_050_8;

/// Centre to a corner. Smaller than the inradius for a pointy-up cell.
pub const CIRCUMRADIUS: f32 = CELL_WIDTH / SQRT_3;

/// How far one step of `r` moves along Z. Three halves of the circumradius.
const ROW_STEP: f32 = CELL_WIDTH * SQRT_3 / 2.0;

/// The six neighbours, as offsets. Starting east and going anticlockwise.
pub const DIRECTIONS: [Hex; 6] = [
    Hex { q: 1, r: 0 },
    Hex { q: 1, r: -1 },
    Hex { q: 0, r: -1 },
    Hex { q: -1, r: 0 },
    Hex { q: -1, r: 1 },
    Hex { q: 0, r: 1 },
];

/// One cell, in axial coordinates. The third cube coordinate is always `-q - r`, so it
/// is derived rather than stored.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct Hex {
    pub q: i32,
    pub r: i32,
}

impl Hex {
    pub const fn new(q: i32, r: i32) -> Self {
        Self { q, r }
    }

    /// The implied third cube coordinate.
    pub const fn s(self) -> i32 {
        -self.q - self.r
    }

    /// Where this cell's centre sits in the world.
    pub fn center(self) -> Vec2 {
        Vec2::new(
            CELL_WIDTH * (self.q as f32 + self.r as f32 * 0.5),
            ROW_STEP * self.r as f32,
        )
    }

    /// Which cell a world point falls in.
    pub fn from_world(point: Vec2) -> Self {
        let r = point.y / ROW_STEP;
        let q = point.x / CELL_WIDTH - r * 0.5;
        Self::round(q, r)
    }

    /// Nearest cell to fractional axial coordinates.
    ///
    /// Rounds in cube space and then repairs whichever coordinate was furthest off, which
    /// is what keeps `q + r + s == 0`. Rounding the two axial coordinates independently
    /// would put points near a corner in the wrong cell.
    fn round(q: f32, r: f32) -> Self {
        let s = -q - r;
        let (mut rq, mut rr, rs) = (q.round(), r.round(), s.round());
        let (dq, dr, ds) = ((rq - q).abs(), (rr - r).abs(), (rs - s).abs());
        if dq > dr && dq > ds {
            rq = -rr - rs;
        } else if dr > ds {
            rr = -rq - rs;
        }
        Self {
            q: rq as i32,
            r: rr as i32,
        }
    }

    /// The six corners, anticlockwise from the east-south-east one.
    pub fn corners(self) -> [Vec2; 6] {
        let centre = self.center();
        std::array::from_fn(|i| {
            let angle = std::f32::consts::FRAC_PI_3 * i as f32 - std::f32::consts::FRAC_PI_6;
            centre + Vec2::new(CIRCUMRADIUS * angle.cos(), CIRCUMRADIUS * angle.sin())
        })
    }

    /// Steps between two cells, which is also shaku between them along the grid.
    pub fn distance(self, other: Self) -> u32 {
        let (dq, dr) = (self.q - other.q, self.r - other.r);
        ((dq.abs() + dr.abs() + (dq + dr).abs()) / 2) as u32
    }

    pub fn neighbour(self, direction: usize) -> Self {
        let offset = DIRECTIONS[direction % 6];
        Self::new(self.q + offset.q, self.r + offset.r)
    }

    /// Every cell within `radius` steps, this one included.
    ///
    /// The bounds on the inner loop are what make this a hexagon rather than a rhombus:
    /// they keep the third cube coordinate inside the radius too.
    pub fn within(self, radius: u32) -> impl Iterator<Item = Self> {
        let radius = radius as i32;
        (-radius..=radius).flat_map(move |dq| {
            ((-radius).max(-dq - radius)..=radius.min(-dq + radius))
                .map(move |dr| Self::new(self.q + dq, self.r + dr))
        })
    }
}

/// How far the debug grid is drawn around the camera's focus. Six ken.
const GRID_RADIUS_KEN: u32 = 6;
const GRID_RADIUS: u32 = GRID_RADIUS_KEN * 6;

/// Just above the ground, and below the sense overlay so the fields read on top of it.
const GRID_HEIGHT: f32 = 0.04;

const GRID_COLOR: Color = Color::srgba(1.0, 1.0, 1.0, 0.10);

/// Draws the grid, so the unit the senses work in is visible rather than notional.
pub struct HexGridPlugin;

impl Plugin for HexGridPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, draw_grid);
    }
}

/// A patch around the camera's focus rather than the whole world: at one shaku a cell,
/// a cho of ground is about 150,000 cells, and almost none of them are ever on screen.
fn draw_grid(mut gizmos: Gizmos, rig: Single<&CameraRig>) {
    let focus = rig.focus();
    let centre = Hex::from_world(Vec2::new(focus.x, focus.z));

    for hex in centre.within(GRID_RADIUS) {
        let corners = hex.corners();
        let loop_ = corners
            .iter()
            .chain(std::iter::once(&corners[0]))
            .map(|corner| Vec3::new(corner.x, GRID_HEIGHT, corner.y));
        gizmos.linestrip(loop_, GRID_COLOR);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_origin_cell_sits_on_the_origin() {
        assert_eq!(Hex::new(0, 0).center(), Vec2::ZERO);
    }

    /// The whole point of one shaku flat to flat: neighbouring centres are one apart, so
    /// a count of cells is a count of shaku and the senses need no conversion.
    #[test]
    fn every_neighbour_is_exactly_one_shaku_away() {
        let origin = Hex::default();
        for direction in 0..6 {
            let step = origin.neighbour(direction).center().length();
            assert!(
                (step - CELL_WIDTH).abs() < 1e-5,
                "neighbour {direction} is {step} away"
            );
        }
    }

    /// Pointy-up: a cell is taller than it is wide.
    #[test]
    fn a_cell_is_one_shaku_wide_and_taller_than_that() {
        let corners = Hex::default().corners();
        let width = corners.iter().map(|c| c.x).fold(f32::MIN, f32::max)
            - corners.iter().map(|c| c.x).fold(f32::MAX, f32::min);
        let height = corners.iter().map(|c| c.y).fold(f32::MIN, f32::max)
            - corners.iter().map(|c| c.y).fold(f32::MAX, f32::min);
        assert!((width - CELL_WIDTH).abs() < 1e-5, "width {width}");
        assert!(
            (height - 2.0 * CIRCUMRADIUS).abs() < 1e-5,
            "height {height}"
        );
        assert!(height > width);
    }

    #[test]
    fn a_cell_centre_maps_back_to_its_own_cell() {
        for hex in Hex::default().within(4) {
            assert_eq!(Hex::from_world(hex.center()), hex, "{hex:?}");
        }
    }

    /// Anywhere inside a cell belongs to it, corners being the hard part — which is what
    /// cube rounding is for.
    #[test]
    fn points_near_a_corner_land_in_the_right_cell() {
        let hex = Hex::new(3, -2);
        for corner in hex.corners() {
            let inward = hex.center() + (corner - hex.center()) * 0.9;
            assert_eq!(Hex::from_world(inward), hex, "near {corner:?}");
        }
    }

    #[test]
    fn distance_counts_steps() {
        let origin = Hex::default();
        assert_eq!(origin.distance(origin), 0);
        for direction in 0..6 {
            assert_eq!(origin.distance(origin.neighbour(direction)), 1);
        }
        assert_eq!(origin.distance(Hex::new(3, 0)), 3);
        // Across the grain, not along it: q and r pulling opposite ways cancel.
        assert_eq!(origin.distance(Hex::new(3, -3)), 3);
        assert_eq!(origin.distance(Hex::new(3, 3)), 6);
    }

    #[test]
    fn cube_coordinates_always_sum_to_zero() {
        for hex in Hex::new(2, -5).within(3) {
            assert_eq!(hex.q + hex.r + hex.s(), 0);
        }
    }

    /// A hexagon of radius n holds 3n(n+1)+1 cells. A rhombus would hold (2n+1)^2, which
    /// is what the bounds in `within` exist to avoid.
    #[test]
    fn a_range_is_a_hexagon_not_a_rhombus() {
        for radius in 0..6u32 {
            let count = Hex::default().within(radius).count();
            assert_eq!(
                count as u32,
                3 * radius * (radius + 1) + 1,
                "radius {radius}"
            );
        }
    }

    #[test]
    fn everything_in_range_is_within_that_distance() {
        let centre = Hex::new(-4, 7);
        assert!(centre.within(5).all(|hex| centre.distance(hex) <= 5));
    }
}

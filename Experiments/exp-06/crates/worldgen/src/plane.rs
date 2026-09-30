//! The plane, `(east, north)` in ri, where a cell's centre is derived from its cube coordinate.
//! Pointy-top; `+r` is south. Only geometry lives here; the API names cells (spec §2).

use super::{F, SQRT_3};
use crate::hex::Cube;

/// The vertical pitch of a row of cells: √3/2 of the flat-to-flat size.
pub const ROW: F = SQRT_3 / 2.0;

/// The centre of the cell `c` of a rung whose cells are `size` ri flat to flat, inside a parent
/// centred on `origin`: the page's `riCentre`, `[x + (q − s)/2, y − r·√3/2]` scaled by the size.
pub fn centre(c: Cube, size: F, origin: [F; 2]) -> [F; 2] {
    [
        origin[0] + (c.q - c.s) as F / 2.0 * size,
        origin[1] - c.r as F * ROW * size,
    ]
}

/// The six corners of a cell of flat-to-flat 1 about its centre: angles 30° + 60°·i, radius 1/√3.
pub fn unit_hex() -> [[F; 2]; 6] {
    core::array::from_fn(|i| {
        let a = super::rad(30.0 + 60.0 * i as F);
        [a.cos() / SQRT_3, a.sin() / SQRT_3]
    })
}

#[cfg(test)]
mod tests {
    use super::super::TOL;
    use super::*;

    fn close(a: F, b: F) -> bool {
        (a - b).abs() <= TOL * b.abs().max(1.0)
    }

    #[test]
    fn a_ri_east_of_the_origin_is_one_ri_east() {
        let c = centre(Cube::new(1, 0, -1), 1.0, [10.0, -5.0]);
        assert!(close(c[0], 11.0) && close(c[1], -5.0), "{c:?}");
    }

    #[test]
    fn south_is_minus_r_rows() {
        // +r is south: (0, 1, −1) sits a row below, half a cell east, as on the page.
        let c = centre(Cube::new(0, 1, -1), 1.0, [0.0, 0.0]);
        assert!(close(c[0], 0.5) && close(c[1], -ROW), "{c:?}");
    }

    #[test]
    fn a_cho_is_a_thirty_sixth_of_a_ri() {
        // the page's chō centre: rc + [(q − s)/72, −r·ROW/36]
        let c = centre(Cube::new(3, -1, -2), 1.0 / 36.0, [2.0, 2.0]);
        assert!(
            close(c[0], 2.0 + 5.0 / 72.0) && close(c[1], 2.0 + ROW / 36.0),
            "{c:?}"
        );
    }

    #[test]
    fn the_unit_hex_is_flat_to_flat_one() {
        let h = unit_hex();
        // corner 0 at 30°, radius 1/√3; flat to flat is the distance between opposite edges = 1
        assert!(close(h[0][0], (30.0 as F).to_radians().cos() / SQRT_3));
        assert!(close(h[0][0], 0.5), "{}", h[0][0]); // the east edge stands half the flat-to-flat out
        assert!(close(h[1][1], 1.0 / SQRT_3), "{}", h[1][1]); // the top corner is a circumradius up
    }
}

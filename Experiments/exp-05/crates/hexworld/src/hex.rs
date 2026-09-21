//! Axial hex coordinates. Every level of the hierarchy is a pointy-top hex lattice
//! with the same orientation, so one `Hex` type serves them all.

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord, Default)]
pub struct Hex {
    pub q: i32,
    pub r: i32,
}

impl Hex {
    pub const ZERO: Hex = Hex { q: 0, r: 0 };

    pub const fn new(q: i32, r: i32) -> Self {
        Hex { q, r }
    }

    /// The third cube coordinate, `-q - r`.
    pub const fn s(self) -> i32 {
        -self.q - self.r
    }
}

impl std::ops::Add for Hex {
    type Output = Hex;
    fn add(self, o: Hex) -> Hex {
        Hex::new(self.q + o.q, self.r + o.r)
    }
}

impl std::ops::Sub for Hex {
    type Output = Hex;
    fn sub(self, o: Hex) -> Hex {
        Hex::new(self.q - o.q, self.r - o.r)
    }
}

/// The six unit steps, anticlockwise from east: direction `k` points at 60°·k.
/// The order matters — the mesher pairs direction `k` with the cell edge between
/// corners `k-1` and `k`, so do not reorder these without changing `corners_m`.
pub const DIRECTIONS: [Hex; 6] = [
    Hex::new(1, 0),  //   0°, east
    Hex::new(0, 1),  //  60°
    Hex::new(-1, 1), // 120°
    Hex::new(-1, 0), // 180°, west
    Hex::new(0, -1), // 240°
    Hex::new(1, -1), // 300°
];

/// Squared hex-plane distance of an offset, in units of the cell width squared, exactly.
/// `i64` because at ri scale the squares approach the `i32` limit.
pub fn d2(h: Hex) -> i64 {
    let (q, r) = (h.q as i64, h.r as i64);
    q * q + q * r + r * r
}

pub fn distance(a: Hex, b: Hex) -> i32 {
    let d = a - b;
    (d.q.abs() + d.r.abs() + d.s().abs()) / 2
}

pub fn neighbours(h: Hex) -> [Hex; 6] {
    DIRECTIONS.map(|d| h + d)
}

/// The cell plus `rings` rings of neighbours at the same level.
pub fn range(centre: Hex, rings: i32) -> Vec<Hex> {
    let mut out = Vec::new();
    for q in -rings..=rings {
        let lo = (-rings).max(-q - rings);
        let hi = rings.min(-q + rings);
        for r in lo..=hi {
            out.push(centre + Hex::new(q, r));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_steps_have_d2_of_one() {
        for d in DIRECTIONS {
            assert_eq!(d2(d), 1, "{d:?}");
        }
    }

    #[test]
    fn distance_counts_steps() {
        assert_eq!(distance(Hex::ZERO, Hex::ZERO), 0);
        assert_eq!(distance(Hex::ZERO, Hex::new(3, 0)), 3);
        assert_eq!(distance(Hex::new(-2, 5), Hex::new(-2, 5)), 0);
        assert_eq!(distance(Hex::ZERO, Hex::new(2, -1)), 2);
    }

    #[test]
    fn range_counts_are_hex_numbers() {
        assert_eq!(range(Hex::ZERO, 0).len(), 1);
        assert_eq!(range(Hex::ZERO, 1).len(), 7);
        assert_eq!(range(Hex::ZERO, 3).len(), 37);
        assert_eq!(range(Hex::new(9, -4), 3).len(), 37);
    }

    #[test]
    fn range_is_centred_and_within_distance() {
        let centre = Hex::new(9, -4);
        let cells = range(centre, 3);
        assert!(cells.contains(&centre));
        assert!(cells.iter().all(|c| distance(*c, centre) <= 3));
    }

    #[test]
    fn neighbours_are_distance_one() {
        for n in neighbours(Hex::new(4, 4)) {
            assert_eq!(distance(n, Hex::new(4, 4)), 1);
        }
    }

    #[test]
    fn directions_run_anticlockwise_from_east() {
        // Direction k must point at 60 degrees times k: the mesher relies on it to pair
        // each neighbour with a cell edge.
        let angle = |h: Hex| {
            let (e, n) = (
                (h.q as f64) + (h.r as f64) / 2.0,
                h.r as f64 * 3f64.sqrt() / 2.0,
            );
            n.atan2(e).to_degrees().rem_euclid(360.0)
        };
        for (k, d) in DIRECTIONS.iter().enumerate() {
            assert!(
                (angle(*d) - 60.0 * k as f64).abs() < 1e-9,
                "direction {k} is {:?}",
                d
            );
        }
    }
}

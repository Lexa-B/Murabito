//! Cube coordinates `(q, r, s)`, pointy-top, `+q` east, `+r` south, `s = −q − r`, with the six edge
//! directions in the main branch's `Direction::ALL` order, and packing B (`owner`) as exp-03 defined it.

/// A cell of any rung, by its cube coordinate within its parent. `s` is carried, never derived at use.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Cube {
    pub q: i32,
    pub r: i32,
    pub s: i32,
}

impl Cube {
    pub const ORIGIN: Cube = Cube { q: 0, r: 0, s: 0 };

    /// Panics if `q + r + s ≠ 0`: a coordinate off the plane is a caller's mistake, never data.
    pub const fn new(q: i32, r: i32, s: i32) -> Cube {
        assert!(q + r + s == 0, "cube coordinate off the plane");
        Cube { q, r, s }
    }

    pub const fn add(self, o: Cube) -> Cube {
        Cube {
            q: self.q + o.q,
            r: self.r + o.r,
            s: self.s + o.s,
        }
    }

    pub const fn scale(self, n: i32) -> Cube {
        Cube {
            q: self.q * n,
            r: self.r * n,
            s: self.s * n,
        }
    }

    pub const fn neighbour(self, d: Direction) -> Cube {
        self.add(d.step())
    }

    /// The six neighbours in `Direction::ALL` order.
    pub fn neighbours(self) -> [Cube; 6] {
        Direction::ALL.map(|d| self.neighbour(d))
    }

    /// Half the squared cube length, `(q² + r² + s²)/2`: the packing's distance, an integer.
    pub const fn d2(self) -> i64 {
        let (q, r, s) = (self.q as i64, self.r as i64, self.s as i64);
        (q * q + r * r + s * s) / 2
    }
}

/// The world is one packing cell of this many ri: 1,296 ri, 36 ri flat to flat.
pub const WORLD_N: i32 = 36;

/// The six edge directions, anticlockwise from east: the even entries of the main branch's twelve.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Direction {
    E,
    NNE,
    NNW,
    W,
    SSW,
    SSE,
}

impl Direction {
    pub const ALL: [Direction; 6] = [
        Direction::E,
        Direction::NNE,
        Direction::NNW,
        Direction::W,
        Direction::SSW,
        Direction::SSE,
    ];

    pub const fn step(self) -> Cube {
        match self {
            Direction::E => Cube { q: 1, r: 0, s: -1 },
            Direction::NNE => Cube { q: 1, r: -1, s: 0 },
            Direction::NNW => Cube { q: 0, r: -1, s: 1 },
            Direction::W => Cube { q: -1, r: 0, s: 1 },
            Direction::SSW => Cube { q: -1, r: 1, s: 0 },
            Direction::SSE => Cube { q: 0, r: 1, s: -1 },
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Direction::E => "E",
            Direction::NNE => "NNE",
            Direction::NNW => "NNW",
            Direction::W => "W",
            Direction::SSW => "SSW",
            Direction::SSE => "SSE",
        }
    }

    /// Position in `ALL`: the world edge's index and the heading in sixths of a turn.
    pub const fn index(self) -> usize {
        self as usize
    }
}

/// JS `Math.round`: a half rounds toward +∞. The page's `owner()` picks its candidate parents with it,
/// and the port keeps the same candidates so the two agree on every tie.
fn js_round(x: f64) -> i32 {
    (x + 0.5).floor() as i32
}

/// exp-03's packing B in cube: the parent `(a, b, c)` whose centre `(N·a, N·b, N·c)` is nearest to the
/// cell, by `d2`; ties go to the greater `(a, b)`.
pub fn owner(c: Cube, n: i32) -> Cube {
    let a0 = js_round(c.q as f64 / n as f64);
    let b0 = js_round(c.r as f64 / n as f64);
    let mut best: Option<(i64, Cube)> = None;
    for a in a0 - 1..=a0 + 1 {
        for b in b0 - 1..=b0 + 1 {
            let parent = Cube::new(a, b, -a - b);
            let d2 = c.add(parent.scale(-n)).d2();
            let better = match best {
                None => true,
                Some((bd, bp)) => d2 < bd || (d2 == bd && (a > bp.q || (a == bp.q && b > bp.r))),
            };
            if better {
                best = Some((d2, parent));
            }
        }
    }
    best.expect("nine candidates").1
}

/// The cells a parent of packing `n` owns, as offsets from its centre: 1,296 for the world's ri
/// (n = 36), 100 for a chō's decichō (10), 36 for a decichō's ken and a ken's shaku (6). Walked with
/// `q` outer and `r` inner over `|q|, |r| ≤ n`, the page's order.
pub fn children(n: i32) -> Vec<Cube> {
    let mut out = Vec::new();
    for q in -n..=n {
        for r in -n..=n {
            let c = Cube::new(q, r, -q - r);
            if owner(c, n) == Cube::ORIGIN {
                out.push(c);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directions_are_the_main_branch_order() {
        let steps: Vec<Cube> = Direction::ALL.iter().map(|d| d.step()).collect();
        assert_eq!(
            steps,
            vec![
                Cube::new(1, 0, -1),
                Cube::new(1, -1, 0),
                Cube::new(0, -1, 1),
                Cube::new(-1, 0, 1),
                Cube::new(-1, 1, 0),
                Cube::new(0, 1, -1),
            ]
        );
        assert_eq!(
            Direction::ALL.map(|d| d.name()),
            ["E", "NNE", "NNW", "W", "SSW", "SSE"]
        );
        assert_eq!(Direction::SSW.index(), 4);
    }

    #[test]
    #[should_panic(expected = "off the plane")]
    fn a_cube_off_the_plane_is_refused() {
        let _ = Cube::new(1, 1, 1);
    }

    #[test]
    fn neighbours_step_in_all_order() {
        let c = Cube::new(2, -3, 1);
        assert_eq!(c.neighbours()[0], Cube::new(3, -3, 0));
        assert_eq!(c.neighbours()[5], Cube::new(2, -2, 0));
        assert_eq!(c.neighbour(Direction::W), Cube::new(1, -3, 2));
    }

    #[test]
    fn d2_is_half_the_squared_cube_length() {
        assert_eq!(Cube::new(18, 0, -18).d2(), 324);
        assert_eq!(Cube::new(1, -1, 0).d2(), 1);
    }

    #[test]
    fn the_world_owns_1296_ri_and_the_finer_packings_100_and_36() {
        assert_eq!(children(36).len(), 1296);
        assert_eq!(children(10).len(), 100);
        assert_eq!(children(6).len(), 36);
        assert_eq!(children(36)[0], Cube::new(-24, 12, 12)); // q outer, r inner, as the page walks them (printed from the page's owner())
    }

    #[test]
    fn owner_ties_go_to_the_greater_parent() {
        // (18, 0, −18) is 324 from the origin's centre and 324 from (1, 0, −1)'s: the greater (a, b) wins.
        assert_eq!(owner(Cube::new(18, 0, -18), 36), Cube::new(1, 0, -1));
        assert_eq!(owner(Cube::new(-18, 0, 18), 36), Cube::ORIGIN);
        assert_eq!(owner(Cube::new(0, 18, -18), 36), Cube::new(0, 1, -1));
        assert_eq!(owner(Cube::new(18, -18, 0), 36), Cube::new(1, -1, 0));
        // an ordinary cell
        assert_eq!(owner(Cube::new(40, -5, -35), 36), Cube::new(1, 0, -1));
        assert_eq!(owner(Cube::new(5, 5, -10), 10), Cube::new(1, 0, -1));
    }

    #[test]
    fn every_child_names_the_origin_as_owner() {
        for c in children(36) {
            assert_eq!(owner(c, 36), Cube::ORIGIN, "{c:?}");
        }
    }

    #[test]
    fn the_world_is_a_packing_cell_of_36_ri() {
        assert_eq!(WORLD_N, 36);
        assert_eq!(children(WORLD_N).len(), 1296);
    }
}

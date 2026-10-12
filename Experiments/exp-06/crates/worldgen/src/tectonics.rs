//! Step 2: the height surface, `h = H(s)·F(a)` in metres, positive on land and negative at sea, zero exactly
//! on the outline. `F(a)` is one cross-section, a monotone cubic through a short table in units of the spine
//! height; `H(s)` is the spine's height along the centreline. Round a cap the section goes polar.

use super::banana::{self, Edge, EdgeKind, Outline, ARC_DEG, PHI_DEG, R_RI, W_RI};
use super::plane;
use super::{rad, F, PI};
use crate::hex::{self, Cube, Direction};

/// The cross-section's control points `(a, F)`: `a` counts half-widths from the centreline, the coasts at ±1,
/// from a deep basin three half-widths inside to a trench three out. The plain was raised twice on
/// 2026-09-29 (the points at 0.5 and 0.8) so the erosion in metres would not eat the lowland.
pub const SECTION: [[F; 2]; 15] = [
    [-3.0, -1.0],
    [-2.2, -0.9],
    [-1.5, -0.35],
    [-1.15, -0.05],
    [-1.0, 0.0],
    [-0.6, 0.35],
    [-0.1, 1.0],
    [0.2, 0.45],
    [0.5, 0.18],
    [0.8, 0.07],
    [1.0, 0.0],
    [1.3, -0.03],
    [1.7, -0.1],
    [2.3, -0.8],
    [3.0, -2.4],
];
/// The spine at the tips, metres.
pub const H_TIP: F = 900.0;
/// The spine at the bend, metres.
pub const H_APEX: F = 2500.0;

/// A monotone cubic through a table (Fritsch–Carlson): smooth, never overshooting the table, clamped outside it.
pub struct Spline {
    x: Vec<F>,
    y: Vec<F>,
    m: Vec<F>,
}

impl Spline {
    pub fn new(points: &[[F; 2]]) -> Spline {
        let n = points.len();
        let x: Vec<F> = points.iter().map(|p| p[0]).collect();
        let y: Vec<F> = points.iter().map(|p| p[1]).collect();
        let d: Vec<F> = (0..n - 1)
            .map(|i| (y[i + 1] - y[i]) / (x[i + 1] - x[i]))
            .collect();
        let mut m = Vec::with_capacity(n);
        m.push(d[0]);
        for i in 1..n - 1 {
            m.push(if d[i - 1] * d[i] <= 0.0 {
                0.0
            } else {
                (d[i - 1] + d[i]) / 2.0
            });
        }
        m.push(d[n - 2]);
        for i in 0..n - 1 {
            if d[i] == 0.0 {
                m[i] = 0.0;
                m[i + 1] = 0.0;
                continue;
            }
            let a = m[i] / d[i];
            let b = m[i + 1] / d[i];
            let s = a * a + b * b;
            if s > 9.0 {
                let t = 3.0 / s.sqrt();
                m[i] = t * a * d[i];
                m[i + 1] = t * b * d[i];
            }
        }
        Spline { x, y, m }
    }

    pub fn at(&self, v: F) -> F {
        let n = self.x.len();
        if v <= self.x[0] {
            return self.y[0];
        }
        if v >= self.x[n - 1] {
            return self.y[n - 1];
        }
        let mut i = 0;
        while v > self.x[i + 1] {
            i += 1;
        }
        let h = self.x[i + 1] - self.x[i];
        let t = (v - self.x[i]) / h;
        let t2 = t * t;
        let t3 = t2 * t;
        (2.0 * t3 - 3.0 * t2 + 1.0) * self.y[i]
            + (t3 - 2.0 * t2 + t) * h * self.m[i]
            + (-2.0 * t3 + 3.0 * t2) * self.y[i + 1]
            + (t3 - t2) * h * self.m[i + 1]
    }
}

/// `H(s)`: the spine's height along the centreline, a raised sine from the tips to the bend; `s` clamped to
/// `[0, 1]`, so past a cap it is the tip's.
pub fn spine_height(s: F) -> F {
    H_TIP + (H_APEX - H_TIP) * (PI * s.clamp(0.0, 1.0)).sin()
}

/// A point in the band's own frame: `s` along the centreline (0 at the bottom tip, 1 at the top), `a` across
/// in half-widths (the coasts at ±1), and beyond a tip `e` along the cap in half-widths, else 0.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Band {
    pub s: F,
    pub a: F,
    pub e: F,
}

pub fn band_coords(p: [F; 2]) -> Band {
    // unturned: the arc is centred on the plane's origin, apex east
    let (c, sn) = (rad(-PHI_DEG).cos(), rad(-PHI_DEG).sin());
    let x = p[0] * c - p[1] * sn;
    let y = p[0] * sn + p[1] * c;
    let r = (x * x + y * y).sqrt();
    let ang = y.atan2(x);
    let half = rad(ARC_DEG) / 2.0;
    let hw = W_RI / 2.0;
    if ang.abs() <= half {
        return Band {
            s: (ang + half) / (2.0 * half),
            a: (r - R_RI) / hw,
            e: 0.0,
        };
    }
    let north = ang > 0.0;
    let end = if north { half } else { -half };
    let sign = if north { 1.0 } else { -1.0 };
    let (cx, cy) = (R_RI * end.cos(), R_RI * end.sin());
    let (nx, ny) = (end.cos(), end.sin());
    let (tx, ty) = (-end.sin() * sign, end.cos() * sign);
    Band {
        s: if north { 1.0 } else { 0.0 },
        a: ((x - cx) * nx + (y - cy) * ny) / hw,
        e: ((x - cx) * tx + (y - cy) * ty) / hw,
    }
}

/// The gradient of any field in its units per ri, by central differences `d` either side.
pub fn gradient_of(field: impl Fn([F; 2]) -> F, p: [F; 2], d: F) -> [F; 2] {
    [
        (field([p[0] + d, p[1]]) - field([p[0] - d, p[1]])) / (2.0 * d),
        (field([p[0], p[1] + d]) - field([p[0], p[1] - d])) / (2.0 * d),
    ]
}

/// The surface: the section spline built once.
pub struct Tectonics {
    section: Spline,
}

impl Default for Tectonics {
    fn default() -> Self {
        Self::new()
    }
}

/// The cross-section at the world's `s`, 301 samples of `(a, H(s)·F(a))` for `a` from −2.5 to 2.5.
pub struct SectionProfile {
    pub s: F,
    pub spine: F,
    /// The world's centre across the band, for marking its span on the chart.
    pub a: F,
    pub points: Vec<[F; 2]>,
}

/// One of the world's ri with its height: keyed by its cube, the centre carried for the drawing.
#[derive(Clone, Copy, Debug)]
pub struct RiHeight {
    pub cell: Cube,
    pub centre: [F; 2],
    pub h: F,
}

/// The page's footer under the height map: ri on land, the highest on land, the deepest at sea, the land mean.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hypsometry {
    pub land: usize,
    pub highest: F,
    pub deepest: F,
    pub land_mean: F,
}

/// Height along one world edge, 121 samples from its first corner to its second.
pub struct EdgeProfile {
    pub direction: Direction,
    pub kind: EdgeKind,
    pub heights: Vec<F>,
    pub bottom: F,
    pub top: F,
}

impl Tectonics {
    pub fn new() -> Tectonics {
        Tectonics {
            section: Spline::new(&SECTION),
        }
    }

    /// `F(a)`, in units of the spine height.
    pub fn f(&self, a: F) -> F {
        self.section.at(a)
    }

    /// Height in metres at a plane point. Along the arc `H(s)·F(a)`; round a cap the distance from the cap's
    /// centre stands in for `a` and the outer and inner profiles are blended by the angle, which meets the
    /// arc's values where the two regions join.
    pub fn height(&self, p: [F; 2]) -> F {
        let b = band_coords(p);
        if b.e <= 0.0 {
            return spine_height(b.s) * self.f(b.a);
        }
        let rho = (b.a * b.a + b.e * b.e).sqrt();
        let t = (b.a / rho + 1.0) / 2.0;
        spine_height(b.s) * (t * self.f(rho) + (1.0 - t) * self.f(-rho))
    }

    /// The surface's slope in metres per ri, central differences half a ri apart (the page's `gradient`).
    pub fn gradient(&self, p: [F; 2]) -> [F; 2] {
        gradient_of(|q| self.height(q), p, 0.25)
    }

    pub fn section_at(&self, origin: [F; 2]) -> SectionProfile {
        let b = band_coords(origin);
        let spine = spine_height(b.s);
        let points = (0..=300)
            .map(|i| {
                let a = -2.5 + (i as F / 300.0) * 5.0;
                [a, spine * self.f(a)]
            })
            .collect();
        SectionProfile {
            s: b.s,
            spine,
            a: b.a,
            points,
        }
    }

    /// Every ri the world owns, with its centre and height, in `hex::children` order.
    pub fn ri_heights(&self, origin: [F; 2]) -> Vec<RiHeight> {
        hex::children(hex::WORLD_N)
            .into_iter()
            .map(|cell| {
                let centre = plane::centre(cell, 1.0, origin);
                RiHeight {
                    cell,
                    centre,
                    h: self.height(centre),
                }
            })
            .collect()
    }

    pub fn edge_profile(&self, e: &Edge) -> EdgeProfile {
        let mut heights = Vec::with_capacity(121);
        let (mut bottom, mut top) = (F::INFINITY, F::NEG_INFINITY);
        for i in 0..=120 {
            let t = i as F / 120.0;
            let p = [
                e.a[0] + (e.b[0] - e.a[0]) * t,
                e.a[1] + (e.b[1] - e.a[1]) * t,
            ];
            let h = self.height(p);
            bottom = bottom.min(h);
            top = top.max(h);
            heights.push(h);
        }
        EdgeProfile {
            direction: e.direction,
            kind: e.kind,
            heights,
            bottom,
            top,
        }
    }

    /// The slope half a ri inland of the outline point at `θ`, metres per ri: the lever the erosion works
    /// through, 100 m of height moving the coast `100 / slope` ri.
    pub fn coast_slope_inland(&self, o: &Outline, theta: F) -> F {
        let c = banana::at(o, theta);
        let p = [c.p[0] - c.normal[0] * 0.5, c.p[1] - c.normal[1] * 0.5];
        let g = self.gradient(p);
        (g[0] * g[0] + g[1] * g[1]).sqrt()
    }
}

/// The footer's numbers, as the page counts them: `highest` and `deepest` start at 0, the mean is over land
/// and 0 when there is none.
pub fn hypsometry(ri: &[RiHeight]) -> Hypsometry {
    let (mut land, mut highest, mut deepest, mut sum) = (0usize, 0.0, 0.0, 0.0);
    for r in ri {
        if r.h > 0.0 {
            land += 1;
            sum += r.h;
            highest = F::max(highest, r.h);
        } else {
            deepest = F::min(deepest, r.h);
        }
    }
    let land_mean = if land > 0 { sum / land as F } else { 0.0 };
    Hypsometry {
        land,
        highest,
        deepest,
        land_mean,
    }
}

#[cfg(test)]
#[allow(clippy::excessive_precision)] // the page's f64 readings, pinned verbatim, read by both twins
mod tests {
    use super::super::banana;
    use super::super::TOL;
    use super::*;

    fn close(a: F, b: F) -> bool {
        (a - b).abs() <= TOL * b.abs().max(1.0)
    }
    fn exact() -> bool {
        TOL < 1e-6
    }

    #[test]
    fn the_spline_passes_through_the_table_and_clamps_outside_it() {
        let t = Tectonics::new();
        for [a, f] in SECTION {
            assert!(close(t.f(a), f), "F({a}) = {}", t.f(a));
        }
        assert!(close(t.f(-3.5), -1.0));
        assert!(close(t.f(3.5), -2.4));
        // between the knots, as the page's spline gives them
        assert!(close(t.f(-2.6), -0.976235343730098), "{}", t.f(-2.6));
        assert!(close(t.f(-1.3), -0.1670553935860059), "{}", t.f(-1.3));
        assert!(close(t.f(-0.8), 0.1508333333333333), "{}", t.f(-0.8));
        assert!(close(t.f(0.35), 0.2875000000000001), "{}", t.f(0.35));
        assert!(close(t.f(0.65), 0.1146875), "{}", t.f(0.65));
        assert!(close(t.f(1.5), -0.04455547685878496), "{}", t.f(1.5));
        assert!(close(t.f(2.9), -2.1645772594752186), "{}", t.f(2.9));
    }

    #[test]
    fn the_spine_is_900_at_the_tips_and_2500_at_the_bend() {
        assert!(close(spine_height(0.0), 900.0));
        assert!(close(spine_height(0.25), 2031.3708498984759));
        assert!(close(spine_height(0.58), 2449.73305780581));
        assert!(close(spine_height(1.0), 900.0));
        assert!(close(spine_height(1.2), 900.0), "clamped past the tip");
        assert!(close(spine_height(-0.3), 900.0));
    }

    #[test]
    fn band_coords_at_the_default_origin() {
        let o = banana::outline();
        let w = banana::place(&o, 12.0, 10.0);
        let b = band_coords(w.origin);
        assert!(close(b.s, 0.576068087586871), "{}", b.s);
        assert!(close(b.a, 0.6666092524998021), "{}", b.a);
        assert_eq!(b.e, 0.0);
        let t = Tectonics::new();
        assert!(close(spine_height(b.s), 2454.5297963717007));
        assert!(
            close(t.height(w.origin), 269.4666483278163),
            "{}",
            t.height(w.origin)
        );
    }

    #[test]
    fn round_a_cap_the_band_goes_polar() {
        let t = Tectonics::new();
        // beyond the north tip
        let p = [261.27949313437693, 34.972847861031724];
        let b = band_coords(p);
        assert_eq!(b.s, 1.0);
        assert!(close(b.a, 0.8093164378125649), "{}", b.a);
        assert!(close(b.e, 1.1657615953677236), "{}", b.e);
        assert!(close(t.height(p), -80.95309411248147), "{}", t.height(p));
        // beyond the south tip
        let p = [14.878628273876359, -265.3752249436621];
        let b = band_coords(p);
        assert_eq!(b.s, 0.0);
        assert!(close(b.a, 0.8975741889273138), "{}", b.a);
        assert!(close(b.e, 1.0476445243706196), "{}", b.e);
        assert!(close(t.height(p), -64.7602542857592), "{}", t.height(p));
    }

    #[test]
    fn the_height_is_continuous_where_the_arc_meets_a_cap() {
        // two points a hair either side of the north join, 10 ri in from the outer coast
        let t = Tectonics::new();
        let half = rad(ARC_DEG) / 2.0;
        let r = R_RI + W_RI / 2.0 - 10.0;
        let point = |ang: F| {
            let (c, s) = (rad(PHI_DEG).cos(), rad(PHI_DEG).sin());
            let (x, y) = (r * ang.cos(), r * ang.sin());
            [x * c - y * s, x * s + y * c]
        };
        let inside = t.height(point(half - 1e-4));
        let outside = t.height(point(half + 1e-4));
        assert!(
            band_coords(point(half - 1e-4)).e == 0.0 && band_coords(point(half + 1e-4)).e > 0.0
        );
        assert!((inside - outside).abs() < 2.0, "{inside} vs {outside}");
    }

    #[test]
    fn the_inner_coast_is_at_sea_level() {
        let o = banana::outline();
        let t = Tectonics::new();
        let p = banana::at(&o, 200.0).p;
        let b = band_coords(p);
        assert!((b.a + 1.0).abs() < 1e-3, "{}", b.a);
        let h = t.height(p);
        if exact() {
            assert!(close(b.a, -1.0000505545179559));
            assert!(close(h, -0.07002509719772386), "{h}");
        } else {
            assert!(h < 0.0 && h > -0.5, "{h}");
        }
    }

    #[test]
    fn a_ri_by_its_cube_and_the_worlds_hypsometry() {
        let o = banana::outline();
        let w = banana::place(&o, 12.0, 10.0);
        let t = Tectonics::new();
        let ri = t.ri_heights(w.origin);
        assert_eq!(ri.len(), 1296);
        let one = ri
            .iter()
            .find(|r| r.cell == Cube::new(5, -7, 2))
            .expect("the world owns (5, -7, 2)");
        assert!(
            close(one.centre[0], 214.77531285149172) && close(one.centre[1], -137.33153055923975),
            "{:?}",
            one.centre
        );
        assert!(close(one.h, 321.5864189748716), "{}", one.h);
        let hy = hypsometry(&ri);
        assert_eq!(hy.land, 1069);
        assert!(close(hy.highest, 2293.2028845553873), "{}", hy.highest);
        assert!(close(hy.deepest, -80.35501752688968), "{}", hy.deepest);
        assert!(close(hy.land_mean, 484.790471937896), "{}", hy.land_mean);
    }

    #[test]
    fn a_world_entirely_at_sea_has_no_land_mean_to_divide() {
        let o = banana::outline();
        let w = banana::place(&o, 12.0, -40.0);
        let hy = hypsometry(&Tectonics::new().ri_heights(w.origin));
        assert_eq!(hy.land, 0);
        assert_eq!(hy.land_mean, 0.0);
        assert_eq!(hy.highest, 0.0);
        assert!(hy.deepest < 0.0);
    }

    #[test]
    fn the_six_edge_profiles_at_the_defaults() {
        let o = banana::outline();
        let w = banana::place(&o, 12.0, 10.0);
        let t = Tectonics::new();
        let edges = banana::read_edges(&o, &banana::world_corners(w.origin));
        let ranges: Vec<(F, F)> = edges
            .iter()
            .map(|e| {
                let p = t.edge_profile(e);
                assert_eq!(p.heights.len(), 121);
                (p.bottom, p.top)
            })
            .collect();
        let want = [
            (-85.15393988473546, 3.8537492149634134),
            (3.8537492149634134, 791.3489978242551),
            (791.3489978242551, 2293.2028845553873),
            (646.1604346405842, 2293.2028845553873),
            (-30.609523806706513, 646.1604346405842),
            (-85.15393988473546, -30.609523806706513),
        ];
        for (i, ((lo, hi), (wlo, whi))) in ranges.iter().zip(want).enumerate() {
            assert!(close(*lo, wlo) && close(*hi, whi), "edge {i}: {lo}…{hi}");
        }
    }

    #[test]
    fn an_edge_entirely_under_the_sea_never_crosses_zero() {
        let o = banana::outline();
        let w = banana::place(&o, 200.0, -4.0);
        let t = Tectonics::new();
        let edges = banana::read_edges(&o, &banana::world_corners(w.origin));
        let nne = t.edge_profile(&edges[1]);
        assert!(
            close(nne.bottom, -1205.9604570803763) && close(nne.top, -19.771523783059592),
            "{}…{}",
            nne.bottom,
            nne.top
        );
        assert!(nne.heights.iter().all(|h| *h < 0.0));
    }

    #[test]
    fn the_section_and_the_coast_slope_at_the_defaults() {
        let o = banana::outline();
        let w = banana::place(&o, 12.0, 10.0);
        let t = Tectonics::new();
        let sec = t.section_at(w.origin);
        assert_eq!(sec.points.len(), 301);
        assert!(
            close(sec.points[0][0], -2.5) && close(sec.points[0][1], -2373.6245587615936),
            "{:?}",
            sec.points[0]
        );
        assert!(
            close(sec.points[60][1], -859.0854287300953),
            "{:?}",
            sec.points[60]
        );
        assert!(
            close(sec.points[150][1], 2179.07700811221),
            "{:?}",
            sec.points[150]
        );
        assert!(
            close(sec.points[300][0], 2.5) && close(sec.points[300][1], -2945.555023371813),
            "{:?}",
            sec.points[300]
        );
        let g = t.gradient([221.1511697907647, -148.70604096870284]);
        assert!(
            close(g[0], -17.852566806267134) && close(g[1], 11.991861112759638),
            "{g:?}"
        );
        assert!(
            close(t.coast_slope_inland(&o, 12.0), 21.50625198680486),
            "{}",
            t.coast_slope_inland(&o, 12.0)
        );
        assert!(
            close(t.coast_slope_inland(&o, 200.0), 48.258966200169745),
            "{}",
            t.coast_slope_inland(&o, 200.0)
        );
    }
}

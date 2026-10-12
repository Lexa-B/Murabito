//! Step 1: the banana, and the world placed on it. A slot bent along a circular arc, sized like Honshu
//! at its Kantō width, turned so the top tip of the centreline heads north. The outline is walked
//! anticlockwise from the outer apex; `θ` is the fraction of its length walked, in degrees. The world
//! is centred on the outline point at `θ` moved inland by the inset. Sizes are Lexa's constants.

use super::{rad, F, PI};
use crate::hex::Direction;

/// The world's flat-to-flat, in ri.
pub const WORLD: F = 36.0;
/// The centreline's radius, in ri (about 1,300 km along the axis).
pub const R_RI: F = 237.0;
/// Coast to coast across the slot, in ri (about 236 km).
pub const W_RI: F = 60.0;
/// The arc the slot bends through, in degrees.
pub const ARC_DEG: F = 80.0;
/// The turn that puts the top tip of the centreline due north.
pub const PHI_DEG: F = -ARC_DEG / 2.0;

const ARC_SAMPLES: usize = 180;
const CAP_SAMPLES: usize = 60;

/// The outline as a dense polyline, anticlockwise from the outer apex, with its cumulative length.
pub struct Outline {
    pub pts: Vec<[F; 2]>,
    pub cum: Vec<F>,
    pub length: F,
}

/// A point on the outline and its outward normal.
#[derive(Clone, Copy, Debug)]
pub struct OutlinePoint {
    pub p: [F; 2],
    pub normal: [F; 2],
}

/// The world's placement: the outline point at `θ`, the origin inset from it, the arc's grain.
#[derive(Clone, Copy, Debug)]
pub struct Placement {
    pub theta: F,
    pub inset: F,
    pub here: OutlinePoint,
    /// The world's centre, "the origin".
    pub origin: [F; 2],
    /// The outline's tangent at `θ`: the normal turned anticlockwise, the arc's grain for steps 4–5.
    pub tangent: [F; 2],
}

/// What a world edge reads: at sea, on land, or crossed by the outline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgeKind {
    Sea,
    Land,
    Shore,
}

impl EdgeKind {
    pub const fn name(self) -> &'static str {
        match self {
            EdgeKind::Sea => "sea",
            EdgeKind::Land => "land",
            EdgeKind::Shore => "shore",
        }
    }
}

/// One of the world's six edges: edge `k` runs between corners `k − 1` and `k`, so edge 0 is the
/// east edge and the neighbour across it lies in `Direction::ALL[k]`.
#[derive(Clone, Debug)]
pub struct Edge {
    pub direction: Direction,
    pub a: [F; 2],
    pub b: [F; 2],
    pub crossings: Vec<[F; 2]>,
    pub kind: EdgeKind,
}

/// A point in the unturned frame, turned by `PHI_DEG` about the arc's centre.
fn turned(x: F, y: F) -> [F; 2] {
    let (s, c) = (rad(PHI_DEG).sin(), rad(PHI_DEG).cos());
    [x * c - y * s, x * s + y * c]
}

/// A cap: the semicircle of radius `w/2` about the centreline's tip at angle `a`, from parameter `t0`.
fn cap(a: F, t0: F) -> impl Iterator<Item = [F; 2]> {
    let (cx, cy) = (R_RI * a.cos(), R_RI * a.sin());
    let (ux, uy) = (a.cos(), a.sin());
    let (vx, vy) = (-a.sin(), a.cos());
    let r = W_RI / 2.0;
    (1..CAP_SAMPLES).map(move |i| {
        let t = t0 + (i as F / CAP_SAMPLES as F) * PI;
        turned(
            cx + r * (t.cos() * ux + t.sin() * vx),
            cy + r * (t.cos() * uy + t.sin() * vy),
        )
    })
}

/// The outline: the outer arc from the apex to `+A/2`, the north cap, the inner arc back, the south
/// cap, and the outer arc from `−A/2` to the apex. 481 points.
pub fn outline() -> Outline {
    let half = rad(ARC_DEG) / 2.0;
    let ro = R_RI + W_RI / 2.0;
    let ri = R_RI - W_RI / 2.0;
    let n = ARC_SAMPLES;
    let mut pts: Vec<[F; 2]> = Vec::with_capacity(2 * n + 2 * CAP_SAMPLES);
    for i in 0..=n / 2 {
        let a = (i as F / (n / 2) as F) * half;
        pts.push(turned(ro * a.cos(), ro * a.sin()));
    }
    pts.extend(cap(half, 0.0));
    for i in 0..=n {
        let a = half - (i as F / n as F) * 2.0 * half;
        pts.push(turned(ri * a.cos(), ri * a.sin()));
    }
    pts.extend(cap(-half, PI));
    for i in 0..=n / 2 {
        let a = -half + (i as F / (n / 2) as F) * half;
        pts.push(turned(ro * a.cos(), ro * a.sin()));
    }
    let mut cum = Vec::with_capacity(pts.len());
    cum.push(0.0);
    for i in 1..pts.len() {
        let (dx, dy) = (pts[i][0] - pts[i - 1][0], pts[i][1] - pts[i - 1][1]);
        cum.push(cum[i - 1] + (dx * dx + dy * dy).sqrt());
    }
    let length = *cum.last().expect("an outline has points");
    Outline { pts, cum, length }
}

/// The point a fraction `θ/360` of the way round the outline, and its outward normal: the outline is
/// anticlockwise, so outward is the tangent turned clockwise.
pub fn at(o: &Outline, theta: F) -> OutlinePoint {
    let s = theta / 360.0 * o.length;
    let mut i = 1;
    while i < o.cum.len() - 1 && o.cum[i] < s {
        i += 1;
    }
    let seg = o.cum[i] - o.cum[i - 1];
    let t = (s - o.cum[i - 1]) / if seg == 0.0 { 1.0 } else { seg };
    let (a, b) = (o.pts[i - 1], o.pts[i]);
    let p = [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
    let (tx, ty) = (b[0] - a[0], b[1] - a[1]);
    let l = (tx * tx + ty * ty).sqrt();
    let l = if l == 0.0 { 1.0 } else { l };
    OutlinePoint {
        p,
        normal: [ty / l, -tx / l],
    }
}

/// Even-odd point in polygon.
pub fn inside(o: &Outline, p: [F; 2]) -> bool {
    let pts = &o.pts;
    let mut c = false;
    let mut j = pts.len() - 1;
    for i in 0..pts.len() {
        let (pi, pj) = (pts[i], pts[j]);
        if (pi[1] > p[1]) != (pj[1] > p[1])
            && p[0] < (pj[0] - pi[0]) * (p[1] - pi[1]) / (pj[1] - pi[1]) + pi[0]
        {
            c = !c;
        }
        j = i;
    }
    c
}

/// Where segment `ab` crosses segment `cd`, if it does.
pub fn seg_cross(a: [F; 2], b: [F; 2], c: [F; 2], d: [F; 2]) -> Option<[F; 2]> {
    let den = (b[0] - a[0]) * (d[1] - c[1]) - (b[1] - a[1]) * (d[0] - c[0]);
    if den.abs() < 1e-12 {
        return None;
    }
    let t = ((c[0] - a[0]) * (d[1] - c[1]) - (c[1] - a[1]) * (d[0] - c[0])) / den;
    let u = ((c[0] - a[0]) * (b[1] - a[1]) - (c[1] - a[1]) * (b[0] - a[0])) / den;
    ((0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u))
        .then(|| [a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])])
}

/// The world placed: its origin is the outline point at `θ` moved inland by `inset` ri.
pub fn place(o: &Outline, theta: F, inset: F) -> Placement {
    let here = at(o, theta);
    let origin = [
        here.p[0] - here.normal[0] * inset,
        here.p[1] - here.normal[1] * inset,
    ];
    let tangent = [-here.normal[1], here.normal[0]];
    Placement {
        theta,
        inset,
        here,
        origin,
        tangent,
    }
}

/// The world hexagon's corners: pointy-top, at 30° + 60°·i, flat to flat `WORLD`.
pub fn world_corners(origin: [F; 2]) -> [[F; 2]; 6] {
    let r = WORLD / 2.0 / (PI / 6.0).cos();
    core::array::from_fn(|i| {
        let a = rad(30.0 + 60.0 * i as F);
        [origin[0] + r * a.cos(), origin[1] + r * a.sin()]
    })
}

/// Each edge read: shore if the outline crosses it, else land or sea by where its midpoint lies.
pub fn read_edges(o: &Outline, corners: &[[F; 2]; 6]) -> Vec<Edge> {
    Direction::ALL
        .iter()
        .enumerate()
        .map(|(k, &direction)| {
            let a = corners[(k + 5) % 6];
            let b = corners[k];
            let crossings: Vec<[F; 2]> = o
                .pts
                .windows(2)
                .filter_map(|w| seg_cross(a, b, w[0], w[1]))
                .collect();
            let mid = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0];
            let kind = if !crossings.is_empty() {
                EdgeKind::Shore
            } else if inside(o, mid) {
                EdgeKind::Land
            } else {
                EdgeKind::Sea
            };
            Edge {
                direction,
                a,
                b,
                crossings,
                kind,
            }
        })
        .collect()
}

/// The centreline arc, 61 points, for the drawing.
pub fn centreline() -> Vec<[F; 2]> {
    (0..=60)
        .map(|i| {
            let a = -rad(ARC_DEG) / 2.0 + (i as F / 60.0) * rad(ARC_DEG);
            turned(R_RI * a.cos(), R_RI * a.sin())
        })
        .collect()
}

/// The θ marks every 30°, for the drawing.
pub fn marks() -> Vec<(F, OutlinePoint)> {
    let o = outline();
    (0..12)
        .map(|i| {
            let deg = 30.0 * i as F;
            (deg, at(&o, deg))
        })
        .collect()
}

#[cfg(test)]
#[allow(clippy::excessive_precision)] // the page's f64 readings, pinned verbatim, read by both twins
mod tests {
    use super::super::TOL;
    use super::*;

    fn close(a: F, b: F) -> bool {
        (a - b).abs() <= TOL * b.abs().max(1.0)
    }
    fn close2(a: [F; 2], b: [F; 2]) -> bool {
        close(a[0], b[0]) && close(a[1], b[1])
    }

    #[test]
    fn the_outline_is_481_points_and_850_ri_round() {
        let o = outline();
        assert_eq!(o.pts.len(), 481);
        assert!(close(o.length, 850.3012208770891), "{}", o.length);
        assert!(
            close2(o.pts[0], [204.53386631276712, -171.62429178630597]),
            "{:?}",
            o.pts[0]
        );
        assert!(close2(o.pts[90], [267.0, 0.0]), "{:?}", o.pts[90]); // the outer arc's top, after the turn
        assert!(close2(o.pts[150], [207.0, 0.0]), "{:?}", o.pts[150]); // the inner arc's top
        assert!(close2(o.pts[480], o.pts[0]));
    }

    #[test]
    fn theta_12_is_the_default_placement_point() {
        let o = outline();
        let here = at(&o, 12.0);
        assert!(
            close2(here.p, [221.5656885770422, -148.9856374204382]),
            "{:?}",
            here.p
        );
        assert!(
            close2(here.normal, [0.829037572555049, -0.559192903470736]),
            "{:?}",
            here.normal
        );
    }

    #[test]
    fn theta_at_the_sliders_ends() {
        let o = outline();
        assert!(close2(at(&o, 0.0).p, o.pts[0]));
        let near_end = at(&o, 359.5);
        let d =
            ((near_end.p[0] - o.pts[0][0]).powi(2) + (near_end.p[1] - o.pts[0][1]).powi(2)).sqrt();
        assert!(d > 1.0 && d < 1.5, "{d} ri short of the apex"); // half a degree is 1.18 ri
    }

    #[test]
    fn the_default_placement_puts_the_origin_ten_ri_inland() {
        let o = outline();
        let w = place(&o, 12.0, 10.0);
        assert!(
            close2(w.origin, [213.27531285149172, -143.39370838573083]),
            "{:?}",
            w.origin
        );
        assert!(
            close2(w.tangent, [0.559192903470736, 0.829037572555049]),
            "{:?}",
            w.tangent
        );
        let corners = world_corners(w.origin);
        assert!(
            close2(corners[0], [231.27531285149172, -133.00140354031757]),
            "{:?}",
            corners[0]
        );
        assert!(
            close2(corners[3], [195.27531285149172, -153.7860132311441]),
            "{:?}",
            corners[3]
        );
        assert!(inside(&o, w.origin));
    }

    #[test]
    fn the_default_edges_read_shore_land_land_land_shore_sea() {
        let o = outline();
        let w = place(&o, 12.0, 10.0);
        let edges = read_edges(&o, &world_corners(w.origin));
        let kinds: Vec<EdgeKind> = edges.iter().map(|e| e.kind).collect();
        use EdgeKind::*;
        assert_eq!(kinds, vec![Shore, Land, Land, Land, Shore, Sea]);
        assert_eq!(edges[0].direction, Direction::E);
        assert_eq!(edges[5].direction, Direction::SSE);
        let crossings: Vec<[F; 2]> = edges
            .iter()
            .flat_map(|e| e.crossings.iter().copied())
            .collect();
        assert_eq!(crossings.len(), 2);
        assert!(
            close2(crossings[0], [231.27531285149172, -133.41539117277335]),
            "{:?}",
            crossings[0]
        );
        assert!(
            close2(crossings[1], [211.39670488496674, -163.0937032613824]),
            "{:?}",
            crossings[1]
        );
    }

    #[test]
    fn the_inner_coast_at_theta_200_reads_the_other_way_round() {
        let o = outline();
        let w = place(&o, 200.0, -4.0);
        assert!(
            close2(w.origin, [121.95619044252986, -162.28084355026252]),
            "{:?}",
            w.origin
        );
        let kinds: Vec<EdgeKind> = read_edges(&o, &world_corners(w.origin))
            .iter()
            .map(|e| e.kind)
            .collect();
        use EdgeKind::*;
        assert_eq!(kinds, vec![Shore, Sea, Sea, Sea, Shore, Land]);
        assert!(!inside(&o, w.origin));
    }

    #[test]
    fn segments_cross_where_they_should() {
        assert!(seg_cross([0.0, 0.0], [2.0, 2.0], [0.0, 2.0], [2.0, 0.0])
            .map(|p| close2(p, [1.0, 1.0]))
            .unwrap_or(false));
        assert!(seg_cross([0.0, 0.0], [1.0, 1.0], [2.0, 2.0], [3.0, 3.0]).is_none()); // parallel
                                                                                      // past the end
        assert!(seg_cross([0.0, 0.0], [1.0, 0.0], [2.0, -1.0], [2.0, 1.0]).is_none());
    }

    #[test]
    fn the_drawing_aids_have_the_pages_sizes() {
        assert_eq!(centreline().len(), 61);
        assert!(close2(centreline()[60], [R_RI, 0.0])); // the top tip, after the turn, is due east of the arc's centre
        let m = marks();
        assert_eq!(m.len(), 12);
        assert!(close(m[1].0, 30.0));
        assert!(close2(m[0].1.p, outline().pts[0]));
    }
}

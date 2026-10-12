# exp-06 port, bite two: step 2, the tectonic surface

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Step 2 of the walkthrough in Rust and on the page: the height surface `h = H(s)·F(a)`, the cross-section chart, the world's 1,296 ri coloured by height, the six edge profiles, and the `facts2` rows, identical to `banana.html`'s at the same settings.

**Architecture:** A new twin file `tectonics.rs` in `worldgen` (the section table, the Fritsch–Carlson spline, the spine height, band coordinates, the height at a plane point, and the readings the page draws: the section profile, the ri heights keyed by cube, the edge profiles, the coast slope). `facts::step2` builds the five rows as strings. The server's response grows four fields; the page gains the step-2 markup and drawing from `banana.html`, reading only response fields. Nothing in bite one is restructured.

**Tech Stack:** as bite one (Rust 1.95.0, `tiny_http`, `serde_json`, node 24).

**Spec:** `Experiments/exp-06/docs/specs/2026-09-30-exp-06-worldgen-port-design.md` §1 (shape; the twins, as Lexa confirmed on 2026-10-01), §2 (the cell rule), §3 (step 2), §9 (verification). Reference: `Experiments/exp-06/docs/reference-readings.md`, the `-- facts2` block. Bite one's plan, for the conventions: `docs/plans/2026-09-30-exp-06-port-bite-1-plan.md`.

## Global Constraints

- **Cube `(q, r, s)` everywhere in runtime code.** The world's ri are `hex::children(hex::WORLD_N)`; their centres come from `plane::centre`; every ri reading is keyed by its cube. The plane exists only inside the geometry: `tectonics::height` takes a plane point because the surface is a function of the plane, and the edge profiles sample the edge line; nothing in the server's API takes a plane point from a caller.
- **`worldgen` has no dependencies at all.** Twin files (`tectonics.rs`, `facts.rs`) are one source compiled under `worldgen::f64` and `worldgen::f32`: float literals untyped, integers via `as F`, `PI` and `rad` from `super`; `#![allow(clippy::unnecessary_cast)]` at the top of a twin file that widens `F` to `f64` for formatting.
- **Every facts row is the literal string the page prints** (`viz/banana.html` `row2(...)` calls, lines 1095–1100 at commit `c0bab59`): JS `toFixed` via `format::js_fixed`, JS `toLocaleString` via `format::js_locale_int`, JS `Math.round` via `format::js_round`. The f64 path must reproduce `docs/reference-readings.md`'s `-- facts2` block exactly; the promise is every printed digit, not bit-equality (the page's `Math.hypot` and the port's `sqrt` may differ by an ulp; spec §9).
- **Build and test from `Experiments/exp-06/`**, never from the repo root; `target/` stays inside the experiment.
- **`banana.html` is not changed in this bite.** `node viz/tools/check.js viz/banana.html` prints the same `-- facts` and `-- facts2` blocks before and after.
- **Commits:** one per task on the local branch `exp-06-worldgen` (Lexa, 2026-10-01: "leave the commits"), `git -C /home/lexa/DevProjects/_GameDev/Murabito/.claude/worktrees/exp-06-worldgen` for every git command, never push, never open a PR. Add only the files the task names; never `target/`.
- **Pinned numbers come from the page.** Every value in a test below was printed from `banana.html` at commit `c0bab59` by a probe at the defaults (θ 12, d +10) and at θ 200, d −4. If a test and the code it tests disagree, the implementer reports the disagreement; the page's formula wins and the controller fixes the plan.
- **f32 twin tests:** pin exactly under f64 and loosely under f32 by branching on the twin's `TOL` (`if TOL < 1e-6 { exact } else { loose }`), since heights near the coast cancel and lose f32 digits.

## Review Focus

1. **The join between the arc and a cap** (a point with `|ang|` just under and just over `A/2`): the height is continuous there to under 2 m (the handoff measured the jump), not a step. Pinned in Task 2.
2. **The spine height outside `s ∈ [0, 1]`** (the caps set `s` to exactly 0 or 1; a point past a cap must never extrapolate): `H(1.2) = H(1) = 900`. Pinned in Task 2.
3. **The spline outside its table** (`a` beyond ±3 at the edge of the drawing): clamped to the end values, no extrapolation. Pinned in Task 2.
4. **An edge entirely under the sea** (θ 200, d −4: the NNE edge runs −1206…−20 m): the profile has no sign change and the range prints both negatives. Pinned in Tasks 3 and 4.
5. **A world with no land ri** (d −40 out to sea): the hypsometry's land mean must not divide by zero; the page prints `0` for the mean when `landRi` is 0. Pinned in Task 2.

---

### Task 1: units, JS integer formatting, `WORLD_N`

**Files:**
- Create: `Experiments/exp-06/crates/worldgen/src/units.rs`
- Modify: `Experiments/exp-06/crates/worldgen/src/format.rs`
- Modify: `Experiments/exp-06/crates/worldgen/src/hex.rs`
- Modify: `Experiments/exp-06/crates/worldgen/src/lib.rs` (add `pub mod units;`)

**Interfaces:**
- Produces: `worldgen::units::{RI_M: f64 = 3927.0, LAYER_M: f64 = 0.15151}`; `worldgen::format::js_round(x: f64) -> f64`; `worldgen::format::js_locale_int(n: i64) -> String`; `worldgen::hex::WORLD_N: i32 = 36`.

- [ ] **Step 1: Write the failing tests**

Append to the test module in `format.rs`:

```rust
    #[test]
    fn js_round_rounds_a_half_toward_plus_infinity() {
        assert_eq!(js_round(2.5), 3.0);
        assert_eq!(js_round(-2.5), -2.0); // Math.round(-2.5) is -2
        assert_eq!(js_round(16200.4), 16200.0);
        assert!(js_round(-0.4).is_sign_negative() && js_round(-0.4) == 0.0, "Math.round(-0.4) is -0");
        assert_eq!(js_round(0.49999999999999994), 0.0);
        assert_eq!(js_round(4503599627370497.0), 4503599627370497.0); // an odd integer above 2^52 must not move
    }

    #[test]
    fn js_locale_int_groups_thousands_with_commas() {
        assert_eq!(js_locale_int(16200), "16,200");
        assert_eq!(js_locale_int(15139), "15,139");
        assert_eq!(js_locale_int(1296), "1,296");
        assert_eq!(js_locale_int(999), "999");
        assert_eq!(js_locale_int(0), "0");
        assert_eq!(js_locale_int(-1234567), "-1,234,567");
    }
```

Add a test module to `hex.rs`'s existing tests:

```rust
    #[test]
    fn the_world_is_a_packing_cell_of_36_ri() {
        assert_eq!(WORLD_N, 36);
        assert_eq!(children(WORLD_N).len(), 1296);
    }
```

Create `units.rs` with only a test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ri_is_3927_metres_and_a_layer_half_a_shaku() {
        assert_eq!(RI_M, 3927.0);
        assert_eq!(LAYER_M, 0.15151);
        assert!((RI_M / LAYER_M - 25919.08).abs() < 0.01); // about 25,919 layers to a ri
    }
}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cd /home/lexa/DevProjects/_GameDev/Murabito/.claude/worktrees/exp-06-worldgen/Experiments/exp-06 && cargo test -p worldgen`
Expected: compile errors on `js_round`, `js_locale_int`, `WORLD_N`, `units`.

- [ ] **Step 3: Write the code**

`units.rs` above the tests:

```rust
//! The units the readings are printed in. A ri is 12,960 shaku of 10/33 m; a layer is 5 sun, half a shaku.

/// Metres in a ri, flat to flat of a ri cell.
pub const RI_M: f64 = 3927.0;
/// Metres in a layer: the page's constant, 5 sun.
pub const LAYER_M: f64 = 0.15151;
```

`format.rs`, after `js_fixed`:

```rust
/// JS `Math.round`: the nearest integer, a half toward +∞ (so −2.5 → −2), as a float like JS's. Written
/// without `x + 0.5`, which rounds before the floor at 0.49999999999999994 and above 2^52.
pub fn js_round(x: f64) -> f64 {
    let f = x.floor();
    let r = if x - f >= 0.5 { f + 1.0 } else { f };
    if r == 0.0 && x < 0.0 {
        -0.0
    } else {
        r
    }
}

/// JS `Number.prototype.toLocaleString()` for an integer in the page's locale: thousands grouped by commas.
pub fn js_locale_int(n: i64) -> String {
    let digits = n.unsigned_abs().to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3 + 1);
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    if n < 0 {
        format!("-{grouped}")
    } else {
        grouped
    }
}
```

`hex.rs`, after the `Cube` impl:

```rust
/// The world is one packing cell of this many ri: 1,296 ri, 36 ri flat to flat.
pub const WORLD_N: i32 = 36;
```

`lib.rs`: add `pub mod units;` after `pub mod settings;`.

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test -p worldgen && cargo clippy -p worldgen --all-targets && cargo fmt --all -- --check` from `Experiments/exp-06`. Expected: green, no warnings.

- [ ] **Step 5: Commit**

```
exp-06 port: units, JS integer formatting, the world's packing size
```

---

### Task 2: `tectonics`, the height surface and its readings

**Files:**
- Create: `Experiments/exp-06/crates/worldgen/src/tectonics.rs`
- Modify: `Experiments/exp-06/crates/worldgen/src/lib.rs` (add `#[path = "../tectonics.rs"] pub mod tectonics;` to BOTH twins, after `plane`)

**Interfaces:**
- Consumes: `super::{rad, F, PI, TOL}`, `super::banana::{self, Edge, EdgeKind, Outline, ARC_DEG, PHI_DEG, R_RI, W_RI}`, `super::plane`, `crate::hex::{self, Cube}`.
- Produces, in each twin's `tectonics`: `SECTION: [[F; 2]; 15]`, `H_TIP: F = 900.0`, `H_APEX: F = 2500.0`; `Spline::new(&[[F; 2]]) -> Spline`, `Spline::at(&self, v: F) -> F`; `spine_height(s: F) -> F`; `Band { s, a, e: F }`, `band_coords(p: [F; 2]) -> Band`; `Tectonics::new()` (and `Default`), `Tectonics::f(&self, a: F) -> F`, `Tectonics::height(&self, p: [F; 2]) -> F`, `Tectonics::gradient(&self, p: [F; 2]) -> [F; 2]` (central differences 0.25 ri); `gradient_of(field: impl Fn([F; 2]) -> F, p: [F; 2], d: F) -> [F; 2]`; `SectionProfile { s, spine, a: F, points: Vec<[F; 2]> }` and `Tectonics::section_at(&self, origin: [F; 2]) -> SectionProfile` (301 points, `a` from −2.5 to 2.5); `RiHeight { cell: Cube, centre: [F; 2], h: F }` and `Tectonics::ri_heights(&self, origin: [F; 2]) -> Vec<RiHeight>` (1,296, in `hex::children` order); `Hypsometry { land: usize, highest: F, deepest: F, land_mean: F }` and `hypsometry(ri: &[RiHeight]) -> Hypsometry`; `EdgeProfile { direction: Direction, kind: EdgeKind, heights: Vec<F>, bottom: F, top: F }` and `Tectonics::edge_profile(&self, e: &Edge) -> EdgeProfile` (121 samples); `Tectonics::coast_slope_inland(&self, o: &Outline, theta: F) -> F`.

- [ ] **Step 1: Write the failing tests**

Create `tectonics.rs` with only:

```rust
#[cfg(test)]
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
        assert!(close(t.height(w.origin), 269.4666483278163), "{}", t.height(w.origin));
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
        assert!(band_coords(point(half - 1e-4)).e == 0.0 && band_coords(point(half + 1e-4)).e > 0.0);
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
        let one = ri.iter().find(|r| r.cell == Cube::new(5, -7, 2)).expect("the world owns (5, -7, 2)");
        assert!(close(one.centre[0], 214.77531285149172) && close(one.centre[1], -137.33153055923975), "{:?}", one.centre);
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
        let ranges: Vec<(F, F)> = edges.iter().map(|e| { let p = t.edge_profile(e); assert_eq!(p.heights.len(), 121); (p.bottom, p.top) }).collect();
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
        assert!(close(nne.bottom, -1205.9604570803763) && close(nne.top, -19.771523783059592), "{}…{}", nne.bottom, nne.top);
        assert!(nne.heights.iter().all(|h| *h < 0.0));
    }

    #[test]
    fn the_section_and_the_coast_slope_at_the_defaults() {
        let o = banana::outline();
        let w = banana::place(&o, 12.0, 10.0);
        let t = Tectonics::new();
        let sec = t.section_at(w.origin);
        assert_eq!(sec.points.len(), 301);
        assert!(close(sec.points[0][0], -2.5) && close(sec.points[0][1], -2373.6245587615936), "{:?}", sec.points[0]);
        assert!(close(sec.points[60][1], -859.0854287300953), "{:?}", sec.points[60]);
        assert!(close(sec.points[150][1], 2179.07700811221), "{:?}", sec.points[150]);
        assert!(close(sec.points[300][0], 2.5) && close(sec.points[300][1], -2945.555023371813), "{:?}", sec.points[300]);
        let g = t.gradient([221.1511697907647, -148.70604096870284]);
        assert!(close(g[0], -17.852566806267134) && close(g[1], 11.991861112759638), "{g:?}");
        assert!(close(t.coast_slope_inland(&o, 12.0), 21.50625198680486), "{}", t.coast_slope_inland(&o, 12.0));
        assert!(close(t.coast_slope_inland(&o, 200.0), 48.258966200169745), "{}", t.coast_slope_inland(&o, 200.0));
    }
}
```

- [ ] **Step 2: Wire the module and run the tests to see them fail**

In `lib.rs`, in both twins after `pub mod plane;`:

```rust
    #[path = "../tectonics.rs"]
    pub mod tectonics;
```

Run `cargo test -p worldgen`. Expected: compile errors on everything in `tectonics`.

- [ ] **Step 3: Write `tectonics.rs`**

Above the tests:

```rust
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
        let d: Vec<F> = (0..n - 1).map(|i| (y[i + 1] - y[i]) / (x[i + 1] - x[i])).collect();
        let mut m = Vec::with_capacity(n);
        m.push(d[0]);
        for i in 1..n - 1 {
            m.push(if d[i - 1] * d[i] <= 0.0 { 0.0 } else { (d[i - 1] + d[i]) / 2.0 });
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
        return Band { s: (ang + half) / (2.0 * half), a: (r - R_RI) / hw, e: 0.0 };
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
        Tectonics { section: Spline::new(&SECTION) }
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
        SectionProfile { s: b.s, spine, a: b.a, points }
    }

    /// Every ri the world owns, with its centre and height, in `hex::children` order.
    pub fn ri_heights(&self, origin: [F; 2]) -> Vec<RiHeight> {
        hex::children(hex::WORLD_N)
            .into_iter()
            .map(|cell| {
                let centre = plane::centre(cell, 1.0, origin);
                RiHeight { cell, centre, h: self.height(centre) }
            })
            .collect()
    }

    pub fn edge_profile(&self, e: &Edge) -> EdgeProfile {
        let mut heights = Vec::with_capacity(121);
        let (mut bottom, mut top) = (F::INFINITY, F::NEG_INFINITY);
        for i in 0..=120 {
            let t = i as F / 120.0;
            let p = [e.a[0] + (e.b[0] - e.a[0]) * t, e.a[1] + (e.b[1] - e.a[1]) * t];
            let h = self.height(p);
            bottom = bottom.min(h);
            top = top.max(h);
            heights.push(h);
        }
        EdgeProfile { direction: e.direction, kind: e.kind, heights, bottom, top }
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
    Hypsometry { land, highest, deepest, land_mean }
}
```

The page's `bandCoords` takes `Math.hypot(x, y)` where this takes `sqrt(x² + y²)`, and `Math.hypot(b.a, b.e)` for `rho`; the spec accepts the ulp.

- [ ] **Step 4: Run the tests to see them pass**

`cargo test -p worldgen` (both twins), `cargo clippy -p worldgen --all-targets` (no warnings), `cargo fmt --all -- --check`. If an f32 test fails on a value the `exact()` branch does not guard, report the number and which assertion; do not loosen `TOL`.

- [ ] **Step 5: Commit**

```
exp-06 port: step 2, the tectonic surface and its readings
```

---

### Task 3: `facts::step2`

**Files:**
- Modify: `Experiments/exp-06/crates/worldgen/src/facts.rs`

**Interfaces:**
- Consumes: `super::tectonics::{EdgeProfile, SectionProfile}`, `crate::format::{js_fixed, js_locale_int, js_round, FactRow}`, `crate::units::LAYER_M`.
- Produces: `facts::step2(section: &SectionProfile, origin_height: F, profiles: &[EdgeProfile], coast_slope: F) -> Vec<FactRow>`.

- [ ] **Step 1: Write the failing tests**

Append to `facts.rs`'s test module:

```rust
    fn rows2_at(theta: F, inset: F) -> Vec<FactRow> {
        let o = banana::outline();
        let w = banana::place(&o, theta, inset);
        let t = tectonics::Tectonics::new();
        let edges = banana::read_edges(&o, &banana::world_corners(w.origin));
        let profiles: Vec<_> = edges.iter().map(|e| t.edge_profile(e)).collect();
        step2(&t.section_at(w.origin), t.height(w.origin), &profiles, t.coast_slope_inland(&o, theta))
    }

    #[test]
    fn the_default_facts2_are_the_reference_readings() {
        let rows = rows2_at(12.0, 10.0);
        assert_eq!(
            printed(&rows),
            vec![
                "spine height here, H(s) | 2455 m ≈ 16,200 layers",
                "world centre: a across, s along | 0.67, 0.58",
                "height at the origin | 269 m",
                "edge ranges, m | E -85…4 · NNE 4…791 · NNW 791…2293 · W 646…2293 · SSW -31…646 · SSE -85…-31",
                "slope just inland of the coast here | 21.5 m per ri: 100 m of height is 4.6 ri of coast",
            ]
        );
        assert!(rows.iter().all(|r| !r.bad));
    }

    #[test]
    fn the_inner_coast_facts2() {
        let rows = rows2_at(200.0, -4.0);
        assert_eq!(
            printed(&rows),
            vec![
                "spine height here, H(s) | 2294 m ≈ 15,139 layers",
                "world centre: a across, s along | -1.13, 0.34",
                "height at the origin | -95 m",
                "edge ranges, m | E -20…1181 · NNE -1206…-20 · NNW -1301…-1206 · W -1299…-146 · SSW -146…848 · SSE 848…1181",
                "slope just inland of the coast here | 48.3 m per ri: 100 m of height is 2.1 ri of coast",
            ]
        );
    }
```

The test module needs `use super::super::tectonics;` beside its `banana` import.

- [ ] **Step 2: Run the tests to see them fail**

`cargo test -p worldgen facts`. Expected: `step2` not found.

- [ ] **Step 3: Write `step2`**

At the top of `facts.rs` add `#![allow(clippy::unnecessary_cast)]` (the f64 twin's `as f64` is a no-op; the f32 twin's is not) directly under the module doc comment, and the imports:

```rust
use super::tectonics::{EdgeProfile, SectionProfile};
use super::F;
use crate::format::{js_fixed, js_locale_int, js_round};
use crate::units::LAYER_M;
```

Then:

```rust
/// Step 2: the spine here, the world's place in the band, the origin's height, the edge ranges, the coast slope.
pub fn step2(section: &SectionProfile, origin_height: F, profiles: &[EdgeProfile], coast_slope: F) -> Vec<FactRow> {
    let spine = section.spine as f64;
    let layers = js_round(spine / LAYER_M) as i64;
    let ranges: Vec<String> = profiles
        .iter()
        .map(|p| format!("{} {}…{}", p.direction.name(), js_fixed(p.bottom as f64, 0), js_fixed(p.top as f64, 0)))
        .collect();
    let slope = coast_slope as f64;
    let ri_per_100_m = if slope > 0.0 { js_fixed(100.0 / slope, 1) } else { "—".to_string() };
    vec![
        FactRow::new("spine height here, H(s)", format!("{} m ≈ {} layers", js_fixed(spine, 0), js_locale_int(layers)), false),
        FactRow::new("world centre: a across, s along", format!("{}, {}", js_fixed(section.a as f64, 2), js_fixed(section.s as f64, 2)), false),
        FactRow::new("height at the origin", format!("{} m", js_fixed(origin_height as f64, 0)), false),
        FactRow::new("edge ranges, m", ranges.join(" · "), false),
        FactRow::new(
            "slope just inland of the coast here",
            format!("{} m per ri: 100 m of height is {} ri of coast", js_fixed(slope, 1), ri_per_100_m),
            false,
        ),
    ]
}
```

- [ ] **Step 4: Run the tests to see them pass**

`cargo test -p worldgen`, clippy, fmt. Under the f32 twin a printed digit may differ from the f64 string where a value sits near a rounding boundary; if one does, report which row and both strings rather than changing the expectation. (The controller then decides whether the f32 facts test is guarded by `exact()`.)

- [ ] **Step 5: Commit**

```
exp-06 port: the step-2 facts rows
```

---

### Task 4: the wire and the render

**Files:**
- Modify: `Experiments/exp-06/crates/exp06/src/wire.rs`
- Modify: `Experiments/exp-06/crates/exp06/src/render.rs`
- Modify: `Experiments/exp-06/crates/exp06/tests/server.rs`

**Interfaces:**
- Consumes: `worldgen::<twin>::tectonics::{Tectonics, hypsometry}`, `worldgen::<twin>::facts::step2`, `worldgen::<twin>::plane::unit_hex`.
- Produces, on `RenderResponse`: `banana.unit_hex: Vec<[f64; 2]>` (six points); `section: SectionOut { s, spine, a: f64, points: Vec<[f64; 2]> }`; `heights: HeightsOut { cells: Vec<RiOut { q, r, s: i32, c: [f64; 2], h: f64 }>, land: usize, highest, deepest, land_mean: f64 }`; `profiles: Vec<ProfileOut { name, kind: String, heights: Vec<f64>, bottom, top: f64 }>`; `facts2: Vec<FactOut>`. Existing fields unchanged. The macro is renamed `render_with!`.

- [ ] **Step 1: Write the failing test**

Append to `tests/server.rs`:

```rust
fn facts2(json: &str) -> Vec<String> {
    let v: serde_json::Value = serde_json::from_str(json).expect("json");
    v["facts2"].as_array().expect("facts2").iter().map(|f| format!("{} | {}", f["key"].as_str().unwrap(), f["value"].as_str().unwrap())).collect()
}

#[test]
fn step_2_at_the_defaults_prints_the_reference_facts2() {
    let addr = start();
    let (status, body) = call(addr, "POST", "/render", "{}");
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        facts2(&body),
        [
            "spine height here, H(s) | 2455 m ≈ 16,200 layers",
            "world centre: a across, s along | 0.67, 0.58",
            "height at the origin | 269 m",
            "edge ranges, m | E -85…4 · NNE 4…791 · NNW 791…2293 · W 646…2293 · SSW -31…646 · SSE -85…-31",
            "slope just inland of the coast here | 21.5 m per ri: 100 m of height is 4.6 ri of coast",
        ]
    );
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["section"]["points"].as_array().unwrap().len(), 301);
    assert_eq!(v["heights"]["cells"].as_array().unwrap().len(), 1296);
    assert_eq!(v["heights"]["land"], 1069);
    assert_eq!(v["heights"]["cells"][0]["q"], -24);
    assert_eq!(v["heights"]["cells"][0]["s"], 12);
    assert_eq!(v["profiles"].as_array().unwrap().len(), 6);
    assert_eq!(v["profiles"][1]["name"], "NNE");
    assert_eq!(v["profiles"][1]["heights"].as_array().unwrap().len(), 121);
    assert_eq!(v["banana"]["unit_hex"].as_array().unwrap().len(), 6);
}

#[test]
fn step_2_on_the_inner_coast_and_in_f32() {
    let addr = start();
    let (_, body) = call(addr, "POST", "/render", r#"{"theta":200,"inset":-4}"#);
    assert_eq!(facts2(&body)[3], "edge ranges, m | E -20…1181 · NNE -1206…-20 · NNW -1301…-1206 · W -1299…-146 · SSW -146…848 · SSE 848…1181");
    let (_, body) = call(addr, "POST", "/render", r#"{"path":"f32"}"#);
    assert_eq!(facts2(&body)[2], "height at the origin | 269 m");
    assert_eq!(facts2(&body)[0], "spine height here, H(s) | 2455 m ≈ 16,200 layers");
}
```

- [ ] **Step 2: Run the test to see it fail**

`cargo test -p exp06`. Expected: the new tests fail on `facts2` being null.

- [ ] **Step 3: Extend the wire and the render**

`wire.rs`: add to `BananaOut` the field `pub unit_hex: Vec<[f64; 2]>`; add

```rust
#[derive(Debug, Serialize)]
pub struct SectionOut {
    pub s: f64,
    pub spine: f64,
    pub a: f64,
    pub points: Vec<[f64; 2]>,
}

#[derive(Debug, Serialize)]
pub struct RiOut {
    pub q: i32,
    pub r: i32,
    pub s: i32,
    pub c: [f64; 2],
    pub h: f64,
}

#[derive(Debug, Serialize)]
pub struct HeightsOut {
    pub cells: Vec<RiOut>,
    pub land: usize,
    pub highest: f64,
    pub deepest: f64,
    pub land_mean: f64,
}

#[derive(Debug, Serialize)]
pub struct ProfileOut {
    pub name: String,
    pub kind: String,
    pub heights: Vec<f64>,
    pub bottom: f64,
    pub top: f64,
}
```

and to `RenderResponse`, after `facts`: `pub section: SectionOut, pub heights: HeightsOut, pub profiles: Vec<ProfileOut>, pub facts2: Vec<FactOut>`.

`render.rs`: rename the macro `render_with!`; inside it, after the step-1 lines and before `ms` is taken:

```rust
        use worldgen::$twin::{plane, tectonics};
        let tect = tectonics::Tectonics::new();
        let section = tect.section_at(placed.origin);
        let ri = tect.ri_heights(placed.origin);
        let hyps = tectonics::hypsometry(&ri);
        let profiles: Vec<_> = edges.iter().map(|e| tect.edge_profile(e)).collect();
        let facts2 = facts::step2(&section, tect.height(placed.origin), &profiles, tect.coast_slope_inland(&o, settings.theta as F));
```

and in the response, `banana.unit_hex: plane::unit_hex().iter().copied().map(wide).collect()` and

```rust
            section: SectionOut {
                s: section.s as f64,
                spine: section.spine as f64,
                a: section.a as f64,
                points: section.points.iter().copied().map(wide).collect(),
            },
            heights: HeightsOut {
                cells: ri.iter().map(|r| RiOut { q: r.cell.q, r: r.cell.r, s: r.cell.s, c: wide(r.centre), h: r.h as f64 }).collect(),
                land: hyps.land,
                highest: hyps.highest as f64,
                deepest: hyps.deepest as f64,
                land_mean: hyps.land_mean as f64,
            },
            profiles: profiles
                .iter()
                .map(|p| ProfileOut {
                    name: p.direction.name().to_string(),
                    kind: p.kind.name().to_string(),
                    heights: p.heights.iter().map(|h| *h as f64).collect(),
                    bottom: p.bottom as f64,
                    top: p.top as f64,
                })
                .collect(),
            facts2: facts2.into_iter().map(|f| FactOut { key: f.key, value: f.value, bad: f.bad }).collect(),
```

Import the new wire types. The `ms` timing now covers both steps.

- [ ] **Step 4: Run the tests to see them pass**

`cargo test --workspace`, `cargo clippy --workspace --all-targets` (no warnings), `cargo fmt --all -- --check`.

- [ ] **Step 5: Commit**

```
exp-06 port: /render carries step 2
```

---

### Task 5: the page, the harness, the docs

**Files:**
- Modify: `Experiments/exp-06/viz/exp-06.html`
- Modify: `Experiments/exp-06/viz/tools/check-port.js`
- Modify: `Experiments/exp-06/README.md`
- Modify: `Experiments/exp-06/docs/specs/2026-09-30-exp-06-worldgen-port-design.md` (§2, one sentence)
- Modify: `Experiments/exp-06/docs/HANDOFF.md` (one paragraph)

**Interfaces:**
- Consumes: the response fields from Task 4.
- Produces: the page's step 2; `check-port.js` printing `-- facts` and `-- facts2` blocks; the README's diff over both.

- [ ] **Step 1: Add the step-2 markup**

In `viz/exp-06.html`, directly after the step-1 `<p class="muted">The edge reading: …</p>` and before `</main>`, insert `banana.html`'s step-2 markup (its lines 68–87 at `c0bab59`), verbatim:

```html
  <h1 id="step2">Step 2: tectonics, as two curves</h1>
  <p>Height anywhere, above or below the sea, is <code>h = H(s) · F(a)</code>. <code>F(a)</code> is one cross-section, the same everywhere: <code>a</code> counts half-widths from the centreline, the coasts at <code>±1</code>, and the table runs from a deep basin three half-widths inside to a trench three half-widths out. From the outer coast inward: a wide plain, the spine a little past the middle, a steep drop to the inner coast; from the outer coast outward: a wide shallow shelf, then the slope to the trench. It is 0 at both coasts with a continuous slope through them. <code>H(s)</code> is the spine's height along the centreline, low at the tips and highest round the bend, and the sea deepens with it. Round the caps the cross-section goes polar, blending the two sides by angle. The sliders are the same <code>θ</code> and <code>d</code> as above, wherever they appear.</p>
  <div class="controls controls-shared"></div>

  <div class="pair">
    <div>
      <div class="panel-title">the band cut across at the world's θ · metres · the world's span marked</div>
      <svg id="section" viewBox="0 0 560 300" role="img" aria-label="The height cross-section of the band at the world's position, with the extent of the world marked."><g id="sectionScene" fill="none" stroke="currentColor"></g></svg>
    </div>
    <div>
      <div class="panel-title">the world, ri by ri, coloured by height · shore in blue</div>
      <svg id="heights" viewBox="0 0 560 560" role="img" aria-label="The world hexagon filled with its 1,296 ri cells coloured by height, with the shoreline through it."><g id="heightsScene" fill="none" stroke="currentColor"></g></svg>
    </div>
  </div>

  <div class="panel-title">the six edge profiles · anticlockwise from each edge's first corner · metres · shore crossings at 0</div>
  <div class="profiles" id="profiles"></div>
  <div class="facts" id="facts2"></div>
  <p class="muted">The cross-section is a smooth spline through a short table of control points, so it is a thing to edit rather than a formula to tune; the spine curve is a raised sine along the centreline. Both are smooth, so the erosion filter of a later step can take a gradient off them anywhere. In layers (5 sun each) the numbers are about 6.6 per metre.</p>
```

Add to the page's `<style>` the two rules banana has for profiles: `.profiles { display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 10px; } .profiles > div { display: grid; gap: 2px; min-width: 0; } .profiles .panel-title { font-size: 0.75rem; }`.

- [ ] **Step 2: Add the step-2 drawing**

In the script, after the `const wide = …, near = …, facts = …;` line add

```js
  const section = document.getElementById("sectionScene"), heights = document.getElementById("heightsScene"), profiles = document.getElementById("profiles"), facts2 = document.getElementById("facts2");
  const heightColour = h => {                                 // hypsometric: trench to snow, by metres
    const stops = h <= 0
      ? [[-6000, [15, 30, 70]], [-2500, [30, 60, 120]], [-800, [70, 110, 165]], [-200, [120, 160, 200]], [0, [175, 205, 220]]]
      : [[0, [190, 205, 150]], [300, [150, 180, 110]], [900, [170, 150, 100]], [1600, [140, 115, 85]], [2200, [200, 195, 190]], [2800, [245, 245, 245]]];
    let i = 0; while (i < stops.length - 2 && h > stops[i + 1][0]) i++;
    const [h0, c0] = stops[i], [h1, c1] = stops[i + 1], t = Math.max(0, Math.min(1, (h - h0) / (h1 - h0)));
    return "rgb(" + c0.map((v, k) => Math.round(v + (c1[k] - v) * t)).join(",") + ")";
  };
```

and at the end of `draw(out)`, after the step-1 facts loop, the step-2 drawing (banana's, reading the response):

```js
    // ---- Step 2 drawings.
    section.replaceChildren();
    const sec = out.section;
    const A_LO = -2.5, A_HI = 2.5, M_LO = -3500, M_HI = 3000;
    const SX = x => 50 + ((x - A_LO) / (A_HI - A_LO)) * 480, SY = m => 30 + ((M_HI - m) / (M_HI - M_LO)) * 240;
    section.appendChild(el("rect", { x: 50, y: 30, width: 480, height: 240, fill: "var(--cell)", stroke: "none" }));
    for (let m = -3000; m <= 3000; m += 1000) { section.appendChild(el("line", { x1: 50, y1: SY(m), x2: 530, y2: SY(m), stroke: m === 0 ? "var(--shore)" : "var(--line)" })); section.appendChild(el("text", { x: 46, y: SY(m) + 4, "text-anchor": "end", fill: "var(--muted)", ...mono, "font-size": 10 }, m + " m")); }
    { const ps = sec.points.map(([a, h]) => SX(a) + "," + SY(h));
      section.appendChild(el("polygon", { points: SX(A_LO) + "," + SY(M_LO) + " " + ps.join(" ") + " " + SX(A_HI) + "," + SY(M_LO), fill: "var(--land)", stroke: "none" }));
      section.appendChild(el("polygon", { points: SX(A_LO) + "," + SY(0) + " " + SX(A_HI) + "," + SY(0) + " " + SX(A_HI) + "," + SY(M_LO) + " " + SX(A_LO) + "," + SY(M_LO), fill: "var(--sea)", "fill-opacity": 0.55, stroke: "none" }));
      section.appendChild(el("polyline", { points: ps.join(" "), stroke: "var(--mountain)", "stroke-width": 2 })); }
    { const aC = sec.a, halfSpan = 18 / (out.banana.w / 2);
      section.appendChild(el("rect", { x: SX(aC - halfSpan), y: 30, width: SX(aC + halfSpan) - SX(aC - halfSpan), height: 240, fill: "var(--world)", "fill-opacity": 0.12, stroke: "var(--world)", "stroke-dasharray": "4 3" }));
      section.appendChild(el("text", { x: SX(aC), y: 24, "text-anchor": "middle", fill: "var(--world)", ...mono, "font-size": 10 }, "the world, 36 ri")); }
    for (const [a, name] of [[-2, "basin"], [-1, "inner coast"], [0, "spine"], [1, "outer coast"], [2, "trench"]]) section.appendChild(el("text", { x: SX(a), y: 288, "text-anchor": "middle", fill: "var(--muted)", ...mono, "font-size": 10 }, name + " · " + (a * out.banana.w / 2 >= 0 ? "+" : "") + (a * out.banana.w / 2).toFixed(0) + " ri"));
    section.appendChild(el("text", { x: 530, y: 24, "text-anchor": "end", fill: "var(--muted)", ...mono, "font-size": 10 }, "s = " + sec.s.toFixed(2) + " · H(s) = " + sec.spine.toFixed(0) + " m"));

    heights.replaceChildren();
    const HV = view(origin[0], origin[1], 520 / (WORLD * 1.15)), hex = out.banana.unit_hex;
    heights.appendChild(el("rect", { x: 0, y: 0, width: 560, height: 560, fill: "var(--bg)", stroke: "none" }));
    for (const cell of out.heights.cells) heights.appendChild(el("polygon", { points: pts(hex.map(k => [cell.c[0] + k[0], cell.c[1] + k[1]]), HV), fill: heightColour(cell.h), stroke: "none" }));
    heights.appendChild(el("polyline", { points: pts(o, HV), stroke: "var(--shore)", "stroke-width": 2 }));
    heights.appendChild(el("polygon", { points: pts(corners, HV), fill: "none", stroke: "var(--world)", "stroke-width": 1.5 }));
    heights.appendChild(el("text", { x: 10, y: 548, fill: "var(--muted)", ...mono }, out.heights.land + " ri on land of 1,296 · highest " + out.heights.highest.toFixed(0) + " m, deepest " + out.heights.deepest.toFixed(0) + " m · land mean " + out.heights.land_mean.toFixed(0) + " m"));

    profiles.replaceChildren();
    for (const e of out.profiles) {
      const wrap = document.createElement("div");
      const title = document.createElement("div"); title.className = "panel-title"; title.textContent = e.name + " · " + e.kind; wrap.appendChild(title);
      const svg = el("svg", { viewBox: "0 0 200 120", role: "img", "aria-label": "height along the " + e.name + " edge" });
      const g = el("g", { fill: "none", stroke: "currentColor" }); svg.appendChild(g); wrap.appendChild(svg); profiles.appendChild(wrap);
      const PX = t => 28 + t * 166, PY = m => 10 + ((3000 - m) / 4000) * 90;    // −1,000 to 3,000 m
      g.appendChild(el("rect", { x: 28, y: 10, width: 166, height: 90, fill: "var(--cell)", stroke: "none" }));
      for (let m = -1000; m <= 3000; m += 1000) { g.appendChild(el("line", { x1: 28, y1: PY(m), x2: 194, y2: PY(m), stroke: m === 0 ? "var(--shore)" : "var(--line)" })); g.appendChild(el("text", { x: 25, y: PY(m) + 3, "text-anchor": "end", fill: "var(--muted)", ...mono, "font-size": 8 }, m / 1000 + "k")); }
      const ps = []; let prevH = null;
      e.heights.forEach((h, i) => {
        const t = i / (e.heights.length - 1);
        ps.push(PX(t) + "," + PY(Math.max(-1000, h)));
        if (prevH !== null && (h > 0) !== (prevH > 0)) g.appendChild(el("circle", { cx: PX(t), cy: PY(0), r: 3, fill: "var(--shore)", stroke: "none" }));
        prevH = h;
      });
      g.appendChild(el("polygon", { points: PX(0) + "," + PY(-1000) + " " + ps.join(" ") + " " + PX(1) + "," + PY(-1000), fill: "var(--land)", stroke: "none" }));
      g.appendChild(el("polygon", { points: PX(0) + "," + PY(0) + " " + PX(1) + "," + PY(0) + " " + PX(1) + "," + PY(-1000) + " " + PX(0) + "," + PY(-1000), fill: "var(--sea)", "fill-opacity": 0.55, stroke: "none" }));
      g.appendChild(el("polyline", { points: ps.join(" "), stroke: e.kind === "sea" ? "var(--shore)" : "var(--mountain)", "stroke-width": 1.5 }));
      g.appendChild(el("text", { x: 194, y: 112, "text-anchor": "end", fill: "var(--muted)", ...mono, "font-size": 8 }, "0 → 36 ri · " + e.bottom.toFixed(0) + " to " + e.top.toFixed(0) + " m"));
    }
    facts2.replaceChildren();
    for (const f of out.facts2) { const d = document.createElement("div"); const a = document.createElement("span"); a.textContent = f.key; const b = document.createElement("span"); b.textContent = f.value; if (f.bad) b.className = "bad"; d.append(a, b); facts2.appendChild(d); }
```

(The step-2 sliders bind through the existing `bindSliders(".controls-shared", SLIDERS)`, which binds every container.)

- [ ] **Step 3: The harness and the README**

`viz/tools/check-port.js`: after the `-- facts` loop add

```js
  console.log("-- facts2");
  for (const f of out.facts2) console.log(f.key + " | " + f.value + (process.env.FLAGS && f.bad ? " [bad]" : ""));
```

`README.md`: the diff commands become

```
    node viz/tools/check.js viz/banana.html | sed -n '/^-- facts$/,/^-- facts3$/p' | sed '$d' > target/banana-facts.txt
    node viz/tools/check-port.js | sed -n '/^-- facts$/,$p' > target/port-facts.txt
    diff target/banana-facts.txt target/port-facts.txt && echo SAME
```

and the sentence introducing them says the diff covers steps 1 and 2.

- [ ] **Step 4: Verify**

`cargo test --workspace`, clippy, fmt. Start the server yourself in the background, run the README's diff (expect `SAME`), then the same at `theta=200 inset=-4` on both scripts (`node viz/tools/check.js viz/banana.html theta=200 inset=-4 | sed -n '/^-- facts$/,/^-- facts3$/p' | sed '$d'` against `node viz/tools/check-port.js theta=200 inset=-4 | sed -n '/^-- facts$/,$p'`; expect no difference), and `node viz/tools/check-port.js path=f32 | sed -n '/^-- facts2$/,$p'` (expect the five rows as the reference). Confirm the page parses (`node -e` with `new Function` over its script blocks). Stop the server by its PID. `node viz/tools/check.js viz/banana.html; echo exit=$?` still exits 0.

- [ ] **Step 5: The docs**

Spec §2, the cell-rule bullet: replace the sentence beginning `**The one named exception** is \`height_at_plane_point\`` with: `**The plane functions that remain** are the surface's own: \`tectonics::height\` takes a plane point because the surface is a function of the plane, and the page's edge profiles sample along an edge line; both live inside \`worldgen\`, and nothing in the server's API takes a plane point from a caller.`

HANDOFF, directly under the "**Bite one is built**" paragraph, add:

**Bite two is built (2026-10-01):** step 2 in `worldgen::<twin>::tectonics` (the section table, the Fritsch–Carlson spline, `spine_height`, `band_coords`, `height`, the gradient, and the readings: the section profile, the ri heights keyed by cube, the hypsometry, the edge profiles, the coast slope), `facts::step2`, and the page's section chart, height map and edge profiles drawn from the response. The facts diff over steps 1 and 2 is `SAME` at the defaults and at θ 200, d −4. `format` gained `js_round` and `js_locale_int`; `units` holds `RI_M` and `LAYER_M`; `hex::WORLD_N` is 36. Lexa ruled on 2026-10-01: keep the twins (spec §1.1 updated), leave the per-task commits. Next: bite 3, step 3 climate (`facts3`, the four maps), then bite 4, the hex noise and the wgpu pipeline.

- [ ] **Step 6: Commit**

```
exp-06 port: step 2 on the page, the harness over both steps, the docs
```

---

## Self-review

**Spec coverage for bite two** (spec §9 bite 2: "Step 2 tectonics: the section chart, the hypsometric, the edge profiles, facts2"): Task 2 the surface and every reading; Task 3 the rows; Task 4 the wire; Task 5 the drawings and the diff over both steps. §3's formulas: `H(s)`, `F` through the table with the raised plain, the caps' polar blend, band coordinates with the unrotation by `−φ`, all in Task 2 with the page's numbers pinned. §2's cell rule: ri readings keyed by cube from `hex::children(WORLD_N)`, centres by `plane::centre`; the spec sentence about the exception is corrected in Task 5.

**Placeholders:** none. Every pinned number is from the probe of `banana.html` at `c0bab59` (the facts2 rows match `reference-readings.md`).

**Type consistency:** `Tectonics::new()`, `.f`, `.height`, `.gradient`, `.section_at`, `.ri_heights`, `.edge_profile`, `.coast_slope_inland` and the free `hypsometry`, `spine_height`, `band_coords`, `gradient_of` are used by Tasks 3 and 4 under those names; `SectionProfile { s, spine, a, points }`, `RiHeight { cell, centre, h }`, `Hypsometry { land, highest, deepest, land_mean }`, `EdgeProfile { direction, kind, heights, bottom, top }` match the wire mapping; `plane::unit_hex()` exists from bite one; `js_round` returns `f64` and is cast to `i64` for `js_locale_int`.

**Review Focus:** item 1 in Task 2 (`the_height_is_continuous_where_the_arc_meets_a_cap`), item 2 (`the_spine_is_900…`), item 3 (`the_spline_passes_through…clamps`), item 4 in Tasks 2 and 3 (`an_edge_entirely_under_the_sea…`, the θ 200 rows), item 5 in Task 2 (`a_world_entirely_at_sea…`).

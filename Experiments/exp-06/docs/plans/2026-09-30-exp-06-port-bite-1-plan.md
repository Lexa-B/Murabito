# exp-06 port, bite one: the workspace, `worldgen::hex`, the server, step 1 on the page

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A Rust workspace at `Experiments/exp-06/` whose `exp06` server serves `viz/exp-06.html` showing step 1 (the world on the banana) computed by `worldgen`, beside `banana.html`, with the same settings in both pages' URL query and the step-1 facts rows identical.

**Architecture:** `worldgen` is an engine-free crate with no dependencies; its integer modules (`hex`, `settings`, `format`) are shared and its numeric modules (`plane`, `banana`, `facts`) are one source compiled twice, as `worldgen::f64` (the reference) and `worldgen::f32` (the shader's twin), through `#[path]` modules and a `type F` alias. `exp06` is a `tiny_http` server that serves the page from disk and answers `POST /render` with JSON the page draws. The page is `banana.html`'s step-1 markup and drawing code with the computing replaced by one fetch.

**Tech Stack:** Rust 1.95.0, `tiny_http` 0.12, `serde` + `serde_json` (in `exp06` only), node 24 for the harness.

**Spec:** `Experiments/exp-06/docs/specs/2026-09-30-exp-06-worldgen-port-design.md` (§1 shape, §2 coordinates and the cell rule, §3 step 1, §9 verification). Reference readings: `Experiments/exp-06/docs/reference-readings.md`, the `-- facts` block.

## Global Constraints

- **Cube `(q, r, s)` everywhere in runtime code**, calculations and keys included, no exceptions. No axial pairs, no constructor from a pair. The plane `(east, north)` exists only inside the geometry, and the API names cells (spec §2). Bite one has one named plane probe only: `banana::inside` and the outline are geometry, not cells.
- **`worldgen` has no dependencies at all**, not even dev-dependencies.
- **Build and test from `Experiments/exp-06/`** (`cd /home/lexa/DevProjects/_GameDev/Murabito/.claude/worktrees/exp-06-worldgen/Experiments/exp-06 && cargo test`), never from the repo root. `target/` stays inside the experiment; never under `/tmp`.
- **Toolchain pinned to `1.95.0`** in the experiment's `rust-toolchain.toml`, the root project's pin.
- **`banana.html` is the reference and is not changed** except by Task 7 (the URL query). `node viz/tools/check.js viz/banana.html` must print the same `-- facts` block before and after.
- **Every facts row is the literal string the page prints**: same key, same value, same double space before a parenthesis, same `bad` flag.
- **Do not commit.** Lexa commits on her word ("commit it"); implementers leave the work in the tree, staged or not. Every git command that is run takes `git -C /home/lexa/DevProjects/_GameDev/Murabito/.claude/worktrees/exp-06-worldgen`. Never push, never open a PR.
- **Steps are actions with results.** Constants Lexa fixed (`R_RI`, `W_RI`, `ARC_DEG`, `WORLD`) are constants, not settings.
- The numeric twin files must compile under both `F = f64` and `F = f32`: float literals stay untyped (`2.0`, not `2.0_f64`), integer-to-float goes through `as F`, and `PI`, `SQRT_3`, `rad` come from `super`.

## Review Focus

1. **θ at the slider's ends, 0 and 359.5:** `at` returns the outer apex for 0 and a point just before it for 359.5, and never indexes past the polyline. Pinned in Task 3.
2. **The inset far out to sea (θ 12, d −25):** all six edges read sea, "no more than four of either" says `broken` and is flagged bad, "shore crossings" says `0` and is not flagged. Pinned in Task 4.
3. **A malformed or empty `/render` body:** the server answers 400 with the parser's message and keeps serving; it never panics on input. Pinned in Task 5.
4. **A compute path that does not exist yet (`"gpu"`):** the f64 path answers and the response says `"path": "f64"`. Pinned in Task 5.
5. **A URL query with some keys missing, unknown keys present, and `windowQRS=q,r,s`:** missing keys keep the defaults, unknown keys are ignored, the window parses to a triple, and the node harness, which has no `location`, renders `banana.html` unchanged. Pinned in Task 7 (harness) and checked by Lexa in a browser.

---

### Task 1: the workspace and `worldgen::hex`

**Files:**
- Create: `Experiments/exp-06/Cargo.toml`
- Create: `Experiments/exp-06/rust-toolchain.toml`
- Create: `Experiments/exp-06/.gitignore`
- Create: `Experiments/exp-06/crates/worldgen/Cargo.toml`
- Create: `Experiments/exp-06/crates/worldgen/src/lib.rs`
- Create: `Experiments/exp-06/crates/worldgen/src/hex.rs`

**Interfaces:**
- Produces: `worldgen::hex::Cube { q, r, s: i32 }` with `Cube::new(q, r, s)` (panics off the plane), `Cube::ORIGIN`, `add`, `scale`, `neighbour`, `neighbours`, `d2() -> i64`; `worldgen::hex::Direction { E, NNE, NNW, W, SSW, SSE }` with `Direction::ALL`, `step() -> Cube`, `name() -> &'static str`, `index() -> usize`; `worldgen::hex::owner(c: Cube, n: i32) -> Cube`; `worldgen::hex::children(n: i32) -> Vec<Cube>`.

- [ ] **Step 1: Create the workspace files**

`Experiments/exp-06/Cargo.toml`:

```toml
[workspace]
resolver = "2"
members = ["crates/worldgen", "crates/exp06"]

[workspace.package]
edition = "2021"
version = "0.1.0"
license = "MIT"
publish = false

# The experiments' profile, as exp-05 has it: our code debuggable, dependencies fast.
[profile.dev]
opt-level = 1

[profile.dev.package."*"]
opt-level = 3
```

`Experiments/exp-06/rust-toolchain.toml`:

```toml
[toolchain]
channel = "1.95.0"
```

`Experiments/exp-06/.gitignore`:

```
target/
```

`Experiments/exp-06/crates/worldgen/Cargo.toml`:

```toml
[package]
name = "worldgen"
edition.workspace = true
version.workspace = true
license.workspace = true
publish.workspace = true

# Engine-free and dependency-free on purpose: this is the crate the game will depend on.
[dependencies]
```

`Experiments/exp-06/crates/worldgen/src/lib.rs` (the numeric twins arrive in Task 2; for now only the shared module):

```rust
#![forbid(unsafe_code)]
//! exp-06 worldgen: the Murabito world as pure functions of seed and position, engine-free.
//!
//! Cube coordinates `(q, r, s)` everywhere. The numeric modules are one source compiled twice
//! (Task 2): `worldgen::f64` is the reference the page's readings are checked against, and
//! `worldgen::f32` mirrors the shader. See `Experiments/exp-06/README.md`.

pub mod hex;
```

The `exp06` member does not exist until Task 5; until then `cargo` complains about the missing member. So for Tasks 1 to 4, list only `"crates/worldgen"` in `members`, and Task 5 adds `"crates/exp06"`.

- [ ] **Step 2: Write the failing tests for `hex`**

Create `Experiments/exp-06/crates/worldgen/src/hex.rs` with only the test module for now:

```rust
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
        assert_eq!(Direction::ALL.map(|d| d.name()), ["E", "NNE", "NNW", "W", "SSW", "SSE"]);
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
}
```

Every pinned value in this test module was printed from the page's own `owner()` run under node (1296 / 100 / 36 children, first child `(−24, 12, 12)`, the four ties and the two ordinary cells), so a failure is the port's, not the pin's.

- [ ] **Step 3: Run the tests to see them fail**

Run: `cd /home/lexa/DevProjects/_GameDev/Murabito/.claude/worktrees/exp-06-worldgen/Experiments/exp-06 && cargo test -p worldgen`
Expected: compile errors, `Cube`, `Direction`, `owner`, `children` not found.

- [ ] **Step 4: Write `hex.rs`**

Put this above the test module in `hex.rs`:

```rust
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
        Cube { q: self.q + o.q, r: self.r + o.r, s: self.s + o.s }
    }

    pub const fn scale(self, n: i32) -> Cube {
        Cube { q: self.q * n, r: self.r * n, s: self.s * n }
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
```

- [ ] **Step 5: Run the tests to see them pass**

Run: `cd /home/lexa/DevProjects/_GameDev/Murabito/.claude/worktrees/exp-06-worldgen/Experiments/exp-06 && cargo test -p worldgen`
Expected: all `hex::tests` pass (see Step 2 about the one pinned first child). Also run `cargo clippy -p worldgen --all-targets` and `cargo fmt --all -- --check`; both clean.

- [ ] **Step 6: Leave the work in the tree**

No commit (Global Constraints). Report the test output.

---

### Task 2: the float twins and `plane`

**Files:**
- Modify: `Experiments/exp-06/crates/worldgen/src/lib.rs`
- Create: `Experiments/exp-06/crates/worldgen/src/plane.rs`

**Interfaces:**
- Consumes: `worldgen::hex::Cube`.
- Produces: `worldgen::f64` and `worldgen::f32`, each with `pub type F`, `pub const PI: F`, `pub const SQRT_3: F`, `pub const TOL: F`, `pub fn rad(deg: F) -> F`, and the modules `plane`, `banana` (Task 3), `facts` (Task 4). `plane::ROW: F` (√3/2), `plane::centre(c: Cube, size: F, origin: [F; 2]) -> [F; 2]`, `plane::unit_hex() -> [[F; 2]; 6]`.

- [ ] **Step 1: Write the failing tests**

Create `Experiments/exp-06/crates/worldgen/src/plane.rs` with only the test module:

```rust
#[cfg(test)]
mod tests {
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
        // +r is south: (0, 1, −1), the SSE neighbour, sits a row below and half a cell east, as on the page
        // (riCentre: x = (q − s)/2 = 0.5, y = −r·ROW).
        let c = centre(Cube::new(0, 1, -1), 1.0, [0.0, 0.0]);
        assert!(close(c[0], 0.5) && close(c[1], -ROW), "{c:?}");
    }

    #[test]
    fn a_cho_is_a_thirty_sixth_of_a_ri() {
        // the page's chō centre: rc + [(q − s)/72, −r·ROW/36]
        let c = centre(Cube::new(3, -1, -2), 1.0 / 36.0, [2.0, 2.0]);
        assert!(close(c[0], 2.0 + 5.0 / 72.0) && close(c[1], 2.0 + ROW / 36.0), "{c:?}");
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
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cd /home/lexa/DevProjects/_GameDev/Murabito/.claude/worktrees/exp-06-worldgen/Experiments/exp-06 && cargo test -p worldgen`
Expected: `plane.rs` is not yet a module of anything, so nothing runs; after Step 3's `lib.rs` change it fails to compile on `centre`, `ROW`, `unit_hex`.

- [ ] **Step 3: Add the twins to `lib.rs` and write `plane.rs`**

Replace `lib.rs` with:

```rust
#![forbid(unsafe_code)]
//! exp-06 worldgen: the Murabito world as pure functions of seed and position, engine-free.
//!
//! Cube coordinates `(q, r, s)` everywhere (`hex`). The numeric modules are one source compiled twice:
//! `worldgen::f64` is the reference the page's readings are checked against, and `worldgen::f32`
//! mirrors the shader, same source, same order of operations. Each twin fixes `F`, the constants
//! and `rad`, and includes the numeric files by `#[path]`. See `Experiments/exp-06/README.md`.

pub mod hex;

/// The reference path: every reading in `docs/reference-readings.md` is checked against this twin.
pub mod f64 {
    pub type F = core::primitive::f64;
    pub const PI: F = core::f64::consts::PI;
    pub const SQRT_3: F = 1.7320508075688772;
    /// Relative tolerance the twin's own tests allow against the page's numbers.
    pub const TOL: F = 1e-9;
    /// Degrees to radians the way the page does it, `d · π / 180`, so the last bit agrees.
    pub fn rad(deg: F) -> F {
        deg * PI / 180.0
    }
    #[path = "../plane.rs"]
    pub mod plane;
}

/// The shader's twin: the same source in f32, in the order of operations the WGSL will use.
pub mod f32 {
    pub type F = core::primitive::f32;
    pub const PI: F = core::f32::consts::PI;
    #[allow(clippy::excessive_precision)]
    pub const SQRT_3: F = 1.7320508075688772;
    pub const TOL: F = 1e-3;
    pub fn rad(deg: F) -> F {
        deg * PI / 180.0
    }
    #[path = "../plane.rs"]
    pub mod plane;
}
```

(`#[path]` inside an inline module of `lib.rs` resolves relative to `src/f64/` and `src/f32/`, so `../plane.rs` is `src/plane.rs` for both.)

Put this above the tests in `plane.rs`:

```rust
//! The plane, `(east, north)` in ri, where a cell's centre is derived from its cube coordinate.
//! Pointy-top; `+r` is south. Only geometry lives here; the API names cells (spec §2).

use super::{F, SQRT_3, TOL};
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
```

`TOL` is imported for the tests; if clippy or rustc warns it is unused outside tests, move the import into the test module.

- [ ] **Step 4: Run the tests to see them pass**

Run: `cd /home/lexa/DevProjects/_GameDev/Murabito/.claude/worktrees/exp-06-worldgen/Experiments/exp-06 && cargo test -p worldgen`
Expected: `f64::plane::tests::*` and `f32::plane::tests::*` both pass (eight tests). `cargo clippy -p worldgen --all-targets` clean.

- [ ] **Step 5: Leave the work in the tree**

No commit. Report.

---

### Task 3: `banana`, step 1

**Files:**
- Create: `Experiments/exp-06/crates/worldgen/src/banana.rs`
- Modify: `Experiments/exp-06/crates/worldgen/src/lib.rs` (add `#[path = "../banana.rs"] pub mod banana;` to both twins, after `plane`)

**Interfaces:**
- Consumes: `super::{F, PI, TOL, rad}`, `crate::hex::Direction`.
- Produces, in each twin's `banana`: constants `WORLD: F = 36.0`, `R_RI: F = 237.0`, `W_RI: F = 60.0`, `ARC_DEG: F = 80.0`, `PHI_DEG: F = -40.0`; `Outline { pts: Vec<[F; 2]>, cum: Vec<F>, length: F }`, `outline() -> Outline`; `OutlinePoint { p: [F; 2], normal: [F; 2] }`, `at(o: &Outline, theta: F) -> OutlinePoint`; `inside(o: &Outline, p: [F; 2]) -> bool`; `seg_cross(a, b, c, d: [F; 2]) -> Option<[F; 2]>`; `Placement { theta, inset: F, here: OutlinePoint, origin: [F; 2], tangent: [F; 2] }`, `place(o: &Outline, theta: F, inset: F) -> Placement`; `world_corners(origin: [F; 2]) -> [[F; 2]; 6]`; `EdgeKind { Sea, Land, Shore }` with `name() -> &'static str` (`"sea"`, `"land"`, `"shore"`); `Edge { direction: Direction, a: [F; 2], b: [F; 2], crossings: Vec<[F; 2]>, kind: EdgeKind }`, `read_edges(o: &Outline, corners: &[[F; 2]; 6]) -> Vec<Edge>`; `centreline() -> Vec<[F; 2]>` (61 points); `marks() -> Vec<(F, OutlinePoint)>` (θ every 30°).

The numbers below were printed from `banana.html` at commit `952747a` by a probe that logged `o.length`, `here`, `centre`, `corners`, the crossings and the edge kinds (the port's harness rule: any value the port needs comes from the page).

- [ ] **Step 1: Write the failing tests**

Create `banana.rs` with only:

```rust
#[cfg(test)]
mod tests {
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
        assert!(close2(o.pts[0], [204.53386631276712, -171.62429178630597]), "{:?}", o.pts[0]);
        assert!(close2(o.pts[90], [267.0, 0.0]), "{:?}", o.pts[90]); // the outer arc's top, after the turn
        assert!(close2(o.pts[150], [207.0, 0.0]), "{:?}", o.pts[150]); // the inner arc's top
        assert!(close2(o.pts[480], o.pts[0]));
    }

    #[test]
    fn theta_12_is_the_default_placement_point() {
        let o = outline();
        let here = at(&o, 12.0);
        assert!(close2(here.p, [221.5656885770422, -148.9856374204382]), "{:?}", here.p);
        assert!(close2(here.normal, [0.829037572555049, -0.559192903470736]), "{:?}", here.normal);
    }

    #[test]
    fn theta_at_the_sliders_ends() {
        let o = outline();
        assert!(close2(at(&o, 0.0).p, o.pts[0]));
        let near_end = at(&o, 359.5);
        let d = ((near_end.p[0] - o.pts[0][0]).powi(2) + (near_end.p[1] - o.pts[0][1]).powi(2)).sqrt();
        assert!(d > 1.0 && d < 1.5, "{d} ri short of the apex"); // half a degree is 1.18 ri
    }

    #[test]
    fn the_default_placement_puts_the_origin_ten_ri_inland() {
        let o = outline();
        let w = place(&o, 12.0, 10.0);
        assert!(close2(w.origin, [213.27531285149172, -143.39370838573083]), "{:?}", w.origin);
        assert!(close2(w.tangent, [0.559192903470736, 0.829037572555049]), "{:?}", w.tangent);
        let corners = world_corners(w.origin);
        assert!(close2(corners[0], [231.27531285149172, -133.00140354031757]), "{:?}", corners[0]);
        assert!(close2(corners[3], [195.27531285149172, -153.7860132311441]), "{:?}", corners[3]);
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
        let crossings: Vec<[F; 2]> = edges.iter().flat_map(|e| e.crossings.iter().copied()).collect();
        assert_eq!(crossings.len(), 2);
        assert!(close2(crossings[0], [231.27531285149172, -133.41539117277335]), "{:?}", crossings[0]);
        assert!(close2(crossings[1], [211.39670488496674, -163.0937032613824]), "{:?}", crossings[1]);
    }

    #[test]
    fn the_inner_coast_at_theta_200_reads_the_other_way_round() {
        let o = outline();
        let w = place(&o, 200.0, -4.0);
        assert!(close2(w.origin, [121.95619044252986, -162.28084355026252]), "{:?}", w.origin);
        let kinds: Vec<EdgeKind> = read_edges(&o, &world_corners(w.origin)).iter().map(|e| e.kind).collect();
        use EdgeKind::*;
        assert_eq!(kinds, vec![Shore, Sea, Sea, Sea, Shore, Land]);
        assert!(!inside(&o, w.origin));
    }

    #[test]
    fn segments_cross_where_they_should() {
        assert!(seg_cross([0.0, 0.0], [2.0, 2.0], [0.0, 2.0], [2.0, 0.0]).map(|p| close2(p, [1.0, 1.0])).unwrap_or(false));
        assert!(seg_cross([0.0, 0.0], [1.0, 1.0], [2.0, 2.0], [3.0, 3.0]).is_none()); // parallel
        assert!(seg_cross([0.0, 0.0], [1.0, 0.0], [2.0, -1.0], [2.0, 1.0]).is_none()); // past the end
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
```

- [ ] **Step 2: Add the module to both twins and run the tests to see them fail**

In `lib.rs`, after each twin's `pub mod plane;` add:

```rust
    #[path = "../banana.rs"]
    pub mod banana;
```

Run: `cd /home/lexa/DevProjects/_GameDev/Murabito/.claude/worktrees/exp-06-worldgen/Experiments/exp-06 && cargo test -p worldgen`
Expected: compile errors, `outline` and friends not found.

- [ ] **Step 3: Write `banana.rs`**

Above the tests:

```rust
//! Step 1: the banana, and the world placed on it. A slot bent along a circular arc, sized like Honshu
//! at its Kantō width, turned so the top tip of the centreline heads north. The outline is walked
//! anticlockwise from the outer apex; `θ` is the fraction of its length walked, in degrees. The world
//! is centred on the outline point at `θ` moved inland by the inset. Sizes are Lexa's constants.

use super::{rad, F, PI, TOL};
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
        turned(cx + r * (t.cos() * ux + t.sin() * vx), cy + r * (t.cos() * uy + t.sin() * vy))
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
    OutlinePoint { p, normal: [ty / l, -tx / l] }
}

/// Even-odd point in polygon.
pub fn inside(o: &Outline, p: [F; 2]) -> bool {
    let pts = &o.pts;
    let mut c = false;
    let mut j = pts.len() - 1;
    for i in 0..pts.len() {
        let (pi, pj) = (pts[i], pts[j]);
        if (pi[1] > p[1]) != (pj[1] > p[1]) && p[0] < (pj[0] - pi[0]) * (p[1] - pi[1]) / (pj[1] - pi[1]) + pi[0] {
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
    ((0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u)).then(|| [a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])])
}

/// The world placed: its origin is the outline point at `θ` moved inland by `inset` ri.
pub fn place(o: &Outline, theta: F, inset: F) -> Placement {
    let here = at(o, theta);
    let origin = [here.p[0] - here.normal[0] * inset, here.p[1] - here.normal[1] * inset];
    let tangent = [-here.normal[1], here.normal[0]];
    Placement { theta, inset, here, origin, tangent }
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
            let crossings: Vec<[F; 2]> = o.pts.windows(2).filter_map(|w| seg_cross(a, b, w[0], w[1])).collect();
            let mid = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0];
            let kind = if !crossings.is_empty() {
                EdgeKind::Shore
            } else if inside(o, mid) {
                EdgeKind::Land
            } else {
                EdgeKind::Sea
            };
            Edge { direction, a, b, crossings, kind }
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
    (0..12).map(|i| {
        let deg = 30.0 * i as F;
        (deg, at(&o, deg))
    }).collect()
}
```

If `TOL` is only used in tests, import it inside the test module instead to keep the build warning-free.

- [ ] **Step 4: Run the tests to see them pass**

Run: `cd /home/lexa/DevProjects/_GameDev/Murabito/.claude/worktrees/exp-06-worldgen/Experiments/exp-06 && cargo test -p worldgen`
Expected: every `banana::tests` test passes under both twins. If an f32 test fails on a coordinate by more than `TOL`, that twin's arithmetic is accumulating error somewhere the f64 twin does not show; report the number rather than loosening `TOL`. `cargo clippy -p worldgen --all-targets` and `cargo fmt --all -- --check` clean.

- [ ] **Step 5: Leave the work in the tree**

No commit. Report.

---

### Task 4: `settings`, `format`, and the step-1 facts

**Files:**
- Create: `Experiments/exp-06/crates/worldgen/src/settings.rs`
- Create: `Experiments/exp-06/crates/worldgen/src/format.rs`
- Create: `Experiments/exp-06/crates/worldgen/src/facts.rs`
- Modify: `Experiments/exp-06/crates/worldgen/src/lib.rs` (add `pub mod settings; pub mod format;` and `#[path = "../facts.rs"] pub mod facts;` in both twins)

**Interfaces:**
- Consumes: `hex::Cube`, each twin's `banana::{Edge, EdgeKind}`.
- Produces: `worldgen::settings::Settings` (all f64 plus `seed: u32`, `first_octave: u32`, `octaves: u32`, `window: Option<Cube>`) with `Default` = the page's defaults; `worldgen::format::FactRow { key: String, value: String, bad: bool }` with `FactRow::new(key: &str, value: impl Into<String>, bad: bool)`; `worldgen::format::js_fixed(x: f64, digits: usize) -> String`; each twin's `facts::step1(edges: &[Edge], origin_inside: bool) -> Vec<FactRow>`.

- [ ] **Step 1: Write the failing tests**

`settings.rs`, tests only for now:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_the_pages() {
        let s = Settings::default();
        assert_eq!((s.theta, s.inset, s.seed), (12.0, 10.0, 1));
        assert_eq!((s.amp, s.hurst, s.stretch), (250.0, 0.65, 2.0));
        assert_eq!((s.erosion, s.first_octave, s.octaves, s.stripes, s.sigma_ref), (200.0, 2, 8, 1.5, 0.03));
        assert_eq!((s.stacking, s.norm_above, s.base_strength, s.keep_deeper, s.gref), (1.0, 0.3, 0.0, 100.0, 150.0));
        assert_eq!(s.window, None);
    }
}
```

`format.rs`, tests only:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn js_fixed_rounds_halves_away_from_zero_like_to_fixed() {
        assert_eq!(js_fixed(2.5, 0), "3");
        assert_eq!(js_fixed(-2.5, 0), "-3");
        assert_eq!(js_fixed(0.125, 2), "0.13"); // Rust's {:.2} would say 0.12
        assert_eq!(js_fixed(1.005, 2), "1.00"); // not a true half: 1.005 is below it in binary, as in JS
        assert_eq!(js_fixed(12.0, 1), "12.0");
        assert_eq!(js_fixed(269.4, 0), "269");
        assert_eq!(js_fixed(-0.04, 1), "-0.0"); // JS says "-0.0" too
        assert_eq!(js_fixed(10.235, 2), "10.23"); // below a half in binary, though 10.235 · 100 rounds onto 1023.5
        assert_eq!(js_fixed(2.675, 2), "2.67"); // the same, the classic case
        assert_eq!(js_fixed(0.5, 0), "1");
        assert_eq!(js_fixed(1.5, 0), "2");
        assert_eq!(js_fixed(-0.125, 2), "-0.13");
    }

    #[test]
    fn a_row_carries_its_flag() {
        let r = FactRow::new("sea edges", "1  (SSE)", false);
        assert_eq!((r.key.as_str(), r.value.as_str(), r.bad), ("sea edges", "1  (SSE)", false));
    }
}
```

`facts.rs`, tests only (this file is a twin, so `super::F` and the twin's `banana` are in scope):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::FactRow;

    fn rows_at(theta: F, inset: F) -> Vec<FactRow> {
        let o = banana::outline();
        let w = banana::place(&o, theta, inset);
        let edges = banana::read_edges(&o, &banana::world_corners(w.origin));
        step1(&edges, banana::inside(&o, w.origin))
    }
    fn printed(rows: &[FactRow]) -> Vec<String> {
        rows.iter().map(|r| format!("{} | {}", r.key, r.value)).collect()
    }

    #[test]
    fn the_default_facts_are_the_reference_readings() {
        // docs/reference-readings.md, the `-- facts` block
        let rows = rows_at(12.0, 10.0);
        assert_eq!(
            printed(&rows),
            vec![
                "sea edges | 1  (SSE)",
                "land edges | 3  (NNE, NNW, W)",
                "shore edges (crossed) | 2  (E, SSW)",
                "shore crossings | 2",
                "no more than four of either | holds",
                "world centre inside the banana | yes",
            ]
        );
        assert!(rows.iter().all(|r| !r.bad));
    }

    #[test]
    fn the_inner_coast_facts() {
        let rows = rows_at(200.0, -4.0);
        assert_eq!(
            printed(&rows),
            vec![
                "sea edges | 3  (NNE, NNW, W)",
                "land edges | 1  (SSE)",
                "shore edges (crossed) | 2  (E, SSW)",
                "shore crossings | 2",
                "no more than four of either | holds",
                "world centre inside the banana | no: out to sea",
            ]
        );
    }

    #[test]
    fn far_out_to_sea_the_rule_breaks_and_says_so() {
        let rows = rows_at(12.0, -25.0);
        assert_eq!(printed(&rows)[0], "sea edges | 6  (E, NNE, NNW, W, SSW, SSE)");
        assert_eq!(printed(&rows)[1], "land edges | 0");
        assert_eq!(printed(&rows)[3], "shore crossings | 0");
        assert!(!rows[3].bad, "zero crossings is fine");
        assert_eq!(printed(&rows)[4], "no more than four of either | broken");
        assert!(rows[0].bad && rows[4].bad);
    }
}
```

- [ ] **Step 2: Wire the modules and run the tests to see them fail**

In `lib.rs` add after `pub mod hex;`:

```rust
pub mod format;
pub mod settings;
```

and in each twin, after `pub mod banana;`:

```rust
    #[path = "../facts.rs"]
    pub mod facts;
```

Run: `cd /home/lexa/DevProjects/_GameDev/Murabito/.claude/worktrees/exp-06-worldgen/Experiments/exp-06 && cargo test -p worldgen`
Expected: compile errors on `Settings`, `js_fixed`, `FactRow`, `step1`.

- [ ] **Step 3: Write the three modules**

`settings.rs`:

```rust
//! Every dial on the page, with the page's defaults. Always in f64; the numeric twins convert on entry.

use crate::hex::Cube;

#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    /// Degrees round the outline, anticlockwise from the outer apex.
    pub theta: f64,
    /// Ri inland from the outline point; negative is out to sea.
    pub inset: f64,
    pub seed: u32,
    /// Structure amplitude at the 36-ri octave, metres.
    pub amp: f64,
    /// Hurst exponent shared by structure and erosion.
    pub hurst: f64,
    /// How much longer features are along the arc than across it.
    pub stretch: f64,
    /// Step 7a's full arrow length, m per ri (a drawing dial the page keeps in its state).
    pub gref: f64,
    /// Erosion amplitude at the 36-ri octave, metres.
    pub erosion: f64,
    pub first_octave: u32,
    pub octaves: u32,
    pub stripes: f64,
    pub sigma_ref: f64,
    pub stacking: f64,
    pub norm_above: f64,
    pub base_strength: f64,
    pub keep_deeper: f64,
    /// The window ri, if one was picked; else the ri nearest 12 ri inland.
    pub window: Option<Cube>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            theta: 12.0,
            inset: 10.0,
            seed: 1,
            amp: 250.0,
            hurst: 0.65,
            stretch: 2.0,
            gref: 150.0,
            erosion: 200.0,
            first_octave: 2,
            octaves: 8,
            stripes: 1.5,
            sigma_ref: 0.03,
            stacking: 1.0,
            norm_above: 0.3,
            base_strength: 0.0,
            keep_deeper: 100.0,
            window: None,
        }
    }
}
```

`format.rs`:

```rust
//! Text the page prints: facts rows, and numbers formatted the way JavaScript formats them, so a row
//! is the same string on both pages and the harness can compare them literally.

/// One line of a step's facts: a key, a value, and whether the page colours it as a warning.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FactRow {
    pub key: String,
    pub value: String,
    pub bad: bool,
}

impl FactRow {
    pub fn new(key: &str, value: impl Into<String>, bad: bool) -> FactRow {
        FactRow { key: key.to_string(), value: value.into(), bad }
    }
}

/// JS `Number.prototype.toFixed`: the nearest decimal, a true half rounding away from zero. Rust's
/// `{:.n}` rounds a true half to even, so the two disagree on exactly representable halves such as
/// 0.125 → "0.13" against "0.12"; everything else is the same correctly rounded value.
pub fn js_fixed(x: f64, digits: usize) -> String {
    if is_true_half(x.abs(), digits) {
        let scale = 10f64.powi(digits as i32);
        let up = ((x.abs() * scale).floor() + 1.0) / scale;
        let s = format!("{:.*}", digits, up);
        return if x < 0.0 { format!("-{s}") } else { s };
    }
    format!("{:.*}", digits, x)
}

/// Whether `x · 10^digits` is exactly a whole number and a half, read off the exact decimal
/// expansion rather than a product that rounds (the product put 10.235 onto 1023.5 exactly; JS
/// says "10.23"). A true half is a double `m / 2^(digits + 1)` with `m` odd, whose expansion ends
/// exactly at place `digits + 1`; any other double is at least an ulp away and shows it.
fn is_true_half(x: f64, digits: usize) -> bool {
    const EXTRA: usize = 20;
    let wide = format!("{:.*}", digits + EXTRA, x);
    let tail = &wide[wide.len() - EXTRA..];
    tail.starts_with('5') && tail[1..].bytes().all(|b| b == b'0')
}
```

`facts.rs` (a twin file):

```rust
//! The facts rows, step by step, as the page prints them. Keys and formats are the page's, literally.

use super::banana::{Edge, EdgeKind};
use crate::format::FactRow;

/// Step 1: the edge reading and the napkin rule.
pub fn step1(edges: &[Edge], origin_inside: bool) -> Vec<FactRow> {
    let count = |k: EdgeKind| edges.iter().filter(|e| e.kind == k).count();
    let with_names = |k: EdgeKind| {
        let n = count(k);
        if n == 0 {
            return n.to_string();
        }
        let names: Vec<&str> = edges.iter().filter(|e| e.kind == k).map(|e| e.direction.name()).collect();
        format!("{n}  ({})", names.join(", "))
    };
    let (sea, land) = (count(EdgeKind::Sea), count(EdgeKind::Land));
    let crossings: usize = edges.iter().map(|e| e.crossings.len()).sum();
    let rule_holds = sea <= 4 && land <= 4;
    vec![
        FactRow::new("sea edges", with_names(EdgeKind::Sea), sea > 4),
        FactRow::new("land edges", with_names(EdgeKind::Land), land > 4),
        FactRow::new("shore edges (crossed)", with_names(EdgeKind::Shore), false),
        FactRow::new("shore crossings", crossings.to_string(), crossings != 2 && crossings != 0),
        FactRow::new("no more than four of either", if rule_holds { "holds" } else { "broken" }, !rule_holds),
        FactRow::new("world centre inside the banana", if origin_inside { "yes" } else { "no: out to sea" }, false),
    ]
}
```

- [ ] **Step 4: Run the tests to see them pass**

Run: `cd /home/lexa/DevProjects/_GameDev/Murabito/.claude/worktrees/exp-06-worldgen/Experiments/exp-06 && cargo test -p worldgen`
Expected: all pass, under both twins for `facts`. Clippy and fmt clean.

- [ ] **Step 5: Leave the work in the tree**

No commit. Report.

---

### Task 5: the `exp06` server and `/render`

**Files:**
- Modify: `Experiments/exp-06/Cargo.toml` (add `"crates/exp06"` to `members`)
- Create: `Experiments/exp-06/crates/exp06/Cargo.toml`
- Create: `Experiments/exp-06/crates/exp06/src/lib.rs`
- Create: `Experiments/exp-06/crates/exp06/src/main.rs`
- Create: `Experiments/exp-06/crates/exp06/src/wire.rs`
- Create: `Experiments/exp-06/crates/exp06/src/render.rs`
- Create: `Experiments/exp-06/crates/exp06/tests/server.rs`

**Interfaces:**
- Consumes: `worldgen::settings::Settings`, `worldgen::{f64, f32}::{banana, facts, F}`, `worldgen::format::FactRow`.
- Produces: `exp06::PORT: u16 = 8060`, `exp06::PAGE: &str` (the page's absolute path), `exp06::run(server: tiny_http::Server)`, `exp06::handle(request: tiny_http::Request)`; `exp06::wire::RenderRequest` (serde, page key names, `path: String`), `RenderRequest::settings() -> Settings`; `exp06::wire::RenderResponse { path, ms, banana: BananaOut { r, w, a, world }, outline, length, here, normal, origin, tangent, corners, centreline, marks: Vec<MarkOut { deg, p, normal }>, edges: Vec<EdgeOut { name, a, b, crossings, kind }>, facts: Vec<FactOut { key, value, bad }> }`; `exp06::render::render(&RenderRequest) -> RenderResponse`. Routes: `GET /` and `GET /exp-06.html` serve the page; `POST /render` takes the page's state as JSON and answers `RenderResponse` as JSON; anything else 404.

- [ ] **Step 1: Add the crate and write the failing integration test**

`Experiments/exp-06/Cargo.toml`: `members = ["crates/worldgen", "crates/exp06"]`.

`Experiments/exp-06/crates/exp06/Cargo.toml`:

```toml
[package]
name = "exp06"
edition.workspace = true
version.workspace = true
license.workspace = true
publish.workspace = true

[dependencies]
worldgen = { path = "../worldgen" }
tiny_http = "0.12"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

`Experiments/exp-06/crates/exp06/tests/server.rs`:

```rust
//! The server answers /render with the page's facts, and refuses what it cannot parse.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};

fn start() -> SocketAddr {
    let server = tiny_http::Server::http("127.0.0.1:0").expect("bind an ephemeral port");
    let addr = server.server_addr().to_ip().expect("an ip listener");
    std::thread::spawn(move || exp06::run(server));
    addr
}

fn call(addr: SocketAddr, method: &str, path: &str, body: &str) -> (u16, String) {
    let mut s = TcpStream::connect(addr).expect("connect");
    let req = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    s.write_all(req.as_bytes()).expect("write");
    let mut raw = Vec::new();
    s.read_to_end(&mut raw).expect("read");
    let text = String::from_utf8_lossy(&raw).to_string();
    let status: u16 = text.split_whitespace().nth(1).and_then(|c| c.parse().ok()).expect("a status line");
    let body = text.split_once("\r\n\r\n").map(|(_, b)| b.to_string()).unwrap_or_default();
    (status, body)
}

fn facts(json: &str) -> Vec<String> {
    let v: serde_json::Value = serde_json::from_str(json).expect("json");
    v["facts"].as_array().expect("facts").iter().map(|f| format!("{} | {}", f["key"].as_str().unwrap(), f["value"].as_str().unwrap())).collect()
}

const DEFAULT_ROWS: [&str; 6] = [
    "sea edges | 1  (SSE)",
    "land edges | 3  (NNE, NNW, W)",
    "shore edges (crossed) | 2  (E, SSW)",
    "shore crossings | 2",
    "no more than four of either | holds",
    "world centre inside the banana | yes",
];

#[test]
fn render_at_the_defaults_prints_the_reference_facts() {
    let addr = start();
    let (status, body) = call(addr, "POST", "/render", "{}");
    assert_eq!(status, 200, "{body}");
    assert_eq!(facts(&body), DEFAULT_ROWS);
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["path"], "f64");
    assert_eq!(v["outline"].as_array().unwrap().len(), 481);
    assert_eq!(v["edges"].as_array().unwrap().len(), 6);
    assert_eq!(v["edges"][0]["name"], "E");
    assert_eq!(v["edges"][0]["kind"], "shore");
    assert_eq!(v["marks"].as_array().unwrap().len(), 12);
    assert_eq!(v["centreline"].as_array().unwrap().len(), 61);
    assert_eq!(v["banana"]["r"], 237.0);
    assert!((v["length"].as_f64().unwrap() - 850.3012208770891).abs() < 1e-9);
}

#[test]
fn render_takes_the_pages_keys_and_partial_bodies() {
    let addr = start();
    let (status, body) = call(addr, "POST", "/render", r#"{"theta":200,"inset":-4,"octaves5":8,"windowQRS":[-6,-8,14],"unknownKey":1}"#);
    assert_eq!(status, 200, "{body}");
    assert_eq!(facts(&body)[0], "sea edges | 3  (NNE, NNW, W)");
    assert_eq!(facts(&body)[5], "world centre inside the banana | no: out to sea");
}

#[test]
fn the_f32_twin_answers_and_says_so() {
    let addr = start();
    let (status, body) = call(addr, "POST", "/render", r#"{"path":"f32"}"#);
    assert_eq!(status, 200, "{body}");
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["path"], "f32");
    assert_eq!(facts(&body), DEFAULT_ROWS);
}

#[test]
fn a_path_that_does_not_exist_yet_falls_back_to_f64() {
    let addr = start();
    let (_, body) = call(addr, "POST", "/render", r#"{"path":"gpu"}"#);
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["path"], "f64");
}

#[test]
fn a_malformed_body_is_a_400_and_the_server_keeps_serving() {
    let addr = start();
    let (status, body) = call(addr, "POST", "/render", "not json");
    assert_eq!(status, 400);
    assert!(body.starts_with("bad settings:"), "{body}");
    let (status, _) = call(addr, "POST", "/render", "");
    assert_eq!(status, 400);
    let (status, _) = call(addr, "POST", "/render", "{}");
    assert_eq!(status, 200);
}

#[test]
fn anything_else_is_a_404() {
    let addr = start();
    assert_eq!(call(addr, "GET", "/nothing", "").0, 404);
    assert_eq!(call(addr, "POST", "/", "").0, 404);
}
```

- [ ] **Step 2: Run the test to see it fail**

Run: `cd /home/lexa/DevProjects/_GameDev/Murabito/.claude/worktrees/exp-06-worldgen/Experiments/exp-06 && cargo test -p exp06`
Expected: fails to build, no `exp06` library. (`cargo` downloads `tiny_http` and its three small dependencies the first time.)

- [ ] **Step 3: Write the server**

`src/lib.rs`:

```rust
#![forbid(unsafe_code)]
//! The exp-06 server: serves `viz/exp-06.html` from disk and answers `POST /render` with everything
//! the page draws and prints, computed by `worldgen`. The only crate here that knows HTTP or JSON.

pub mod render;
pub mod wire;

use std::io::{Cursor, Read};

use tiny_http::{Header, Method, Request, Response, Server};

/// The fixed port; printed on start.
pub const PORT: u16 = 8060;
/// The page, read from disk on every request so an edit to it needs no restart.
pub const PAGE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../viz/exp-06.html");

type Reply = Response<Cursor<Vec<u8>>>;

/// Serve until the listener closes.
pub fn run(server: Server) {
    for request in server.incoming_requests() {
        handle(request);
    }
}

/// One request: the page, a render, or 404.
pub fn handle(mut request: Request) {
    let path = request.url().split('?').next().unwrap_or("/").to_string();
    let method = request.method().clone();
    let reply = match (method, path.as_str()) {
        (Method::Get, "/") | (Method::Get, "/exp-06.html") => page(),
        (Method::Post, "/render") => {
            let mut body = String::new();
            match request.as_reader().read_to_string(&mut body) {
                Ok(_) => render_json(&body),
                Err(e) => text(400, &format!("unreadable body: {e}")),
            }
        }
        _ => text(404, "not found"),
    };
    // A client that went away is not our error.
    let _ = request.respond(reply);
}

fn header(name: &str, value: &str) -> Header {
    Header::from_bytes(name.as_bytes(), value.as_bytes()).expect("ascii header")
}

fn text(status: u16, body: &str) -> Reply {
    Response::from_string(body)
        .with_status_code(status)
        .with_header(header("Content-Type", "text/plain; charset=utf-8"))
}

fn page() -> Reply {
    match std::fs::read(PAGE) {
        Ok(bytes) => Response::from_data(bytes).with_header(header("Content-Type", "text/html; charset=utf-8")),
        Err(e) => text(500, &format!("cannot read {PAGE}: {e}")),
    }
}

fn render_json(body: &str) -> Reply {
    let request: wire::RenderRequest = match serde_json::from_str(body) {
        Ok(r) => r,
        Err(e) => return text(400, &format!("bad settings: {e}")),
    };
    let out = render::render(&request);
    Response::from_string(serde_json::to_string(&out).expect("a response serialises"))
        .with_header(header("Content-Type", "application/json"))
}
```

`src/main.rs`:

```rust
fn main() {
    let server = tiny_http::Server::http(("127.0.0.1", exp06::PORT)).unwrap_or_else(|e| {
        eprintln!("cannot listen on 127.0.0.1:{}: {e}", exp06::PORT);
        std::process::exit(1)
    });
    println!("exp-06 worldgen at http://127.0.0.1:{}/  serving {}", exp06::PORT, exp06::PAGE);
    exp06::run(server);
}
```

`src/wire.rs`:

```rust
//! What crosses the wire: the page's state in, the drawing's data out. Key names are the page's.

use serde::{Deserialize, Serialize};
use worldgen::hex::Cube;
use worldgen::settings::Settings;

/// The page's `state`, plus which compute path to run. Missing keys keep the page's defaults;
/// unknown keys are ignored.
#[derive(Debug, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RenderRequest {
    pub theta: f64,
    pub inset: f64,
    pub seed: u32,
    pub amp: f64,
    pub hurst: f64,
    pub stretch: f64,
    pub gref: f64,
    pub erosion: f64,
    pub first_octave: u32,
    pub octaves5: u32,
    pub stripes: f64,
    pub sigma_ref: f64,
    pub stacking: f64,
    pub norm_above: f64,
    pub base_strength: f64,
    pub keep_deeper: f64,
    #[serde(rename = "windowQRS")]
    pub window_qrs: Option<[i32; 3]>,
    /// `"f64"` (the reference), `"f32"` (the shader's twin); anything else runs f64.
    pub path: String,
}

impl Default for RenderRequest {
    fn default() -> Self {
        let s = Settings::default();
        RenderRequest {
            theta: s.theta,
            inset: s.inset,
            seed: s.seed,
            amp: s.amp,
            hurst: s.hurst,
            stretch: s.stretch,
            gref: s.gref,
            erosion: s.erosion,
            first_octave: s.first_octave,
            octaves5: s.octaves,
            stripes: s.stripes,
            sigma_ref: s.sigma_ref,
            stacking: s.stacking,
            norm_above: s.norm_above,
            base_strength: s.base_strength,
            keep_deeper: s.keep_deeper,
            window_qrs: None,
            path: "f64".to_string(),
        }
    }
}

impl RenderRequest {
    pub fn settings(&self) -> Settings {
        Settings {
            theta: self.theta,
            inset: self.inset,
            seed: self.seed,
            amp: self.amp,
            hurst: self.hurst,
            stretch: self.stretch,
            gref: self.gref,
            erosion: self.erosion,
            first_octave: self.first_octave,
            octaves: self.octaves5,
            stripes: self.stripes,
            sigma_ref: self.sigma_ref,
            stacking: self.stacking,
            norm_above: self.norm_above,
            base_strength: self.base_strength,
            keep_deeper: self.keep_deeper,
            window: self.window_qrs.map(|[q, r, s]| Cube::new(q, r, s)),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct BananaOut {
    pub r: f64,
    pub w: f64,
    pub a: f64,
    pub world: f64,
}

#[derive(Debug, Serialize)]
pub struct MarkOut {
    pub deg: f64,
    pub p: [f64; 2],
    pub normal: [f64; 2],
}

#[derive(Debug, Serialize)]
pub struct EdgeOut {
    pub name: String,
    pub a: [f64; 2],
    pub b: [f64; 2],
    pub crossings: Vec<[f64; 2]>,
    pub kind: String,
}

#[derive(Debug, Serialize)]
pub struct FactOut {
    pub key: String,
    pub value: String,
    pub bad: bool,
}

/// Everything step 1 draws and prints. Later bites append fields; nothing here is renamed.
#[derive(Debug, Serialize)]
pub struct RenderResponse {
    pub path: String,
    /// Server time for the render, milliseconds.
    pub ms: f64,
    pub banana: BananaOut,
    pub outline: Vec<[f64; 2]>,
    pub length: f64,
    pub here: [f64; 2],
    pub normal: [f64; 2],
    pub origin: [f64; 2],
    pub tangent: [f64; 2],
    pub corners: Vec<[f64; 2]>,
    pub centreline: Vec<[f64; 2]>,
    pub marks: Vec<MarkOut>,
    pub edges: Vec<EdgeOut>,
    pub facts: Vec<FactOut>,
}
```

`src/render.rs`:

```rust
//! One render: the chosen twin computes, the result is widened to f64 for the wire.

#![allow(clippy::unnecessary_cast)] // the f64 twin's `as f64` is a no-op; the f32 twin's is not

use crate::wire::{BananaOut, EdgeOut, FactOut, MarkOut, RenderRequest, RenderResponse};

macro_rules! step1_with {
    ($twin:ident, $request:expr) => {{
        use worldgen::$twin::{banana, facts, F};
        let settings = $request.settings();
        let started = std::time::Instant::now();
        let o = banana::outline();
        let placed = banana::place(&o, settings.theta as F, settings.inset as F);
        let corners = banana::world_corners(placed.origin);
        let edges = banana::read_edges(&o, &corners);
        let origin_inside = banana::inside(&o, placed.origin);
        let facts = facts::step1(&edges, origin_inside);
        let ms = started.elapsed().as_secs_f64() * 1000.0;
        let wide = |p: [F; 2]| [p[0] as f64, p[1] as f64];
        RenderResponse {
            path: stringify!($twin).to_string(),
            ms,
            banana: BananaOut {
                r: banana::R_RI as f64,
                w: banana::W_RI as f64,
                a: banana::ARC_DEG as f64,
                world: banana::WORLD as f64,
            },
            outline: o.pts.iter().copied().map(wide).collect(),
            length: o.length as f64,
            here: wide(placed.here.p),
            normal: wide(placed.here.normal),
            origin: wide(placed.origin),
            tangent: wide(placed.tangent),
            corners: corners.iter().copied().map(wide).collect(),
            centreline: banana::centreline().into_iter().map(wide).collect(),
            marks: banana::marks()
                .into_iter()
                .map(|(deg, m)| MarkOut { deg: deg as f64, p: wide(m.p), normal: wide(m.normal) })
                .collect(),
            edges: edges
                .iter()
                .map(|e| EdgeOut {
                    name: e.direction.name().to_string(),
                    a: wide(e.a),
                    b: wide(e.b),
                    crossings: e.crossings.iter().copied().map(wide).collect(),
                    kind: e.kind.name().to_string(),
                })
                .collect(),
            facts: facts.into_iter().map(|f| FactOut { key: f.key, value: f.value, bad: f.bad }).collect(),
        }
    }};
}

pub fn render(request: &RenderRequest) -> RenderResponse {
    match request.path.as_str() {
        "f32" => step1_with!(f32, request),
        _ => step1_with!(f64, request),
    }
}
```

- [ ] **Step 4: Run the tests to see them pass**

Run: `cd /home/lexa/DevProjects/_GameDev/Murabito/.claude/worktrees/exp-06-worldgen/Experiments/exp-06 && cargo test --workspace`
Expected: all six server tests pass, `worldgen` still green. `cargo clippy --workspace --all-targets` and `cargo fmt --all -- --check` clean. Then run the binary once: `cargo run -p exp06` prints `exp-06 worldgen at http://127.0.0.1:8060/ …`; stop it with Ctrl-C (it is your process). If port 8060 is taken by a process you did not start, do not touch that process; report it.

- [ ] **Step 5: Leave the work in the tree**

No commit. Report. `Cargo.lock` is part of the work.

---

### Task 6: `viz/exp-06.html`, the README, the port's harness

**Files:**
- Create: `Experiments/exp-06/viz/exp-06.html`
- Create: `Experiments/exp-06/viz/tools/check-port.js`
- Create: `Experiments/exp-06/README.md`
- Modify: `Experiments/exp-06/crates/exp06/tests/server.rs` (add the GET test)

**Interfaces:**
- Consumes: `POST /render` and its `RenderResponse` fields, from Task 5.
- Produces: the page; `node viz/tools/check-port.js [key=value ...]`, which prints `-- facts` then `key | value` rows exactly as `check.js` prints `banana.html`'s.

- [ ] **Step 1: Add the failing GET test**

Append to `crates/exp06/tests/server.rs`:

```rust
#[test]
fn the_page_is_served_at_the_root() {
    let addr = start();
    let (status, body) = call(addr, "GET", "/", "");
    assert_eq!(status, 200, "{body}");
    assert!(body.contains("<title>exp-06 worldgen</title>"), "not the page: {}", &body[..body.len().min(200)]);
    let (status, _) = call(addr, "GET", "/exp-06.html", "");
    assert_eq!(status, 200);
}
```

Run: `cd /home/lexa/DevProjects/_GameDev/Murabito/.claude/worktrees/exp-06-worldgen/Experiments/exp-06 && cargo test -p exp06 the_page_is_served`
Expected: FAIL, status 500 (`cannot read …/viz/exp-06.html`).

- [ ] **Step 2: Write the page**

`Experiments/exp-06/viz/exp-06.html`. The head, the styles, the step-1 markup and the drawing are `banana.html`'s (commit `952747a`), with the computing replaced by a fetch; the title differs, the paragraph says what the page is, and a compute-path selector and a status line are added under the sliders.

```html
<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>exp-06 worldgen</title>
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@400;500&family=IBM+Plex+Sans:wght@400;600&display=swap">
<style>
/* banana.html's look, unchanged, so the two pages sit side by side. */
:root { --bg: #f6f5f0; --fg: #22261f; --muted: #6b7064; --line: #c9ccc2; --cell: #e6e8de; --sea: #d3dfe3; --land: #e9e3cf;
  --world: #b7410e; --shore: #2b6f8c; --mountain: #6f5a3a;
  --font-body: "IBM Plex Sans", system-ui, sans-serif; --font-mono: "IBM Plex Mono", ui-monospace, monospace; }
@media (prefers-color-scheme: dark) { :root:not([data-theme="light"]) { --bg: #1b1d19; --fg: #e4e6dc; --muted: #9aa092; --line: #444a3f; --cell: #262a23; --sea: #1f2a2e; --land: #2e2b22; --world: #f0803a; --shore: #6db3d1; --mountain: #c9a87a; color-scheme: dark; } }
:root[data-theme="dark"] { --bg: #1b1d19; --fg: #e4e6dc; --muted: #9aa092; --line: #444a3f; --cell: #262a23; --sea: #1f2a2e; --land: #2e2b22; --world: #f0803a; --shore: #6db3d1; --mountain: #c9a87a; color-scheme: dark; }
body { background: var(--bg); color: var(--fg); font: 15px/1.5 var(--font-body); margin: 0; padding: 24px 16px 40px; }
main { max-width: 1280px; margin: 0 auto; display: grid; gap: 16px; }
h1 { font-size: 1.4rem; margin: 0; font-weight: 600; text-wrap: balance; }
p { margin: 0; max-width: 70ch; }
.muted { color: var(--muted); }
.pair { display: flex; gap: 12px; flex-wrap: wrap; align-items: flex-start; }
.pair > div { flex: 1 1 320px; min-width: 0; display: grid; gap: 4px; }
.panel-title { font-family: var(--font-mono); font-size: 0.8rem; color: var(--muted); }
svg { width: 100%; height: auto; display: block; }
.controls { display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: 8px 24px; font-family: var(--font-mono); font-size: 0.9rem; }
.controls label { display: grid; grid-template-columns: 12ch 1fr 8ch; gap: 10px; align-items: center; }
.controls input[type=range] { width: 100%; min-width: 0; accent-color: var(--shore); }
.controls output { text-align: right; font-variant-numeric: tabular-nums; }
.controls select { font: inherit; padding: 4px 6px; border: 1px solid var(--line); border-radius: 4px; background: var(--cell); color: var(--fg); }
.controls button { font: inherit; padding: 6px 12px; border: 1px solid var(--line); border-radius: 4px; background: var(--cell); color: var(--fg); cursor: pointer; justify-self: start; }
.controls button:disabled { opacity: 0.6; cursor: wait; }
.facts { font-family: var(--font-mono); font-size: 0.9rem; display: grid; gap: 4px; font-variant-numeric: tabular-nums; max-width: 560px; }
.facts div { display: grid; grid-template-columns: 1fr auto; gap: 12px; border-bottom: 1px solid var(--line); padding: 3px 0; }
.facts .bad { color: var(--world); }
.status { font-family: var(--font-mono); font-size: 0.85rem; color: var(--muted); min-height: 1.5em; }
</style>
</head>
<body>
<main>
  <h1>Step 1: the world on the banana</h1>
  <p>The banana is a slot bent along a circular arc, sized like Honshu at its Kantō width: arc <code>A = 80°</code>, centreline radius <code>R = 237</code> ri (about 1,300 km along the axis), width <code>w = 60</code> ri (about 236 km, coast to coast across Kantō). It is turned so the top tip of the centreline heads due north (90°), which puts the outer apex facing ESE, the Pacific side. The outline is walked anticlockwise from the outer apex, and <code>θ</code> is the fraction of its length walked, in degrees. The world, 36 ri flat to flat and north-up, is centred on the outline point at <code>θ</code> moved inland by <code>d</code>. Units are ri.</p>
  <p class="muted">This page draws what the Rust server computes (<code>cargo run -p exp06</code> in <code>Experiments/exp-06</code>); <code>banana.html</code> beside it is the reference. The settings travel in the URL query, so the same link with the other filename is the same world.</p>

  <div class="pair">
    <div>
      <div class="panel-title">the whole banana · θ marked every 30° · north up</div>
      <svg id="wide" viewBox="0 0 560 560" role="img" aria-label="The bent slot shape with degree marks along its outline and the world hexagon placed on it."><g id="wideScene" fill="none" stroke="currentColor"></g></svg>
    </div>
    <div>
      <div class="panel-title">round the world · edges read as sea, land or crossed</div>
      <svg id="near" viewBox="0 0 560 560" role="img" aria-label="The world hexagon with the shoreline through it, each edge coloured by whether it is at sea, on land or crossed by the shore."><g id="nearScene" fill="none" stroke="currentColor"></g></svg>
    </div>
  </div>

  <div class="controls controls-shared"></div>
  <div class="controls">
    <label><span>compute path</span><select id="path"><option value="f64">CPU f64, the reference</option><option value="f32">CPU f32, the shader's twin</option></select><span></span></label>
  </div>
  <div class="status" id="status">rendering…</div>

  <div class="facts" id="facts"></div>
  <p class="muted">The edge reading: an edge the outline crosses is a shore edge; an edge it does not cross is sea or land by where its midpoint lies. The rule from the napkin, no more than four of either, is checked live. The two shore crossings are where the surface of step 2 passes through sea level on the world's edges.</p>
</main>
<script>
(function () {
  const NS = "http://www.w3.org/2000/svg";
  const el = (t, a, tx) => { const e = document.createElementNS(NS, t); for (const k in a) e.setAttribute(k, a[k]); if (tx !== undefined) e.textContent = tx; return e; };
  const mono = { stroke: "none", "font-size": 11, "font-family": "var(--font-mono)" };
  const WORLD = 36;                                   // ri, flat to flat
  // The page's whole state, banana.html's keys and defaults, so a URL from either page reads on the other.
  const state = { theta: 12, inset: 10, seed: 1, amp: 250, hurst: 0.65, stretch: 2, gref: 150, erosion: 200, firstOctave: 2, octaves5: 8, stripes: 1.5, sigmaRef: 0.03, windowQRS: null, stacking: 1, normAbove: 0.3, baseStrength: 0, keepDeeper: 100 };
  let path = "f64";
  // Settings in the URL query: read once, written on every change.
  const QUERY_KEYS = Object.keys(state);
  function readQuery() {
    if (typeof location === "undefined") return;
    const qs = new URLSearchParams(location.search);
    for (const k of QUERY_KEYS) if (qs.has(k)) state[k] = k === "windowQRS" ? qs.get(k).split(",").map(Number) : +qs.get(k);
    if (qs.has("path")) path = qs.get("path");
  }
  function writeQuery() {
    if (typeof history === "undefined") return;
    const qs = new URLSearchParams();
    for (const k of QUERY_KEYS) { const v = state[k]; if (v === null) continue; qs.set(k, Array.isArray(v) ? v.join(",") : String(v)); }
    if (path !== "f64") qs.set("path", path);
    try { history.replaceState(null, "", "?" + qs); } catch (e) { /* a file: URL may refuse; the page still works */ }
  }
  const SLIDERS = [
    { key: "theta", label: "θ, degrees", min: 0, max: 359.5, step: 0.5, fmt: x => x.toFixed(1) + "°" },
    { key: "inset", label: "inset d, ri", min: -25, max: 45, step: 0.5, fmt: x => (x >= 0 ? "+" : "") + x.toFixed(1) },
  ];
  function bindSliders(selector, sliders) {
    return [...document.querySelectorAll(selector)].map(container => {
      const bound = sliders.map(s => {
        const label = document.createElement("label");
        const name = document.createElement("span"); name.textContent = s.label;
        const input = document.createElement("input"); input.type = "range"; input.min = s.min; input.max = s.max; input.step = s.step; input.value = state[s.key];
        const out = document.createElement("output");
        label.append(name, input, out); container.appendChild(label);
        input.addEventListener("input", () => { state[s.key] = +input.value; scheduleUpdate(); });
        return { s, input, out };
      });
      return { sync() { for (const b of bound) { b.input.value = state[b.s.key]; b.out.textContent = b.s.fmt(state[b.s.key]); } } };
    });
  }
  readQuery();
  const controls = [...bindSliders(".controls-shared", SLIDERS)];
  const pathSelect = document.getElementById("path"), status = document.getElementById("status");
  pathSelect.value = path;
  pathSelect.addEventListener("change", () => { path = pathSelect.value; update(); });
  function update() { controls.forEach(c => c.sync()); writeQuery(); render(); }
  let pendingRender = null;
  function scheduleUpdate() { controls.forEach(c => c.sync()); writeQuery(); clearTimeout(pendingRender); pendingRender = setTimeout(render, 150); }

  const wide = document.getElementById("wideScene"), near = document.getElementById("nearScene"), facts = document.getElementById("facts");
  function view(cx, cy, scale) { return p => ({ x: 280 + (p[0] - cx) * scale, y: 280 - (p[1] - cy) * scale }); }
  const pts = (ps, f) => ps.map(p => { const s = f(p); return s.x + "," + s.y; }).join(" ");

  let renderRun = 0, sent = null;
  async function render() {
    const run = ++renderRun, t0 = performance.now(), body = { ...state, path };
    let out;
    try {
      const res = await fetch("/render", { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(body) });
      if (run !== renderRun) return;
      if (!res.ok) { status.textContent = "server error " + res.status + ": " + await res.text() + " · the drawing is from the last good render"; return; }
      out = await res.json();
    } catch (e) { if (run !== renderRun) return; status.textContent = "no server: " + e.message + " (cargo run -p exp06) · the drawing is from the last good render"; return; }
    if (run !== renderRun) return;
    sent = body;
    draw(out);
    status.textContent = "server " + out.ms.toFixed(2) + " ms (" + out.path + ") · round trip " + (performance.now() - t0).toFixed(0) + " ms";
  }

  function draw(out) {
    const o = out.outline, origin = out.origin, corners = out.corners;
    // Wide view: fit the whole banana
    wide.replaceChildren();
    const xs = o.map(p => p[0]), ys = o.map(p => p[1]);
    const cx = (Math.min(...xs) + Math.max(...xs)) / 2, cy = (Math.min(...ys) + Math.max(...ys)) / 2;
    const span = Math.max(Math.max(...xs) - Math.min(...xs), Math.max(...ys) - Math.min(...ys)) + 60;
    const W = view(cx, cy, 520 / span);
    wide.appendChild(el("rect", { x: 0, y: 0, width: 560, height: 560, fill: "var(--sea)", stroke: "none" }));
    wide.appendChild(el("polygon", { points: pts(o, W), fill: "var(--land)", stroke: "currentColor", "stroke-width": 1.2 }));
    // the centreline arc, and a north arrow at its top tip
    { const ps = out.centreline;
      wide.appendChild(el("polyline", { points: pts(ps, W), stroke: "var(--mountain)", "stroke-width": 1, "stroke-dasharray": "4 4" }));
      const tip = ps[ps.length - 1], t0 = W(tip), t1 = W([tip[0], tip[1] + 24]);
      wide.appendChild(el("line", { x1: t0.x, y1: t0.y, x2: t1.x, y2: t1.y, stroke: "var(--mountain)", "stroke-width": 1.5 }));
      wide.appendChild(el("text", { x: t1.x, y: t1.y - 4, "text-anchor": "middle", fill: "var(--mountain)", ...mono, "font-size": 10 }, "90°")); }
    for (const q of out.marks) {
      const s = W(q.p), o2 = W([q.p[0] + q.normal[0] * 14, q.p[1] + q.normal[1] * 14]);
      wide.appendChild(el("line", { x1: s.x, y1: s.y, x2: o2.x, y2: o2.y, stroke: "var(--muted)" }));
      wide.appendChild(el("text", { x: o2.x + (o2.x - s.x) * 0.9, y: o2.y + (o2.y - s.y) * 0.9 + 4, "text-anchor": "middle", fill: "var(--muted)", ...mono, "font-size": 10 }, q.deg + "°"));
    }
    wide.appendChild(el("polygon", { points: pts(corners, W), fill: "var(--world)", "fill-opacity": 0.35, stroke: "var(--world)", "stroke-width": 1.5 }));
    { const s = W(out.here); wide.appendChild(el("circle", { cx: s.x, cy: s.y, r: 3, fill: "var(--shore)", stroke: "none" })); }
    wide.appendChild(el("text", { x: 10, y: 548, fill: "var(--muted)", ...mono }, "outline " + out.length.toFixed(0) + " ri round · 1° of θ = " + (out.length / 360).toFixed(2) + " ri · R " + out.banana.r + ", w " + out.banana.w + ", A " + out.banana.a + "° · top tip heads north"));

    // Near view: the world and its surroundings
    near.replaceChildren();
    const N = view(origin[0], origin[1], 520 / (WORLD * 2.2));
    near.appendChild(el("rect", { x: 0, y: 0, width: 560, height: 560, fill: "var(--sea)", stroke: "none" }));
    near.appendChild(el("polygon", { points: pts(o, N), fill: "var(--land)", stroke: "currentColor", "stroke-width": 1.5 }));
    for (const e of out.edges) {
      const a = N(e.a), b = N(e.b);
      const colour = e.kind === "shore" ? "var(--shore)" : e.kind === "land" ? "var(--mountain)" : "currentColor";
      near.appendChild(el("line", { x1: a.x, y1: a.y, x2: b.x, y2: b.y, stroke: colour, "stroke-width": e.kind === "sea" ? 2 : 4, opacity: e.kind === "sea" ? 0.5 : 1 }));
      const m = N([(e.a[0] + e.b[0]) / 2, (e.a[1] + e.b[1]) / 2]), c = N(origin);
      const lab = { x: m.x + (m.x - c.x) * 0.16, y: m.y + (m.y - c.y) * 0.16 + 4 };
      near.appendChild(el("text", { x: lab.x, y: lab.y, "text-anchor": "middle", fill: colour, ...mono }, e.name + " · " + e.kind));
    }
    for (const e of out.edges) for (const x of e.crossings) { const s = N(x); near.appendChild(el("circle", { cx: s.x, cy: s.y, r: 5, fill: "var(--shore)", stroke: "var(--bg)", "stroke-width": 1.5 })); }
    { const s = N(origin); near.appendChild(el("circle", { cx: s.x, cy: s.y, r: 4, fill: "var(--world)", stroke: "none" })); near.appendChild(el("text", { x: s.x + 8, y: s.y + 4, fill: "var(--world)", ...mono }, "origin")); }
    { const s = N(out.here); near.appendChild(el("circle", { cx: s.x, cy: s.y, r: 3, fill: "var(--shore)", stroke: "none" })); near.appendChild(el("text", { x: s.x + 8, y: s.y - 6, fill: "var(--shore)", ...mono, "font-size": 10 }, "θ = " + sent.theta.toFixed(1) + "°")); }
    near.appendChild(el("text", { x: 10, y: 548, fill: "var(--muted)", ...mono }, "36 ri across · north up"));

    facts.replaceChildren();
    const row = (k, val, bad) => { const d = document.createElement("div"); const a = document.createElement("span"); a.textContent = k; const b = document.createElement("span"); b.textContent = val; if (bad) b.className = "bad"; d.append(a, b); facts.appendChild(d); };
    for (const f of out.facts) row(f.key, f.value, f.bad);
  }

  update();
})();
</script>
</body>
</html>
```

- [ ] **Step 3: Run the GET test, then the page in a browser**

Run: `cd /home/lexa/DevProjects/_GameDev/Murabito/.claude/worktrees/exp-06-worldgen/Experiments/exp-06 && cargo test -p exp06`
Expected: all pass. Then `cargo run -p exp06` in the background of your own shell, and check with `curl -s http://127.0.0.1:8060/ | head -5` that the page comes back (Lexa opens it in the browser; there is no screenshot tool). Stop the server you started.

- [ ] **Step 4: Write the port's harness**

`Experiments/exp-06/viz/tools/check-port.js`:

```js
// Ask the running exp-06 server for a render and print the facts rows the way check.js prints banana.html's,
// so the two outputs diff. Usage: node viz/tools/check-port.js [key=value ...]   e.g. theta=200 inset=-4 path=f32
// The server: cargo run -p exp06 (EXP06=http://127.0.0.1:8060 to point elsewhere).
const base = process.env.EXP06 || "http://127.0.0.1:8060";
const settings = {};
for (const kv of process.argv.slice(2)) {
  const [k, v] = kv.split("=");
  settings[k] = k === "path" ? v : k === "windowQRS" ? v.split(",").map(Number) : +v;
}
(async () => {
  let res;
  try { res = await fetch(base + "/render", { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(settings) }); }
  catch (e) { console.error("no server at", base, "-", e.message, "(cargo run -p exp06)"); process.exit(1); }
  if (!res.ok) { console.error("server", res.status, await res.text()); process.exit(1); }
  const out = await res.json();
  console.log("render ms", out.ms.toFixed(2), "path", out.path);
  console.log("-- facts");
  for (const f of out.facts) console.log(f.key + " | " + f.value);
})();
```

With the server running, from `Experiments/exp-06`:

```bash
node viz/tools/check.js viz/banana.html | sed -n '/^-- facts$/,/^-- facts2$/p' | sed '$d' > target/banana-facts.txt
node viz/tools/check-port.js | sed -n '/^-- facts$/,$p' > target/port-facts.txt
diff target/banana-facts.txt target/port-facts.txt && echo SAME
```

Expected: `SAME`. Also `node viz/tools/check-port.js path=f32` prints the same six rows.

- [ ] **Step 5: Write the README**

`Experiments/exp-06/README.md`:

```markdown
# exp-06: worldgen

The Murabito world as pure functions of seed and position, in Rust, checked cell by cell against the design
walkthrough on `viz/banana.html`. The spec is `docs/specs/2026-09-30-exp-06-worldgen-port-design.md`; the state
and history are `docs/HANDOFF.md`; the numbers the port must reproduce are `docs/reference-readings.md`.

## Layout

- `crates/worldgen`: the engine-free crate, no dependencies. `hex` (cube coordinates, directions, packing),
  `settings`, `format` are shared. The numeric modules (`plane`, `banana`, `facts`, and the steps to come) are one
  source compiled twice: `worldgen::f64` is the reference and `worldgen::f32` the shader's twin. Each twin fixes
  `type F`, `PI`, `SQRT_3`, `TOL` and `rad`, and includes the files by `#[path]`. Write float literals untyped
  (`2.0`), convert integers with `as F`, and the same line serves both.
- `crates/exp06`: the local server. `cargo run -p exp06`, then open `http://127.0.0.1:8060/`. It serves
  `viz/exp-06.html` from disk (edit the page, reload) and answers `POST /render` with what the page draws.
- `viz/banana.html`: the reference page, opened from disk. `viz/exp-06.html`: the port's page. Both read and
  write the same settings in the URL query, so one link with the other filename is the same world.
- `viz/tools/check.js`: renders `banana.html` headlessly and prints every facts row. `viz/tools/check-port.js`:
  prints the port's facts rows from the running server, in the same format, for a `diff`.

## Build, test, check

From this directory (never from the repo root):

    cargo test --workspace
    cargo clippy --workspace --all-targets
    cargo run -p exp06

    node viz/tools/check.js viz/banana.html | sed -n '/^-- facts$/,/^-- facts2$/p' | sed '$d' > target/banana-facts.txt
    node viz/tools/check-port.js | sed -n '/^-- facts$/,$p' > target/port-facts.txt
    diff target/banana-facts.txt target/port-facts.txt && echo SAME

## Rules

Cube `(q, r, s)` everywhere in runtime code. The API names cells at a rung; the plane is derived inside.
Sizes Lexa fixed are constants. One bite at a time, each checked against the page. Commit only when Lexa says.
```

- [ ] **Step 6: Leave the work in the tree**

No commit. Report the `diff` result and the test output.

---

### Task 7: the URL query on `banana.html`, the spec's key names

**Files:**
- Modify: `Experiments/exp-06/viz/banana.html` (the `state` block near line 349, and `update`/`scheduleUpdate` near lines 421 and 424)
- Modify: `Experiments/exp-06/docs/specs/2026-09-30-exp-06-worldgen-port-design.md` §1.2, the list of query keys

**Interfaces:**
- Produces: `banana.html` reads `?theta=…&inset=…&…&windowQRS=q,r,s` on load and rewrites the query on every change, with the same keys `exp-06.html` uses.

- [ ] **Step 1: Record the harness output before the change**

Run: `cd /home/lexa/DevProjects/_GameDev/Murabito/.claude/worktrees/exp-06-worldgen/Experiments/exp-06 && node viz/tools/check.js viz/banana.html > target/check-before.txt`
Expected: about 3 s, the full readings.

- [ ] **Step 2: Add the query to `banana.html`**

Directly after the line that begins `const state = { theta: 12, inset: 10, seed: 1,` insert:

```js
  // Settings in the URL query, the same keys as viz/exp-06.html, so the side-by-side is one link each (the port, 2026-09-30).
  const QUERY_KEYS = Object.keys(state);
  if (typeof location !== "undefined") { const qs = new URLSearchParams(location.search); for (const k of QUERY_KEYS) if (qs.has(k)) state[k] = k === "windowQRS" ? qs.get(k).split(",").map(Number) : +qs.get(k); }
  function writeQuery() { if (typeof history === "undefined") return; const qs = new URLSearchParams(); for (const k of QUERY_KEYS) { const v = state[k]; if (v === null) continue; qs.set(k, Array.isArray(v) ? v.join(",") : String(v)); } try { history.replaceState(null, "", "?" + qs); } catch (e) { /* a file: URL may refuse */ } }
```

Then change

```js
  function update() { controls.forEach(c => c.sync()); render(); }
```

to

```js
  function update() { controls.forEach(c => c.sync()); writeQuery(); render(); }
```

and

```js
  function scheduleUpdate() { controls.forEach(c => c.sync()); clearTimeout(pendingRender); pendingRender = setTimeout(render, 150); }
```

to

```js
  function scheduleUpdate() { controls.forEach(c => c.sync()); writeQuery(); clearTimeout(pendingRender); pendingRender = setTimeout(render, 150); }
```

Nothing else in the file changes.

- [ ] **Step 3: Check the harness output is unchanged**

Run: `cd /home/lexa/DevProjects/_GameDev/Murabito/.claude/worktrees/exp-06-worldgen/Experiments/exp-06 && node viz/tools/check.js viz/banana.html > target/check-after.txt && diff <(grep -v '^render ms' target/check-before.txt) <(grep -v '^render ms' target/check-after.txt) && echo UNCHANGED`

If the shell refuses process substitution, write the two filtered files with `grep -v '^render ms' … > …` and `diff` them. Expected: `UNCHANGED` (the render-time line differs by a few ms and is excluded). Also: `git -C /home/lexa/DevProjects/_GameDev/Murabito/.claude/worktrees/exp-06-worldgen diff --stat -- Experiments/exp-06/viz/banana.html` shows only the lines above changed.

- [ ] **Step 4: Update the spec's key list**

In the spec's §1.2 replace the parenthesised key list

```
(`seed, theta, inset, amp, hurst, stretch, erosion, firstOctave, octaves, stripes, sigmaRef, stacking, normAbove, baseStrength, keepDeeper, window` as `q,r,s`)
```

with the page's own key names, which both pages use:

```
(the page's state keys: `theta, inset, seed, amp, hurst, stretch, gref, erosion, firstOctave, octaves5, stripes, sigmaRef, stacking, normAbove, baseStrength, keepDeeper, windowQRS` as `q,r,s`, plus `path` on `exp-06.html`)
```

- [ ] **Step 5: The side-by-side, for Lexa**

With the server running, the two links to open are:

- `file:///home/lexa/DevProjects/_GameDev/Murabito/.claude/worktrees/exp-06-worldgen/Experiments/exp-06/viz/banana.html?theta=12&inset=10`
- `http://127.0.0.1:8060/?theta=12&inset=10`

Moving a slider on either rewrites its query; pasting one page's query onto the other shows the same world. Report both links and the `diff` results; Lexa looks and says "commit it".

- [ ] **Step 6: Leave the work in the tree**

No commit. Report.

---

## Self-review

**Spec coverage for bite one** (spec §9, bite 1: the workspace, `worldgen::hex`, `exp06` serving a page that shows step 1 beside banana, the URL query on both pages): Task 1 the workspace and `hex` with 1,296 / 100 / 36 and the tie rule (§2); Task 2 the twins (§1.1, f64 and f32 from one source); Task 3 step 1 (§3: outline, `at`, the origin, the tangent, edge kinds); Task 4 settings with the page's defaults, the facts as strings (§1.2), JS number formatting for later rows; Task 5 the server and `/render` (§1.2); Task 6 the page and the harness's second mode (§9.2); Task 7 the query on `banana.html`, the one change it receives (§1.2). Band coordinates and `height_at_plane_point` are bite two (step 2). The GPU crate is bite four.

**Placeholders:** none; every step carries its code, and every pinned number was printed from the page.

**Type consistency:** `Cube::new(q, r, s)` throughout; `Direction::name()` returns `&'static str`; `EdgeKind::name()` likewise; `FactRow { key, value, bad }` in `worldgen::format` used by both twins' `facts` and mapped to `FactOut`; `RenderRequest::settings()` builds `worldgen::settings::Settings` with `octaves5 → octaves` and `windowQRS → window`; `banana::marks()` returns `Vec<(F, OutlinePoint)>` and the wire carries `MarkOut { deg, p, normal }`, which the page reads as `q.deg`, `q.p`, `q.normal`.

**Review Focus:** item 1 in Task 3 (`theta_at_the_sliders_ends`), item 2 in Task 4 (`far_out_to_sea_the_rule_breaks_and_says_so`), items 3 and 4 in Task 5, item 5 in Tasks 5 (`render_takes_the_pages_keys_and_partial_bodies`) and 7 (the harness diff, and Lexa's browser check).

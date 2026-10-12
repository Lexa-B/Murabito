# exp-05 main-coordinates conversion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move exp-05 onto the main project's coordinate conventions:
- a vendored `murabito_hexcoords`;
- cube-only `Hex`, with +r pointing SSE;
- world space as Bevy `Vec3` in shaku, `f32`;
- the layer fixed at 0.5 shaku.

**Architecture:** Main's `crates/hexcoords/` is copied into exp-05 unchanged, and `hexworld` builds on it. The integer label maths (packing, ownership, drawn sets, windows, the handover) is untouched. Only the edges change: the `Hex` API, the label-to-position formula, the direction table, and the unit and type of every position. The conversion goes in orientation-then-units order, so each task has a clear test story.

**Tech Stack:** Rust (stable, per `rust-toolchain.toml`), Bevy 0.19.1, bevy_egui 0.42. Cargo workspace at `Experiments/exp-05/`.

**Spec:** `Experiments/exp-05/docs/specs/2026-09-22-exp-05-main-coords-design.md`. It amends `Experiments/exp-05/docs/specs/2026-09-20-exp-05-hex-voxel-chunks-design.md`. Read both.

## Global Constraints

- **Worktree:** all work happens in `/home/lexa/DevProjects/_GameDev/Murabito/.claude/worktrees/exp-05-main-coords`, on branch `exp-05-main-coords`.
  - Never touch the main checkout at `/home/lexa/DevProjects/_GameDev/Murabito`, and never touch another worktree.
  - Use `git -C <absolute path>` for every git command.
  - Print `pwd` and `git -C … rev-parse --abbrev-ref HEAD` before each commit.
- **Build and test from `Experiments/exp-05`:** `cd /home/lexa/DevProjects/_GameDev/Murabito/.claude/worktrees/exp-05-main-coords/Experiments/exp-05 && cargo test`. From the repo root, cargo builds the main project instead.
  - Never put a target directory under `/tmp`.
  - Check cargo's own exit status. Don't trust the exit code of a `| tail`.
- **Never push, and never open a PR.**
- **Never edit `crates/hexcoords/src/lib.rs` in exp-05.** It is a byte-for-byte copy of main's `crates/hexcoords/src/lib.rs` at commit `79335cd`. If something seems to need a change there, stop and report it.
- **`hexworld` depends only on `murabito_hexcoords` and on `bevy` with `default-features = false`**, for `bevy::math::Vec3`. It holds no rendering, ECS or asset code.
- **The API speaks cube.** Code outside `hexworld/src` (integration tests, `hexworld_bevy`, `viewer`) builds a `Hex` only with `Hex::new(q, r, s)`. Inside `hexworld/src`, the crate-private `Hex::axial(q, r)` is allowed.
- **Units:**
  - World space is `Vec3` in shaku, `f32`, with Y up. East is +X, north is −Z.
  - `+q` is east, and `+r` is SSE (+Z, down the screen).
  - Conversions: 1 m = 3.3 shaku, and 1 shaku = 10/33 m (`SHAKU_M`).
  - The layer height is 0.5 shaku (5 sun), fixed.
- **Label-level tests must not change what they assert.** That covers the exp-03 fixtures, ownership, drawn-set counts, window counts and handover logic. If one fails after a task, the task is wrong, not the test.
- **Tests that pin orientation or units** are rewritten deliberately, and only in the task that says so.
- **Style:** match the surrounding code. Keep comment density as it is. Doc comments say what and why.
  - Don't name games that inspired the project.
  - Run `cargo fmt` and `cargo clippy --workspace --all-targets` clean at the end of every task.
- **Commit messages** end with the line `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>`.

## File map

| File | Change |
|---|---|
| `crates/hexcoords/{Cargo.toml, README.md, src/lib.rs}` | new, vendored (Task 1) |
| `Cargo.toml` (workspace) | add the member (Task 1) |
| `crates/hexworld/Cargo.toml` | add the dependencies (Task 1) |
| `crates/hexworld/src/hex.rs` | cube API (Task 2), directions from `Direction` (Task 3) |
| `crates/hexworld/src/plane.rs` | +r SSE (Task 3), `Vec3` shaku (Task 4) |
| `crates/hexworld/src/layer.rs` | new: the fixed layer height and height↔layer (Task 4) |
| `crates/hexworld/src/{level, config, column, placeholder_terrain, noise, store, mesh, address, lib}.rs` | shaku, `f32` and `Vec3` (Task 4) |
| `crates/hexworld/src/{owner, chunk, world}.rs` | getters and `Hex::axial` only (Task 2) |
| `crates/hexworld/tests/{exp03_agreement, handover}.rs` | cube constructor (Task 2), `Vec3` (Task 4) |
| `crates/hexworld_bevy/src/axes.rs` | deleted (Task 5) |
| `crates/hexworld_bevy/src/{lib, loader, entities}.rs`, `tests/*.rs` | cube constructor (Task 2), `Vec3` (Task 5) |
| `crates/viewer/src/{camera, panel, main, headless}.rs` | cube constructor (Task 2), `Vec3` shaku (Task 5) |
| `AGENTS.md` (repo root), `Experiments/exp-05/README.md`, `Experiments/manifest.md`, the original spec | docs (Tasks 1 and 6) |

---

### Task 1: Vendor `murabito_hexcoords`, wire it in, and record a baseline

**Files:**
- Create: `Experiments/exp-05/crates/hexcoords/src/lib.rs` (copied), `Experiments/exp-05/crates/hexcoords/Cargo.toml`, `Experiments/exp-05/crates/hexcoords/README.md`
- Modify: `Experiments/exp-05/Cargo.toml`, `Experiments/exp-05/crates/hexworld/Cargo.toml`, `Experiments/exp-05/crates/hexworld/src/lib.rs` (the crate doc comment and re-exports), and `AGENTS.md` at the repo root (the "Rust + Bevy experiments" section).

**Interfaces:**
- Produces:
  - crate `murabito_hexcoords`, with `VoxelCoord`, `VoxelspacePos`, `Direction`, `NotOnHexPlane` and `ON_PLANE_TOLERANCE`;
  - `hexworld` re-exports `pub use murabito_hexcoords::{Direction, NotOnHexPlane, VoxelCoord, VoxelspacePos};`, so the plugin and viewer can name them without their own dependency.

- [ ] **Step 1: Capture baseline screenshots of the current viewer**, before anything changes, for comparison in Task 6.

```bash
cd /home/lexa/DevProjects/_GameDev/Murabito/.claude/worktrees/exp-05-main-coords/Experiments/exp-05
export $(systemctl --user show-environment | rg '^(DISPLAY|XAUTHORITY)=' | xargs)
SHOTS=/tmp/claude-1000/-home-lexa-DevProjects--GameDev-Murabito/bb7eaf7e-a996-4e45-8d06-6915e27b5ca9/scratchpad/baseline
mkdir -p $SHOTS/near $SHOTS/far
cargo run -p viewer --release -- --frames 240 --screenshot-dir $SHOTS/near --screenshot-every 120 --zoom 120 --line-mode nested
cargo run -p viewer --release -- --frames 240 --screenshot-dir $SHOTS/far --screenshot-every 120 --zoom 2500 --line-mode nested
ls $SHOTS/near $SHOTS/far
```

Expected: PNGs in both directories. If the flag names differ from `crates/viewer/src/headless.rs`, use the real names and note them in the report.

- [ ] **Step 2: Copy the crate source byte for byte**

```bash
W=/home/lexa/DevProjects/_GameDev/Murabito/.claude/worktrees/exp-05-main-coords
mkdir -p $W/Experiments/exp-05/crates/hexcoords/src
git -C $W show 79335cd:crates/hexcoords/src/lib.rs > $W/Experiments/exp-05/crates/hexcoords/src/lib.rs
cmp <(git -C $W show 79335cd:crates/hexcoords/src/lib.rs) $W/Experiments/exp-05/crates/hexcoords/src/lib.rs && echo IDENTICAL
```

Expected: `IDENTICAL`.

- [ ] **Step 3: Write `crates/hexcoords/Cargo.toml`**

```toml
[package]
name = "murabito_hexcoords"
version.workspace = true
edition = "2024"
license.workspace = true

# Vendored from the main project's crates/hexcoords at 79335cd: see README.md.
# Only this manifest differs from main's; src/lib.rs is a byte-for-byte copy.
[dependencies]
bevy = { version = "0.19", default-features = false }
```

- [ ] **Step 4: Write `crates/hexcoords/README.md`**

```markdown
# murabito_hexcoords (vendored)

A copy of the main project's `crates/hexcoords/`, taken at commit `79335cd`, so that exp-05 speaks the same coordinates as the main project while staying self-contained.

- `src/lib.rs` is a byte-for-byte copy. Don't edit it here: change it in the main project, then copy it again and update the commit above.
- `Cargo.toml` differs from main's only because exp-05's workspace is not main's: the edition is set on the crate, and Bevy comes without default features, since only `bevy::math` is used.
- The conventions it implements are written up in the main project's `Docs/hex_units.md`.

Check that the copy is still identical:

    git show 79335cd:crates/hexcoords/src/lib.rs | cmp - crates/hexcoords/src/lib.rs
```

- [ ] **Step 5: Add the workspace member and `hexworld`'s dependencies**

In `Experiments/exp-05/Cargo.toml`: `members = ["crates/hexcoords", "crates/hexworld", "crates/hexworld_bevy", "crates/viewer"]`.

In `crates/hexworld/Cargo.toml`, replace the empty `[dependencies]` with:

```toml
[dependencies]
murabito_hexcoords = { path = "../hexcoords" }
bevy = { version = "0.19", default-features = false }
```

- [ ] **Step 6: Update `hexworld/src/lib.rs`'s crate doc and add the re-export**

Replace the doc comment's paragraph "This crate is engine-free on purpose: no rendering, no ECS, no dependencies. Positions are `(east, north, height)` in metres; mapping to an engine's axes is the caller's job." with the following (positions are still metres until Task 4, which rewrites the second sentence):

```rust
//! This crate holds no rendering, ECS or asset code. Its only dependencies are
//! `murabito_hexcoords`, the main project's coordinate crate (vendored), and Bevy's math
//! types. Positions are `(east, north, height)` in metres; mapping to an engine's axes is
//! the caller's job.
```

Add after the other `pub use` lines:

```rust
pub use murabito_hexcoords::{Direction, NotOnHexPlane, VoxelCoord, VoxelspacePos};
```

- [ ] **Step 7: Update the rule in `AGENTS.md`** (repo root, section "Rust + Bevy experiments"). Replace the bullet starting "**Keep an experiment's core crate engine-free.**" with:

```markdown
- **Keep an experiment's core crate engine-free.** exp-05's `hexworld` holds no rendering, ECS or asset code, and depends only on `murabito_hexcoords` (the main project's coordinate crate, vendored into the experiment) and on `bevy` without default features, for its math types. Engine types and rendering code belong in the plugin crate or the app.
```

- [ ] **Step 8: Build and test**

Run: `cd …/Experiments/exp-05 && cargo test --workspace`

Expected: every existing test passes (132, plus 1 ignored), plus `murabito_hexcoords`'s own tests.

Then run `cargo clippy --workspace --all-targets`. Expected: clean. If clippy reports lints inside `crates/hexcoords` because exp-05's toolchain differs from main's pinned 1.95.0, report them; don't edit the file.

- [ ] **Step 9: Commit**

```bash
W=/home/lexa/DevProjects/_GameDev/Murabito/.claude/worktrees/exp-05-main-coords
pwd; git -C $W rev-parse --abbrev-ref HEAD
git -C $W add Experiments/exp-05/Cargo.toml Experiments/exp-05/Cargo.lock Experiments/exp-05/crates/hexcoords Experiments/exp-05/crates/hexworld/Cargo.toml Experiments/exp-05/crates/hexworld/src/lib.rs AGENTS.md
git -C $W commit -m "exp-05: vendor murabito_hexcoords and depend on it from hexworld

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 2: `Hex` takes main's shape (cube-only constructor, private fields)

No behaviour changes. Positions, directions and orientation stay exactly as they are.

**Files:**
- Modify: `crates/hexworld/src/hex.rs`, plus every `Hex::new(q, r)` call and every `.q` / `.r` field access across the workspace. Today that's:
  - in `hexworld/src`: `address`, `chunk`, `hex`, `mesh`, `owner`, `placeholder_terrain`, `plane`, `store`, `world`;
  - `hexworld/tests/{exp03_agreement, handover}.rs`;
  - `hexworld_bevy/src/entities.rs`;
  - `hexworld_bevy/tests/{handover, plugin}.rs`;
  - wherever the viewer uses one.

  Find them all with `rg -n 'Hex::new|Hex \{|\.q\b|\.r\b' crates`.

**Interfaces:**
- Produces:
  - `Hex::new(q: i32, r: i32, s: i32) -> Result<Hex, NotOnHexPlane>`;
  - `pub(crate) const fn Hex::axial(q: i32, r: i32) -> Hex`;
  - `pub const fn q(self) -> i32`, `r(self)` and `s(self)`;
  - `impl fmt::Display for Hex`, written `(q, r, s)`;
  - `Hex::ZERO`, `Add`, `Sub`, `Ord` (lexicographic `(q, r)`, from the field order), `Hash` and `Default` stay.

- [ ] **Step 1: Write the failing tests** in `hex.rs`'s test module:

```rust
    #[test]
    fn the_cube_constructor_checks_the_plane() {
        let h = Hex::new(2, -5, 3).expect("sums to zero");
        assert_eq!((h.q(), h.r(), h.s()), (2, -5, 3));
        let err = Hex::new(1, 1, 1).unwrap_err();
        assert_eq!((err.q, err.r, err.s), (1, 1, 1));
    }

    #[test]
    fn the_cube_constructor_sums_without_overflowing() {
        // i32::MAX + 1 + i32::MIN is exactly 0; i32::MAX + i32::MAX wraps to -2 in i32,
        // and must not be mistaken for anything on the plane.
        assert!(Hex::new(i32::MAX, 1, i32::MIN).is_ok());
        assert!(Hex::new(i32::MAX, i32::MAX, 0).is_err());
    }

    #[test]
    fn axial_and_cube_agree() {
        assert_eq!(Hex::axial(4, -9), Hex::new(4, -9, 5).unwrap());
    }

    #[test]
    fn a_hex_displays_as_its_cube_triple() {
        assert_eq!(Hex::axial(3, -1).to_string(), "(3, -1, -2)");
    }
```

- [ ] **Step 2: Run them and confirm they fail**

Run: `cargo test -p hexworld hex::tests`

Expected: compile errors (`new` takes 2 arguments, and there's no `axial`).

- [ ] **Step 3: Replace the type definition in `hex.rs`**

```rust
use std::fmt;

use murabito_hexcoords::NotOnHexPlane;

/// A cell at any level of the hierarchy. The API speaks cube, `(q, r, s)` with
/// `q + r + s = 0`, and stores axial `(q, r)`, as `murabito_hexcoords::VoxelCoord` does.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord, Default)]
pub struct Hex {
    q: i32,
    r: i32,
}

impl Hex {
    pub const ZERO: Hex = Hex { q: 0, r: 0 };

    /// A cell from cube coordinates, which must sum to zero. The only public way in.
    /// Summed in `i64`, so no three `i32`s can wrap around to a false zero.
    pub fn new(q: i32, r: i32, s: i32) -> Result<Self, NotOnHexPlane> {
        if i64::from(q) + i64::from(r) + i64::from(s) != 0 {
            return Err(NotOnHexPlane { q, r, s });
        }
        Ok(Hex { q, r })
    }

    /// A cell from its stored axial pair. Crate-private: the packing and ownership maths
    /// works in `(q, r)` throughout, and everything outside the crate goes through `new`.
    pub(crate) const fn axial(q: i32, r: i32) -> Self {
        Hex { q, r }
    }

    pub const fn q(self) -> i32 {
        self.q
    }

    pub const fn r(self) -> i32 {
        self.r
    }

    /// The third cube coordinate, `-q - r`.
    pub const fn s(self) -> i32 {
        -self.q - self.r
    }
}

impl fmt::Display for Hex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {}, {})", self.q, self.r, self.s())
    }
}
```

`Add` and `Sub` use `Hex::axial(self.q + o.q, self.r + o.r)`. `DIRECTIONS` keeps its current six values, written with `Hex::axial`; Task 3 changes them.

- [ ] **Step 4: Update every call site.** Mechanical, and the compiler will list them.
  - **Inside `hexworld/src`**, including its unit tests: `Hex::new(a, b)` becomes `Hex::axial(a, b)`, and `h.q` / `h.r` become `h.q()` / `h.r()`.
  - **Outside `hexworld/src`** (integration tests, `hexworld_bevy`, `viewer`): `Hex::new(a, b)` becomes `Hex::new(a, b, -(a) - (b)).unwrap()`, written with the literal third value. For example, `Hex::new(500, -300)` becomes `Hex::new(500, -300, -200).unwrap()`. Field accesses become getters.
  - **`Address`'s `Display`** writes each part with `Hex`'s `Display`: `"ri {} / cho {} / ken {} / shaku {} / layer {}"`. Update any test that checks the old `(q,r)` text to the new cube text; only the text changes, not the cells.
  - **The fixture parser** in `tests/exp03_agreement.rs` reads axial pairs (storage) and builds each label with `Hex::new(q, r, -q - r).expect("fixture labels are on the plane")`. The fixture file doesn't change.
  - **`entities.rs`'s `sort_by_key(|(dir, _)| (dir.q, dir.r))`** becomes `(dir.q(), dir.r())`.
  - If a `const` context needed `Hex::new` (a `const` item outside `hexworld/src`), turn it into a `fn` or a local `let`; `new` is not `const fn`, because it returns a `Result`.

- [ ] **Step 5: Run the whole suite**

Run: `cd …/Experiments/exp-05 && cargo test --workspace`

Expected: everything passes, the four new tests included. No existing assertion changed, apart from the address text format.

Then run `cargo fmt --all && cargo clippy --workspace --all-targets`. Expected: clean.

- [ ] **Step 6: Commit** with the message `exp-05: make Hex cube-only with private fields, as in murabito_hexcoords`, after the `pwd` and branch check.

---

### Task 3: +r points SSE, and the directions come from `Direction`

Still in metres. This task changes which label sits where, and nothing else.

**Files:**
- Modify: `crates/hexworld/src/plane.rs` (`cell_centre_m`, `metres_to_axial`, tests), `crates/hexworld/src/hex.rs` (`DIRECTIONS`, the new `EDGES` and `step`, tests), and `crates/hexworld/src/lib.rs` (exports).

**Interfaces:**
- Consumes: `murabito_hexcoords::{Direction, VoxelCoord}` (Task 1); `Hex::axial` (Task 2).
- Produces:
  - `pub const EDGES: [Direction; 6]`: `E, NNE, NNW, W, SSW, SSE`, in index order;
  - `pub fn step(direction: Direction) -> Hex`: the axial step for any of the 12 directions;
  - `pub static DIRECTIONS: LazyLock<[Hex; 6]>`: `EDGES.map(step)`. It is indexed and iterated as before (`DIRECTIONS[k]`, `DIRECTIONS.iter()`). A by-value loop needs `*DIRECTIONS`.

- [ ] **Step 1: Rewrite the orientation tests first**

In `hex.rs`, replace `directions_run_anticlockwise_from_east` and add two tests:

```rust
    #[test]
    fn directions_run_anticlockwise_from_east() {
        // Direction k must point at 60 degrees times k, anticlockwise with north up: the
        // mesher pairs each neighbour with a cell edge by that order. +r is SSE, so a
        // cell's north coordinate is -r·√3/2.
        let angle = |h: Hex| {
            let (e, n) = (
                h.q() as f64 + h.r() as f64 / 2.0,
                -(h.r() as f64) * 3f64.sqrt() / 2.0,
            );
            n.atan2(e).to_degrees().rem_euclid(360.0)
        };
        for (k, d) in DIRECTIONS.iter().enumerate() {
            assert!(
                (angle(*d) - 60.0 * k as f64).abs() < 1e-9,
                "direction {k} is {d:?}"
            );
        }
    }

    #[test]
    fn the_edges_are_directions_even_indices_in_order() {
        for (k, d) in EDGES.iter().enumerate() {
            assert_eq!(usize::from(d.index()), 2 * k, "{d:?}");
            assert!(d.is_edge());
        }
    }

    #[test]
    fn the_edge_steps_match_main_table() {
        // Docs/hex_units.md in the main project: E, NNE, NNW, W, SSW, SSE.
        let expected = [(1, 0), (1, -1), (0, -1), (-1, 0), (-1, 1), (0, 1)];
        let got: Vec<(i32, i32)> = DIRECTIONS.iter().map(|h| (h.q(), h.r())).collect();
        assert_eq!(got, expected);
    }
```

In `plane.rs`, add:

```rust
    #[test]
    fn a_plus_r_step_is_half_a_cell_east_and_a_row_south() {
        // As murabito_hexcoords' `one_step_along_r_is_half_a_cell_right_and_a_row_down_the_screen`.
        let w = Level::Shaku.width_m();
        let (e, n) = cell_centre_m(Hex::axial(0, 1), Level::Shaku);
        assert!((e - w / 2.0).abs() < 1e-12, "{e}");
        assert!((n + w * 3f64.sqrt() / 2.0).abs() < 1e-12, "{n}");
    }
```

- [ ] **Step 2: Run them and confirm the right ones fail**

Run: `cargo test -p hexworld hex:: plane::`

Expected: `EDGES` doesn't exist yet (a compile error). Stub it to see the rest fail: `the_edge_steps_match_main_table` and `a_plus_r_step…` fail.

- [ ] **Step 3: Implement.** In `hex.rs`, replace the `DIRECTIONS` const:

```rust
use std::sync::LazyLock;

use murabito_hexcoords::{Direction, VoxelCoord};

/// The six edge directions, in `Direction`'s index order: anticlockwise from east, 60°
/// apart. The mesher pairs edge `k` with the cell edge between corners `k-1` and `k`, so
/// this order and `plane::corners_m` go together.
pub const EDGES: [Direction; 6] = [
    Direction::E,
    Direction::NNE,
    Direction::NNW,
    Direction::W,
    Direction::SSW,
    Direction::SSE,
];

/// The axial step one cell in a direction, read from `murabito_hexcoords` so that the
/// main project's direction table stays the only one.
pub fn step(direction: Direction) -> Hex {
    let origin = VoxelCoord::new(0, 0, 0, 0).expect("the origin is on the plane");
    let next = origin.neighbour(direction);
    Hex::axial(next.q(), next.r())
}

/// The six unit steps, `EDGES` as cells: direction `k` points at 60°·k.
pub static DIRECTIONS: LazyLock<[Hex; 6]> = LazyLock::new(|| EDGES.map(step));
```

Update users: `neighbours` becomes `DIRECTIONS.map(|d| h + d)`. `for d in DIRECTIONS` becomes `for d in *DIRECTIONS`. Anything else the compiler flags gets the same treatment.

In `plane.rs`:

```rust
/// The centre of a cell, in metres east and north of the world centre. `+r` is SSE,
/// as in `murabito_hexcoords`: half a cell east and one row south.
pub fn cell_centre_m(cell: Hex, level: Level) -> (f64, f64) {
    let w = level.width_m();
    let q = cell.q() as f64;
    let r = cell.r() as f64;
    ((q + r / 2.0) * w, -r * SQRT3 / 2.0 * w)
}

/// Fractional axial coordinates of a point at a level.
pub fn metres_to_axial(east: f64, north: f64, level: Level) -> (f64, f64) {
    let w = level.width_m();
    let r = -north / (SQRT3 / 2.0 * w);
    (east / w - r / 2.0, r)
}
```

`corners_m` doesn't change: the corners are physical, and the edges are still at 0°, 60°, … anticlockwise. Update the module doc's formula to `(q + r/2, −r·√3/2)`. Export `EDGES` and `step` from `lib.rs`.

- [ ] **Step 4: Run the whole suite**

Run: `cargo test --workspace`

Expected: everything passes. Label-level tests (the fixtures, ownership, counts, windows, the handover) pass untouched.

If a test fails because it hard-codes a **position** that assumed +r was NNE (for example, an expected east/north pair for a labelled cell), update the expected position. Name each such test in the commit body. If a test fails that asserts only labels or counts, stop: the task is wrong. Investigate; don't edit that test.

Then run `cargo fmt --all && cargo clippy --workspace --all-targets`. Expected: clean.

- [ ] **Step 5: Commit** with `exp-05: point +r SSE and take the edge directions from murabito_hexcoords`, listing any position-literal tests that changed.

---

### Task 4: The core works in shaku, as `Vec3` and `f32`, with the layer fixed

Only `hexworld` (the crate, its unit tests and its integration tests) has to build and pass at the end of this task. `hexworld_bevy` and `viewer` are adapted in Task 5, so they won't compile between the two commits. That's expected: check `cargo test -p hexworld`, not the workspace.

**Files:**
- Create: `crates/hexworld/src/layer.rs`
- Modify: `crates/hexworld/src/{level, config, column, placeholder_terrain, noise, plane, store, mesh, address, lib}.rs`, `crates/hexworld/tests/handover.rs`, and any other `hexworld/tests` file the compiler flags.

**Interfaces:**
- Consumes: `murabito_hexcoords::{VoxelCoord, VoxelspacePos}`, `bevy::math::Vec3`; `EDGES`, `DIRECTIONS` (Task 3).
- Produces (the exact names Task 5 uses):
  - `hexworld::layer::{LAYER_HEIGHT: f32 = 0.5, layer_of_height(y: f32) -> i32, layer_bottom(layer: i32) -> f32, layer_top(layer: i32) -> f32}`, re-exported at the crate root;
  - `Level::width(self) -> f32` (shaku), which replaces `width_m`; `SHAKU_M: f32 = 10.0 / 33.0` stays, for display only; `SUN_M` goes;
  - `plane::cell_centre(cell: Hex, level: Level) -> Vec3` (`y = 0`); `plane::round_at(pos: Vec3, level: Level) -> Hex` (ignores `y`); `plane::corners(level: Level) -> [Vec3; 6]` (`y = 0`). Gone: `cell_centre_m`, `corners_m`, `metres_to_axial`, `hex_round`;
  - `WorldConfig { seed: u64, world_radius_ri: i32, bottom_layer: i32 }`. `layer_thickness_sun` and the four `layer_*` methods go;
  - `Column::surface_height(&self) -> f32` (no config argument);
  - `ChunkStore::surface_height(&self, pos: Vec3) -> Option<f32>`;
  - `placeholder_terrain::height(cfg: &WorldConfig, x: f32, z: f32, level: Level) -> f32`, re-exported as `height`;
  - `noise::gradient_2d(seed: u64, x: f32, y: f32) -> f32`;
  - `address_of(voxel: VoxelCoord) -> Address` and `voxel_of(addr: &Address) -> VoxelCoord` (replacing `shaku_of`). `Address`'s fields stay `ri, cho, ken, shaku: Hex, layer: i32`;
  - `MeshData.positions`: chunk-relative, in Bevy's axes `[x, y, z]` and shaku. `normals` are likewise in Bevy's axes (top faces `[0, 1, 0]`).

- [ ] **Step 1: Write `layer.rs` test-first**

```rust
//! The vertical layer: a fixed 0.5 shaku (5 sun), as in `murabito_hexcoords`.

/// Height of one layer, in shaku. `murabito_hexcoords` keeps its own copy private, so a
/// test pins this one to it: if the main project ever changes it, that test fails.
pub const LAYER_HEIGHT: f32 = 0.5;

/// The layer containing a height. Layer k spans [k·h, (k+1)·h).
pub fn layer_of_height(y: f32) -> i32 {
    (y / LAYER_HEIGHT).floor() as i32
}

pub fn layer_bottom(layer: i32) -> f32 {
    layer as f32 * LAYER_HEIGHT
}

pub fn layer_top(layer: i32) -> f32 {
    (layer + 1) as f32 * LAYER_HEIGHT
}

#[cfg(test)]
mod tests {
    use super::*;
    use murabito_hexcoords::VoxelCoord;

    #[test]
    fn the_layer_height_is_main_projects() {
        let one_up = VoxelCoord::new(0, 0, 0, 1).unwrap().to_world();
        assert_eq!(one_up.y, LAYER_HEIGHT);
    }

    #[test]
    fn heights_fall_in_the_layer_below_them() {
        assert_eq!(layer_of_height(0.0), 0);
        assert_eq!(layer_of_height(0.49), 0);
        assert_eq!(layer_of_height(0.5), 1);
        assert_eq!(layer_of_height(-0.01), -1);
        assert_eq!(layer_bottom(3), 1.5);
        assert_eq!(layer_top(3), 2.0);
    }
}
```

Carry over whatever `config.rs`'s layer tests checked, rewritten against these functions, and delete them from `config.rs`.

- [ ] **Step 2: Write the new `plane.rs` tests** in place of the metre ones:

```rust
    #[test]
    fn the_origin_is_the_centre_of_every_level() {
        for level in crate::level::CELL_LEVELS {
            assert_eq!(cell_centre(Hex::ZERO, level), Vec3::ZERO, "{level:?}");
        }
    }

    #[test]
    fn a_step_east_is_one_width_along_x() {
        let p = cell_centre(Hex::axial(1, 0), Level::Ken);
        assert!((p - Vec3::new(6.0, 0.0, 0.0)).length() < 1e-5, "{p}");
    }

    #[test]
    fn a_plus_r_step_is_half_a_cell_east_and_a_row_down_the_screen() {
        // As murabito_hexcoords' `one_step_along_r_is_half_a_cell_right_and_a_row_down_the_screen`.
        let p = cell_centre(Hex::axial(0, 1), Level::Shaku);
        assert!((p - Vec3::new(0.5, 0.0, 3f32.sqrt() / 2.0)).length() < 1e-6, "{p}");
    }

    #[test]
    fn shaku_centres_agree_with_voxel_coords() {
        for (q, r) in [(0, 0), (5, -3), (-17, 41), (4_211, -9_003)] {
            let voxel = VoxelCoord::new(q, r, -q - r, 0).unwrap();
            let p = cell_centre(Hex::axial(q, r), Level::Shaku);
            assert!((p - voxel.to_world()).length() < 1e-3, "{q},{r}");
        }
    }

    #[test]
    fn rounding_inverts_the_centre_at_every_level() {
        for level in crate::level::CELL_LEVELS {
            for cell in [Hex::ZERO, Hex::axial(5, -3), Hex::axial(-17, 41)] {
                assert_eq!(round_at(cell_centre(cell, level), level), cell, "{level:?} {cell:?}");
            }
        }
    }

    #[test]
    fn rounding_picks_the_nearest_centre_and_ignores_height() {
        let c = cell_centre(Hex::axial(3, 2), Level::Ken);
        let nudged = c + Vec3::new(0.1 * Level::Ken.width(), 40.0, 0.0);
        assert_eq!(round_at(nudged, Level::Ken), Hex::axial(3, 2));
    }

    #[test]
    fn corners_are_a_pointy_top_hexagon_with_north_at_minus_z() {
        let corners = corners(Level::Shaku);
        let w = Level::Shaku.width();
        // Pointy top: the northernmost corner (most negative z) is width/√3 out.
        let min_z = corners.iter().map(|c| c.z).fold(f32::MAX, f32::min);
        assert!((min_z + w / 3f32.sqrt()).abs() < 1e-6, "{min_z}");
        let max_x = corners.iter().map(|c| c.x).fold(f32::MIN, f32::max);
        assert!((max_x - w / 2.0).abs() < 1e-6, "{max_x}");
        assert!(corners.iter().all(|c| c.y == 0.0));
        // The first corner is at 30°, anticlockwise on screen: east and a little north.
        assert!(corners[0].x > 0.0 && corners[0].z < 0.0);
    }
```

- [ ] **Step 3: Implement `plane.rs`** (replacing the whole non-test body):

```rust
//! The hex plane in world space: Bevy's `Vec3`, in shaku, with east +X and north −Z, as
//! in `murabito_hexcoords`. Pointy-top cells: the position of axial (q, r), in units of
//! the cell's flat-to-flat width, is x = q + r/2, z = r·√3/2, so +r is SSE.

use std::f32::consts::{FRAC_PI_3, FRAC_PI_6};

use bevy::math::Vec3;
use murabito_hexcoords::VoxelspacePos;

use crate::hex::Hex;
use crate::level::Level;

const ROW_SPACING: f32 = 0.866_025_4; // √3 / 2

/// The centre of a cell on the ground plane (y = 0).
pub fn cell_centre(cell: Hex, level: Level) -> Vec3 {
    let w = level.width();
    let (q, r) = (cell.q() as f32, cell.r() as f32);
    Vec3::new((q + r / 2.0) * w, 0.0, r * ROW_SPACING * w)
}

/// The cell whose ideal hexagon contains the point, at the given level. Height is
/// ignored. Rounding is `murabito_hexcoords`'s, on the point scaled to the level's cells.
pub fn round_at(pos: Vec3, level: Level) -> Hex {
    let w = level.width();
    let voxel = VoxelspacePos::from(Vec3::new(pos.x / w, 0.0, pos.z / w)).round();
    Hex::axial(voxel.q(), voxel.r())
}

/// The six corners of a cell at this level, relative to its centre, anticlockwise on
/// screen from 30° (east and a little north). Corner k-1 to corner k is the edge facing
/// `hex::EDGES[k]`.
pub fn corners(level: Level) -> [Vec3; 6] {
    let radius = level.width() / 3f32.sqrt(); // centre to corner
    std::array::from_fn(|i| {
        let angle = FRAC_PI_6 + i as f32 * FRAC_PI_3;
        Vec3::new(radius * angle.cos(), 0.0, -radius * angle.sin())
    })
}
```

- [ ] **Step 4: `level.rs`, `config.rs`, `column.rs`**
  - **`level.rs`:** `pub const SHAKU_M: f32 = 10.0 / 33.0;` with the doc "Metres per shaku, for showing lengths to people; world space is in shaku." Remove `SUN_M`. Replace `width_m` with:

    ```rust
    /// Flat-to-flat width in shaku.
    pub fn width(self) -> f32 {
        self.scale_shaku() as f32
    }
    ```

    Update its tests.
  - **`config.rs`:** remove `layer_thickness_sun`, its default and its four methods, and the `SUN_M` import. `defaults_match_the_spec` checks `seed`, `world_radius_ri` and `bottom_layer` only.
  - **`column.rs`:**

    ```rust
    /// The height of the top of the column, in shaku.
    pub fn surface_height(&self) -> f32 {
        self.top_layer().map_or(0.0, crate::layer::layer_top)
    }
    ```

    Update its tests.

- [ ] **Step 5: `noise.rs` and `placeholder_terrain.rs`**
  - **`noise.rs`:** change `gradient`, `fade` and `gradient_2d` from `f64` to `f32`. Lattice indices stay `i64`, from `x.floor() as i64`. The hash-to-angle becomes `(h >> 40) as f32 / (1u64 << 24) as f32 * std::f32::consts::TAU`. Use `std::f32::consts::SQRT_2`. Loosen test tolerances from `1e-12` to `1e-6`.
  - **`placeholder_terrain.rs`:** the shape stays exactly the same, and the constants are converted by ×3.3:

    ```rust
    /// Number of octaves. The first is about 27,000 shaku across (about twice the ri width),
    /// the last about 1.65 shaku.
    pub const OCTAVES: usize = 15;
    const BASE_WAVELENGTH: f32 = 27_033.6; // 8,192 m
    const BASE_AMPLITUDE: f32 = 33.0; // 10 m
    const GAIN: f32 = 0.75;

    /// One octave's contribution at a point. The noise is sampled at (x, −z), so north
    /// is up in noise space, as it was when this worked in (east, north) metres: the
    /// landscape is the same one.
    pub fn octave_value(cfg: &WorldConfig, x: f32, z: f32, octave: usize) -> f32 {
        let w = wavelength(octave);
        let seed = cfg.seed ^ (octave as u64).wrapping_mul(0x9E37_79B9);
        gradient_2d(seed, x / w, -z / w) * amplitude(octave)
    }

    /// Terrain height in shaku at a point, at a level's detail.
    pub fn height(cfg: &WorldConfig, x: f32, z: f32, level: Level) -> f32 { … same sum … }
    ```

    `octave_kept` compares against `2.0 * level.width()`. **The set of octaves each level keeps must come out the same as before**, which ×3.3 on both sides guarantees. Add a test asserting it for every octave and level against the old table:
    - octave k is kept at level L iff `8192 / 2^k >= 2 · (scale_shaku(L) · 10/33)`;
    - compute that expectation in `f64` inside the test.

    `column_at` uses `let c = cell_centre(cell, level); let h = height(cfg, c.x, c.z, level); let top = layer_of_height(h).max(cfg.bottom_layer);`.

    Convert the test coordinates in metres by ×3.3 (for example, `(321.0, 654.0)` m becomes `(1059.3, -2158.2)`, with z = −3.3·north). Use `f32` tolerances of `1e-3` where the tests used `1e-9`.

- [ ] **Step 6: `store.rs`**
  - `load_order` measures chunk-to-focus distance as `cell_centre(key.cell, key.level).distance_squared(cell_centre(l.focus, Level::Shaku))`. The ordering rule is unchanged.
  - `surface_height`:

    ```rust
    /// The height of the ground at a point, at the finest detail loaded there.
    pub fn surface_height(&self, pos: Vec3) -> Option<f32> {
        for level in crate::level::CELL_LEVELS {
            let cell = crate::plane::round_at(pos, level);
            if let Some(column) = self.column(level, cell) {
                return Some(column.surface_height());
            }
        }
        None
    }
    ```

    Update its test (`s.surface_height(Vec3::ZERO).is_some()`).

- [ ] **Step 7: `address.rs`**

```rust
pub fn address_of(voxel: VoxelCoord) -> Address {
    let shaku = Hex::axial(voxel.q(), voxel.r());
    let ken = parent_of(shaku, Level::Shaku);
    let cho = parent_of(ken, Level::Ken);
    let ri = parent_of(cho, Level::Cho);
    Address {
        ri,
        cho: local(cho, Level::Cho),
        ken: local(ken, Level::Ken),
        shaku: local(shaku, Level::Shaku),
        layer: voxel.layer(),
    }
}

/// The shaku voxel an address names.
pub fn voxel_of(addr: &Address) -> VoxelCoord {
    // … the same arithmetic as the old `shaku_of`, producing `shaku: Hex` …
    VoxelCoord::new(shaku.q(), shaku.r(), shaku.s(), addr.layer)
        .expect("a Hex is always on the plane")
}
```

`addresses_round_trip` becomes `voxel_of(&address_of(v)) == v` over the same cells, as `VoxelCoord`s with a few layers.

- [ ] **Step 8: `mesh.rs`: emit Bevy-axis positions in shaku**

Rewrite the position arithmetic in `mesh_chunk`. The topology, the loops, the culling, the colours and the line data stay the same.
- `let origin = if key.level == Level::World { Vec3::ZERO } else { cell_centre(key.cell, key.level) };`
- `let corners = corners(child);` and `let bottom = layer_bottom(cfg.bottom_layer);`
- Per cell: `let c = cell_centre(cell, child) - origin;`
- **Top fan:** the centre vertex is `(c + Vec3::Y * top).to_array()` with normal `[0.0, 1.0, 0.0]`, where `top = layer_top(run.top)`. Corner vertices are `(c + corners[i] + Vec3::Y * top).to_array()`. The index order is unchanged: `(centre, corners[(k+5)%6], corners[k])`.
- **Side quads:** the vertices are `c + a + Vec3::Y * bottom_of_wall`, `c + b + Vec3::Y * bottom_of_wall`, `c + b + Vec3::Y * top`, `c + a + Vec3::Y * top`, in that order, where `a = corners[(k+5)%6]` and `b = corners[k]`. The normal is `((a + b) / 2.0).normalize_or_zero().to_array()`, which has `y = 0`. Keep the same two triangles, `(v0, v1, v2)` and `(v0, v2, v3)`.
- Rename `bottom_m` / `top_m` to `bottom` / `top`. Update the module doc: "Positions are Bevy-axis `[x, y, z]` in shaku, relative to the chunk's own centre." Update the same on `MeshData.positions`.

Why the winding is unchanged: the old output was `(e, n, h)`, and the plugin mapped it by `(e, h, −n)`. That mapping is a proper rotation, and the new positions are exactly the old ones mapped and scaled by 3.3. A test pins this:

```rust
    #[test]
    fn top_faces_point_up_and_wind_anticlockwise() {
        // Anticlockwise seen from above (+Y), with Bevy's front faces anticlockwise:
        // (b - a) × (c - a) points up.
        let (cfg, chunk) = ken_chunk();
        let mesh = mesh_chunk(&cfg, &chunk, &HashSet::new());
        let mut tops = 0;
        for tri in mesh.indices.chunks(3) {
            let p: Vec<Vec3> = tri.iter().map(|i| Vec3::from(mesh.positions[*i as usize])).collect();
            let n = Vec3::from(mesh.normals[tri[0] as usize]);
            if n == Vec3::Y {
                tops += 1;
                assert!((p[1] - p[0]).cross(p[2] - p[0]).y > 0.0, "{p:?}");
            }
        }
        assert!(tops > 0);
    }
```

It replaces the existing test of that name. Also add one pinning side walls to face outward: for every triangle whose normal has `y == 0`, `(p1 − p0) × (p2 − p0)` points the same way as its normal (a positive dot product).

In the other mesh tests, the height index moves from `[2]` to `[1]`, and the plane coordinates from `(e, n)` = `[0]`, `[1]` to `(x, z)` = `[0]`, `[2]`. Heights and plane lengths are now in shaku. Each test keeps asserting the same property.

- [ ] **Step 9: `lib.rs` exports**, and `hexworld/tests/handover.rs`
  - **In `lib.rs`:**
    - add `pub mod layer;` and `pub use layer::{layer_bottom, layer_of_height, layer_top, LAYER_HEIGHT};`;
    - `pub use level::{Level, CELL_LEVELS, SHAKU_M};`;
    - `pub use plane::{cell_centre, corners, round_at};`;
    - `pub use placeholder_terrain::{column_at, height};`;
    - `pub use address::{address_of, voxel_of, Address};`;
    - `pub use hex::{d2, distance, neighbours, range, step, Hex, DIRECTIONS, EDGES};`.

    Finish the crate doc's second sentence: "Positions are Bevy's `Vec3` in shaku, with east +X, north −Z and up +Y, as in `murabito_hexcoords`."
  - **In `tests/handover.rs`:** the local-centre helpers become `cell_centre(cell, Level::Ken) - cell_centre(cho_key.cell, Level::Cho)`, compared on `x` and `z` (positions `[0]` and `[2]`). The bottom is `layer_bottom(cfg.bottom_layer)`. The wall-corner expectations use `corners(Level::Ken)`, with the normal computed as in the mesher. `j = (k + 3) % 6` stays: the opposite of edge k is still `k + 3`. Every assertion keeps its meaning.

- [ ] **Step 10: Run `hexworld`'s suite**

Run: `cd …/Experiments/exp-05 && cargo test -p hexworld && cargo test -p murabito_hexcoords`

Expected: all pass. The exp-03 fixture test passes with no change to its assertions.

Then run `cargo fmt --all && cargo clippy -p hexworld --all-targets`. Expected: clean. The workspace doesn't build yet (Task 5).

- [ ] **Step 11: Commit** with `exp-05: hexworld works in shaku as Vec3, with the layer fixed at 5 sun`. Say in the body that `hexworld_bevy` and `viewer` are adapted in the next commit.

---

### Task 5: The plugin and the viewer speak shaku `Vec3`; `axes` goes

**Files:**
- Delete: `crates/hexworld_bevy/src/axes.rs`
- Modify:
  - `crates/hexworld_bevy/src/{lib, loader, entities}.rs`;
  - `crates/hexworld_bevy/tests/{plugin, handover, config}.rs`, and any other file the compiler flags;
  - `crates/viewer/src/{camera, panel, main, headless}.rs`.

**Interfaces:**
- Consumes (from Task 4): `hexworld::{cell_centre, round_at, layer_of_height, SHAKU_M, address_of, voxel_of, VoxelCoord}`, `ChunkStore::surface_height(Vec3) -> Option<f32>`, and `WorldConfig` without `layer_thickness_sun`.
- Produces:
  - `CameraRig { focus: Vec3, zoom: f32, last_height: f32 }`, with `focus.y` always 0;
  - `CameraRig::pan(&mut self, delta: Vec3, cfg: &WorldConfig)`;
  - `CameraRig::resolve_height(&mut self, loaded: Option<f32>) -> f32`;
  - the headless flags `--start-x` / `--start-z` (shaku, `f32`), replacing `--start-east` / `--start-north`, and `Headless.start: Option<(f32, f32)>` as `(x, z)`.

- [ ] **Step 1: `hexworld_bevy`**
  - Remove `pub mod axes;` and delete `axes.rs`.
  - **`loader.rs`:** `focus: round_at(transform.translation(), Level::Shaku)`.
  - **`entities.rs`:**
    - `chunk_origin` returns `cell_centre(key.cell, key.level)` (or `Vec3::ZERO` for `World`);
    - `to_bevy_mesh` inserts `data.positions.clone()` and `data.normals.clone()` directly, with no mapping. Its doc becomes "Build a Bevy mesh from the core's data, which is already in Bevy's axes.";
    - update `material.rs`'s module doc if it mentions the mapping.
  - **`tests/plugin.rs`:** delete the four `axes` tests (`north_maps_to_negative_z`, `the_mapping_round_trips`, `the_mapping_keeps_its_handedness`, `mesh_positions_use_the_same_mapping`). Orientation is now pinned in `hexworld`'s `plane` and `mesh` tests. The second-loader test spawns at `Transform::from_translation(hexworld::cell_centre(far_cell, Level::Shaku))`.
  - **`tests/handover.rs`:** positions come from `cell_centre(...) - cell_centre(key...)`, compared against `MeshData` positions directly, without `mesh_position`. Keep every assertion's meaning.
  - **`tests/config.rs`:** the test that changed `layer_thickness_sun: 10.0` to force a new world changes `seed` instead (for example, `seed: cfg.seed + 1`). Keep what it asserts about the rebuild.

- [ ] **Step 2: The viewer camera** (`camera.rs`)
  - `use hexworld_bevy::axes::to_bevy;` goes.
  - Zoom limits, with the doc "Camera distance from the focus, in shaku.":

    ```rust
    pub const MIN_ZOOM: f32 = 9.9; // 3 m
    pub const MAX_ZOOM: f32 = 13_200.0; // 4 km
    ```

  - `pan_speed_for_zoom`'s doc becomes "Shaku per second of panning". The formula is unchanged.
  - The rig:

    ```rust
    #[derive(Component, Clone, Copy, Debug)]
    pub struct CameraRig {
        /// The point the camera looks at, on the ground plane (y = 0); height is added
        /// from the terrain each frame.
        pub focus: Vec3,
        pub zoom: f32,
        pub last_height: f32,
    }

    pub fn pan(&mut self, delta: Vec3, cfg: &WorldConfig) {
        let next = self.focus + Vec3::new(delta.x, 0.0, delta.z);
        // Stay in the world: keep the focus inside the outermost ri.
        let ri = hexworld::plane::round_at(next, Level::Ri);
        if hexworld::world::in_world(ri, cfg) {
            self.focus = next;
        }
    }

    pub fn resolve_height(&mut self, loaded_height: Option<f32>) -> f32 { … same logic … }

    pub fn rig_transforms(rig: &CameraRig) -> (Vec3, Vec3) {
        let focus = rig.focus;
        … same tilt maths …
        // +Z is behind the focus, i.e. south, since north is −Z.
        (focus, focus + Vec3::new(0.0, up, back))
    }
    ```

  - **`handle_input`:** keys build `dir` as now, with `dir.y` positive for up-screen (north). The pan is `rig.pan(Vec3::new(dir.x, 0.0, -dir.y) * speed * dt, &cfg)`.
  - **Middle drag** keeps its feel: the old code was `east −= dx·pp`, `north += dy·pp`, which becomes `rig.pan(Vec3::new(-delta.x, 0.0, -delta.y) * per_pixel, &cfg)`.
  - **`follow_ground`:** `world.store().surface_height(rig.focus)`.
  - Update the doc comment on `resolve_height` ("about 3,600 m/s" becomes "about 11,900 shaku/s (3.6 km/s)").
  - Update camera tests to the new fields, keeping each test's property. `the_focus_stays_in_the_world` pans by a `Vec3`.

- [ ] **Step 3: The viewer's `main.rs` and `headless.rs`**
  - **`headless.rs`:** the flags `--start-x` and `--start-z` are parsed as `f32`, and `start: start_x.zip(start_z)`. Update the flag docs and tests: rename the flag strings, and write the values in shaku. The pairing rule ("both or neither") is unchanged.
  - **`main.rs` setup:** `let (x, z) = headless.start.unwrap_or((0.0, 0.0));`, `focus: Vec3::new(x, 0.0, z)`. The default zoom goes from `120.0` to `396.0` (120 m).
  - **`auto_pan_system`:** `rig.pan(Vec3::X * speed * dt, &cfg)`.
  - **Projection and shadows:** keep the metre-era look in shaku.
    - Look up in the registry source (`~/.cargo/registry/src/*/bevy_camera-0.19*/` or wherever `PerspectiveProjection` lives in 0.19.1) what `PerspectiveProjection::default()`'s `near` and `far` are, and what `CascadeShadowConfig`'s defaults are.
    - Set them explicitly to 3.3 × those defaults: `near`, `far`, and the cascade builder's `first_cascade_far_bound` and `maximum_distance`. That way what's visible and shadowed at a given real distance doesn't change.
    - Write the values as `3.3 * default` with a comment saying so, and name the source file you checked in the commit body.

- [ ] **Step 4: The viewer panel** (`panel.rs`)
  - The cursor or focus read-outs call `round_at(pos, Level::Shaku)`.
  - `layer_at(world, pos: Vec3) -> i32` uses `world.store().surface_height(pos).map(layer_of_height).unwrap_or(0)`. Keep its current fallback if that differs.
  - Addresses come from `address_of(VoxelCoord::new(shaku.q(), shaku.r(), shaku.s(), layer).unwrap())` and display with `Address`'s `Display`.
  - Where the panel shows a position or length to a person, show shaku with metres beside it: `format!("({:.1}, {:.1}) shaku ≈ ({:.2}, {:.2}) m", p.x, p.z, p.x * SHAKU_M, p.z * SHAKU_M)`.
  - Loader markers spawn at `Transform::from_translation(cell_centre(shaku, Level::Shaku))`.
  - **Remove the layer-thickness slider.** Keep the seed control. Commit on drag release or lost focus as now, with `WorldConfig { seed, ..cfg }`. Update the comment above it to speak only of the seed.

- [ ] **Step 5: Run the whole suite**

Run: `cd …/Experiments/exp-05 && cargo test --workspace && cargo clippy --workspace --all-targets && cargo fmt --all --check`

Expected: all pass and clean. The count is the old 132, minus the 4 `axes` tests, plus the new tests from Tasks 2–4. Report the exact count.

- [ ] **Step 6: Run the viewer headless** and look at the result.

```bash
export $(systemctl --user show-environment | rg '^(DISPLAY|XAUTHORITY)=' | xargs)
SHOTS=/tmp/claude-1000/-home-lexa-DevProjects--GameDev-Murabito/bb7eaf7e-a996-4e45-8d06-6915e27b5ca9/scratchpad/after
mkdir -p $SHOTS/near $SHOTS/far
cargo run -p viewer --release -- --frames 240 --screenshot-dir $SHOTS/near --screenshot-every 120 --zoom 396 --line-mode nested
cargo run -p viewer --release -- --frames 240 --screenshot-dir $SHOTS/far --screenshot-every 120 --zoom 8250 --line-mode nested
```

Expected: PNGs showing terrain. Report their paths. The controller compares them against Task 1's baseline: the same landscape and framing, with battlements mirrored left to right.

- [ ] **Step 7: Commit** with `exp-05: plugin and viewer speak shaku Vec3; drop the axes mapping and the thickness slider`.

---

### Task 6: Docs, and the final check

**Files:**
- Modify: `Experiments/exp-05/README.md`, `Experiments/exp-05/docs/specs/2026-09-20-exp-05-hex-voxel-chunks-design.md`, `Experiments/manifest.md` (the exp-05 entry).

- [ ] **Step 1: The original spec.** Add a line under its title: "> Amended 2026-09-22 by `2026-09-22-exp-05-main-coords-design.md`: coordinates now follow the main project's `murabito_hexcoords` (shaku `Vec3`, +r SSE, cube API, layer fixed at 5 sun). Where the two disagree, that one wins." Don't rewrite the body.

- [ ] **Step 2: The README.** Search it: `rg -n 'metre|metres|north|east|axes|thickness|dependenc|sun|--start' README.md`. Update every statement the change made false:
  - the units are shaku `Vec3`, with +r SSE;
  - `hexworld`'s dependencies are now the vendored crate plus bevy math;
  - the thickness slider is gone;
  - the headless flags are now `--start-x` / `--start-z`, in shaku, and zoom is in shaku;
  - the battlement pattern is mirrored compared with exp-03.

  Add the vendored crate to the layout section, with a one-line pointer to `crates/hexcoords/README.md`. Update the test count to the real number from Task 5.

- [ ] **Step 3: The manifest.** In the exp-05 entry of `Experiments/manifest.md`, add one sentence: "Since 2026-09-22 it uses the main project's coordinate conventions, through a vendored copy of `murabito_hexcoords`." Update any run flag the entry quotes.

- [ ] **Step 4: Final verification**

```bash
cd /home/lexa/DevProjects/_GameDev/Murabito/.claude/worktrees/exp-05-main-coords/Experiments/exp-05
cargo test --workspace; echo "cargo exit: $?"
cargo clippy --workspace --all-targets; echo "clippy exit: $?"
cargo fmt --all --check; echo "fmt exit: $?"
git -C /home/lexa/DevProjects/_GameDev/Murabito/.claude/worktrees/exp-05-main-coords show 79335cd:crates/hexcoords/src/lib.rs | cmp - crates/hexcoords/src/lib.rs && echo "vendored copy identical"
rg -n 'cell_centre_m|corners_m|width_m|surface_height_m|height_m|layer_thickness|axes::|to_bevy\(|from_bevy|mesh_position|SUN_M|metres_to_axial|hex_round' crates; echo "stale-name rg exit: $? (1 = none found)"
```

Expected: every exit is 0, except the stale-name `rg`, which should exit 1 (no matches). The vendored copy is identical.

- [ ] **Step 5: Commit** with `exp-05: document the move onto the main project's coordinates`.

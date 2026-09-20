# exp-05 Hex Coordinates, Addresses and Chunks Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build one uniform hex coordinate system in Rust — ri / cho / ken / shaku addresses with a vertical layer, voxel columns, hot-loaded chunks at nested scales — with a Bevy viewer that draws it.

**Architecture:** Three crates in one Cargo workspace. `hexworld` is the engine-free core: integer hex maths ported from exp-03, run-list columns, a chunk store with delayed unloading, and a pure mesher. `hexworld_bevy` is a thin plugin: a `Loader` component, background generation and meshing tasks, chunk entities, and the hex-line material. `viewer` is a Bevy app with an overhead camera and an egui tuning panel.

**Tech Stack:** Rust 1.95 (stable, already installed), Bevy 0.19.1, bevy_egui 0.42, egui 0.36. No other runtime dependencies. One Python script (stdlib only, run through `uv`) generates test fixtures from exp-03.

**Spec:** `Experiments/exp-05/docs/specs/2026-09-20-exp-05-hex-voxel-chunks-design.md` — read it before Task 1. The plan implements it; where the plan refines it, see "Refinements to the spec" below.

## Global Constraints

- **Workspace root:** `Experiments/exp-05/`. Everything in this plan is inside it unless a path says otherwise. Nothing experiment-specific goes at the repo root.
- **Git:** every command uses `git -C /home/lexa/DevProjects/_GameDev/Murabito`. Work happens on the branch `exp-05-design` (already created from `origin/main`, holds the spec commit). **Never push and never open a PR** — the user approves that separately.
- **`hexworld` has no dependencies at all**, not even dev-dependencies: no Bevy, no `serde`, no `rand`, no `glam`. Fixtures are a plain text format parsed by hand. This is what makes the core reusable in another engine.
- **Rust edition 2021**, `rustfmt` defaults, `cargo clippy -- -D warnings` clean.
- **Units, exact values:** shaku = 10/33 m flat to flat; sun = 1/33 m; ken = 6 shaku; cho = 60 ken; ri = 36 cho. Default layer thickness 5 sun = 5/33 m. Default `world_radius_ri` = 0 (one ri). Default unload delay 5 s. Default rings 3 / 3 / 3.
- **Ownership rule (exact):** owner of child `c` on a lattice scaled by `n` is the candidate parent minimising `d2 = dq² + dq·dr + dr²`; **ties go to the lexicographically greatest `(a, b)`**. All integer arithmetic, `d2` computed in `i64`.
- **Axes:** the core speaks `(east, north, height)` only. The Bevy mapping east → +X, height → +Y, north → **−Z** lives in exactly one function, `hexworld_bevy::to_bevy`, and its inverse `from_bevy`.
- **Placeholder terrain:** the module is named `placeholder_terrain` and its doc comment says it is a placeholder, not the real generator.
- **Licensing:** MIT code, no engine code or third-party source pasted in. No game names in code or docs.
- **Searching:** use `rg`, never `grep`.
- **Build artifacts:** `target/` is gitignored. Do **not** put a Rust build directory under `/tmp` — it is a RAM-backed tmpfs on this machine and a Bevy build fills it.
- **The user may have things open; never kill a process you did not start.**

## File Structure

```
Experiments/exp-05/
├─ Cargo.toml                      workspace members, shared profile settings
├─ .gitignore                      target/, screenshots/
├─ rust-toolchain.toml             stable
├─ tools/make_fixtures.py          reads exp-03's hexaddr.py, writes the fixture file
├─ crates/
│  ├─ hexworld/
│  │  ├─ Cargo.toml
│  │  ├─ src/lib.rs                re-exports, crate docs
│  │  ├─ src/hex.rs                Hex, DIRECTIONS, d2, distance, neighbours, range
│  │  ├─ src/level.rs              Level, packing, scale, widths in metres
│  │  ├─ src/owner.rs              owner, parent_of, up, centre_child, local, templates
│  │  ├─ src/plane.rs              cell centres in metres, metres → axial, hex_round, round_at
│  │  ├─ src/config.rs             WorldConfig, layer ↔ height
│  │  ├─ src/address.rs            Address, address_of, shaku_of, Display
│  │  ├─ src/world.rs              in_world, world_ri, cell_in_world
│  │  ├─ src/noise.rs              hashed-lattice gradient noise
│  │  ├─ src/placeholder_terrain.rs  height with per-level octave cut, column_at
│  │  ├─ src/column.rs             Material, Run, Column
│  │  ├─ src/chunk.rs              ChunkKey, Chunk, generate
│  │  ├─ src/store.rs              Rings, Loader, StoreSettings, ChunkStore, Stats
│  │  ├─ src/mesh.rs               MeshData, mesh_chunk
│  │  └─ tests/                    integration tests + fixtures/exp03_cases.txt
│  ├─ hexworld_bevy/
│  │  ├─ Cargo.toml
│  │  ├─ src/lib.rs                HexWorldPlugin, resources, system order
│  │  ├─ src/axes.rs               to_bevy / from_bevy — the only axis mapping
│  │  ├─ src/loader.rs             Loader component, gathering loaders
│  │  ├─ src/tasks.rs              generation + meshing tasks, insertion
│  │  ├─ src/entities.rs           chunk entities, handover, despawn
│  │  └─ src/material.rs           ATTRIBUTE_LINE, GroundMaterial, mode uniforms
│  │  └─ assets/shaders/ground.wgsl
│  └─ viewer/
│     ├─ Cargo.toml
│     ├─ src/main.rs               app setup, CLI flags
│     ├─ src/camera.rs             overhead rig, input, ground following
│     ├─ src/panel.rs              egui panel
│     └─ src/headless.rs           --frames / --screenshot-dir
└─ docs/specs/, docs/plans/
```

## Refinements to the spec

Decided while writing this plan. They implement the spec's intent; the spec's "guests" and "no holes" requirements are unchanged.

1. **The drawn set is "owned plus tie-guests", computed in integer arithmetic, not `round_at`.** A chunk draws every child cell at least as close (by `d2`) to its own centre as to any of the eight neighbouring parent centres — that is, every candidate whose distance to this cell's centre matches the minimum over all nine candidate parents. This is exactly the set of cells this parent **owns**, plus the "guests": cells sitting exactly on a tie between this parent and a neighbour, whose ownership (broken by `owner`'s lexicographic rule) went to the neighbour, but whose distance is still tied, so their own ideal hexagon straddles the border. Drawn is therefore **owned ∪ tie-guests** — it never excludes an owned cell. Deriving this from `round_at` in `f64` does not work: floating-point error is direction-dependent, so the same boundary tie resolves differently depending on which side you approach it from, and the result stops being an exact partition (an earlier version of this plan derived `drawn_offsets` from `round_at`, and its own tiling test caught the gap during Task 9). Integer `d2` ties are exact and translation-invariant, so drawn sets tile the plane exactly: any mixture of loaded and unloaded chunks covers the ground once, no holes, no overlaps. Ownership is unchanged and still decides addresses, chunk contents and border lines.
2. **The mesher generates the columns it needs that the chunk does not hold** (guests, and neighbours just outside the drawn set for face culling) by calling the placeholder generator, which is pure. Recorded as an open question in the spec for when edits arrive.
3. **Line data rides on the mesh in the UVs**, not in a custom vertex attribute: `UV0 = (edge_w, border_level)` and `UV1 = (cell_level, 0)`. Bevy's standard vertex shader already passes both to the fragment shader, so the ground material needs a fragment shader only — no custom vertex shader and no pipeline specialization. The core still produces one `[f32; 4]` per vertex (`MeshData::line`); the plugin splits it into the two UV sets. Top faces are fanned from the cell centre so each triangle has exactly one outer edge, and `edge_w` is 0 at the centre and 1 at the corners. No hex maths in the shader.

---

### Task 1: Workspace, `Hex` and `Level`

**Files:**
- Create: `Experiments/exp-05/Cargo.toml`, `.gitignore`, `rust-toolchain.toml`
- Create: `crates/hexworld/Cargo.toml`, `src/lib.rs`, `src/hex.rs`, `src/level.rs`
- Test: `crates/hexworld/src/hex.rs` (unit tests in-file), `crates/hexworld/src/level.rs`

**Interfaces:**
- Consumes: nothing.
- Produces: `Hex { q: i32, r: i32 }` with `ZERO`, `new`, `s()`, `Add`/`Sub`, `Ord`; `DIRECTIONS: [Hex; 6]`; `d2(Hex) -> i64`; `distance(Hex, Hex) -> i32`; `neighbours(Hex) -> [Hex; 6]`; `range(Hex, i32) -> Vec<Hex>`; `Level { Shaku, Ken, Cho, Ri, World }` with `packing() -> i32`, `scale_shaku() -> i32`, `width_m() -> f64`, `child() -> Option<Level>`, `parent() -> Option<Level>`, `name() -> &'static str`, `ALL_LEVELS`.

- [ ] **Step 1: Create the workspace files**

`Experiments/exp-05/Cargo.toml`:

```toml
[workspace]
resolver = "2"
members = ["crates/hexworld", "crates/hexworld_bevy", "crates/viewer"]

[workspace.package]
edition = "2021"
version = "0.1.0"
license = "MIT"

[profile.dev]
opt-level = 1

[profile.dev.package."*"]
opt-level = 3
```

`Experiments/exp-05/.gitignore`:

```
target/
screenshots/
```

`Experiments/exp-05/rust-toolchain.toml`:

```toml
[toolchain]
channel = "stable"
```

`crates/hexworld/Cargo.toml`:

```toml
[package]
name = "hexworld"
edition.workspace = true
version.workspace = true
license.workspace = true

[dependencies]
```

The workspace lists three members but only `hexworld` exists yet, so create placeholder crates for the other two now: `cargo new --lib crates/hexworld_bevy` and `cargo new --bin crates/viewer`, then empty their `[dependencies]`. They are filled in from Task 11 on.

- [ ] **Step 2: Write the failing tests for `hex.rs`**

```rust
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
            let (e, n) = ((h.q as f64) + (h.r as f64) / 2.0, h.r as f64 * 3f64.sqrt() / 2.0);
            n.atan2(e).to_degrees().rem_euclid(360.0)
        };
        for (k, d) in DIRECTIONS.iter().enumerate() {
            assert!((angle(*d) - 60.0 * k as f64).abs() < 1e-9, "direction {k} is {:?}", d);
        }
    }
}
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cd Experiments/exp-05 && cargo test -p hexworld`
Expected: FAIL to compile — `Hex`, `DIRECTIONS`, `d2`, `distance`, `range`, `neighbours` not found.

- [ ] **Step 4: Implement `hex.rs`**

```rust
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
    Hex::new(1, 0),   //   0°, east
    Hex::new(0, 1),   //  60°
    Hex::new(-1, 1),  // 120°
    Hex::new(-1, 0),  // 180°, west
    Hex::new(0, -1),  // 240°
    Hex::new(1, -1),  // 300°
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
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cd Experiments/exp-05 && cargo test -p hexworld`
Expected: PASS, 6 tests.

- [ ] **Step 6: Write the failing tests for `level.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packings_and_scales_are_exact() {
        assert_eq!(Level::Shaku.packing(), 1);
        assert_eq!(Level::Ken.packing(), 6);
        assert_eq!(Level::Cho.packing(), 60);
        assert_eq!(Level::Ri.packing(), 36);
        assert_eq!(Level::Shaku.scale_shaku(), 1);
        assert_eq!(Level::Ken.scale_shaku(), 6);
        assert_eq!(Level::Cho.scale_shaku(), 360);
        assert_eq!(Level::Ri.scale_shaku(), 12_960);
    }

    #[test]
    fn widths_match_the_units() {
        assert!((Level::Shaku.width_m() - 10.0 / 33.0).abs() < 1e-12);
        assert!((Level::Ken.width_m() - 60.0 / 33.0).abs() < 1e-12);
        assert!((Level::Cho.width_m() - 3_600.0 / 33.0).abs() < 1e-9);
        assert!((Level::Ri.width_m() - 129_600.0 / 33.0).abs() < 1e-9);
    }

    #[test]
    fn levels_chain_both_ways() {
        assert_eq!(Level::Shaku.parent(), Some(Level::Ken));
        assert_eq!(Level::Ri.parent(), Some(Level::World));
        assert_eq!(Level::World.parent(), None);
        assert_eq!(Level::World.child(), Some(Level::Ri));
        assert_eq!(Level::Shaku.child(), None);
    }
}
```

- [ ] **Step 7: Run the tests to verify they fail**

Run: `cd Experiments/exp-05 && cargo test -p hexworld level`
Expected: FAIL to compile — `Level` not found.

- [ ] **Step 8: Implement `level.rs`**

```rust
//! The levels of the hierarchy. `World` is not a unit of length: it exists so that the
//! chunk holding every ri has a parent level like any other chunk.

pub const SHAKU_M: f64 = 10.0 / 33.0;
pub const SUN_M: f64 = 1.0 / 33.0;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub enum Level {
    Shaku,
    Ken,
    Cho,
    Ri,
    World,
}

/// Fine to coarse. `World` is deliberately absent: it is a chunk level, not a cell level.
pub const CELL_LEVELS: [Level; 4] = [Level::Shaku, Level::Ken, Level::Cho, Level::Ri];

impl Level {
    /// Cells of the level below per side of this cell. `Shaku` has none, so 1.
    pub fn packing(self) -> i32 {
        match self {
            Level::Shaku => 1,
            Level::Ken => 6,
            Level::Cho => 60,
            Level::Ri => 36,
            Level::World => panic!("the world level has no packing"),
        }
    }

    /// Shaku per side of this cell.
    pub fn scale_shaku(self) -> i32 {
        match self {
            Level::Shaku => 1,
            Level::Ken => 6,
            Level::Cho => 360,
            Level::Ri => 12_960,
            Level::World => panic!("the world level has no scale"),
        }
    }

    /// Flat-to-flat width in metres.
    pub fn width_m(self) -> f64 {
        self.scale_shaku() as f64 * SHAKU_M
    }

    pub fn child(self) -> Option<Level> {
        match self {
            Level::Shaku => None,
            Level::Ken => Some(Level::Shaku),
            Level::Cho => Some(Level::Ken),
            Level::Ri => Some(Level::Cho),
            Level::World => Some(Level::Ri),
        }
    }

    pub fn parent(self) -> Option<Level> {
        match self {
            Level::Shaku => Some(Level::Ken),
            Level::Ken => Some(Level::Cho),
            Level::Cho => Some(Level::Ri),
            Level::Ri => Some(Level::World),
            Level::World => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Level::Shaku => "shaku",
            Level::Ken => "ken",
            Level::Cho => "cho",
            Level::Ri => "ri",
            Level::World => "world",
        }
    }
}
```

- [ ] **Step 9: Run the tests to verify they pass**

Run: `cd Experiments/exp-05 && cargo test -p hexworld`
Expected: PASS, 9 tests.

- [ ] **Step 10: Wire up `lib.rs`**

```rust
//! Murabito's hex world: ri / cho / ken / shaku addresses with a vertical layer,
//! voxel columns, and chunks that load at nested scales.
//!
//! This crate is engine-free on purpose: no rendering, no ECS, no dependencies.
//! Positions are `(east, north, height)` in metres; mapping to an engine's axes is
//! the caller's job.

pub mod hex;
pub mod level;

pub use hex::{d2, distance, neighbours, range, Hex, DIRECTIONS};
pub use level::{Level, CELL_LEVELS, SHAKU_M, SUN_M};
```

- [ ] **Step 11: Check formatting and lints, then commit**

```bash
cd /home/lexa/DevProjects/_GameDev/Murabito/Experiments/exp-05
cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test
git -C /home/lexa/DevProjects/_GameDev/Murabito add Experiments/exp-05
git -C /home/lexa/DevProjects/_GameDev/Murabito commit -m "exp-05: workspace, Hex and Level

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 2: Ownership and the hierarchy

**Files:**
- Create: `crates/hexworld/src/owner.rs`
- Modify: `crates/hexworld/src/lib.rs` (add `pub mod owner;` and re-exports)

**Interfaces:**
- Consumes: `Hex`, `d2`, `Level` from Task 1.
- Produces: `owner(Hex, i32) -> Hex`; `parent_of(Hex, Level) -> Hex`; `up(Hex, Level, Level) -> Hex`; `centre_child(Hex, Level) -> Hex`; `centre_shaku(Hex, Level) -> Hex`; `local(Hex, Level) -> Hex`; `owned_offsets(Level) -> &'static [Hex]`; `children(Hex, Level) -> Vec<Hex>`.

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::hex::distance;

    #[test]
    fn a_parent_owns_exactly_n_squared_children() {
        assert_eq!(owned_offsets(Level::Ken).len(), 36);
        assert_eq!(owned_offsets(Level::Cho).len(), 3_600);
        assert_eq!(owned_offsets(Level::Ri).len(), 1_296);
    }

    #[test]
    fn every_child_has_exactly_one_owner() {
        // Over a patch spanning several ken, each shaku is owned by exactly one ken,
        // and that ken lists it among its children.
        for q in -20..=20 {
            for r in -20..=20 {
                let c = Hex::new(q, r);
                let ken = owner(c, Level::Ken.packing());
                let listed = children(ken, Level::Ken);
                assert!(listed.contains(&c), "{c:?} not listed by {ken:?}");
            }
        }
    }

    #[test]
    fn the_centre_child_is_owned_by_its_parent() {
        for cell in [Hex::ZERO, Hex::new(3, -7), Hex::new(-12, 5)] {
            let centre = centre_child(cell, Level::Cho);
            assert_eq!(owner(centre, Level::Cho.packing()), cell);
        }
    }

    #[test]
    fn ownership_is_translation_invariant() {
        for c in [Hex::new(1, 2), Hex::new(-4, 3), Hex::new(31, -17)] {
            let shifted = Hex::new(c.q + 6 * 5, c.r - 6 * 2);
            let a = owner(c, 6);
            let b = owner(shifted, 6);
            assert_eq!(Hex::new(b.q - a.q, b.r - a.r), Hex::new(5, -2));
        }
    }

    #[test]
    fn ties_go_to_the_greatest_parent() {
        // A shaku exactly between two ken centres: both candidates have the same d2,
        // so the lexicographically greatest (a, b) wins.
        let n = 6;
        let mut found_tie = false;
        for q in -12..=12 {
            for r in -12..=12 {
                let c = Hex::new(q, r);
                let win = owner(c, n);
                let best = crate::hex::d2(Hex::new(c.q - n * win.q, c.r - n * win.r));
                for a in -3..=3 {
                    for b in -3..=3 {
                        let cand = Hex::new(a, b);
                        if cand == win {
                            continue;
                        }
                        let d = crate::hex::d2(Hex::new(c.q - n * a, c.r - n * b));
                        if d == best {
                            found_tie = true;
                            assert!((win.q, win.r) > (cand.q, cand.r), "{c:?}: {win:?} vs {cand:?}");
                        }
                    }
                }
            }
        }
        assert!(found_tie, "the patch should contain at least one tie");
    }

    #[test]
    fn owned_children_stay_near_the_centre() {
        // Packing B: owned children lie within 2n/3 of the centre, so the reach used to
        // build the template (n) is generous enough.
        for level in [Level::Ken, Level::Cho, Level::Ri] {
            let n = level.packing();
            for off in owned_offsets(level) {
                assert!(distance(*off, Hex::ZERO) <= 2 * n / 3 + 1, "{level:?} {off:?}");
            }
        }
    }

    #[test]
    fn up_chains_levels() {
        let shaku = Hex::new(77, -31);
        let ken = parent_of(shaku, Level::Shaku);
        let cho = parent_of(ken, Level::Ken);
        assert_eq!(up(shaku, Level::Shaku, Level::Cho), cho);
        assert_eq!(up(shaku, Level::Shaku, Level::Shaku), shaku);
    }

    #[test]
    fn local_offsets_are_small() {
        let shaku = Hex::new(77, -31);
        let off = local(shaku, Level::Shaku);
        assert!(off.q.abs() <= 6 && off.r.abs() <= 6, "{off:?}");
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cd Experiments/exp-05 && cargo test -p hexworld owner`
Expected: FAIL to compile — `owner`, `owned_offsets`, `children` not found.

- [ ] **Step 3: Implement `owner.rs`**

```rust
//! Packing B and ownership: which parent each child belongs to.
//!
//! A level-L cell (a, b) is centred on the level-(L-1) cell (N·a, N·b), with no rotation
//! between levels. A child belongs to exactly one parent: the nearest parent centre, with
//! ties going to the lexicographically greatest (a, b). All integer arithmetic, so ties are
//! exact. Ownership is the same for every parent at a level, so the owned offsets are built
//! once per level and reused.

use std::sync::OnceLock;

use crate::hex::{d2, Hex};
use crate::level::Level;

/// The parent, on the lattice scaled by `n`, that owns this child cell.
pub fn owner(cell: Hex, n: i32) -> Hex {
    let a0 = (cell.q as f64 / n as f64).round() as i32;
    let b0 = (cell.r as f64 / n as f64).round() as i32;
    let mut best = Hex::ZERO;
    let mut best_key: Option<(i64, i64, i64)> = None;
    for a in (a0 - 1)..=(a0 + 1) {
        for b in (b0 - 1)..=(b0 + 1) {
            let off = Hex::new(cell.q - n * a, cell.r - n * b);
            // Smallest d2 wins; ties go to the greatest (a, b), hence the negations.
            let key = (d2(off), -(a as i64), -(b as i64));
            if best_key.is_none_or(|k| key < k) {
                best_key = Some(key);
                best = Hex::new(a, b);
            }
        }
    }
    best
}

/// The cell at the next level up that owns this one.
pub fn parent_of(cell: Hex, level: Level) -> Hex {
    let parent = level.parent().expect("the world level has no parent");
    if parent == Level::World {
        // Every ri belongs to the single world cell.
        return Hex::ZERO;
    }
    owner(cell, parent.packing())
}

/// Walk up the hierarchy from one level to another.
pub fn up(cell: Hex, from: Level, to: Level) -> Hex {
    let mut cell = cell;
    let mut level = from;
    while level != to {
        cell = parent_of(cell, level);
        level = level.parent().expect("walked past the world level");
    }
    cell
}

/// The child cell at the centre of this cell.
pub fn centre_child(cell: Hex, level: Level) -> Hex {
    let n = level.packing();
    Hex::new(cell.q * n, cell.r * n)
}

/// The shaku at the centre of this cell.
pub fn centre_shaku(cell: Hex, level: Level) -> Hex {
    let s = level.scale_shaku();
    Hex::new(cell.q * s, cell.r * s)
}

/// A cell's offset from its parent's centre, in cells of its own level.
pub fn local(cell: Hex, level: Level) -> Hex {
    let p = parent_of(cell, level);
    let n = level.parent().expect("no parent").packing();
    Hex::new(cell.q - n * p.q, cell.r - n * p.r)
}

/// Offsets, from the centre child, of the children a cell at this level owns.
/// The same for every cell at the level, so it is built once.
pub fn owned_offsets(level: Level) -> &'static [Hex] {
    static CACHE: [OnceLock<Vec<Hex>>; 5] = [
        OnceLock::new(),
        OnceLock::new(),
        OnceLock::new(),
        OnceLock::new(),
        OnceLock::new(),
    ];
    let slot = &CACHE[level as usize];
    slot.get_or_init(|| {
        let n = level.packing();
        let mut out = Vec::new();
        for q in -n..=n {
            for r in -n..=n {
                let c = Hex::new(q, r);
                if owner(c, n) == Hex::ZERO {
                    out.push(c);
                }
            }
        }
        out
    })
}

/// The children this cell owns, in a fixed order.
pub fn children(cell: Hex, level: Level) -> Vec<Hex> {
    let centre = centre_child(cell, level);
    owned_offsets(level).iter().map(|o| centre + *o).collect()
}
```

Note: `parent_of` returns `Hex::ZERO` for a ri, because the world has one cell. `owner` uses `f64::round` for the first guess, which rounds halves away from zero where Python rounds to even; it makes no difference, because all nine candidates around the guess are tested.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cd Experiments/exp-05 && cargo test -p hexworld`
Expected: PASS. The `every_child_has_exactly_one_owner` and `owned_offsets` tests are the slow ones (a few seconds in debug).

- [ ] **Step 5: Export from `lib.rs` and commit**

Add `pub mod owner;` and `pub use owner::{children, centre_child, centre_shaku, local, owner, parent_of, up};`

```bash
cd /home/lexa/DevProjects/_GameDev/Murabito/Experiments/exp-05
cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test
git -C /home/lexa/DevProjects/_GameDev/Murabito add Experiments/exp-05
git -C /home/lexa/DevProjects/_GameDev/Murabito commit -m "exp-05: packing B ownership and the level hierarchy

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 3: Cross-check against exp-03

**Files:**
- Create: `Experiments/exp-05/tools/make_fixtures.py`
- Create: `crates/hexworld/tests/fixtures/exp03_cases.txt` (generated, then committed)
- Create: `crates/hexworld/tests/exp03_agreement.rs`

**Interfaces:**
- Consumes: `owner`, `up`, `local`, `children` from Task 2.
- Produces: nothing for later tasks; this is a guard that the port is faithful.

The fixture file is a plain text format, one case per line, so `hexworld` needs no parsing dependency:

```
owner <n> <q> <r> -> <a> <b>
up <from_level> <q> <r> <to_level> -> <a> <b>
local <level> <q> <r> -> <a> <b>
owned_count <level> -> <count>
```

- [ ] **Step 1: Write the fixture generator**

`Experiments/exp-05/tools/make_fixtures.py`:

```python
"""Write reference cases from exp-03's tested hexaddr.py, for hexworld's Rust port to
check itself against. exp-03 is read only; nothing there is changed.

Run:  uv run --no-project python tools/make_fixtures.py
"""

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
EXP03_SRC = HERE.parent.parent / "exp-03" / "src"
OUT = HERE.parent / "crates" / "hexworld" / "tests" / "fixtures" / "exp03_cases.txt"

sys.path.insert(0, str(EXP03_SRC))

import hexaddr  # noqa: E402

LEVEL_NAMES = {hexaddr.SHAKU: "shaku", hexaddr.KEN: "ken", hexaddr.CHO: "cho", hexaddr.RI: "ri"}

# A spread of cells: around the origin, across ken/cho/ri borders, and negative, where
# floor and rounding behaviour differs between languages.
CELLS = [(q, r) for q in range(-14, 15) for r in range(-14, 15)]
CELLS += [(q, r) for q in (-12_960, -361, -359, -7, 359, 361, 12_960) for r in (-181, -6, 0, 6, 181)]


def main() -> None:
    lines = []
    for n in (6, 36, 60):
        for q, r in CELLS:
            a, b = hexaddr.owner((q, r), n)
            lines.append(f"owner {n} {q} {r} -> {a} {b}")
    for level in (hexaddr.SHAKU, hexaddr.KEN, hexaddr.CHO):
        for q, r in CELLS:
            for to in range(level + 1, hexaddr.RI + 1):
                a, b = hexaddr.up((q, r), level, to)
                lines.append(f"up {LEVEL_NAMES[level]} {q} {r} {LEVEL_NAMES[to]} -> {a} {b}")
            a, b = hexaddr.local((q, r), level)
            lines.append(f"local {LEVEL_NAMES[level]} {q} {r} -> {a} {b}")
    for level in (hexaddr.KEN, hexaddr.CHO, hexaddr.RI):
        lines.append(f"owned_count {LEVEL_NAMES[level]} -> {len(hexaddr.child_offsets(level))}")

    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text("\n".join(lines) + "\n")
    print(f"{len(lines)} cases -> {OUT}")


if __name__ == "__main__":
    main()
```

- [ ] **Step 2: Generate the fixture and check it looks right**

```bash
cd /home/lexa/DevProjects/_GameDev/Murabito/Experiments/exp-05
uv run --no-project python tools/make_fixtures.py
wc -l crates/hexworld/tests/fixtures/exp03_cases.txt
head -3 crates/hexworld/tests/fixtures/exp03_cases.txt
rg -c "^owner " crates/hexworld/tests/fixtures/exp03_cases.txt
```

Expected: roughly 20,000 lines; the first lines look like `owner 6 -14 -14 -> -2 -2`. Confirm `git -C ... status` shows exp-03 unchanged.

- [ ] **Step 3: Write the failing agreement test**

`crates/hexworld/tests/exp03_agreement.rs`:

```rust
//! The Rust port must agree with exp-03's tested Python, case for case.
//! Regenerate with: uv run --no-project python tools/make_fixtures.py

use hexworld::{local, owner, owned_offsets, up, Hex, Level};

fn level_by_name(name: &str) -> Level {
    match name {
        "shaku" => Level::Shaku,
        "ken" => Level::Ken,
        "cho" => Level::Cho,
        "ri" => Level::Ri,
        other => panic!("unknown level {other}"),
    }
}

#[test]
fn agrees_with_exp03() {
    let text = include_str!("fixtures/exp03_cases.txt");
    let mut checked = 0usize;
    for line in text.lines() {
        let (lhs, rhs) = line.split_once(" -> ").expect(line);
        let f: Vec<&str> = lhs.split_whitespace().collect();
        let r: Vec<&str> = rhs.split_whitespace().collect();
        match f[0] {
            "owner" => {
                let n: i32 = f[1].parse().unwrap();
                let cell = Hex::new(f[2].parse().unwrap(), f[3].parse().unwrap());
                let want = Hex::new(r[0].parse().unwrap(), r[1].parse().unwrap());
                assert_eq!(owner(cell, n), want, "{line}");
            }
            "up" => {
                let from = level_by_name(f[1]);
                let cell = Hex::new(f[2].parse().unwrap(), f[3].parse().unwrap());
                let to = level_by_name(f[4]);
                let want = Hex::new(r[0].parse().unwrap(), r[1].parse().unwrap());
                assert_eq!(up(cell, from, to), want, "{line}");
            }
            "local" => {
                let level = level_by_name(f[1]);
                let cell = Hex::new(f[2].parse().unwrap(), f[3].parse().unwrap());
                let want = Hex::new(r[0].parse().unwrap(), r[1].parse().unwrap());
                assert_eq!(local(cell, level), want, "{line}");
            }
            "owned_count" => {
                let level = level_by_name(f[1]);
                let want: usize = r[0].parse().unwrap();
                assert_eq!(owned_offsets(level).len(), want, "{line}");
            }
            other => panic!("unknown case {other}"),
        }
        checked += 1;
    }
    assert!(checked > 10_000, "only {checked} cases checked");
}
```

- [ ] **Step 4: Run the test**

Run: `cd Experiments/exp-05 && cargo test -p hexworld --test exp03_agreement`
Expected: PASS. If a case disagrees, the port is wrong, not exp-03 — fix `owner.rs`.

- [ ] **Step 5: Commit**

```bash
git -C /home/lexa/DevProjects/_GameDev/Murabito add Experiments/exp-05
git -C /home/lexa/DevProjects/_GameDev/Murabito commit -m "exp-05: check the address port against exp-03's Python

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 4: Plane geometry, config, layers and addresses

**Files:**
- Create: `crates/hexworld/src/plane.rs`, `src/config.rs`, `src/address.rs`, `src/world.rs`
- Modify: `crates/hexworld/src/lib.rs`

**Interfaces:**
- Consumes: `Hex`, `Level`, `owner`, `up`, `local`, `parent_of` from Tasks 1–2.
- Produces:
  - `plane`: `cell_centre_m(Hex, Level) -> (f64, f64)` returning `(east, north)`; `metres_to_axial(f64, f64, Level) -> (f64, f64)`; `hex_round(f64, f64) -> Hex`; `round_at(f64, f64, Level) -> Hex`; `corners_m(Level) -> [(f64, f64); 6]`.
  - `config`: `WorldConfig { seed: u64, world_radius_ri: i32, layer_thickness_sun: f64, bottom_layer: i32 }` with `Default`, `layer_thickness_m()`, `layer_of_height(f64) -> i32`, `layer_bottom_m(i32) -> f64`, `layer_top_m(i32) -> f64`.
  - `address`: `Address { ri, cho, ken, shaku: Hex, layer: i32 }`; `address_of(Hex, i32) -> Address`; `shaku_of(&Address) -> (Hex, i32)`; `Display`.
  - `world`: `in_world(Hex, &WorldConfig) -> bool`; `world_ri(&WorldConfig) -> Vec<Hex>`; `cell_in_world(Hex, Level, &WorldConfig) -> bool`.

- [ ] **Step 1: Write the failing tests for `plane.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_origin_is_the_centre_of_every_level() {
        for level in crate::level::CELL_LEVELS {
            let (e, n) = cell_centre_m(Hex::ZERO, level);
            assert!(e.abs() < 1e-12 && n.abs() < 1e-12, "{level:?}");
        }
    }

    #[test]
    fn a_step_east_is_one_width() {
        let (e, n) = cell_centre_m(Hex::new(1, 0), Level::Shaku);
        assert!((e - Level::Shaku.width_m()).abs() < 1e-12);
        assert!(n.abs() < 1e-12);
    }

    #[test]
    fn metres_and_axial_round_trip() {
        for level in crate::level::CELL_LEVELS {
            for cell in [Hex::new(0, 0), Hex::new(5, -3), Hex::new(-17, 41)] {
                let (e, n) = cell_centre_m(cell, level);
                assert_eq!(round_at(e, n, level), cell, "{level:?} {cell:?}");
            }
        }
    }

    #[test]
    fn rounding_picks_the_nearest_centre() {
        // A point a tenth of a width east of a centre still belongs to that cell.
        let (e, n) = cell_centre_m(Hex::new(3, 2), Level::Ken);
        let nudged = (e + 0.1 * Level::Ken.width_m(), n);
        assert_eq!(round_at(nudged.0, nudged.1, Level::Ken), Hex::new(3, 2));
    }

    #[test]
    fn corners_are_a_pointy_top_hexagon() {
        let corners = corners_m(Level::Shaku);
        let w = Level::Shaku.width_m();
        // Pointy top: the tallest corner is at half the corner-to-corner height,
        // which is the width / sqrt(3).
        let max_north = corners.iter().map(|c| c.1).fold(f64::MIN, f64::max);
        assert!((max_north - w / 3f64.sqrt()).abs() < 1e-12, "{max_north}");
        let max_east = corners.iter().map(|c| c.0).fold(f64::MIN, f64::max);
        assert!((max_east - w / 2.0).abs() < 1e-12, "{max_east}");
    }
}
```

- [ ] **Step 2: Run to verify they fail**

Run: `cd Experiments/exp-05 && cargo test -p hexworld plane`
Expected: FAIL to compile.

- [ ] **Step 3: Implement `plane.rs`**

```rust
//! The hex plane in metres. Positions are `(east, north)`; height is separate.
//! Pointy-top cells: the position of axial (q, r), in units of the cell's flat-to-flat
//! width, is (q + r/2, r·√3/2).

use crate::hex::Hex;
use crate::level::Level;

const SQRT3: f64 = 1.732_050_807_568_877_2;

/// The centre of a cell, in metres east and north of the world centre.
pub fn cell_centre_m(cell: Hex, level: Level) -> (f64, f64) {
    let w = level.width_m();
    let q = cell.q as f64;
    let r = cell.r as f64;
    ((q + r / 2.0) * w, r * SQRT3 / 2.0 * w)
}

/// Fractional axial coordinates of a point at a level.
pub fn metres_to_axial(east: f64, north: f64, level: Level) -> (f64, f64) {
    let w = level.width_m();
    let r = north / (SQRT3 / 2.0 * w);
    (east / w - r / 2.0, r)
}

/// Round fractional axial coordinates to the nearest cell.
pub fn hex_round(fq: f64, fr: f64) -> Hex {
    let fs = -fq - fr;
    let (mut q, mut r, s) = (fq.round(), fr.round(), fs.round());
    let (dq, dr, ds) = ((q - fq).abs(), (r - fr).abs(), (s - fs).abs());
    if dq > dr && dq > ds {
        q = -r - s;
    } else if dr > ds {
        r = -q - s;
    }
    Hex::new(q as i32, r as i32)
}

/// The cell whose ideal hexagon contains the point, at the given level.
pub fn round_at(east: f64, north: f64, level: Level) -> Hex {
    let (fq, fr) = metres_to_axial(east, north, level);
    hex_round(fq, fr)
}

/// The six corners of a cell at this level, relative to its centre, anticlockwise.
/// Pointy top: a corner points north.
pub fn corners_m(level: Level) -> [(f64, f64); 6] {
    let w = level.width_m();
    let radius = w / SQRT3; // centre to corner
    let mut out = [(0.0, 0.0); 6];
    for (i, slot) in out.iter_mut().enumerate() {
        let angle = std::f64::consts::FRAC_PI_6 + i as f64 * std::f64::consts::FRAC_PI_3;
        *slot = (radius * angle.cos(), radius * angle.sin());
    }
    out
}
```

- [ ] **Step 4: Run to verify they pass**

Run: `cd Experiments/exp-05 && cargo test -p hexworld plane`
Expected: PASS, 5 tests. If `corners_are_a_pointy_top_hexagon` fails, the corner angles are out by 30°: a pointy-top cell has a corner due north, so the first corner is at 30°.

- [ ] **Step 5: Write the failing tests for `config.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_the_spec() {
        let c = WorldConfig::default();
        assert_eq!(c.world_radius_ri, 0);
        assert_eq!(c.layer_thickness_sun, 5.0);
        assert!((c.layer_thickness_m() - 5.0 / 33.0).abs() < 1e-12);
    }

    #[test]
    fn layer_zero_starts_at_height_zero() {
        let c = WorldConfig::default();
        assert_eq!(c.layer_of_height(0.0), 0);
        assert!((c.layer_bottom_m(0)).abs() < 1e-12);
        assert!((c.layer_top_m(0) - 5.0 / 33.0).abs() < 1e-12);
    }

    #[test]
    fn layers_are_signed_and_floor_downwards() {
        let c = WorldConfig::default();
        let t = c.layer_thickness_m();
        assert_eq!(c.layer_of_height(t * 3.5), 3);
        assert_eq!(c.layer_of_height(-0.001), -1);
        assert_eq!(c.layer_of_height(-t * 2.5), -3);
    }

    #[test]
    fn thickness_is_tunable() {
        let c = WorldConfig { layer_thickness_sun: 10.0, ..WorldConfig::default() };
        assert!((c.layer_thickness_m() - 10.0 / 33.0).abs() < 1e-12);
        assert_eq!(c.layer_of_height(10.0 / 33.0), 1);
    }
}
```

- [ ] **Step 6: Run to verify they fail, then implement `config.rs`**

```rust
//! World-wide settings. Everything generated is a pure function of these.

use crate::level::SUN_M;

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct WorldConfig {
    pub seed: u64,
    /// Radius in ri. 0 is the single ri (0, 0); 12 is exp-03's world.
    pub world_radius_ri: i32,
    /// Thickness of one layer, in sun. 5 sun is half a shaku.
    pub layer_thickness_sun: f64,
    /// The lowest layer any column reaches.
    pub bottom_layer: i32,
}

impl Default for WorldConfig {
    fn default() -> Self {
        WorldConfig {
            seed: 1,
            world_radius_ri: 0,
            layer_thickness_sun: 5.0,
            bottom_layer: -256,
        }
    }
}

impl WorldConfig {
    pub fn layer_thickness_m(&self) -> f64 {
        self.layer_thickness_sun * SUN_M
    }

    /// The layer containing a height. Layer k spans [k·t, (k+1)·t).
    pub fn layer_of_height(&self, height_m: f64) -> i32 {
        (height_m / self.layer_thickness_m()).floor() as i32
    }

    pub fn layer_bottom_m(&self, layer: i32) -> f64 {
        layer as f64 * self.layer_thickness_m()
    }

    pub fn layer_top_m(&self, layer: i32) -> f64 {
        (layer + 1) as f64 * self.layer_thickness_m()
    }
}
```

Run: `cd Experiments/exp-05 && cargo test -p hexworld config` — expected PASS, 4 tests.

- [ ] **Step 7: Write the failing tests for `address.rs` and `world.rs`**

```rust
// in address.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses_round_trip() {
        for shaku in [Hex::ZERO, Hex::new(1, -1), Hex::new(4_211, -9_003), Hex::new(-12_961, 7)] {
            for layer in [-7, 0, 213] {
                let addr = address_of(shaku, layer);
                assert_eq!(shaku_of(&addr), (shaku, layer), "{shaku:?} {layer}");
            }
        }
    }

    #[test]
    fn the_origin_is_the_centre_of_everything() {
        let addr = address_of(Hex::ZERO, 0);
        assert_eq!(addr.ri, Hex::ZERO);
        assert_eq!(addr.cho, Hex::ZERO);
        assert_eq!(addr.ken, Hex::ZERO);
        assert_eq!(addr.shaku, Hex::ZERO);
    }

    #[test]
    fn neighbouring_shaku_across_a_ken_border_differ_in_ken() {
        // The shaku east of a ken's eastern edge belongs to the next ken.
        let a = Hex::new(3, 0);
        let b = Hex::new(4, 0);
        let addr_a = address_of(a, 0);
        let addr_b = address_of(b, 0);
        assert_ne!(addr_a.ken, addr_b.ken, "{addr_a} vs {addr_b}");
    }

    #[test]
    fn display_lists_every_part() {
        let text = format!("{}", address_of(Hex::new(7, -3), 12));
        assert!(text.contains("ri ("), "{text}");
        assert!(text.contains("cho ("), "{text}");
        assert!(text.contains("ken ("), "{text}");
        assert!(text.contains("shaku ("), "{text}");
        assert!(text.contains("layer 12"), "{text}");
    }
}

// in world.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_world_is_one_ri() {
        let cfg = WorldConfig::default();
        assert_eq!(world_ri(&cfg), vec![Hex::ZERO]);
        assert!(in_world(Hex::ZERO, &cfg));
        assert!(!in_world(Hex::new(1, 0), &cfg));
    }

    #[test]
    fn radius_twelve_is_exp03s_world() {
        let cfg = WorldConfig { world_radius_ri: 12, ..WorldConfig::default() };
        assert_eq!(world_ri(&cfg).len(), 469);
    }

    #[test]
    fn cells_are_tested_through_their_ri() {
        let cfg = WorldConfig::default();
        assert!(cell_in_world(Hex::ZERO, Level::Shaku, &cfg));
        // A shaku far outside ri (0, 0): 3 ri east.
        let far = Hex::new(3 * Level::Ri.scale_shaku(), 0);
        assert!(!cell_in_world(far, Level::Shaku, &cfg));
    }
}
```

- [ ] **Step 8: Implement `address.rs` and `world.rs`**

```rust
// address.rs
//! Hierarchical addresses: ri (position in the world), then offsets at each level,
//! then the vertical layer.

use std::fmt;

use crate::hex::Hex;
use crate::level::Level;
use crate::owner::{local, parent_of};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Address {
    /// The ri's position in the world.
    pub ri: Hex,
    /// The cho's offset from its ri's centre, in cho.
    pub cho: Hex,
    /// The ken's offset from its cho's centre, in ken.
    pub ken: Hex,
    /// The shaku's offset from its ken's centre, in shaku.
    pub shaku: Hex,
    pub layer: i32,
}

pub fn address_of(shaku: Hex, layer: i32) -> Address {
    let ken = parent_of(shaku, Level::Shaku);
    let cho = parent_of(ken, Level::Ken);
    let ri = parent_of(cho, Level::Cho);
    Address {
        ri,
        cho: local(cho, Level::Cho),
        ken: local(ken, Level::Ken),
        shaku: local(shaku, Level::Shaku),
        layer,
    }
}

pub fn shaku_of(addr: &Address) -> (Hex, i32) {
    let cho = Hex::new(
        addr.ri.q * Level::Ri.packing() + addr.cho.q,
        addr.ri.r * Level::Ri.packing() + addr.cho.r,
    );
    let ken = Hex::new(
        cho.q * Level::Cho.packing() + addr.ken.q,
        cho.r * Level::Cho.packing() + addr.ken.r,
    );
    let shaku = Hex::new(
        ken.q * Level::Ken.packing() + addr.shaku.q,
        ken.r * Level::Ken.packing() + addr.shaku.r,
    );
    (shaku, addr.layer)
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ri ({},{}) / cho ({},{}) / ken ({},{}) / shaku ({},{}) / layer {}",
            self.ri.q, self.ri.r,
            self.cho.q, self.cho.r,
            self.ken.q, self.ken.r,
            self.shaku.q, self.shaku.r,
            self.layer
        )
    }
}
```

```rust
// world.rs
//! World bounds. Nothing outside them is generated or drawn.

use crate::config::WorldConfig;
use crate::hex::{distance, Hex};
use crate::level::Level;
use crate::owner::up;

pub fn in_world(ri: Hex, cfg: &WorldConfig) -> bool {
    distance(ri, Hex::ZERO) <= cfg.world_radius_ri
}

pub fn world_ri(cfg: &WorldConfig) -> Vec<Hex> {
    crate::hex::range(Hex::ZERO, cfg.world_radius_ri)
}

/// Whether a cell at any level lies in the world, tested through the ri that owns it.
pub fn cell_in_world(cell: Hex, level: Level, cfg: &WorldConfig) -> bool {
    if level == Level::World {
        return true;
    }
    in_world(up(cell, level, Level::Ri), cfg)
}
```

- [ ] **Step 9: Run the tests, export, and commit**

```bash
cd /home/lexa/DevProjects/_GameDev/Murabito/Experiments/exp-05
cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test -p hexworld
git -C /home/lexa/DevProjects/_GameDev/Murabito add Experiments/exp-05
git -C /home/lexa/DevProjects/_GameDev/Murabito commit -m "exp-05: plane geometry, world config, layers, addresses

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

Add to `lib.rs`: `pub mod address; pub mod config; pub mod plane; pub mod world;` and re-export `Address`, `address_of`, `shaku_of`, `WorldConfig`, `cell_centre_m`, `round_at`, `in_world`, `world_ri`, `cell_in_world`.

---

### Task 5: Noise and the placeholder terrain

**Files:**
- Create: `crates/hexworld/src/noise.rs`, `src/column.rs`, `src/placeholder_terrain.rs`
- Modify: `crates/hexworld/src/lib.rs`

**Interfaces:**
- Consumes: `WorldConfig`, `Level`, `Hex`, `cell_centre_m`.
- Produces:
  - `noise::gradient_2d(seed: u64, x: f64, y: f64) -> f64` in [−1, 1].
  - `column::{Material, Run, Column}`: `Material { Rock, Dirt, Grass }`; `Run { bottom: i32, top: i32, material: Material }` (inclusive both ends); `Column { runs: Vec<Run> }` with `top_layer() -> Option<i32>`, `material_at(i32) -> Option<Material>`, `is_solid(i32) -> bool`, `surface_height_m(&WorldConfig) -> f64`.
  - `placeholder_terrain::{height_m(&WorldConfig, f64, f64, Level) -> f64, column_at(&WorldConfig, Hex, Level) -> Column, OCTAVES}`.

- [ ] **Step 1: Write the failing tests for `noise.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_deterministic() {
        assert_eq!(gradient_2d(7, 1.5, -2.25), gradient_2d(7, 1.5, -2.25));
    }

    #[test]
    fn depends_on_the_seed() {
        assert_ne!(gradient_2d(1, 3.3, 4.4), gradient_2d(2, 3.3, 4.4));
    }

    #[test]
    fn stays_in_range_and_varies() {
        let mut min = f64::MAX;
        let mut max = f64::MIN;
        for i in 0..2_000 {
            let x = i as f64 * 0.37;
            let y = i as f64 * -0.11;
            let v = gradient_2d(42, x, y);
            assert!((-1.0..=1.0).contains(&v), "{v} out of range");
            min = min.min(v);
            max = max.max(v);
        }
        assert!(max - min > 0.5, "noise barely varies: {min}..{max}");
    }

    #[test]
    fn is_zero_at_lattice_points() {
        // Gradient noise vanishes on the lattice; this catches a value-noise mix-up.
        for i in -3..=3 {
            assert!(gradient_2d(9, i as f64, 2.0).abs() < 1e-12);
        }
    }
}
```

- [ ] **Step 2: Run to verify they fail, then implement `noise.rs`**

```rust
//! Hashed-lattice gradient noise. Pure, seeded, and dependency-free: the same point
//! always gives the same value, whichever chunk or thread asks.

/// A 64-bit mix (splitmix64's finaliser) for hashing lattice points.
fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut x = z;
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

fn gradient(seed: u64, ix: i64, iy: i64) -> (f64, f64) {
    let h = mix(seed ^ mix((ix as u64).wrapping_mul(0x1234_5678_9ABC_DEF1) ^ (iy as u64)));
    let angle = (h >> 11) as f64 / (1u64 << 53) as f64 * std::f64::consts::TAU;
    (angle.cos(), angle.sin())
}

fn fade(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// Gradient noise at a point, in [-1, 1].
pub fn gradient_2d(seed: u64, x: f64, y: f64) -> f64 {
    let (x0, y0) = (x.floor(), y.floor());
    let (ix, iy) = (x0 as i64, y0 as i64);
    let (fx, fy) = (x - x0, y - y0);
    let mut corners = [0.0; 4];
    for (i, slot) in corners.iter_mut().enumerate() {
        let (dx, dy) = ((i & 1) as f64, (i >> 1) as f64);
        let g = gradient(seed, ix + dx as i64, iy + dy as i64);
        *slot = g.0 * (fx - dx) + g.1 * (fy - dy);
    }
    let (u, v) = (fade(fx), fade(fy));
    let top = corners[0] + u * (corners[1] - corners[0]);
    let bottom = corners[2] + u * (corners[3] - corners[2]);
    // Gradient noise peaks near 1/sqrt(2); scale so the range is about [-1, 1].
    ((top + v * (bottom - top)) * std::f64::consts::SQRT_2).clamp(-1.0, 1.0)
}
```

Run: `cd Experiments/exp-05 && cargo test -p hexworld noise` — expected PASS, 4 tests.

- [ ] **Step 3: Write the failing tests for `column.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::WorldConfig;

    fn sample() -> Column {
        Column { runs: vec![
            Run { bottom: -64, top: 10, material: Material::Rock },
            Run { bottom: 11, top: 12, material: Material::Dirt },
            Run { bottom: 13, top: 13, material: Material::Grass },
        ] }
    }

    #[test]
    fn reports_its_top_layer() {
        assert_eq!(sample().top_layer(), Some(13));
        assert_eq!(Column { runs: vec![] }.top_layer(), None);
    }

    #[test]
    fn reads_material_by_layer() {
        let c = sample();
        assert_eq!(c.material_at(-64), Some(Material::Rock));
        assert_eq!(c.material_at(10), Some(Material::Rock));
        assert_eq!(c.material_at(11), Some(Material::Dirt));
        assert_eq!(c.material_at(13), Some(Material::Grass));
        assert_eq!(c.material_at(14), None);
        assert_eq!(c.material_at(-65), None);
    }

    #[test]
    fn solidity_follows_the_runs() {
        let c = Column { runs: vec![
            Run { bottom: 0, top: 2, material: Material::Rock },
            Run { bottom: 6, top: 7, material: Material::Rock },
        ] };
        assert!(c.is_solid(2));
        assert!(!c.is_solid(3), "the gap between runs is air");
        assert!(c.is_solid(6));
    }

    #[test]
    fn surface_height_is_the_top_of_the_top_run() {
        let cfg = WorldConfig::default();
        let expected = cfg.layer_top_m(13);
        assert!((sample().surface_height_m(&cfg) - expected).abs() < 1e-12);
    }
}
```

- [ ] **Step 4: Implement `column.rs`**

```rust
//! A column of voxels: a sorted list of runs of material, over the full height.
//! Gaps between runs are air, so overhangs and caves can be represented.

use crate::config::WorldConfig;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Material {
    Rock,
    Dirt,
    Grass,
}

impl Material {
    /// Placeholder colours, tuned by eye.
    pub fn colour(self) -> [f32; 3] {
        match self {
            Material::Rock => [0.45, 0.44, 0.42],
            Material::Dirt => [0.42, 0.31, 0.20],
            Material::Grass => [0.34, 0.50, 0.24],
        }
    }
}

/// A run of one material, from `bottom` to `top` in layers, both inclusive.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Run {
    pub bottom: i32,
    pub top: i32,
    pub material: Material,
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Column {
    /// Sorted by `bottom`, non-overlapping.
    pub runs: Vec<Run>,
}

impl Column {
    pub fn top_layer(&self) -> Option<i32> {
        self.runs.last().map(|r| r.top)
    }

    pub fn material_at(&self, layer: i32) -> Option<Material> {
        self.runs
            .iter()
            .find(|r| layer >= r.bottom && layer <= r.top)
            .map(|r| r.material)
    }

    pub fn is_solid(&self, layer: i32) -> bool {
        self.material_at(layer).is_some()
    }

    /// The height of the top of the column, in metres.
    pub fn surface_height_m(&self, cfg: &WorldConfig) -> f64 {
        self.top_layer().map_or(0.0, |l| cfg.layer_top_m(l))
    }
}
```

Run: `cd Experiments/exp-05 && cargo test -p hexworld column` — expected PASS, 4 tests.

- [ ] **Step 5: Write the failing tests for `placeholder_terrain.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::hex::Hex;

    #[test]
    fn height_is_deterministic() {
        let cfg = WorldConfig::default();
        let a = height_m(&cfg, 123.0, -45.0, Level::Shaku);
        let b = height_m(&cfg, 123.0, -45.0, Level::Shaku);
        assert_eq!(a, b);
    }

    #[test]
    fn a_coarse_sample_is_the_fine_sample_minus_the_dropped_octaves() {
        let cfg = WorldConfig::default();
        let (e, n) = (321.0, 654.0);
        let fine = height_m(&cfg, e, n, Level::Shaku);
        let coarse = height_m(&cfg, e, n, Level::Cho);
        let dropped: f64 = (0..OCTAVES)
            .filter(|k| !octave_kept(*k, Level::Cho))
            .map(|k| octave_value(&cfg, e, n, k))
            .sum();
        assert!((fine - coarse - dropped).abs() < 1e-9, "{fine} {coarse} {dropped}");
    }

    #[test]
    fn coarse_levels_keep_fewer_octaves() {
        let kept = |level| (0..OCTAVES).filter(|k| octave_kept(*k, level)).count();
        assert!(kept(Level::Shaku) > kept(Level::Ken));
        assert!(kept(Level::Ken) > kept(Level::Cho));
        assert!(kept(Level::Cho) >= kept(Level::Ri));
    }

    #[test]
    fn relief_is_in_the_expected_range() {
        // Placeholder country: tens of metres of relief across a ri, not hundreds.
        let cfg = WorldConfig::default();
        let mut min = f64::MAX;
        let mut max = f64::MIN;
        for i in 0..200 {
            let e = -1_900.0 + i as f64 * 19.0;
            let h = height_m(&cfg, e, 0.0, Level::Shaku);
            min = min.min(h);
            max = max.max(h);
        }
        assert!(max - min > 5.0, "too flat: {min}..{max}");
        assert!(max - min < 120.0, "too wild: {min}..{max}");
    }

    #[test]
    fn a_column_is_rock_then_dirt_then_grass() {
        let cfg = WorldConfig::default();
        let col = column_at(&cfg, Hex::new(3, -2), Level::Shaku);
        assert_eq!(col.runs.first().unwrap().material, Material::Rock);
        assert_eq!(col.runs.first().unwrap().bottom, cfg.bottom_layer);
        assert_eq!(col.runs.last().unwrap().material, Material::Grass);
        // Runs are sorted, touching and non-overlapping.
        for pair in col.runs.windows(2) {
            assert_eq!(pair[1].bottom, pair[0].top + 1, "{:?}", col.runs);
        }
    }

    #[test]
    fn a_columns_top_follows_the_height_function() {
        let cfg = WorldConfig::default();
        let cell = Hex::new(11, 5);
        let (e, n) = crate::plane::cell_centre_m(cell, Level::Shaku);
        let expected = cfg.layer_of_height(height_m(&cfg, e, n, Level::Shaku));
        assert_eq!(column_at(&cfg, cell, Level::Shaku).top_layer(), Some(expected));
    }

    #[test]
    fn columns_are_seeded() {
        let a = WorldConfig::default();
        let b = WorldConfig { seed: 99, ..a };
        let cell = Hex::new(4, 4);
        assert_ne!(column_at(&a, cell, Level::Shaku), column_at(&b, cell, Level::Shaku));
    }
}
```

- [ ] **Step 6: Implement `placeholder_terrain.rs`**

```rust
//! PLACEHOLDER terrain. This is not the real generator: it exists so that chunks carry
//! something real while the coordinate system and the chunk machinery are built. It is
//! point-evaluable, so any chunk at any level can be generated on its own, and a coarse
//! sample is exactly the fine sample minus the octaves that are too fine for the level.

use crate::column::{Column, Material, Run};
use crate::config::WorldConfig;
use crate::hex::Hex;
use crate::level::Level;
use crate::noise::gradient_2d;
use crate::plane::cell_centre_m;

/// Number of octaves. The first is 8 km across (about twice the ri width), the last about 0.5 m.
pub const OCTAVES: usize = 15;
const BASE_WAVELENGTH_M: f64 = 8_192.0;
const BASE_AMPLITUDE_M: f64 = 10.0;
const GAIN: f64 = 0.75;

fn wavelength(octave: usize) -> f64 {
    BASE_WAVELENGTH_M / (1 << octave) as f64
}

fn amplitude(octave: usize) -> f64 {
    BASE_AMPLITUDE_M * GAIN.powi(octave as i32)
}

/// Whether a level keeps an octave: only those at least twice its cell width.
pub fn octave_kept(octave: usize, level: Level) -> bool {
    if level == Level::World {
        return false;
    }
    wavelength(octave) >= 2.0 * level.width_m()
}

/// One octave's contribution at a point.
pub fn octave_value(cfg: &WorldConfig, east: f64, north: f64, octave: usize) -> f64 {
    let w = wavelength(octave);
    let seed = cfg.seed ^ (octave as u64).wrapping_mul(0x9E37_79B9);
    gradient_2d(seed, east / w, north / w) * amplitude(octave)
}

/// Terrain height in metres at a point, at a level's detail.
pub fn height_m(cfg: &WorldConfig, east: f64, north: f64, level: Level) -> f64 {
    (0..OCTAVES)
        .filter(|k| octave_kept(*k, level))
        .map(|k| octave_value(cfg, east, north, k))
        .sum()
}

/// The column at a cell, sampled at that cell's centre.
pub fn column_at(cfg: &WorldConfig, cell: Hex, level: Level) -> Column {
    let (east, north) = cell_centre_m(cell, level);
    let top = cfg.layer_of_height(height_m(cfg, east, north, level));
    let bottom = cfg.bottom_layer;
    if top < bottom {
        return Column::default();
    }
    let mut runs = Vec::with_capacity(3);
    // One layer of grass on top, two of dirt below it, rock all the way down.
    let dirt_bottom = (top - 2).max(bottom);
    let rock_top = dirt_bottom - 1;
    if rock_top >= bottom {
        runs.push(Run { bottom, top: rock_top, material: Material::Rock });
    }
    if dirt_bottom <= top - 1 {
        runs.push(Run { bottom: dirt_bottom, top: top - 1, material: Material::Dirt });
    }
    runs.push(Run { bottom: top, top, material: Material::Grass });
    Column { runs }
}
```

- [ ] **Step 7: Run the tests, export, and commit**

Run: `cd Experiments/exp-05 && cargo test -p hexworld` — expected PASS. If `relief_is_in_the_expected_range` fails, adjust `BASE_AMPLITUDE_M`; it is a placeholder, tuned by eye, and the test's bounds are deliberately loose.

```bash
cd /home/lexa/DevProjects/_GameDev/Murabito/Experiments/exp-05
cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test -p hexworld
git -C /home/lexa/DevProjects/_GameDev/Murabito add Experiments/exp-05
git -C /home/lexa/DevProjects/_GameDev/Murabito commit -m "exp-05: noise, columns and placeholder terrain

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 6: Chunks

**Files:**
- Create: `crates/hexworld/src/chunk.rs`
- Modify: `crates/hexworld/src/lib.rs`

**Interfaces:**
- Consumes: `Level`, `Hex`, `children`, `world_ri`, `column_at`, `WorldConfig`.
- Produces: `ChunkKey { level: Level, cell: Hex }` (`level` is the **parent's** level; the chunk holds columns for cells at `level.child()`), with `Ord`, `child_level()`, `WORLD: ChunkKey`, `parent_key() -> Option<ChunkKey>`, `ancestors() -> Vec<ChunkKey>`; `Chunk { key, cells: Vec<Hex>, columns: Vec<Column> }` with `column(Hex) -> Option<&Column>`, `len()`; `chunk_cells(&WorldConfig, ChunkKey) -> Vec<Hex>`; `generate(&WorldConfig, ChunkKey) -> Chunk`.

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_sizes_match_the_hierarchy() {
        let cfg = WorldConfig::default();
        assert_eq!(generate(&cfg, ChunkKey::new(Level::Ken, Hex::ZERO)).len(), 36);
        assert_eq!(generate(&cfg, ChunkKey::new(Level::Cho, Hex::ZERO)).len(), 3_600);
        assert_eq!(generate(&cfg, ChunkKey::new(Level::Ri, Hex::ZERO)).len(), 1_296);
        assert_eq!(generate(&cfg, ChunkKey::WORLD).len(), 1, "one ri in the default world");
    }

    #[test]
    fn the_world_chunk_grows_with_the_radius() {
        let cfg = WorldConfig { world_radius_ri: 12, ..WorldConfig::default() };
        assert_eq!(generate(&cfg, ChunkKey::WORLD).len(), 469);
    }

    #[test]
    fn generation_is_pure() {
        let cfg = WorldConfig::default();
        let key = ChunkKey::new(Level::Ken, Hex::new(2, -3));
        assert_eq!(generate(&cfg, key), generate(&cfg, key));
    }

    #[test]
    fn a_column_can_be_looked_up_by_cell() {
        let cfg = WorldConfig::default();
        let key = ChunkKey::new(Level::Ken, Hex::new(2, -3));
        let chunk = generate(&cfg, key);
        let cell = chunk.cells[7];
        assert_eq!(chunk.column(cell), Some(&crate::placeholder_terrain::column_at(&cfg, cell, Level::Shaku)));
        assert_eq!(chunk.column(Hex::new(99_999, 0)), None);
    }

    #[test]
    fn ancestors_run_up_to_the_world() {
        let key = ChunkKey::new(Level::Ken, Hex::new(2, -3));
        let ancestors = key.ancestors();
        assert_eq!(ancestors.len(), 3);
        assert_eq!(ancestors[0].level, Level::Cho);
        assert_eq!(ancestors[1].level, Level::Ri);
        assert_eq!(ancestors[2], ChunkKey::WORLD);
        assert!(ChunkKey::WORLD.ancestors().is_empty());
    }

    #[test]
    fn a_chunks_cells_are_its_parents_owned_children() {
        let key = ChunkKey::new(Level::Cho, Hex::new(1, 1));
        let cfg = WorldConfig::default();
        let cells = chunk_cells(&cfg, key);
        assert_eq!(cells.len(), 3_600);
        for c in &cells {
            assert_eq!(crate::owner::parent_of(*c, Level::Ken), Hex::new(1, 1));
        }
    }
}
```

- [ ] **Step 2: Run to verify they fail, then implement `chunk.rs`**

```rust
//! A chunk is one parent cell's owned children: the unit of generation, loading and
//! unloading. Its key names the parent's level and cell; its columns are at the level below.

use crate::column::Column;
use crate::config::WorldConfig;
use crate::hex::Hex;
use crate::level::Level;
use crate::owner::children;
use crate::placeholder_terrain::column_at;
use crate::world::world_ri;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct ChunkKey {
    /// The parent's level. The chunk holds columns for cells at `level.child()`.
    pub level: Level,
    pub cell: Hex,
}

impl ChunkKey {
    pub const WORLD: ChunkKey = ChunkKey { level: Level::World, cell: Hex::ZERO };

    pub const fn new(level: Level, cell: Hex) -> Self {
        ChunkKey { level, cell }
    }

    pub fn child_level(self) -> Level {
        self.level.child().expect("a chunk's level always has a child")
    }

    /// The chunk that holds this chunk's own cell as one of its columns.
    pub fn parent_key(self) -> Option<ChunkKey> {
        let parent_level = self.level.parent()?;
        Some(ChunkKey::new(parent_level, crate::owner::parent_of(self.cell, self.level)))
    }

    /// Every chunk above this one, nearest first.
    pub fn ancestors(self) -> Vec<ChunkKey> {
        let mut out = Vec::new();
        let mut key = self;
        while let Some(parent) = key.parent_key() {
            out.push(parent);
            key = parent;
        }
        out
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Chunk {
    pub key: ChunkKey,
    /// The cells this chunk holds, in a fixed order.
    pub cells: Vec<Hex>,
    /// One column per cell, in the same order.
    pub columns: Vec<Column>,
}

impl Chunk {
    pub fn column(&self, cell: Hex) -> Option<&Column> {
        self.cells.iter().position(|c| *c == cell).map(|i| &self.columns[i])
    }

    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }
}

/// The cells a chunk holds: its parent's owned children, or every ri for the world chunk.
pub fn chunk_cells(cfg: &WorldConfig, key: ChunkKey) -> Vec<Hex> {
    if key.level == Level::World {
        world_ri(cfg)
    } else {
        children(key.cell, key.level)
    }
}

/// Generate a chunk. A pure function of the config and the key.
pub fn generate(cfg: &WorldConfig, key: ChunkKey) -> Chunk {
    let cells = chunk_cells(cfg, key);
    let child_level = key.child_level();
    let columns = cells.iter().map(|c| column_at(cfg, *c, child_level)).collect();
    Chunk { key, cells, columns }
}
```

Note: `Chunk::column` scans the cell list. A cho chunk has 3,600 cells, so the mesher should walk `cells`/`columns` together rather than calling `column` in a loop. If profiling later shows this matters, add an index; don't add one now.

- [ ] **Step 3: Run the tests, export, and commit**

```bash
cd /home/lexa/DevProjects/_GameDev/Murabito/Experiments/exp-05
cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test -p hexworld
git -C /home/lexa/DevProjects/_GameDev/Murabito add Experiments/exp-05
git -C /home/lexa/DevProjects/_GameDev/Murabito commit -m "exp-05: chunks and chunk generation

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 7: Loaders, windows and the load order

**Files:**
- Create: `crates/hexworld/src/store.rs` (first half: requests and ordering)
- Modify: `crates/hexworld/src/lib.rs`

**Interfaces:**
- Consumes: `ChunkKey`, `Chunk`, `generate`, `Hex`, `Level`, `range`, `up`, `cell_in_world`, `cell_centre_m`, `WorldConfig`.
- Produces: `Rings { shaku: i32, ken: i32, cho: i32 }` with `Default` (3/3/3); `Loader { focus: Hex, rings: Rings }`; `requests(&WorldConfig, &Loader) -> Vec<ChunkKey>` (includes ancestors, clipped to the world); `coarseness_rank(Level) -> u8`; `load_order(&WorldConfig, &[Loader], keys: &mut Vec<ChunkKey>)`.

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn at_origin() -> Loader {
        Loader { focus: Hex::ZERO, rings: Rings::default() }
    }

    #[test]
    fn default_rings_request_the_expected_chunks() {
        let cfg = WorldConfig::default();
        let keys = requests(&cfg, &at_origin());
        let count = |level| keys.iter().filter(|k| k.level == level).count();
        assert_eq!(count(Level::Ken), 37, "shaku detail: the focus ken plus 3 rings");
        assert_eq!(count(Level::Cho), 37, "ken detail: the focus cho plus 3 rings");
        assert_eq!(count(Level::Ri), 1, "cho detail, clipped to a one-ri world");
        assert_eq!(count(Level::World), 1);
        assert_eq!(keys.len(), 76);
    }

    #[test]
    fn the_shaku_window_holds_1332_columns() {
        // The success criterion in the spec: 1,332 shaku columns in 37 ken chunks.
        let cfg = WorldConfig::default();
        let keys = requests(&cfg, &at_origin());
        let columns: usize = keys
            .iter()
            .filter(|k| k.level == Level::Ken)
            .map(|k| crate::chunk::chunk_cells(&cfg, *k).len())
            .sum();
        assert_eq!(columns, 1_332);
    }

    #[test]
    fn requests_always_include_ancestors() {
        let cfg = WorldConfig::default();
        // Odd rings: a wide shaku window with no ken or cho window at all.
        let loader = Loader { focus: Hex::new(500, -200), rings: Rings { shaku: 5, ken: 0, cho: 0 } };
        let keys = requests(&cfg, &loader);
        for key in &keys {
            for ancestor in key.ancestors() {
                assert!(keys.contains(&ancestor), "{key:?} is missing ancestor {ancestor:?}");
            }
        }
    }

    #[test]
    fn windows_are_clipped_to_the_world() {
        let cfg = WorldConfig::default();
        // A focus near the ri's edge: some of its cho window falls in neighbouring ri.
        let edge = Hex::new(Level::Ri.scale_shaku() / 2, 0);
        let keys = requests(&cfg, &Loader { focus: edge, rings: Rings::default() });
        for key in &keys {
            assert!(crate::world::cell_in_world(key.cell, key.level, &cfg), "{key:?} is outside");
        }
        assert!(keys.iter().filter(|k| k.level == Level::Cho).count() < 37, "should be clipped");
    }

    #[test]
    fn two_loaders_union_their_requests() {
        let cfg = WorldConfig::default();
        let a = at_origin();
        let b = Loader { focus: Hex::new(600, 600), rings: Rings::default() };
        let ka = requests(&cfg, &a);
        let kb = requests(&cfg, &b);
        let mut both: Vec<ChunkKey> = ka.iter().chain(kb.iter()).copied().collect();
        both.sort();
        both.dedup();
        assert!(both.len() > ka.len(), "the second loader should add chunks");
        for key in ka.iter().chain(kb.iter()) {
            assert!(both.contains(key));
        }
    }

    #[test]
    fn load_order_is_coarsest_first_then_nearest() {
        let cfg = WorldConfig::default();
        let loaders = [at_origin()];
        let mut keys = requests(&cfg, &loaders[0]);
        keys.reverse();
        load_order(&cfg, &loaders, &mut keys);
        assert_eq!(keys[0], ChunkKey::WORLD, "the world chunk loads first");
        let ranks: Vec<u8> = keys.iter().map(|k| coarseness_rank(k.level)).collect();
        assert!(ranks.windows(2).all(|w| w[0] <= w[1]), "coarse levels must come first");
        // Within the ken level, the focus's own chunk comes before the far edge of the window.
        let kens: Vec<ChunkKey> = keys.iter().filter(|k| k.level == Level::Ken).copied().collect();
        assert_eq!(kens[0].cell, Hex::ZERO);
    }

    #[test]
    fn load_order_is_deterministic() {
        let cfg = WorldConfig::default();
        let loaders = [at_origin()];
        let mut a = requests(&cfg, &loaders[0]);
        let mut b = a.clone();
        b.reverse();
        load_order(&cfg, &loaders, &mut a);
        load_order(&cfg, &loaders, &mut b);
        assert_eq!(a, b, "order must not depend on the input order");
    }
}
```

- [ ] **Step 2: Run to verify they fail**

Run: `cd Experiments/exp-05 && cargo test -p hexworld store`
Expected: FAIL to compile — `Rings`, `Loader`, `requests` not found.

- [ ] **Step 3: Implement the first half of `store.rs`**

```rust
//! Loaders, their windows, and the store that loads and unloads chunks.
//!
//! The store has no threads, no clock and no engine in it: the caller passes the time,
//! generates the chunks it is told to, and hands them back.

use std::collections::{HashMap, HashSet};

use crate::chunk::{Chunk, ChunkKey};
use crate::config::WorldConfig;
use crate::hex::{range, Hex};
use crate::level::Level;
use crate::owner::up;
use crate::plane::cell_centre_m;
use crate::world::cell_in_world;

/// How many rings of neighbouring cells a loader wants at each detail level.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Rings {
    /// Rings of ken around the loader's ken, loaded at shaku detail.
    pub shaku: i32,
    /// Rings of cho around the loader's cho, loaded at ken detail.
    pub ken: i32,
    /// Rings of ri around the loader's ri, loaded at cho detail.
    pub cho: i32,
}

impl Default for Rings {
    fn default() -> Self {
        Rings { shaku: 3, ken: 3, cho: 3 }
    }
}

/// Anything that wants the world loaded around it: the camera, an NPC, a building site.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Loader {
    /// Where it is, as a global shaku.
    pub focus: Hex,
    pub rings: Rings,
}

/// Coarse levels sort first. The world chunk is coarsest of all.
pub fn coarseness_rank(level: Level) -> u8 {
    match level {
        Level::World => 0,
        Level::Ri => 1,
        Level::Cho => 2,
        Level::Ken => 3,
        Level::Shaku => 4,
    }
}

/// Every chunk a loader wants, with ancestors, clipped to the world.
pub fn requests(cfg: &WorldConfig, loader: &Loader) -> Vec<ChunkKey> {
    let mut out: HashSet<ChunkKey> = HashSet::new();
    let windows = [
        (Level::Ken, loader.rings.shaku),
        (Level::Cho, loader.rings.ken),
        (Level::Ri, loader.rings.cho),
    ];
    for (level, rings) in windows {
        let centre = up(loader.focus, Level::Shaku, level);
        for cell in range(centre, rings) {
            if cell_in_world(cell, level, cfg) {
                out.insert(ChunkKey::new(level, cell));
            }
        }
    }
    out.insert(ChunkKey::WORLD);
    // Ancestors, so a loaded chunk's parent column always exists whatever the rings are.
    for key in out.clone() {
        for ancestor in key.ancestors() {
            out.insert(ancestor);
        }
    }
    let mut keys: Vec<ChunkKey> = out.into_iter().collect();
    keys.sort();
    keys
}

/// Sort chunks into load order: coarsest level first, then nearest to any loader,
/// with the key as a tie-break so the order never depends on the input order.
pub fn load_order(cfg: &WorldConfig, loaders: &[Loader], keys: &mut [ChunkKey]) {
    let _ = cfg;
    let distance2 = |key: &ChunkKey| -> f64 {
        if key.level == Level::World {
            return 0.0;
        }
        let (ce, cn) = cell_centre_m(key.cell, key.level);
        loaders
            .iter()
            .map(|l| {
                let (fe, fn_) = cell_centre_m(l.focus, Level::Shaku);
                (ce - fe).powi(2) + (cn - fn_).powi(2)
            })
            .fold(f64::MAX, f64::min)
    };
    keys.sort_by(|a, b| {
        coarseness_rank(a.level)
            .cmp(&coarseness_rank(b.level))
            .then(distance2(a).partial_cmp(&distance2(b)).expect("finite distances"))
            .then(a.cmp(b))
    });
}
```

- [ ] **Step 4: Run to verify the tests pass**

Run: `cd Experiments/exp-05 && cargo test -p hexworld store`
Expected: PASS, 7 tests.

- [ ] **Step 5: Commit**

```bash
cd /home/lexa/DevProjects/_GameDev/Murabito/Experiments/exp-05
cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test -p hexworld
git -C /home/lexa/DevProjects/_GameDev/Murabito add Experiments/exp-05
git -C /home/lexa/DevProjects/_GameDev/Murabito commit -m "exp-05: loader windows, ancestors and load order

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 8: The chunk store, with delayed unloading

**Files:**
- Modify: `crates/hexworld/src/store.rs` (second half)
- Modify: `crates/hexworld/src/lib.rs`

**Interfaces:**
- Consumes: everything from Task 7.
- Produces: `StoreSettings { unload_delay_s: f64, max_in_flight: usize }` with `Default` (5.0, 8); `StoreUpdate { to_load: Vec<ChunkKey>, to_unload: Vec<ChunkKey> }`; `ChunkStore` with `new(WorldConfig, StoreSettings)`, `update(&[Loader], f64) -> StoreUpdate`, `begin_load(ChunkKey)`, `insert(Chunk) -> bool`, `is_loaded(ChunkKey) -> bool`, `is_requested(ChunkKey) -> bool`, `is_lingering(ChunkKey) -> bool`, `chunk(ChunkKey) -> Option<&Chunk>`, `column(Level, Hex) -> Option<&Column>`, `finest_at(Hex) -> Option<Level>`, `surface_height_m(f64, f64) -> Option<f64>`, `config() -> &WorldConfig`, `settings_mut() -> &mut StoreSettings`, `clear()`, `stats() -> &Stats`; `Stats` with `per_level: [LevelStats; 5]`, `LevelStats { chunks: usize, columns: usize, lingering: usize, loads_last_second: usize, unloads_last_second: usize }`.

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod store_tests {
    use super::*;

    fn store() -> ChunkStore {
        ChunkStore::new(WorldConfig::default(), StoreSettings::default())
    }

    /// Drain the queue: generate and insert everything the store asks for.
    fn settle(store: &mut ChunkStore, loaders: &[Loader], now: f64) {
        loop {
            let update = store.update(loaders, now);
            if update.to_load.is_empty() {
                break;
            }
            for key in update.to_load {
                store.begin_load(key);
                let chunk = crate::chunk::generate(store.config(), key);
                store.insert(chunk);
            }
        }
    }

    #[test]
    fn settles_with_every_requested_chunk_loaded() {
        let mut s = store();
        let loaders = [Loader { focus: Hex::ZERO, rings: Rings::default() }];
        settle(&mut s, &loaders, 0.0);
        for key in requests(&WorldConfig::default(), &loaders[0]) {
            assert!(s.is_loaded(key), "{key:?} should be loaded");
        }
        assert_eq!(s.stats().per_level[Level::Ken as usize].chunks, 37);
        assert_eq!(s.stats().per_level[Level::Ken as usize].columns, 1_332);
    }

    #[test]
    fn parents_load_before_their_children() {
        let mut s = store();
        let loaders = [Loader { focus: Hex::ZERO, rings: Rings::default() }];
        let mut loaded: Vec<ChunkKey> = Vec::new();
        loop {
            let update = s.update(&loaders, 0.0);
            if update.to_load.is_empty() {
                break;
            }
            for key in update.to_load {
                for ancestor in key.ancestors() {
                    assert!(loaded.contains(&ancestor), "{key:?} before its ancestor {ancestor:?}");
                }
                s.begin_load(key);
                s.insert(crate::chunk::generate(s.config(), key));
                loaded.push(key);
            }
        }
    }

    #[test]
    fn a_chunk_out_of_range_lingers_then_unloads() {
        let mut s = store();
        let here = [Loader { focus: Hex::ZERO, rings: Rings::default() }];
        settle(&mut s, &here, 0.0);
        let far_key = ChunkKey::new(Level::Ken, up(Hex::new(18, 0), Level::Shaku, Level::Ken));
        assert!(s.is_loaded(far_key), "should be in the starting window");

        // Move the loader away. The chunk is no longer requested, but it stays loaded.
        let away = [Loader { focus: Hex::new(600, 0), rings: Rings::default() }];
        let update = s.update(&away, 1.0);
        assert!(update.to_unload.is_empty(), "nothing unloads before the delay");
        assert!(s.is_loaded(far_key) && s.is_lingering(far_key));

        // Still inside the 5 s delay.
        let update = s.update(&away, 4.0);
        assert!(update.to_unload.is_empty());
        assert!(s.is_loaded(far_key));

        // Past the delay.
        let update = s.update(&away, 6.5);
        assert!(update.to_unload.contains(&far_key), "should unload after the delay");
        assert!(!s.is_loaded(far_key));
    }

    #[test]
    fn a_chunk_requested_again_in_time_is_kept_and_not_reloaded() {
        let mut s = store();
        let here = [Loader { focus: Hex::ZERO, rings: Rings::default() }];
        settle(&mut s, &here, 0.0);
        let away = [Loader { focus: Hex::new(600, 0), rings: Rings::default() }];
        s.update(&away, 1.0);
        // Back again before the delay runs out.
        let update = s.update(&here, 3.0);
        assert!(update.to_load.is_empty(), "nothing needs reloading: {:?}", update.to_load);
        assert!(update.to_unload.is_empty());
        // And it no longer lingers, so it will not expire later.
        let update = s.update(&here, 99.0);
        assert!(update.to_unload.is_empty());
    }

    #[test]
    fn a_zero_delay_unloads_at_once() {
        let mut s = ChunkStore::new(
            WorldConfig::default(),
            StoreSettings { unload_delay_s: 0.0, ..StoreSettings::default() },
        );
        let here = [Loader { focus: Hex::ZERO, rings: Rings::default() }];
        settle(&mut s, &here, 0.0);
        let away = [Loader { focus: Hex::new(600, 0), rings: Rings::default() }];
        let update = s.update(&away, 0.0);
        assert!(!update.to_unload.is_empty(), "immediate unloading");
    }

    #[test]
    fn a_lingering_ancestor_is_kept_while_a_descendant_stays() {
        // Rings that keep a ken chunk but drop its cho: the cho must not unload.
        let mut s = store();
        let wide = [Loader { focus: Hex::ZERO, rings: Rings { shaku: 3, ken: 3, cho: 3 } }];
        settle(&mut s, &wide, 0.0);
        let narrow = [Loader { focus: Hex::ZERO, rings: Rings { shaku: 3, ken: 0, cho: 0 } }];
        s.update(&narrow, 1.0);
        let update = s.update(&narrow, 20.0);
        for key in &update.to_unload {
            for other in s.loaded_keys() {
                assert!(!other.ancestors().contains(key), "{key:?} is still an ancestor of {other:?}");
            }
        }
        // The ken chunks themselves are still requested, so their cho parents survive.
        assert!(s.is_loaded(ChunkKey::new(Level::Cho, Hex::ZERO)));
    }

    #[test]
    fn insert_drops_a_chunk_nobody_wants_any_more() {
        let mut s = store();
        let here = [Loader { focus: Hex::ZERO, rings: Rings::default() }];
        let update = s.update(&here, 0.0);
        let key = *update.to_load.last().expect("something to load");
        s.begin_load(key);
        // The loader leaves before the chunk arrives.
        let away = [Loader { focus: Hex::new(50_000, 0), rings: Rings::default() }];
        s.update(&away, 0.0);
        let accepted = s.insert(crate::chunk::generate(s.config(), key));
        assert!(!accepted, "a chunk nobody wants is dropped");
        assert!(!s.is_loaded(key));
    }

    #[test]
    fn lookups_report_the_finest_detail_loaded() {
        let mut s = store();
        let here = [Loader { focus: Hex::ZERO, rings: Rings::default() }];
        settle(&mut s, &here, 0.0);
        assert_eq!(s.finest_at(Hex::ZERO), Some(Level::Shaku));
        // Far away, inside the ri, only coarse detail is loaded.
        let far = Hex::new(3_000, 0);
        let level = s.finest_at(far).expect("something is loaded everywhere in the ri");
        assert!(level > Level::Shaku, "{level:?}");
        assert!(s.column(Level::Shaku, Hex::ZERO).is_some());
        assert!(s.surface_height_m(0.0, 0.0).is_some());
    }

    #[test]
    fn stats_count_loads_and_unloads_in_the_last_second() {
        let mut s = store();
        let here = [Loader { focus: Hex::ZERO, rings: Rings::default() }];
        settle(&mut s, &here, 0.0);
        assert!(s.stats().per_level[Level::Ken as usize].loads_last_second > 0);
        // Two seconds later, with nothing happening, the counts fall back to zero.
        s.update(&here, 2.0);
        assert_eq!(s.stats().per_level[Level::Ken as usize].loads_last_second, 0);
    }
}
```

- [ ] **Step 2: Run to verify they fail, then implement the rest of `store.rs`**

```rust
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct StoreSettings {
    /// How long a chunk stays loaded after nothing wants it. 0 unloads at once.
    pub unload_delay_s: f64,
    /// How many chunks the caller should generate at once. The store only reports it.
    pub max_in_flight: usize,
}

impl Default for StoreSettings {
    fn default() -> Self {
        StoreSettings { unload_delay_s: 5.0, max_in_flight: 8 }
    }
}

#[derive(Clone, Default, Debug)]
pub struct StoreUpdate {
    /// In load order: generate these and hand them back with `insert`.
    pub to_load: Vec<ChunkKey>,
    /// Already removed from the store; drop whatever the caller built from them.
    pub to_unload: Vec<ChunkKey>,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct LevelStats {
    pub chunks: usize,
    pub columns: usize,
    pub lingering: usize,
    pub loads_last_second: usize,
    pub unloads_last_second: usize,
}

#[derive(Clone, Default, Debug)]
pub struct Stats {
    /// Indexed by `Level as usize`.
    pub per_level: [LevelStats; 5],
}

pub struct ChunkStore {
    cfg: WorldConfig,
    settings: StoreSettings,
    loaded: HashMap<ChunkKey, Chunk>,
    /// Loaded but no longer requested, with the time it fell out of every window.
    lingering: HashMap<ChunkKey, f64>,
    /// Handed out by `update` and not yet returned by `insert`.
    in_flight: HashSet<ChunkKey>,
    requested: HashSet<ChunkKey>,
    /// (time, level, was_load) for the last second, for the stats.
    events: Vec<(f64, Level, bool)>,
    stats: Stats,
}

impl ChunkStore {
    pub fn new(cfg: WorldConfig, settings: StoreSettings) -> Self {
        ChunkStore {
            cfg,
            settings,
            loaded: HashMap::new(),
            lingering: HashMap::new(),
            in_flight: HashSet::new(),
            requested: HashSet::new(),
            events: Vec::new(),
            stats: Stats::default(),
        }
    }

    pub fn config(&self) -> &WorldConfig {
        &self.cfg
    }

    pub fn settings_mut(&mut self) -> &mut StoreSettings {
        &mut self.settings
    }

    pub fn is_loaded(&self, key: ChunkKey) -> bool {
        self.loaded.contains_key(&key)
    }

    pub fn is_requested(&self, key: ChunkKey) -> bool {
        self.requested.contains(&key)
    }

    pub fn is_lingering(&self, key: ChunkKey) -> bool {
        self.lingering.contains_key(&key)
    }

    pub fn chunk(&self, key: ChunkKey) -> Option<&Chunk> {
        self.loaded.get(&key)
    }

    pub fn loaded_keys(&self) -> Vec<ChunkKey> {
        self.loaded.keys().copied().collect()
    }

    /// Everything the loaders want, then what to load and what has just been dropped.
    pub fn update(&mut self, loaders: &[Loader], now: f64) -> StoreUpdate {
        self.requested.clear();
        for loader in loaders {
            for key in requests(&self.cfg, loader) {
                self.requested.insert(key);
            }
        }

        // Anything requested and not already here or on its way.
        let mut to_load: Vec<ChunkKey> = self
            .requested
            .iter()
            .copied()
            .filter(|k| !self.loaded.contains_key(k) && !self.in_flight.contains(k))
            .collect();
        load_order(&self.cfg, loaders, &mut to_load);

        // Lingering: requested again clears the stamp; newly unwanted gets one.
        for key in self.loaded.keys().copied().collect::<Vec<_>>() {
            if self.requested.contains(&key) {
                self.lingering.remove(&key);
            } else {
                self.lingering.entry(key).or_insert(now);
            }
        }

        // Expired, unless something still loaded needs them as an ancestor.
        let expired: HashSet<ChunkKey> = self
            .lingering
            .iter()
            .filter(|(_, since)| now - **since >= self.settings.unload_delay_s)
            .map(|(k, _)| *k)
            .collect();
        let mut keep: HashSet<ChunkKey> = HashSet::new();
        for key in self.loaded.keys() {
            if !expired.contains(key) {
                for ancestor in key.ancestors() {
                    keep.insert(ancestor);
                }
            }
        }
        let mut to_unload: Vec<ChunkKey> = expired.difference(&keep).copied().collect();
        to_unload.sort();
        for key in &to_unload {
            self.loaded.remove(key);
            self.lingering.remove(key);
            self.events.push((now, key.level, false));
        }

        self.refresh_stats(now);
        StoreUpdate { to_load, to_unload }
    }

    /// Tell the store a chunk is being generated, so it is not handed out again.
    pub fn begin_load(&mut self, key: ChunkKey) {
        self.in_flight.insert(key);
    }

    /// Store a finished chunk. Returns false if nothing wants it any more.
    pub fn insert(&mut self, chunk: Chunk) -> bool {
        let key = chunk.key;
        self.in_flight.remove(&key);
        if !self.requested.contains(&key) {
            return false;
        }
        self.events.push((self.events.last().map_or(0.0, |e| e.0), key.level, true));
        self.loaded.insert(key, chunk);
        self.lingering.remove(&key);
        true
    }

    pub fn clear(&mut self) {
        self.loaded.clear();
        self.lingering.clear();
        self.in_flight.clear();
        self.requested.clear();
        self.events.clear();
        self.stats = Stats::default();
    }

    /// The column for a cell at a level, if its chunk is loaded.
    pub fn column(&self, level: Level, cell: Hex) -> Option<&crate::column::Column> {
        let parent_level = level.parent()?;
        let key = if parent_level == Level::World {
            ChunkKey::WORLD
        } else {
            ChunkKey::new(parent_level, crate::owner::parent_of(cell, level))
        };
        self.loaded.get(&key)?.column(cell)
    }

    /// The finest level loaded at a shaku, if any.
    pub fn finest_at(&self, shaku: Hex) -> Option<Level> {
        for level in crate::level::CELL_LEVELS {
            let cell = up(shaku, Level::Shaku, level);
            if self.column(level, cell).is_some() {
                return Some(level);
            }
        }
        None
    }

    /// The height of the ground at a point, at the finest detail loaded there.
    pub fn surface_height_m(&self, east: f64, north: f64) -> Option<f64> {
        for level in crate::level::CELL_LEVELS {
            let cell = crate::plane::round_at(east, north, level);
            if let Some(column) = self.column(level, cell) {
                return Some(column.surface_height_m(&self.cfg));
            }
        }
        None
    }

    pub fn stats(&self) -> &Stats {
        &self.stats
    }

    fn refresh_stats(&mut self, now: f64) {
        self.events.retain(|(t, _, _)| now - *t < 1.0);
        let mut stats = Stats::default();
        for (key, chunk) in &self.loaded {
            let slot = &mut stats.per_level[key.level as usize];
            slot.chunks += 1;
            slot.columns += chunk.len();
            if self.lingering.contains_key(key) {
                slot.lingering += 1;
            }
        }
        for (_, level, was_load) in &self.events {
            let slot = &mut stats.per_level[*level as usize];
            if *was_load {
                slot.loads_last_second += 1;
            } else {
                slot.unloads_last_second += 1;
            }
        }
        self.stats = stats;
    }
}
```

Note on `insert` and time: the store has no clock, so a load event is stamped with the time of the last event. That is good enough for a per-second counter that `update` refreshes every frame. If the counts look wrong in the viewer, give `insert` a `now` parameter rather than inventing a clock in the core.

- [ ] **Step 3: Run the tests**

Run: `cd Experiments/exp-05 && cargo test -p hexworld store`
Expected: PASS, 16 tests (7 from Task 7, 9 here).

- [ ] **Step 4: Export from `lib.rs` and commit**

At this point `lib.rs` declares every module and re-exports the names later tasks and the
plugin's tests use. It must read:

```rust
pub mod address;
pub mod chunk;
pub mod column;
pub mod config;
pub mod hex;
pub mod level;
pub mod mesh;      // added in Task 9
pub mod noise;
pub mod owner;
pub mod placeholder_terrain;
pub mod plane;
pub mod store;
pub mod world;

pub use address::{address_of, shaku_of, Address};
pub use chunk::{generate, Chunk, ChunkKey};
pub use column::{Column, Material, Run};
pub use config::WorldConfig;
pub use hex::{d2, distance, neighbours, range, Hex, DIRECTIONS};
pub use level::{Level, CELL_LEVELS, SHAKU_M, SUN_M};
pub use owner::{children, drawn_offsets, owned_offsets, owner, parent_of, up};
pub use store::{ChunkStore, LevelStats, Loader, Rings, Stats, StoreSettings, StoreUpdate};
pub use world::{cell_in_world, in_world, world_ri};
```

`mesh` is added in Task 9; leave it out until then.

```bash
cd /home/lexa/DevProjects/_GameDev/Murabito/Experiments/exp-05
cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test -p hexworld
git -C /home/lexa/DevProjects/_GameDev/Murabito add Experiments/exp-05
git -C /home/lexa/DevProjects/_GameDev/Murabito commit -m "exp-05: chunk store with delayed unloading, lookups and stats

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 9: The mesher — drawn set, prisms and faces

**Files:**
- Create: `crates/hexworld/src/mesh.rs`
- Modify: `crates/hexworld/src/owner.rs` (add `drawn_offsets`), `src/lib.rs`

**Interfaces:**
- Consumes: `Chunk`, `ChunkKey`, `Column`, `Run`, `Material`, `corners_m`, `cell_centre_m`, `d2`, `owner`, `column_at`, `cell_in_world`, `WorldConfig`.
- Produces:
  - `owner::drawn_offsets(Level) -> &'static [Hex]` — the child cells a chunk draws: its owned children, plus the "guests" tied on the boundary of its ideal hexagon. All integer arithmetic (a Voronoi membership test against `d2`), not `round_at`.
  - `mesh::MeshData { positions: Vec<[f32; 3]>, normals: Vec<[f32; 3]>, colours: Vec<[f32; 4]>, line: Vec<[f32; 4]>, indices: Vec<u32> }` with `triangle_count() -> usize`.
  - `mesh::mesh_chunk(&WorldConfig, &Chunk, &HashSet<Hex>) -> MeshData` — the third argument is the child cells drawn by their own chunk, which this chunk leaves out.
  - `mesh::drawn_cells(&WorldConfig, ChunkKey) -> Vec<Hex>`.

Positions are `(east, north, height)` in metres **relative to the chunk's own centre**, so the caller places the chunk entity at that centre.

- [ ] **Step 1: Write the failing tests for `drawn_offsets`**

```rust
// in owner.rs
#[cfg(test)]
mod drawn_tests {
    use super::*;

    #[test]
    fn drawn_counts_are_exact() {
        // 36 owned + 7 guests, 3,600 + 61, 1,296 + 37: guests are ties on the hexagon
        // border, in addition to (never instead of) the owned cells.
        assert_eq!(drawn_offsets(Level::Ken).len(), 43);
        assert_eq!(drawn_offsets(Level::Cho).len(), 3_661);
        assert_eq!(drawn_offsets(Level::Ri).len(), 1_333);
    }

    #[test]
    fn the_drawn_set_contains_every_owned_cell() {
        for level in [Level::Ken, Level::Cho, Level::Ri] {
            let drawn: Vec<Hex> = drawn_offsets(level).to_vec();
            for off in owned_offsets(level) {
                assert!(drawn.contains(off), "{level:?} {off:?} owned but not drawn");
            }
        }
    }

    #[test]
    fn every_cell_is_drawn_by_its_owner() {
        // Over a patch of child cells, each cell's owner lists it among its children,
        // and the offset from that owner's centre is in the owner's drawn set.
        // No `round_at`: everything here is integer arithmetic, so it holds everywhere,
        // not just away from a boundary.
        let level = Level::Ken;
        let n = level.packing();
        for q in -9..=9 {
            for r in -9..=9 {
                let cell = Hex::new(q, r);
                let parent = owner(cell, n);
                assert!(
                    children(parent, level).contains(&cell),
                    "{cell:?} not listed by its owner {parent:?}"
                );
                let centre = centre_child(parent, level);
                let offset = Hex::new(cell.q - centre.q, cell.r - centre.r);
                assert!(
                    drawn_offsets(level).contains(&offset),
                    "{cell:?} offset {offset:?} not drawn by its owner {parent:?}"
                );
            }
        }
    }

    #[test]
    fn guests_are_tie_cells_owned_by_a_neighbour() {
        // Every offset drawn but not owned is a genuine tie — its d2 to the centre
        // matches the minimum over all nine candidate parents — and belongs to one of
        // those neighbours, not to the centre itself.
        for level in [Level::Ken, Level::Cho, Level::Ri] {
            let n = level.packing();
            let owned = owned_offsets(level);
            for off in drawn_offsets(level) {
                if owned.contains(off) {
                    continue;
                }
                assert_ne!(
                    owner(*off, n),
                    Hex::ZERO,
                    "{level:?} {off:?} is a guest but owns itself"
                );
                let mine = d2(*off);
                let nearest = (-1..=1)
                    .flat_map(|a| (-1..=1).map(move |b| (a, b)))
                    .map(|(a, b)| d2(Hex::new(off.q - n * a, off.r - n * b)))
                    .min()
                    .expect("nine candidates");
                assert_eq!(mine, nearest, "{level:?} {off:?} is a guest but not a tie");
            }
        }
    }

    #[test]
    fn the_drawn_set_is_about_the_size_of_the_owned_set() {
        // Same area, different shape: the ideal hexagon instead of battlements.
        for level in [Level::Ken, Level::Cho, Level::Ri] {
            let drawn = drawn_offsets(level).len() as f64;
            let owned = owned_offsets(level).len() as f64;
            assert!((drawn - owned).abs() / owned < 0.2, "{level:?}: {drawn} vs {owned}");
        }
    }
}
```

- [ ] **Step 2: Implement `drawn_offsets` in `owner.rs`**

```rust
/// Offsets, from the centre child, of the children a cell at this level *draws*: every
/// child whose centre lies in this cell's ideal hexagon, boundary included. That is the
/// cells it owns, plus the "guests" — cells sitting exactly on the hexagon's border whose
/// ownership tie went to a neighbour. A guest's own hexagon straddles the border, so
/// drawing it is what stops a half-cell gap appearing along a handover.
///
/// All integer arithmetic. Deriving this from `round_at` in f64 does not work: ties on the
/// border resolve inconsistently and the result stops being a partition.
pub fn drawn_offsets(level: Level) -> &'static [Hex] {
    static CACHE: [OnceLock<Vec<Hex>>; 5] = [
        OnceLock::new(),
        OnceLock::new(),
        OnceLock::new(),
        OnceLock::new(),
        OnceLock::new(),
    ];
    let slot = &CACHE[level as usize];
    slot.get_or_init(|| {
        let n = level.packing();
        let mut out = Vec::new();
        for q in -n..=n {
            for r in -n..=n {
                let c = Hex::new(q, r);
                let mine = d2(c);
                let nearest = (-1..=1)
                    .flat_map(|a| (-1..=1).map(move |b| (a, b)))
                    .map(|(a, b)| d2(Hex::new(q - n * a, r - n * b)))
                    .min()
                    .expect("nine candidates");
                if mine == nearest {
                    out.push(c);
                }
            }
        }
        out
    })
}
```

- [ ] **Step 3: Run to verify the `drawn_offsets` tests pass**

Run: `cd Experiments/exp-05 && cargo test -p hexworld drawn`
Expected: PASS, 5 tests.

- [ ] **Step 4: Write the failing tests for `mesh.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn ken_chunk() -> (WorldConfig, crate::chunk::Chunk) {
        let cfg = WorldConfig::default();
        let chunk = crate::chunk::generate(&cfg, ChunkKey::new(Level::Ken, Hex::ZERO));
        (cfg, chunk)
    }

    #[test]
    fn draws_the_ideal_hexagons_worth_of_cells() {
        let cfg = WorldConfig::default();
        let cells = drawn_cells(&cfg, ChunkKey::new(Level::Ken, Hex::ZERO));
        assert_eq!(cells.len(), crate::owner::drawn_offsets(Level::Ken).len());
    }

    #[test]
    fn produces_consistent_buffers() {
        let (cfg, chunk) = ken_chunk();
        let mesh = mesh_chunk(&cfg, &chunk, &HashSet::new());
        assert_eq!(mesh.positions.len(), mesh.normals.len());
        assert_eq!(mesh.positions.len(), mesh.colours.len());
        assert_eq!(mesh.positions.len(), mesh.line.len());
        assert_eq!(mesh.indices.len() % 3, 0);
        assert!(mesh.indices.iter().all(|i| (*i as usize) < mesh.positions.len()));
        assert!(mesh.triangle_count() > 0);
    }

    #[test]
    fn top_faces_point_up_and_wind_anticlockwise() {
        let (cfg, chunk) = ken_chunk();
        let mesh = mesh_chunk(&cfg, &chunk, &HashSet::new());
        let mut top_faces = 0;
        for tri in mesh.indices.chunks(3) {
            let p: Vec<[f32; 3]> = tri.iter().map(|i| mesh.positions[*i as usize]).collect();
            let up = mesh.normals[tri[0] as usize][2] > 0.9; // height is the third component
            if !up {
                continue;
            }
            top_faces += 1;
            // Cross product of the edges, height component, must be positive: anticlockwise
            // seen from above, which is Bevy's front face after the axis mapping.
            let (a, b, c) = (p[0], p[1], p[2]);
            let (ux, uy) = (b[0] - a[0], b[1] - a[1]);
            let (vx, vy) = (c[0] - a[0], c[1] - a[1]);
            assert!(ux * vy - uy * vx > 0.0, "clockwise top face: {p:?}");
        }
        assert!(top_faces > 0, "a chunk should have top faces");
    }

    #[test]
    fn a_flat_columns_top_sits_at_its_layer_top() {
        let cfg = WorldConfig::default();
        let chunk = crate::chunk::generate(&cfg, ChunkKey::new(Level::Ken, Hex::ZERO));
        let mesh = mesh_chunk(&cfg, &chunk, &HashSet::new());
        let centre_column = chunk.column(Hex::ZERO).expect("the centre shaku");
        let expected = centre_column.surface_height_m(&cfg) as f32;
        // The centre cell's top face is at the chunk's own centre, at (0, 0) horizontally.
        let at_centre = mesh
            .positions
            .iter()
            .filter(|p| p[0].abs() < 1e-4 && p[1].abs() < 1e-4)
            .map(|p| p[2])
            .fold(f32::MIN, f32::max);
        assert!((at_centre - expected).abs() < 1e-3, "{at_centre} vs {expected}");
    }

    #[test]
    fn an_omitted_cell_is_gone_and_its_hole_is_walled() {
        let (cfg, chunk) = ken_chunk();
        let all = mesh_chunk(&cfg, &chunk, &HashSet::new());
        let mut omitted = HashSet::new();
        omitted.insert(Hex::ZERO);
        let fewer = mesh_chunk(&cfg, &chunk, &omitted);

        // The omitted cell's own geometry is gone. Only a cell's own top-face fan puts a
        // vertex exactly at its centre, so no vertex there means the cell is not drawn.
        assert!(!fewer.positions.iter().any(|p| p[0].abs() < 1e-4 && p[1].abs() < 1e-4));
        assert!(all.positions.iter().any(|p| p[0].abs() < 1e-4 && p[1].abs() < 1e-4));

        // Omitting a cell opens a boundary between detail levels, so its neighbours wall the
        // hole down to the world bottom: that is what stops a height step becoming a crack.
        // We deliberately do NOT assert on the raw triangle count here: whether omitting an
        // interior cell nets more or fewer triangles overall depends on meshing details
        // (how many material runs a column has, how many full-depth walls happen to merge)
        // that have nothing to do with the behaviour under test. Measured for this chunk and
        // config: 354 triangles all-drawn vs 352 with the centre omitted, and 0 vs 12 vertices
        // at the world bottom near the hole — fewer total triangles, but still walled.
        let bottom = cfg.layer_bottom_m(cfg.bottom_layer) as f32;
        let walls_at_bottom = |m: &MeshData| {
            m.positions
                .iter()
                .filter(|p| (p[2] - bottom).abs() < 1e-3)
                .filter(|p| (p[0] * p[0] + p[1] * p[1]).sqrt() < Level::Shaku.width_m() as f32 * 1.2)
                .count()
        };
        // Exactly one full-depth quad per walled edge: 6 edges × 2 bottom vertices = 12.
        // An inequality would not discriminate — emitting one quad per run (the bug this
        // replaced) gives 36 and would still satisfy `> 0`.
        assert_eq!(walls_at_bottom(&all), 0);
        assert_eq!(walls_at_bottom(&fewer), 12);
    }

    #[test]
    fn a_full_depth_wall_is_one_quad_even_when_the_column_has_a_gap() {
        use crate::column::{Material, Run};

        let cfg = WorldConfig::default();
        let key = ChunkKey::new(Level::Ken, Hex::ZERO);
        let mut chunk = crate::chunk::generate(&cfg, key);
        // A column with an air gap: rock low down, grass higher up, nothing between.
        let gapped = Column {
            runs: vec![
                Run {
                    bottom: cfg.bottom_layer,
                    top: -8,
                    material: Material::Rock,
                },
                Run {
                    bottom: -3,
                    top: -1,
                    material: Material::Grass,
                },
            ],
        };
        // Put it on any owned cell, and force one of its outward faces to be full depth by
        // omitting one neighbour — with the default world (radius 0) a `Ken` chunk near the
        // origin sits nowhere near the world's actual edge, so "the chunk's own border" does
        // not by itself make a neighbour full-depth; omitting one does, regardless of chunk
        // or world geometry.
        let target = chunk.cells[0];
        let index = chunk.cells.iter().position(|c| *c == target).unwrap();
        chunk.columns[index] = gapped;
        let mut omitted = HashSet::new();
        omitted.insert(target + DIRECTIONS[0]);

        let mesh = mesh_chunk(&cfg, &chunk, &omitted);

        // Vertices at the world bottom, facing exactly direction 0 (the omitted neighbour's
        // direction) and close to target. Filtering on the outward normal as well as
        // position is what makes this robust: `target` sits on this chunk's own perimeter
        // (`chunk.cells[0]`, `Hex { q: -4, r: 2 }` for this chunk), so most of its edges are
        // full depth already (this chunk's own border walls down to the world bottom all the
        // way around, per this round's fix), and several *other* cells elsewhere on the same
        // perimeter also happen to face direction 0. Those are far away (the nearest is
        // more than a chunk-width away) and have a different outward normal from any cell
        // whose edge faces a *different* direction, including target's own other four
        // full-depth edges and the one drawn neighbour that shares a corner with this edge
        // (its wall faces direction 5, not 0) — so normal plus a modest radius isolates
        // target's own direction-0 quad exactly, regardless of how the rest of the chunk's
        // perimeter behaves. One quad is 2 vertices; the old per-run code gave 4 for this
        // two-run column (one quad per run, each starting at the world bottom).
        let bottom = cfg.layer_bottom_m(cfg.bottom_layer) as f32;
        let (tx, ty) = cell_centre_m(target, Level::Shaku);
        let (tx, ty) = (tx as f32, ty as f32);
        let corners = corners_m(Level::Shaku);
        let (ax, ay) = corners[5];
        let (bx, by) = corners[0];
        let (mx, my) = ((ax + bx) / 2.0, (ay + by) / 2.0);
        let len = (mx * mx + my * my).sqrt();
        let (dir0_nx, dir0_ny) = ((mx / len) as f32, (my / len) as f32);
        let walls_at_bottom = mesh
            .positions
            .iter()
            .enumerate()
            .filter(|(_, p)| (p[2] - bottom).abs() < 1e-3)
            .filter(|(_, p)| {
                let (dx, dy) = (p[0] - tx, p[1] - ty);
                (dx * dx + dy * dy).sqrt() < Level::Shaku.width_m() as f32
            })
            .filter(|(i, _)| {
                let n = mesh.normals[*i];
                (n[0] - dir0_nx).abs() < 1e-4 && (n[1] - dir0_ny).abs() < 1e-4
            })
            .count();
        assert_eq!(walls_at_bottom, 2);

        // No two triangles may share all three vertex positions: overlapping full-depth
        // walls would produce exactly that.
        let mut seen: Vec<[[f32; 3]; 3]> = Vec::new();
        for tri in mesh.indices.chunks(3) {
            let mut v = [
                mesh.positions[tri[0] as usize],
                mesh.positions[tri[1] as usize],
                mesh.positions[tri[2] as usize],
            ];
            v.sort_by(|a, b| a.partial_cmp(b).unwrap());
            assert!(!seen.contains(&v), "duplicate triangle: {v:?}");
            seen.push(v);
        }
    }

    #[test]
    fn a_chunks_perimeter_walls_down_even_with_nothing_omitted() {
        let (cfg, chunk) = ken_chunk();
        // `chunk.cells[0]` (`Hex { q: -4, r: 2 }`) is a real perimeter cell of this chunk:
        // its neighbours in directions 1 through 4 belong to sibling chunks, not this one.
        // Its direction-1 neighbour is in-world and not omitted — the *only* reason this
        // edge is full depth is that the neighbour is outside this chunk's own drawn set.
        // Without the fix (testing membership rather than `cell_in_world`), this edge would
        // be culled as an ordinary, same-level neighbour instead.
        let target = chunk.cells[0];
        let mesh = mesh_chunk(&cfg, &chunk, &HashSet::new());

        let bottom = cfg.layer_bottom_m(cfg.bottom_layer) as f32;
        let (tx, ty) = cell_centre_m(target, Level::Shaku);
        let (tx, ty) = (tx as f32, ty as f32);
        let corners = corners_m(Level::Shaku);
        let (ax, ay) = corners[0];
        let (bx, by) = corners[1];
        let (mx, my) = ((ax + bx) / 2.0, (ay + by) / 2.0);
        let len = (mx * mx + my * my).sqrt();
        let (dir1_nx, dir1_ny) = ((mx / len) as f32, (my / len) as f32);
        let walls_at_bottom = mesh
            .positions
            .iter()
            .enumerate()
            .filter(|(_, p)| (p[2] - bottom).abs() < 1e-3)
            .filter(|(_, p)| {
                let (dx, dy) = (p[0] - tx, p[1] - ty);
                (dx * dx + dy * dy).sqrt() < Level::Shaku.width_m() as f32
            })
            .filter(|(i, _)| {
                let n = mesh.normals[*i];
                (n[0] - dir1_nx).abs() < 1e-4 && (n[1] - dir1_ny).abs() < 1e-4
            })
            .count();
        assert_eq!(walls_at_bottom, 2);
    }

    #[test]
    fn meshing_is_deterministic() {
        let (cfg, chunk) = ken_chunk();
        let a = mesh_chunk(&cfg, &chunk, &HashSet::new());
        let b = mesh_chunk(&cfg, &chunk, &HashSet::new());
        assert_eq!(a.indices, b.indices);
        assert_eq!(a.positions, b.positions);
    }

    #[test]
    fn every_level_meshes() {
        let cfg = WorldConfig::default();
        for level in [Level::Ken, Level::Cho, Level::Ri, Level::World] {
            let chunk = crate::chunk::generate(&cfg, ChunkKey::new(level, Hex::ZERO));
            let mesh = mesh_chunk(&cfg, &chunk, &HashSet::new());
            assert!(mesh.triangle_count() > 0, "{level:?} produced nothing");
        }
    }
}
```

- [ ] **Step 5: Implement `mesh.rs`**

```rust
//! Turning chunks into triangles. Pure geometry: no engine types, no GPU.
//!
//! Positions are (east, north, height) in metres, relative to the chunk's own centre.
//! A chunk draws every child cell whose centre lies in its parent's ideal hexagon
//! (`drawn_offsets`), minus the cells whose own chunk is on screen.

use std::collections::{HashMap, HashSet};

use crate::chunk::{Chunk, ChunkKey};
use crate::column::Column;
use crate::config::WorldConfig;
use crate::hex::{Hex, DIRECTIONS};
use crate::level::Level;
use crate::owner::{centre_child, drawn_offsets};
use crate::placeholder_terrain::column_at;
use crate::plane::{cell_centre_m, corners_m};
use crate::world::{cell_in_world, world_ri};

#[derive(Clone, Default, Debug)]
pub struct MeshData {
    /// (east, north, height) in metres, relative to the chunk's centre.
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub colours: Vec<[f32; 4]>,
    /// (edge_w, border_level, cell_level, 0): see `border_level`.
    pub line: Vec<[f32; 4]>,
    pub indices: Vec<u32>,
}

impl MeshData {
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    fn push_vertex(&mut self, pos: [f32; 3], normal: [f32; 3], colour: [f32; 4], line: [f32; 4]) -> u32 {
        self.positions.push(pos);
        self.normals.push(normal);
        self.colours.push(colour);
        self.line.push(line);
        (self.positions.len() - 1) as u32
    }

    fn push_triangle(&mut self, a: u32, b: u32, c: u32) {
        self.indices.extend_from_slice(&[a, b, c]);
    }
}

/// The cells a chunk draws, in world coordinates at the chunk's child level.
pub fn drawn_cells(cfg: &WorldConfig, key: ChunkKey) -> Vec<Hex> {
    if key.level == Level::World {
        return world_ri(cfg);
    }
    let centre = centre_child(key.cell, key.level);
    let child = key.child_level();
    drawn_offsets(key.level)
        .iter()
        .map(|o| centre + *o)
        .filter(|c| cell_in_world(*c, child, cfg))
        .collect()
}

/// The highest level at which two neighbouring cells belong to different parents.
/// Their own level if they only differ there; this is what the hex lines are styled by.
fn border_level(a: Hex, b: Hex, child: Level) -> Level {
    let mut result = child;
    let mut level = child;
    while let Some(up_level) = level.parent() {
        if up_level == Level::World {
            break;
        }
        if crate::owner::up(a, child, up_level) != crate::owner::up(b, child, up_level) {
            result = up_level;
        }
        level = up_level;
    }
    result
}

/// Mesh one chunk. `omitted` holds the child cells whose own chunk is on screen.
pub fn mesh_chunk(cfg: &WorldConfig, chunk: &Chunk, omitted: &HashSet<Hex>) -> MeshData {
    let key = chunk.key;
    let child = key.child_level();
    let (origin_e, origin_n) = if key.level == Level::World {
        (0.0, 0.0)
    } else {
        cell_centre_m(key.cell, key.level)
    };

    // The chunk's own columns, plus any guest or neighbour column generated on demand.
    let mut columns: HashMap<Hex, Column> = HashMap::with_capacity(chunk.len() * 2);
    for (cell, column) in chunk.cells.iter().zip(chunk.columns.iter()) {
        columns.insert(*cell, column.clone());
    }
    let column_for = |cell: Hex, columns: &mut HashMap<Hex, Column>| -> Column {
        if let Some(c) = columns.get(&cell) {
            return c.clone();
        }
        let generated = if cell_in_world(cell, child, cfg) {
            column_at(cfg, cell, child)
        } else {
            Column::default() // outside the world: air, so the world ends in a wall
        };
        columns.insert(cell, generated.clone());
        generated
    };

    let corners = corners_m(child);
    let mut mesh = MeshData::default();
    let cell_level = child as u32 as f32;
    let bottom_m = cfg.layer_bottom_m(cfg.bottom_layer) as f32;

    // This chunk's own drawn set, built once: full depth is for edges facing a cell this
    // chunk does not draw, not merely a cell some other chunk might. A neighbour outside
    // this set is only ever culled against a same-level column generated on demand for a
    // *different* chunk's territory — a coarser or absent neighbour there can sit at a
    // different height, and only a full-depth wall is guaranteed not to crack against it.
    let drawn_list = drawn_cells(cfg, key);
    let drawn: HashSet<Hex> = drawn_list.iter().copied().collect();

    for cell in &drawn_list {
        let cell = *cell;
        if omitted.contains(&cell) {
            continue;
        }
        let column = column_for(cell, &mut columns);
        if column.runs.is_empty() {
            continue;
        }
        let (ce, cn) = cell_centre_m(cell, child);
        let (cx, cy) = ((ce - origin_e) as f32, (cn - origin_n) as f32);

        // Neighbours, for face culling and for the border levels of the lines.
        let neighbour_cells: Vec<Hex> = DIRECTIONS.iter().map(|d| cell + *d).collect();
        let neighbour_columns: Vec<Column> = neighbour_cells
            .iter()
            .map(|n| column_for(*n, &mut columns))
            .collect();
        let neighbour_drawn: Vec<bool> = neighbour_cells
            .iter()
            .map(|n| drawn.contains(n) && !omitted.contains(n))
            .collect();

        // --- top faces: one fan per run whose layer above is air ---
        for run in &column.runs {
            if column.is_solid(run.top + 1) {
                continue;
            }
            let top_m = cfg.layer_top_m(run.top) as f32;
            let colour = shaded(run.material, cell);
            let centre_v = mesh.push_vertex(
                [cx, cy, top_m],
                [0.0, 0.0, 1.0],
                colour,
                [0.0, 0.0, cell_level, 0.0],
            );
            for k in 0..6 {
                let border = border_level(cell, neighbour_cells[k], child) as u32 as f32;
                let (ax, ay) = corners[(k + 5) % 6];
                let (bx, by) = corners[k];
                let va = mesh.push_vertex(
                    [cx + ax as f32, cy + ay as f32, top_m],
                    [0.0, 0.0, 1.0],
                    colour,
                    [1.0, border, cell_level, 0.0],
                );
                let vb = mesh.push_vertex(
                    [cx + bx as f32, cy + by as f32, top_m],
                    [0.0, 0.0, 1.0],
                    colour,
                    [1.0, border, cell_level, 0.0],
                );
                mesh.push_triangle(centre_v, va, vb);
            }
        }

        // --- side faces: where the neighbour is air over those layers ---
        for (k, neighbour) in neighbour_columns.iter().enumerate() {
            let (ax, ay) = corners[(k + 5) % 6];
            let (bx, by) = corners[k];
            // Outward normal: the edge's midpoint direction.
            let (nx, ny) = {
                let (mx, my) = ((ax + bx) / 2.0, (ay + by) / 2.0);
                let len = (mx * mx + my * my).sqrt().max(1e-9);
                ((mx / len) as f32, (my / len) as f32)
            };
            let normal = [nx, ny, 0.0];
            let line = [0.0, 0.0, cell_level, 0.0];
            let full_depth = !neighbour_drawn[k];
            if full_depth {
                // A full-depth wall is one silhouette quad for the whole column, from the
                // world bottom to the topmost run's top — not one quad per run, which would
                // overlap (and z-fight) even for an ordinary contiguous rock/dirt/grass
                // column: each run started its own quad from the world bottom, so three
                // touching runs produced three nested, overlapping quads instead of one.
                let topmost = column.runs.last().expect("checked non-empty above");
                let top_m = cfg.layer_top_m(topmost.top) as f32;
                let colour = shaded(topmost.material, cell);
                let v0 = mesh.push_vertex([cx + ax as f32, cy + ay as f32, bottom_m], normal, colour, line);
                let v1 = mesh.push_vertex([cx + bx as f32, cy + by as f32, bottom_m], normal, colour, line);
                let v2 = mesh.push_vertex([cx + bx as f32, cy + by as f32, top_m], normal, colour, line);
                let v3 = mesh.push_vertex([cx + ax as f32, cy + ay as f32, top_m], normal, colour, line);
                // Seen from outside the cell, anticlockwise.
                mesh.push_triangle(v0, v1, v2);
                mesh.push_triangle(v0, v2, v3);
                continue;
            }
            for run in &column.runs {
                let mut layer = run.bottom;
                while layer <= run.top {
                    if neighbour.is_solid(layer) {
                        layer += 1;
                        continue;
                    }
                    // Extend while the wall continues.
                    let start = layer;
                    while layer <= run.top && !neighbour.is_solid(layer) {
                        layer += 1;
                    }
                    let top_m = cfg.layer_top_m(layer - 1) as f32;
                    let bottom_of_wall = cfg.layer_bottom_m(start) as f32;
                    let colour = shaded(run.material, cell);
                    let v0 = mesh.push_vertex([cx + ax as f32, cy + ay as f32, bottom_of_wall], normal, colour, line);
                    let v1 = mesh.push_vertex([cx + bx as f32, cy + by as f32, bottom_of_wall], normal, colour, line);
                    let v2 = mesh.push_vertex([cx + bx as f32, cy + by as f32, top_m], normal, colour, line);
                    let v3 = mesh.push_vertex([cx + ax as f32, cy + ay as f32, top_m], normal, colour, line);
                    // Seen from outside the cell, anticlockwise.
                    mesh.push_triangle(v0, v1, v2);
                    mesh.push_triangle(v0, v2, v3);
                }
            }
        }
    }
    mesh
}

/// A material's colour, shaded slightly per cell so neighbouring cells read apart.
fn shaded(material: crate::column::Material, cell: Hex) -> [f32; 4] {
    let base = material.colour();
    let hash = crate::noise::gradient_2d(0x5EED, cell.q as f64 * 0.37, cell.r as f64 * 0.51);
    let k = 1.0 + 0.06 * hash as f32;
    [base[0] * k, base[1] * k, base[2] * k, 1.0]
}
```

- [ ] **Step 6: Run the tests**

Run: `cd Experiments/exp-05 && cargo test -p hexworld mesh`
Expected: PASS, 9 tests. If `top_faces_point_up_and_wind_anticlockwise` fails, the corner pairing is reversed: the fan must go from `corners[(k + 5) % 6]` to `corners[k]`, which is anticlockwise.

- [ ] **Step 7: Commit**

```bash
cd /home/lexa/DevProjects/_GameDev/Murabito/Experiments/exp-05
cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test -p hexworld
git -C /home/lexa/DevProjects/_GameDev/Murabito add Experiments/exp-05
git -C /home/lexa/DevProjects/_GameDev/Murabito commit -m "exp-05: chunk mesher — drawn set, hex prisms, faces and line data

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 10: The mesher — handover guarantees

**Files:**
- Create: `crates/hexworld/tests/handover.rs`
- Modify: `crates/hexworld/src/mesh.rs` if the tests find gaps

**Interfaces:**
- Consumes: `mesh_chunk`, `drawn_cells`, `generate`, `ChunkKey`.
- Produces: no new API. This task proves the properties the spec's success criteria rest on.

- [ ] **Step 1: Write the failing tests**

`crates/hexworld/tests/handover.rs`:

```rust
//! The handover properties the spec's "no holes and no cracks" criterion rests on.

use std::collections::HashSet;

use hexworld::chunk::{generate, ChunkKey};
use hexworld::level::Level;
use hexworld::mesh::{drawn_cells, mesh_chunk};
use hexworld::owner::{centre_child, parent_of};
use hexworld::plane::{cell_centre_m, round_at};
use hexworld::{Hex, WorldConfig};

#[test]
fn a_parent_and_its_children_cover_the_ground_exactly_once() {
    // Every ken a cho chunk draws is drawn by that cho chunk, or by the ken's own chunk
    // when it is loaded — never by both, never by neither.
    let cfg = WorldConfig::default();
    let cho_key = ChunkKey::new(Level::Cho, Hex::ZERO);
    let cho_chunk = generate(&cfg, cho_key);
    let all_kens = drawn_cells(&cfg, cho_key);

    let shown_ken = all_kens[17];
    let mut omitted = HashSet::new();
    omitted.insert(shown_ken);

    let parent_mesh = mesh_chunk(&cfg, &cho_chunk, &omitted);
    let child_mesh = mesh_chunk(&cfg, &generate(&cfg, ChunkKey::new(Level::Ken, shown_ken)), &HashSet::new());
    assert!(parent_mesh.triangle_count() > 0);
    assert!(child_mesh.triangle_count() > 0);

    // The child covers its own cell's ideal hexagon: every shaku it draws rounds to it.
    for shaku in drawn_cells(&cfg, ChunkKey::new(Level::Ken, shown_ken)) {
        let (e, n) = cell_centre_m(shaku, Level::Shaku);
        assert_eq!(round_at(e, n, Level::Ken), shown_ken, "{shaku:?} is outside the hexagon");
    }
}

#[test]
fn neighbouring_chunks_do_not_draw_the_same_cell() {
    let cfg = WorldConfig::default();
    let a = ChunkKey::new(Level::Ken, Hex::ZERO);
    let b = ChunkKey::new(Level::Ken, Hex::new(1, 0));
    let cells_a: HashSet<Hex> = drawn_cells(&cfg, a).into_iter().collect();
    let cells_b: HashSet<Hex> = drawn_cells(&cfg, b).into_iter().collect();
    let shared: Vec<&Hex> = cells_a.intersection(&cells_b).collect();
    assert!(shared.is_empty(), "drawn twice: {shared:?}");
}

#[test]
fn a_patch_of_neighbouring_chunks_leaves_no_hole() {
    // Take a ken and its six neighbours; every shaku in the middle ken's area is drawn
    // by exactly one of them.
    let cfg = WorldConfig::default();
    let centre = Hex::ZERO;
    let keys: Vec<ChunkKey> = hexworld::hex::range(centre, 1)
        .into_iter()
        .map(|c| ChunkKey::new(Level::Ken, c))
        .collect();
    let mut drawn: HashSet<Hex> = HashSet::new();
    for key in &keys {
        for cell in drawn_cells(&cfg, *key) {
            assert!(drawn.insert(cell), "{cell:?} drawn twice");
        }
    }
    // Every shaku owned by the centre ken is drawn by something in the patch.
    for shaku in hexworld::owner::children(centre, Level::Ken) {
        assert!(drawn.contains(&shaku), "{shaku:?} is a hole");
    }
}

#[test]
fn outer_walls_reach_the_world_bottom() {
    let cfg = WorldConfig::default();
    let chunk = generate(&cfg, ChunkKey::new(Level::Ken, Hex::ZERO));
    let mesh = mesh_chunk(&cfg, &chunk, &HashSet::new());
    let lowest = mesh.positions.iter().map(|p| p[2]).fold(f32::MAX, f32::min);
    let expected = cfg.layer_bottom_m(cfg.bottom_layer) as f32;
    assert!((lowest - expected).abs() < 1e-3, "{lowest} vs {expected}");
}

#[test]
fn an_omitted_cells_neighbours_grow_full_depth_walls() {
    // When a child chunk takes over a cell, the cells around the hole must wall down to
    // the bottom, so a height difference between detail levels cannot show as a crack.
    let cfg = WorldConfig::default();
    let key = ChunkKey::new(Level::Cho, Hex::ZERO);
    let chunk = generate(&cfg, key);
    let target = drawn_cells(&cfg, key)[50];
    let mut omitted = HashSet::new();
    omitted.insert(target);
    let mesh = mesh_chunk(&cfg, &chunk, &omitted);

    let bottom = cfg.layer_bottom_m(cfg.bottom_layer) as f32;
    let (te, tn) = cell_centre_m(target, Level::Ken);
    let (oe, on) = cell_centre_m(key.cell, Level::Cho);
    let (tx, ty) = ((te - oe) as f32, (tn - on) as f32);
    let near_hole = mesh
        .positions
        .iter()
        .filter(|p| ((p[0] - tx).powi(2) + (p[1] - ty).powi(2)).sqrt() < Level::Ken.width_m() as f32 * 1.2)
        .filter(|p| (p[2] - bottom).abs() < 1e-3)
        .count();
    assert!(near_hole >= 6, "expected full-depth walls around the hole, found {near_hole}");
}

#[test]
fn the_world_ends_in_a_wall() {
    // A chunk at the edge of a one-ri world draws nothing outside it, and walls the edge.
    let cfg = WorldConfig::default();
    let edge_ken = hexworld::owner::up(
        Hex::new(Level::Ri.scale_shaku() * 2 / 3, 0),
        Level::Shaku,
        Level::Ken,
    );
    let key = ChunkKey::new(Level::Ken, edge_ken);
    for cell in drawn_cells(&cfg, key) {
        assert!(hexworld::world::cell_in_world(cell, Level::Shaku, &cfg), "{cell:?} is outside");
    }
}
```

- [ ] **Step 2: Run the tests**

Run: `cd Experiments/exp-05 && cargo test -p hexworld --test handover`
Expected: PASS, 6 tests. If `a_patch_of_neighbouring_chunks_leaves_no_hole` fails, `drawn_offsets` and `round_at` disagree somewhere — fix `drawn_offsets`, not the test, since the tiling property is what the whole handover rests on.

- [ ] **Step 3: Measure the mesh, to size the triangle budget**

Add a small ignored test that prints counts, so the numbers are recorded rather than guessed:

```rust
#[test]
#[ignore = "measurement, not a check: cargo test -p hexworld --test handover -- --ignored --nocapture"]
fn measure_triangle_counts() {
    let cfg = WorldConfig::default();
    for (level, chunks) in [(Level::Ken, 37), (Level::Cho, 37), (Level::Ri, 1)] {
        let chunk = generate(&cfg, ChunkKey::new(level, Hex::ZERO));
        let started = std::time::Instant::now();
        let mesh = mesh_chunk(&cfg, &chunk, &HashSet::new());
        let elapsed = started.elapsed();
        println!(
            "{:>5} chunk: {:>8} triangles, {:>8} vertices, {:?} to mesh; window of {chunks}: {} triangles",
            level.name(),
            mesh.triangle_count(),
            mesh.positions.len(),
            elapsed,
            mesh.triangle_count() * chunks
        );
    }
}
```

Run it with `--release`, and **write the numbers into the task's commit message**. The spec expects roughly 2–3 million triangles at ken detail; if the measurement is far above that, stop and tell the user before building the renderer on it.

- [ ] **Step 4: Commit**

```bash
cd /home/lexa/DevProjects/_GameDev/Murabito/Experiments/exp-05
cargo test -p hexworld --release --test handover -- --ignored --nocapture
cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test -p hexworld
git -C /home/lexa/DevProjects/_GameDev/Murabito add Experiments/exp-05
git -C /home/lexa/DevProjects/_GameDev/Murabito commit -m "exp-05: handover tests — tiling, full-depth walls, world edge

Measured triangle counts: <paste the numbers from step 3>

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Bevy notes (verified, not remembered)

Everything below was checked by compiling a throwaway spike against Bevy 0.19.1 and bevy_egui 0.42 on this machine before this plan was written. Use these exact forms.

- **Features.** This machine has no `libudev` or Wayland development packages, so the crates use `default-features = false` and the list in Task 11. That drops gamepad support (`bevy_gilrs`) and Wayland; X11 is present and works. The list also needs `zstd_rust`: `tonemapping_luts` enables `bevy_image/zstd` without picking a backend, and `bevy_image` hard-errors (`compile_error!`) unless `zstd_rust` or `zstd_c` is on too. The spike this list came from compiled without it only because `bevy_egui` pulled in a zstd backend of its own, and Cargo's feature unification satisfied `bevy_image/zstd` for the whole graph; a crate that depends on `bevy` alone, like `hexworld_bevy`, hits the missing-backend error directly. `zstd_rust` is the pure-Rust backend, so it adds no system dependency — in keeping with X11-only, no gamepad support.
- `ShaderRef` is at **`bevy::shader::ShaderRef`**, not in `render_resource`.
- Buffered events are **messages**: `MessageWriter<AppExit>`, written with `.write(AppExit::Success)`.
- `AmbientLight` is a **component on the camera**, not a resource.
- `DirectionalLight`'s field is **`shadow_maps_enabled`**.
- Entities use required components: `Mesh3d(handle)`, `MeshMaterial3d(handle)`, `Transform`, `Camera3d::default()`, `Projection::Perspective(PerspectiveProjection { fov, ..default() })`.
- Screenshots: `commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path))`, from `bevy::render::view::screenshot`. Frame numbers come from `bevy::diagnostic::FrameCount`.
- Background work: `AsyncComputeTaskPool::get().spawn(async move { .. })`, polled with `block_on(future::poll_once(&mut task))` from `bevy::tasks::{block_on, futures_lite::future}`.
- Extended materials: `ExtendedMaterial<StandardMaterial, Ext>` with fields `base` and `extension`, registered with `MaterialPlugin::<ExtendedMaterial<StandardMaterial, Ext>>::default()`, uniforms at `@group(2) @binding(100)`.
- **egui 0.36 has no `SidePanel`.** Panels are `egui::Panel::right(id)`, and they take a **root `Ui`**, not a context:

  ```rust
  let ctx = contexts.ctx_mut()?;
  let mut viewport_ui = egui::Ui::new(
      ctx.clone(),
      "viewport".into(),
      egui::UiBuilder::new().layer_id(egui::LayerId::background()).max_rect(ctx.viewport_rect()),
  );
  egui::Panel::right("panel").default_size(340.0).show(&mut viewport_ui, |ui| { .. });
  ```

  egui systems go in the **`EguiPrimaryContextPass`** schedule, and return `Result` because `ctx_mut()` can fail.
- Mouse picking on the ground: `camera.viewport_to_world(camera_transform, cursor)` gives a `Ray3d`; intersect it with `InfinitePlane3d::new(Vec3::Y)` via `ray.intersect_plane(origin, plane)`.

---

### Task 11: The Bevy plugin — loaders, background jobs and chunk entities

**Files:**
- Modify: `crates/hexworld_bevy/Cargo.toml`
- Create: `crates/hexworld_bevy/src/lib.rs`, `src/axes.rs`, `src/loader.rs`, `src/tasks.rs`, `src/entities.rs`
- Test: `crates/hexworld_bevy/tests/plugin.rs`

**Interfaces:**
- Consumes: the whole `hexworld` API.
- Produces:
  - `axes::{to_bevy(f64, f64, f64) -> Vec3, from_bevy(Vec3) -> (f64, f64, f64), mesh_position([f32; 3]) -> [f32; 3]}`.
  - `HexWorldPlugin { config: WorldConfig, settings: StoreSettings }` with `Default`.
  - `HexWorld` resource wrapping `ChunkStore`, with `store()` and `store_mut()`.
  - `Loader { rings: Rings }` component (entity needs a `Transform`).
  - `ChunkView(ChunkKey)` component on each chunk entity.
  - `HexWorldSet` system set, so the viewer can order against it.
  - `GroundMaterialHandle` resource (a plain `StandardMaterial` in this task; Task 13 replaces it with the extended one).

- [ ] **Step 1: Set up the crate**

`crates/hexworld_bevy/Cargo.toml`:

```toml
[package]
name = "hexworld_bevy"
edition.workspace = true
version.workspace = true
license.workspace = true

[dependencies]
hexworld = { path = "../hexworld" }
bevy = { version = "0.19", default-features = false, features = [
  "bevy_winit", "bevy_window", "bevy_render", "bevy_core_pipeline", "bevy_pbr",
  "bevy_asset", "bevy_log", "bevy_ui", "bevy_ui_render", "bevy_text",
  "x11", "multi_threaded", "tonemapping_luts", "zstd_rust", "png", "default_font", "std",
] }
```

- [ ] **Step 2: Write the failing test for the axis mapping**

`crates/hexworld_bevy/tests/plugin.rs`:

```rust
use bevy::prelude::*;
use hexworld_bevy::axes::{from_bevy, mesh_position, to_bevy};

#[test]
fn north_maps_to_negative_z() {
    // Bevy is right-handed with +Y up, so north must be -Z or the world comes out mirrored.
    let v = to_bevy(3.0, 5.0, 7.0);
    assert_eq!(v, Vec3::new(3.0, 7.0, -5.0));
}

#[test]
fn the_mapping_round_trips() {
    let (e, n, h) = (12.5, -4.25, 100.0);
    let (e2, n2, h2) = from_bevy(to_bevy(e, n, h));
    assert!((e - e2).abs() < 1e-9 && (n - n2).abs() < 1e-9 && (h - h2).abs() < 1e-9);
}

#[test]
fn the_mapping_keeps_its_handedness() {
    // east cross north must point up, in both frames: otherwise triangles wind backwards
    // and every hexagon is a mirror image of the core's.
    let east = to_bevy(1.0, 0.0, 0.0);
    let north = to_bevy(0.0, 1.0, 0.0);
    let up = to_bevy(0.0, 0.0, 1.0);
    assert!(east.cross(north).dot(up) > 0.0, "the axis mapping mirrors the world");
}

#[test]
fn mesh_positions_use_the_same_mapping() {
    assert_eq!(mesh_position([3.0, 5.0, 7.0]), [3.0, 7.0, -5.0]);
}
```

- [ ] **Step 3: Implement `axes.rs`**

```rust
//! The only place where the core's (east, north, height) meets Bevy's axes.
//!
//! Bevy is right-handed with +Y up, and its forward is -Z. Mapping north to +Z instead
//! would flip the ground plane's handedness and mirror the whole world, so: north is -Z.

use bevy::prelude::Vec3;

pub fn to_bevy(east: f64, north: f64, height: f64) -> Vec3 {
    Vec3::new(east as f32, height as f32, -north as f32)
}

pub fn from_bevy(v: Vec3) -> (f64, f64, f64) {
    (v.x as f64, -v.z as f64, v.y as f64)
}

/// The same mapping for a mesh vertex, which the core gives as (east, north, height).
pub fn mesh_position(p: [f32; 3]) -> [f32; 3] {
    [p[0], p[2], -p[1]]
}
```

- [ ] **Step 4: Run the axis tests**

Run: `cd Experiments/exp-05 && cargo test -p hexworld_bevy`
Expected: PASS, 4 tests.

- [ ] **Step 5: Write the failing plugin tests**

Append to `crates/hexworld_bevy/tests/plugin.rs`:

```rust
use std::time::{Duration, Instant};

use hexworld::{ChunkKey, Hex, Level, Rings, WorldConfig};
use hexworld_bevy::{HexWorld, HexWorldPlugin, Loader};

/// How long a `run_until` loop is allowed to spend waiting for background work, and how
/// many frames it is allowed to spend doing it, whichever comes first. Frames are not a
/// reliable unit here: a headless app with no render loop burns through hundreds of them
/// in a few milliseconds, while real chunk generation is genuinely CPU-bound (tens of
/// milliseconds per cho chunk in the dev profile), so waiting is bound by wall-clock time
/// with the frame count only as a backstop against a truly stuck test.
const SETTLE_TIMEOUT: Duration = Duration::from_secs(30);
const SETTLE_FRAME_CAP: usize = 20_000;

/// A headless app: no window, no renderer, just the schedule.
///
/// `TransformPlugin` is included even though nothing else here renders, because
/// `drive_store` reads loaders' `GlobalTransform`, and that is only ever kept in sync
/// with `Transform` by `TransformPlugin`'s propagation systems (`MinimalPlugins` does
/// not include them). Propagation runs in `PostUpdate`, so a loader's `GlobalTransform`
/// is still identity on the very first frame it is spawned; a settle loop runs many
/// frames, so this does not affect these tests.
fn headless_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::transform::TransformPlugin)
        .add_plugins(bevy::asset::AssetPlugin::default())
        .init_asset::<Mesh>()
        .init_asset::<StandardMaterial>()
        .add_plugins(HexWorldPlugin::default());
    app
}

/// Run frames until `condition` is true, or the settle timeout/frame cap is hit,
/// whichever comes first. Returns the number of frames it took. `describe` builds the
/// panic message on timeout, so a genuine failure is diagnosable rather than just
/// "never settled".
fn run_until(
    app: &mut App,
    mut condition: impl FnMut(&HexWorld) -> bool,
    describe: impl Fn(&HexWorld) -> String,
) -> usize {
    let started = Instant::now();
    for frame in 0..SETTLE_FRAME_CAP {
        app.update();
        let world = app.world().resource::<HexWorld>();
        if condition(world) {
            return frame;
        }
        if started.elapsed() >= SETTLE_TIMEOUT {
            panic!(
                "timed out after {:?} and {} frames: {}",
                started.elapsed(),
                frame + 1,
                describe(world)
            );
        }
    }
    let world = app.world().resource::<HexWorld>();
    panic!(
        "hit the {SETTLE_FRAME_CAP}-frame cap after {:?}: {}",
        started.elapsed(),
        describe(world)
    );
}

/// Run frames until the store reports at least 37 loaded ken chunks (the default
/// window's full shaku-detail ring), or the settle timeout/frame cap is hit.
///
/// This is *not* a guarantee that every requested chunk is loaded: `max_in_flight` is
/// shared across levels, and `load_order` only decides which chunk starts next, not
/// when it finishes — once the last cho/ri/world chunk has merely started, freed
/// capacity immediately pulls in cheaper ken chunks, so ken can reach 37 while some of
/// the caller's own cho chunks are still in flight. Good enough for tests that only
/// check specific keys or that loaded entities match store state; not good enough for
/// anything that needs an exact count (see `run_until_fully_settled`).
fn run_until_settled(app: &mut App) -> usize {
    run_until(
        app,
        |world| world.store().stats().per_level[Level::Ken as usize].chunks >= 37,
        |world| {
            format!(
                "{} ken chunks loaded",
                world.store().stats().per_level[Level::Ken as usize].chunks
            )
        },
    )
}

/// Run frames until the store has *genuinely* settled: nothing outstanding
/// (`in_flight_count() == 0`) and at least `expected` chunks loaded. Both conditions are
/// needed together: `in_flight_count()` can read 0 for a single frame in the middle of a
/// settle too, if every currently in-flight job happens to finish in the same frame
/// while more requested chunks are still waiting for a free slot (they only get picked
/// up on the *next* frame's `drive_store`) — pairing it with a known target count closes
/// that gap, the same way `in_flight` alone would not.
fn run_until_fully_settled(app: &mut App, expected: usize) -> usize {
    run_until(
        app,
        move |world| {
            world.store().in_flight_count() == 0 && world.store().loaded_keys().len() >= expected
        },
        move |world| {
            format!(
                "in_flight={} loaded={} (want in_flight == 0 and loaded >= {expected})",
                world.store().in_flight_count(),
                world.store().loaded_keys().len()
            )
        },
    )
}

#[test]
fn a_loader_entity_drives_loading() {
    let mut app = headless_app();
    app.world_mut().spawn((Transform::default(), Loader { rings: Rings::default() }));
    run_until_settled(&mut app);
    let world = app.world().resource::<HexWorld>();
    assert!(world.store().is_loaded(ChunkKey::WORLD));
    assert!(world.store().is_loaded(ChunkKey::new(Level::Ken, Hex::ZERO)));
}

#[test]
fn chunk_entities_appear_for_loaded_chunks() {
    let mut app = headless_app();
    app.world_mut().spawn((Transform::default(), Loader { rings: Rings::default() }));
    run_until_settled(&mut app);
    let mut query = app.world_mut().query::<&hexworld_bevy::ChunkView>();
    let views: Vec<ChunkKey> = query.iter(app.world()).map(|v| v.0).collect();
    assert!(views.contains(&ChunkKey::new(Level::Ken, Hex::ZERO)), "{views:?}");
    let world = app.world().resource::<HexWorld>();
    for key in &views {
        assert!(world.store().is_loaded(*key), "{key:?} has an entity but is not loaded");
    }
}

#[test]
fn a_second_loader_adds_its_windows_with_no_other_changes() {
    let mut app = headless_app();
    app.world_mut().spawn((Transform::default(), Loader { rings: Rings::default() }));
    // 76 is not a guess: Task 7's `default_rings_request_the_expected_chunks` pins it as
    // the exact request count for default rings at the origin (37 ken + 37 cho + 1 ri +
    // 1 world). Waiting for a genuine settle against that known total, rather than just
    // "ken reached 37", is what makes `before` a fact instead of an observation.
    run_until_fully_settled(&mut app, 76);
    let before = app.world().resource::<HexWorld>().store().loaded_keys().len();
    assert_eq!(before, 76, "a single loader's whole window");

    // One more entity with a Loader: that is the whole API.
    let far_cell = Hex::new(400, 200);
    let cfg = WorldConfig::default();
    assert!(
        hexworld::cell_in_world(far_cell, Level::Shaku, &cfg),
        "the second loader's focus must be inside the default (single-ri) world, or its \
         window is hollow and this test proves nothing"
    );
    let far = hexworld::plane::cell_centre_m(far_cell, Level::Shaku);
    app.world_mut().spawn((
        Transform::from_translation(hexworld_bevy::axes::to_bevy(far.0, far.1, 0.0)),
        Loader { rings: Rings::default() },
    ));

    // The exact union both loaders should settle to, computed the same way
    // `ChunkStore::update` computes it internally (union `requests()` per loader) — this
    // is what "genuinely settled with two loaders" means, not just "grew by one".
    let loader1 = hexworld::Loader { focus: Hex::ZERO, rings: Rings::default() };
    let loader2 = hexworld::Loader { focus: far_cell, rings: Rings::default() };
    let mut expected: Vec<ChunkKey> = hexworld::requests(&cfg, &loader1);
    expected.extend(hexworld::requests(&cfg, &loader2));
    expected.sort();
    expected.dedup();
    let expected_total = expected.len();

    run_until_fully_settled(&mut app, expected_total);
    let after = app.world().resource::<HexWorld>().store().loaded_keys().len();
    assert_eq!(after, expected_total, "two loaders' unioned window: before={before}");
    assert!(
        after >= before + 30,
        "expected roughly a whole ken window more (~37 chunks), got {before} -> {after}"
    );
}
```

Plugin tests wait on real background generation, not just frame counts: a cho chunk takes
tens of milliseconds to generate and mesh in the dev profile, and a headless app with no
render loop can burn through hundreds of near-empty frames in a few milliseconds, so
`run_until` is bound by wall-clock time (with a frame count only as a backstop) rather
than a fixed number of frames.

- [ ] **Step 6: Implement the plugin**

`crates/hexworld_bevy/src/lib.rs`:

```rust
//! Bevy glue for `hexworld`. The core does the thinking; this crate turns loader entities
//! into requests, runs generation and meshing on background tasks, and keeps one entity
//! per chunk on screen.

pub mod axes;
pub mod entities;
pub mod loader;
pub mod tasks;

use bevy::prelude::*;
use hexworld::{ChunkStore, StoreSettings, WorldConfig};

pub use entities::ChunkView;
pub use loader::Loader;

/// Everything this plugin does, in one set, so a viewer can order against it.
#[derive(SystemSet, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct HexWorldSet;

#[derive(Resource)]
pub struct HexWorld {
    store: ChunkStore,
}

impl HexWorld {
    pub fn store(&self) -> &ChunkStore {
        &self.store
    }

    pub fn store_mut(&mut self) -> &mut ChunkStore {
        &mut self.store
    }
}

#[derive(Resource, Clone, Copy)]
pub struct GroundMaterialHandle(pub Handle<StandardMaterial>);

pub struct HexWorldPlugin {
    pub config: WorldConfig,
    pub settings: StoreSettings,
}

impl Default for HexWorldPlugin {
    fn default() -> Self {
        HexWorldPlugin {
            config: WorldConfig::default(),
            settings: StoreSettings::default(),
        }
    }
}

impl Plugin for HexWorldPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(HexWorld { store: ChunkStore::new(self.config, self.settings) })
            .add_systems(Startup, entities::setup_material)
            .add_systems(
                Update,
                (loader::drive_store, tasks::collect_finished_jobs)
                    .chain()
                    .in_set(HexWorldSet),
            );
    }
}
```

`crates/hexworld_bevy/src/loader.rs`:

```rust
//! Loader entities: anything with a `Transform` and a `Loader` pulls the world in around it.

use bevy::prelude::*;
use hexworld::{plane::round_at, Level, Rings};

use crate::axes::from_bevy;
use crate::tasks::spawn_jobs;
use crate::{entities, HexWorld};

/// Put this on any entity with a transform to make it a loader.
#[derive(Component, Clone, Copy, Debug)]
pub struct Loader {
    pub rings: Rings,
}

impl Default for Loader {
    fn default() -> Self {
        Loader { rings: Rings::default() }
    }
}

pub fn drive_store(
    mut commands: Commands,
    time: Res<Time>,
    mut world: ResMut<HexWorld>,
    loaders: Query<(&GlobalTransform, &Loader)>,
    views: Query<(Entity, &crate::ChunkView)>,
) {
    let core_loaders: Vec<hexworld::Loader> = loaders
        .iter()
        .map(|(transform, loader)| {
            let (east, north, _) = from_bevy(transform.translation());
            hexworld::Loader { focus: round_at(east, north, Level::Shaku), rings: loader.rings }
        })
        .collect();

    let now = time.elapsed_secs_f64();
    let update = world.store_mut().update(&core_loaders, now);
    entities::despawn_unloaded(&mut commands, &views, &update.to_unload);
    spawn_jobs(&mut commands, &mut world, update.to_load);
}
```

`crates/hexworld_bevy/src/tasks.rs`:

```rust
//! Generation and meshing on Bevy's background task pool. Chunks are pure functions of
//! the config and the key, so a job needs nothing from the main thread and can finish in
//! any order.

use std::collections::HashSet;

use bevy::prelude::*;
use bevy::tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task};
use hexworld::{mesh::MeshData, Chunk, ChunkKey, WorldConfig};

use crate::{entities, HexWorld};

pub struct JobOutput {
    pub key: ChunkKey,
    pub chunk: Chunk,
    pub mesh: MeshData,
}

#[derive(Component)]
pub struct ChunkJob(pub Task<JobOutput>);

/// Start generating the chunks the store asked for, up to the in-flight cap.
pub fn spawn_jobs(commands: &mut Commands, world: &mut HexWorld, to_load: Vec<ChunkKey>) {
    let cap = world.store().settings().max_in_flight;
    let running = world.store().in_flight_count();
    let pool = AsyncComputeTaskPool::get();
    for key in to_load.into_iter().take(cap.saturating_sub(running)) {
        let cfg: WorldConfig = *world.store().config();
        world.store_mut().begin_load(key);
        let task = pool.spawn(async move {
            let chunk = hexworld::chunk::generate(&cfg, key);
            let mesh = hexworld::mesh::mesh_chunk(&cfg, &chunk, &HashSet::new());
            JobOutput { key, chunk, mesh }
        });
        commands.spawn(ChunkJob(task));
    }
}

/// Take finished jobs, store the chunk, and put it on screen.
pub fn collect_finished_jobs(
    mut commands: Commands,
    mut world: ResMut<HexWorld>,
    mut meshes: ResMut<Assets<Mesh>>,
    material: Option<Res<crate::GroundMaterialHandle>>,
    mut jobs: Query<(Entity, &mut ChunkJob)>,
) {
    for (entity, mut job) in &mut jobs {
        let Some(output) = block_on(future::poll_once(&mut job.0)) else {
            continue;
        };
        commands.entity(entity).despawn();
        if !world.store_mut().insert(output.chunk) {
            continue; // nobody wants it any more
        }
        if let Some(material) = material.as_ref() {
            entities::spawn_chunk_entity(
                &mut commands,
                &mut meshes,
                material.0.clone(),
                output.key,
                &output.mesh,
                world.store().config(),
            );
        }
    }
}
```

`crates/hexworld_bevy/src/entities.rs`:

```rust
//! One entity per chunk on screen.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use hexworld::{mesh::MeshData, plane::cell_centre_m, ChunkKey, Level, WorldConfig};

use crate::axes::{mesh_position, to_bevy};

#[derive(Component, Clone, Copy, Debug)]
pub struct ChunkView(pub ChunkKey);

pub fn setup_material(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>) {
    let handle = materials.add(StandardMaterial {
        perceptual_roughness: 0.95,
        ..default()
    });
    commands.insert_resource(crate::GroundMaterialHandle(handle));
}

/// Build a Bevy mesh from the core's data, mapping the axes once.
pub fn to_bevy_mesh(data: &MeshData) -> Mesh {
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        data.positions.iter().map(|p| mesh_position(*p)).collect::<Vec<[f32; 3]>>(),
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_NORMAL,
        data.normals.iter().map(|n| mesh_position(*n)).collect::<Vec<[f32; 3]>>(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, data.colours.clone());
    // Line data rides in the UVs, so no custom vertex shader is needed:
    // UV0 = (edge_w, border_level), UV1 = (cell_level, 0).
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_UV_0,
        data.line.iter().map(|l| [l[0], l[1]]).collect::<Vec<[f32; 2]>>(),
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_UV_1,
        data.line.iter().map(|l| [l[2], 0.0]).collect::<Vec<[f32; 2]>>(),
    );
    mesh.insert_indices(Indices::U32(data.indices.clone()));
    mesh
}

/// Where a chunk's mesh sits in the world: the centre of its own cell.
pub fn chunk_origin(key: ChunkKey) -> Vec3 {
    if key.level == Level::World {
        Vec3::ZERO
    } else {
        let (east, north) = cell_centre_m(key.cell, key.level);
        to_bevy(east, north, 0.0)
    }
}

pub fn spawn_chunk_entity(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    material: Handle<StandardMaterial>,
    key: ChunkKey,
    data: &MeshData,
    _cfg: &WorldConfig,
) -> Entity {
    let handle = meshes.add(to_bevy_mesh(data));
    commands
        .spawn((
            Mesh3d(handle),
            MeshMaterial3d(material),
            Transform::from_translation(chunk_origin(key)),
            ChunkView(key),
        ))
        .id()
}

pub fn despawn_unloaded(
    commands: &mut Commands,
    views: &Query<(Entity, &ChunkView)>,
    unloaded: &[ChunkKey],
) {
    for (entity, view) in views.iter() {
        if unloaded.contains(&view.0) {
            commands.entity(entity).despawn();
        }
    }
}
```

- [ ] **Step 7: Add the two small accessors the plugin needs to `hexworld::ChunkStore`**

```rust
pub fn settings(&self) -> &StoreSettings {
    &self.settings
}

pub fn in_flight_count(&self) -> usize {
    self.in_flight.len()
}
```

Add a test in `store.rs` for `in_flight_count`:

```rust
#[test]
fn in_flight_counts_chunks_being_generated() {
    let mut s = store();
    let here = [Loader { focus: Hex::ZERO, rings: Rings::default() }];
    let update = s.update(&here, 0.0);
    let key = update.to_load[0];
    assert_eq!(s.in_flight_count(), 0);
    s.begin_load(key);
    assert_eq!(s.in_flight_count(), 1);
    s.insert(crate::chunk::generate(s.config(), key));
    assert_eq!(s.in_flight_count(), 0);
}
```

- [ ] **Step 8: Run everything**

Run: `cd Experiments/exp-05 && cargo test -p hexworld && cargo test -p hexworld_bevy`
Expected: PASS. The headless tests settle well under a second of wall-clock time (with
`max_in_flight` at 8, clearing the 76-chunk default window), bounded by `run_until`'s
30 s timeout rather than a frame count — see the note after Step 5.

- [ ] **Step 9: Commit**

```bash
cd /home/lexa/DevProjects/_GameDev/Murabito/Experiments/exp-05
cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test
git -C /home/lexa/DevProjects/_GameDev/Murabito add Experiments/exp-05
git -C /home/lexa/DevProjects/_GameDev/Murabito commit -m "exp-05: Bevy plugin — loader entities, background jobs, chunk entities

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 12: The handover, in one frame

**Files:**
- Modify: `crates/hexworld_bevy/src/entities.rs`, `src/tasks.rs`, `src/lib.rs`
- Test: `crates/hexworld_bevy/tests/handover.rs`

**Interfaces:**
- Consumes: Task 11's plugin.
- Produces: `Shown` resource: `HashMap<ChunkKey, Entity>` plus `omitted: HashMap<ChunkKey, HashSet<Hex>>`, with `omitted_for(ChunkKey) -> &HashSet<Hex>`.

  **As implemented (Ruling, dispatch): the `parent_remesh` field named above is void.**
  `JobOutput` keeps its original three fields (`key`, `chunk`, `mesh`); the parent is
  re-meshed on the main thread via `remesh_shown` (Step 2), not carried back from the
  background job. The async route stays available as a fallback if a viewer later shows a
  frame hitch from a cho chunk's re-mesh cost (Task 10 measured ~18.4 ms release /
  22–27 ms dev — over the task's own ~4 ms threshold — but that is a decision for the
  viewer to make from a real frame-time readout, not from this task).

The rule: a chunk's mesh leaves out the cells whose own chunk is on screen. So a child appearing means its parent must be re-meshed without that cell, and both must reach the screen **in the same frame**, or there is a hole (child late) or a doubled surface (parent late).

**As implemented, this also covers same-level siblings, not only parent/child** (carried
in from Task 9's dispatch note: "`omitted` must also cover guest cells whose owning chunk
is shown"). Two neighbouring chunks at the same level can share a guest (tie) cell by
design (Task 9); once both are shown, the non-owner omits it and the owner never does —
enforced structurally, since `guest_cells_to_omit` never proposes a cell a chunk owns as
one to omit from itself, so there is no code path where both sides could omit the same
cell. See `entities.rs`'s `sync_guests_on_load` / `release_guests_on_unload` /
`guest_cells_to_omit`, and `tests/handover.rs`'s
`the_guest_sides_mesh_drops_the_shared_vertex_the_frame_both_neighbours_are_shown` (added
in fix round 1, see below).

Also implemented, defensively, beyond the brief: `absorb_already_shown_children`. The
store's `to_load` excludes a chunk once it is merely *in-flight*, not once it is *loaded*,
and `load_order` only decides which chunk starts next, coarsest first — so once every
coarser chunk has started, freed capacity pulls in finer chunks even while some coarser
jobs are still generating. A finer chunk's async job can therefore finish, and be shown,
before its own coarser parent's job does. When a chunk becomes newly shown, it now also
checks whether any already-shown chunk at its own child level turns out to be its child,
and omits that cell too, in the same frame. See `entities::tests::a_late_parent_absorbs_an_already_shown_child`.

- [ ] **Step 1: Write the failing tests**

`crates/hexworld_bevy/tests/handover.rs`. `headless_app`, `run_until`,
`run_until_fully_settled` and their wall-clock-bounded settle discipline are shared with
`tests/plugin.rs` via `tests/common/mod.rs` (Task 11's helpers, generalised to take a
`StoreSettings` argument), rather than duplicated per test file.

```rust
use std::collections::HashSet;

use bevy::prelude::*;
use hexworld::{ChunkKey, Hex, Level, Rings, StoreSettings, WorldConfig};
use hexworld_bevy::{ChunkView, HexWorld, Loader, Shown};

#[path = "common/mod.rs"]
mod common;
use common::{headless_app, run_until_fully_settled};

// The default rings settle to 76 chunks (Task 7's known total for default rings at the
// origin: 37 ken + 37 cho + 1 ri + 1 world). Waiting for a genuine settle, rather than a
// fixed frame budget, is what makes the state these tests inspect a fact instead of an
// observation caught mid-flight.
const DEFAULT_WINDOW_TOTAL: usize = 76;

#[test]
fn a_shown_child_is_left_out_of_its_parents_mesh() {
    let mut app = headless_app(StoreSettings::default());
    app.world_mut().spawn((Transform::default(), Loader { rings: Rings::default() }));
    run_until_fully_settled(&mut app, DEFAULT_WINDOW_TOTAL);
    let shown = app.world().resource::<Shown>();
    let cho_key = ChunkKey::new(Level::Cho, Hex::ZERO);
    let omitted = shown.omitted_for(cho_key);
    // Every ken chunk on screen must be omitted from the cho chunk that would draw it.
    for key in shown.keys() {
        if key.level == Level::Ken && key.parent_key() == Some(cho_key) {
            assert!(omitted.contains(&key.cell), "{key:?} is drawn twice");
        }
    }
    assert!(!omitted.is_empty(), "nothing was handed over");
}

#[test]
fn every_shown_chunk_has_exactly_one_entity() {
    let mut app = headless_app(StoreSettings::default());
    app.world_mut().spawn((Transform::default(), Loader { rings: Rings::default() }));
    run_until_fully_settled(&mut app, DEFAULT_WINDOW_TOTAL);
    let mut query = app.world_mut().query::<&ChunkView>();
    let mut seen: HashSet<ChunkKey> = HashSet::new();
    for view in query.iter(app.world()) {
        assert!(seen.insert(view.0), "{:?} has two entities", view.0);
    }
    let shown = app.world().resource::<Shown>();
    assert_eq!(seen.len(), shown.keys().len());
}

#[test]
fn unloading_puts_the_cell_back_into_its_parent() {
    // Ruling 3: a short unload delay, or the test would wait out the real 5 s default.
    let mut app = headless_app(StoreSettings { unload_delay_s: 0.05, ..StoreSettings::default() });
    let entity = app
        .world_mut()
        .spawn((Transform::default(), Loader { rings: Rings::default() }))
        .id();
    run_until_fully_settled(&mut app, DEFAULT_WINDOW_TOTAL);
    let cho_key = ChunkKey::new(Level::Cho, Hex::ZERO);
    assert!(!app.world().resource::<Shown>().omitted_for(cho_key).is_empty());

    // Shrink the loader's shaku window to nothing and wait out the unload delay.
    app.world_mut().entity_mut(entity).insert(Loader { rings: Rings { shaku: 0, ken: 3, cho: 3 } });
    // The new window (shaku=0) is a strict subset of the old one: it never grows, so a
    // genuine settle is simply in_flight==0.
    common::run_until(
        &mut app,
        |world| world.store().in_flight_count() == 0,
        |world| format!("in_flight={}", world.store().in_flight_count()),
    );
    let shown = app.world().resource::<Shown>();
    let still_shown: Vec<ChunkKey> = shown
        .keys()
        .into_iter()
        .filter(|k| k.level == Level::Ken && k.parent_key() == Some(cho_key))
        .collect();
    // As implemented: `cho_key`'s omissions are a mix of two mechanisms (see the guest-cell
    // note above), so "every omitted cell is a still-shown ken child" does not hold on its
    // own — it must be partitioned by actual owner. Verified concretely during
    // implementation: Hex{37,-23} stays omitted from cho(0,0) because the *neighbouring*
    // cho(1,-1) is shown, with no ken chunk involved at all, since the cho window
    // (`rings.ken`) was never shrunk by this test.
    for cell in shown.omitted_for(cho_key) {
        let owner_cho = hexworld::owner::parent_of(*cell, Level::Ken);
        if owner_cho == cho_key.cell {
            assert!(
                still_shown.iter().any(|k| k.cell == *cell),
                "{cell:?} is owned by {cho_key:?} and omitted from it, but no ken chunk draws it"
            );
        } else {
            let owning_cho = ChunkKey::new(Level::Cho, owner_cho);
            assert!(
                shown.entity(owning_cho).is_some(),
                "{cell:?} is a guest cell omitted from {cho_key:?}, but its true owner \
                 {owning_cho:?} is not shown to draw it instead"
            );
        }
    }
}
```

**As implemented, three more tests were added** to make the one-frame property itself
testable — the three tests above only check state after many frames have run, which
proves nothing about any single frame:

- `the_parents_omission_never_lags_a_shown_childs_entity_by_even_one_frame` — checks the
  first test's bookkeeping invariant on *every* frame of the settle loop, not just at the
  end.
- `the_parents_mesh_drops_the_vertex_the_same_frame_the_childs_entity_appears` — the same
  idea against the actual `Mesh3d` vertex data (with a positive control: the vertex must
  have been present the frame before), for the parent/child path.
- `the_guest_sides_mesh_drops_the_shared_vertex_the_frame_both_neighbours_are_shown`
  (**fix round 1**) — the same vertex-level check for the same-level guest-dedup path. The
  first two tests, plus a bookkeeping-only unit test in `entities.rs`
  (`a_guest_chunk_omits_once_its_owner_is_shown`), all still pass when the production code
  is deliberately sabotaged to update `Shown` without calling `remesh_shown` — bookkeeping
  alone has no teeth. This test, checking real vertex positions, fails against that same
  sabotage (falsification evidence in `task-12-report.md`).

`entities.rs` also gained three direct unit tests of the guest/absorb helpers against a
bare `Shown` with fake `Entity` values (`a_guest_chunk_omits_once_its_owner_is_shown`,
`unloading_the_owner_restores_the_guest`, `a_late_parent_absorbs_an_already_shown_child`) —
fast, no `App`/ECS needed, and they pin the ordering rule directly.

- [ ] **Step 2: Implement `Shown` and the handover in `entities.rs`**

```rust
/// What is on screen, and which cells each chunk leaves out because a finer chunk draws them.
#[derive(Resource, Default)]
pub struct Shown {
    entities: HashMap<ChunkKey, Entity>,
    omitted: HashMap<ChunkKey, HashSet<Hex>>,
}

impl Shown {
    pub fn keys(&self) -> Vec<ChunkKey> {
        self.entities.keys().copied().collect()
    }

    pub fn entity(&self, key: ChunkKey) -> Option<Entity> {
        self.entities.get(&key).copied()
    }

    pub fn omitted_for(&self, key: ChunkKey) -> &HashSet<Hex> {
        static EMPTY: std::sync::OnceLock<HashSet<Hex>> = std::sync::OnceLock::new();
        self.omitted.get(&key).unwrap_or_else(|| EMPTY.get_or_init(HashSet::new))
    }

    pub fn omit(&mut self, parent: ChunkKey, cell: Hex) {
        self.omitted.entry(parent).or_default().insert(cell);
    }

    pub fn restore(&mut self, parent: ChunkKey, cell: Hex) {
        if let Some(set) = self.omitted.get_mut(&parent) {
            set.remove(&cell);
        }
    }
}
```

The flow in `collect_finished_jobs`:

1. A job finishes for key `K`. Insert the chunk; if the store refuses it, drop everything and carry on.
2. Work out `K`'s parent `P` (`K.parent_key()`). If `P` is on screen, mark `K.cell` omitted from `P` and **re-mesh `P` now, on the main thread**, then swap `P`'s mesh handle and spawn `K`'s entity in the same frame.
   - Re-meshing a cho chunk on the main thread costs a few milliseconds. If the measurement from Task 10 step 3 shows that a cho re-mesh is over about 4 ms, do it as a follow-up task instead and hold `K`'s mesh until it lands; the two are then applied together. Decide from the measured number, not from taste.
3. On unload of `K`: restore `K.cell` in `P`, re-mesh `P` the same way, and despawn `K`'s entity in the same frame.

```rust
/// Re-mesh a chunk that is already on screen, with its current omissions.
pub fn remesh_shown(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    world: &HexWorld,
    shown: &Shown,
    key: ChunkKey,
) {
    let (Some(entity), Some(chunk)) = (shown.entity(key), world.store().chunk(key)) else {
        return;
    };
    let data = hexworld::mesh::mesh_chunk(world.store().config(), chunk, shown.omitted_for(key));
    let handle = meshes.add(to_bevy_mesh(&data));
    commands.entity(entity).insert(Mesh3d(handle));
}
```

- [ ] **Step 2b: Change what Task 11 left simplified**

Two edits, both needed for the tests above to pass:

1. `HexWorldPlugin::build` gains `.init_resource::<Shown>()`.
2. `tasks::spawn_jobs` currently meshes with an empty omission set. It must pass the
   chunk's current omissions instead, so a chunk that arrives while its children are
   already on screen does not draw over them:

```rust
let omitted: HashSet<Hex> = shown.omitted_for(key).clone();
let task = pool.spawn(async move {
    let chunk = hexworld::chunk::generate(&cfg, key);
    let mesh = hexworld::mesh::mesh_chunk(&cfg, &chunk, &omitted);
    JobOutput { key, chunk, mesh }
});
```

`spawn_jobs` therefore takes `&Shown` as well, and `drive_store` passes it through.

**As implemented**, `Shown` also gained `insert_entity`/`remove` (so despawning a chunk
forgets its entity and its omission set together, and there is one owner of "does this key
have an entity" — the brief's own Step 3 note), and `entities.rs` gained the guest-cell
functions (`guest_cells_to_omit`, `sync_guests_on_load`, `release_guests_on_unload`) and
`absorb_already_shown_children` described under Interfaces above. `collect_finished_jobs`
and `drive_store` call all of it inline, in the same system call as the entity
spawn/despawn.

- [ ] **Step 3: Run the tests**

Run: `cd Experiments/exp-05 && cargo test -p hexworld_bevy`
Expected: PASS. If `every_shown_chunk_has_exactly_one_entity` fails, an entity is being spawned before the old one is despawned; do both through `Shown` so there is one owner of that fact.

- [ ] **Step 4: Commit**

```bash
cd /home/lexa/DevProjects/_GameDev/Murabito/Experiments/exp-05
cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test
git -C /home/lexa/DevProjects/_GameDev/Murabito add Experiments/exp-05
git -C /home/lexa/DevProjects/_GameDev/Murabito commit -m "exp-05: tier handover — parent re-mesh and child swap in one frame

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 13: Hex lines and the detail-level tint

**Files:**
- Create: `crates/hexworld_bevy/src/material.rs`, `crates/hexworld_bevy/assets/shaders/ground.wgsl`
- Modify: `crates/hexworld_bevy/src/lib.rs`, `src/entities.rs`, `Cargo.toml` (nothing new; assets are loaded from the viewer's asset folder)

**Interfaces:**
- Produces: `GroundExtension { mode: u32, tint: u32 }` (an `AsBindGroup` at binding 100); `GroundMaterial = ExtendedMaterial<StandardMaterial, GroundExtension>`; `LineMode { Off, ShakuOnly, Nested }` resource, default `ShakuOnly`; `TintByLevel(bool)` resource, default false. `GroundMaterialHandle` now holds `Handle<GroundMaterial>`.

The mesh already carries what the shader needs, in the UVs: `UV0 = (edge_w, border_level)`, `UV1 = (cell_level, 0)`. `edge_w` is 0 at a cell's centre and 1 at its corners, so `1 - edge_w` measured against `fwidth` gives a constant-width line along the cell edge. No hex maths in the shader.

- [ ] **Step 1: Write the material**

```rust
//! The ground material: Bevy's standard material plus hex lines and a detail-level tint.

use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;

pub const SHADER_PATH: &str = "shaders/ground.wgsl";

#[derive(Asset, AsBindGroup, Reflect, Clone, Default, Debug)]
pub struct GroundExtension {
    /// 0 off, 1 shaku only, 2 nested.
    #[uniform(100)]
    pub mode: u32,
    /// 1 tints the ground by the detail level drawn there.
    #[uniform(101)]
    pub tint: u32,
}

impl MaterialExtension for GroundExtension {
    fn fragment_shader() -> ShaderRef {
        SHADER_PATH.into()
    }
}

pub type GroundMaterial = ExtendedMaterial<StandardMaterial, GroundExtension>;

#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum LineMode {
    Off,
    #[default]
    ShakuOnly,
    Nested,
}

#[derive(Resource, Clone, Copy, Default)]
pub struct TintByLevel(pub bool);
```

Registration in the plugin: `app.add_plugins(MaterialPlugin::<GroundMaterial>::default())`, and a system that writes `LineMode` and `TintByLevel` into the material asset when they change.

- [ ] **Step 2: Write the shader**

`crates/hexworld_bevy/assets/shaders/ground.wgsl`:

```wgsl
// Hex lines drawn from the mesh's own edge data, plus an optional tint by detail level.
// UV0 = (edge_w, border_level): edge_w is 0 at a cell's centre and 1 at its corners.
// UV1 = (cell_level, 0): 0 shaku, 1 ken, 2 cho, 3 ri.

#import bevy_pbr::forward_io::{VertexOutput, FragmentOutput}
#import bevy_pbr::pbr_fragment::pbr_input_from_standard_material
#import bevy_pbr::pbr_functions::{alpha_discard, apply_pbr_lighting, main_pass_post_lighting_processing}

@group(2) @binding(100) var<uniform> line_mode: u32;
@group(2) @binding(101) var<uniform> tint_by_level: u32;

fn level_colour(level: f32) -> vec4<f32> {
    if level < 0.5 { return vec4<f32>(0.10, 0.10, 0.12, 1.0); }   // shaku: thin and dark
    if level < 1.5 { return vec4<f32>(0.85, 0.87, 0.90, 1.0); }   // ken: light
    if level < 2.5 { return vec4<f32>(0.90, 0.80, 0.25, 1.0); }   // cho: yellow
    return vec4<f32>(0.85, 0.25, 0.20, 1.0);                      // ri: red
}

fn level_tint(level: f32) -> vec4<f32> {
    if level < 0.5 { return vec4<f32>(1.00, 1.00, 1.00, 1.0); }
    if level < 1.5 { return vec4<f32>(0.80, 0.90, 1.00, 1.0); }
    if level < 2.5 { return vec4<f32>(1.00, 0.90, 0.75, 1.0); }
    return vec4<f32>(1.00, 0.75, 0.75, 1.0);
}

fn line_width_px(border_level: f32) -> f32 {
    return 1.0 + border_level * 0.8;
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    let edge_w = in.uv.x;
    let border_level = in.uv.y;
    let cell_level = in.uv_b.x;

    if tint_by_level == 1u {
        pbr_input.material.base_color = pbr_input.material.base_color * level_tint(cell_level);
    }

    // Only top faces carry edge data; side faces have edge_w == 0 everywhere.
    var draw_line = false;
    if line_mode == 1u {
        draw_line = cell_level < 0.5;              // the borders of loaded shaku only
    } else if line_mode == 2u {
        draw_line = true;                          // every level, nested
    }

    if draw_line {
        let distance_px = (1.0 - edge_w) / max(fwidth(edge_w), 1e-6);
        let width = line_width_px(border_level);
        let strength = 1.0 - smoothstep(width - 1.0, width, distance_px);
        pbr_input.material.base_color = mix(
            pbr_input.material.base_color,
            level_colour(border_level),
            strength,
        );
    }

    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);

    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
```

- [ ] **Step 3: Check it actually compiles on the GPU**

A shader error only shows at run time, and the viewer does not exist yet, so add a small
example in this crate: `crates/hexworld_bevy/examples/shader_check.rs`. It opens a window
with `DefaultPlugins` and `HexWorldPlugin::default()`, spawns one `Loader` entity at the
origin, a `Camera3d` 60 m up looking down, a `DirectionalLight`, and exits after 120 frames.

```bash
cd Experiments/exp-05
cargo run -p hexworld_bevy --example shader_check 2>&1 | rg -i "error|shader|wgsl" | head -20
```

Expected: no shader or WGSL errors, and ground visible in the window. If the shader fails to
compile, the message names the line and the `@group(2)` binding numbers are the usual cause. **Fallback if the extension proves too fiddly:** drop the material extension and draw the lines as a second mesh of line-list geometry from the same edge data. Take it only if the shader costs more than about an hour; note the decision in the commit message.

- [ ] **Step 4: Write a test for the mode plumbing (not the pixels)**

```rust
#[test]
fn line_mode_reaches_the_material() {
    let mut app = headless_app_with_materials();
    app.world_mut().insert_resource(LineMode::Nested);
    app.update();
    let handle = app.world().resource::<GroundMaterialHandle>().0.clone();
    let materials = app.world().resource::<Assets<GroundMaterial>>();
    assert_eq!(materials.get(&handle).unwrap().extension.mode, 2);
}
```

- [ ] **Step 5: Commit**

```bash
cd /home/lexa/DevProjects/_GameDev/Murabito/Experiments/exp-05
cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test
git -C /home/lexa/DevProjects/_GameDev/Murabito add Experiments/exp-05
git -C /home/lexa/DevProjects/_GameDev/Murabito commit -m "exp-05: hex lines and detail-level tint in the ground material

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 14: The viewer's camera

**Files:**
- Modify: `crates/viewer/Cargo.toml`
- Create: `crates/viewer/src/main.rs`, `src/camera.rs`
- Create: `crates/viewer/assets/` → a symlink or copy of `crates/hexworld_bevy/assets/shaders`

**Interfaces:**
- Produces: `CameraRig { focus_east: f64, focus_north: f64, zoom: f32 }` component on the focus entity, which also carries `Loader`; `rig_transforms(&CameraRig) -> (Vec3, Vec3)` (focus position, camera position); `tilt_for_zoom(f32) -> f32`; `pan_speed_for_zoom(f32) -> f32`. The camera is a child entity of the focus.

The rig: the focus sits on the ground, the camera sits behind and above it looking at it. Zoom is the distance from the focus, from 3 m to 4,000 m. Tilt goes from 45° close in to 75° far out. Pan speed is proportional to zoom, so panning always feels the same.

- [ ] **Step 1: Write the failing tests for the camera maths**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tilt_goes_from_45_close_to_75_far() {
        assert!((tilt_for_zoom(MIN_ZOOM).to_degrees() - 45.0).abs() < 1.0);
        assert!((tilt_for_zoom(MAX_ZOOM).to_degrees() - 75.0).abs() < 1.0);
        // and it is monotonic
        let mut previous = 0.0;
        for i in 0..=20 {
            let zoom = MIN_ZOOM + (MAX_ZOOM - MIN_ZOOM) * i as f32 / 20.0;
            let tilt = tilt_for_zoom(zoom);
            assert!(tilt >= previous, "tilt should not decrease as you zoom out");
            previous = tilt;
        }
    }

    #[test]
    fn pan_speed_scales_with_zoom() {
        assert!(pan_speed_for_zoom(MAX_ZOOM) > pan_speed_for_zoom(MIN_ZOOM) * 10.0);
    }

    #[test]
    fn the_camera_sits_above_and_behind_the_focus() {
        let rig = CameraRig { focus_east: 0.0, focus_north: 0.0, zoom: 50.0 };
        let (focus, camera) = rig_transforms(&rig);
        assert!(camera.y > focus.y + 10.0, "the camera should be above the focus");
        assert!(camera.z > focus.z, "and south of it, looking north");
        assert!((camera.distance(focus) - 50.0).abs() < 0.5, "zoom is the distance");
    }

    #[test]
    fn zoom_is_clamped() {
        let mut rig = CameraRig { focus_east: 0.0, focus_north: 0.0, zoom: 50.0 };
        rig.zoom_by(-100.0);
        assert!(rig.zoom >= MIN_ZOOM);
        rig.zoom_by(1e9);
        assert!(rig.zoom <= MAX_ZOOM);
    }

    #[test]
    fn the_focus_stays_in_the_world() {
        let cfg = WorldConfig::default();
        let mut rig = CameraRig { focus_east: 0.0, focus_north: 0.0, zoom: 50.0 };
        rig.pan(1e6, 0.0, &cfg);
        let cell = hexworld::plane::round_at(rig.focus_east, rig.focus_north, Level::Ri);
        assert!(hexworld::world::in_world(cell, &cfg), "panned out of the world");
    }
}
```

- [ ] **Step 2: Implement `camera.rs`**

Key pieces:

```rust
pub const MIN_ZOOM: f32 = 3.0;
pub const MAX_ZOOM: f32 = 4_000.0;

/// 45 degrees close in, 75 far out, easing on the logarithm of the zoom so the change
/// feels even across the whole range.
pub fn tilt_for_zoom(zoom: f32) -> f32 {
    let t = ((zoom / MIN_ZOOM).ln() / (MAX_ZOOM / MIN_ZOOM).ln()).clamp(0.0, 1.0);
    (45.0 + 30.0 * t).to_radians()
}

/// Metres per second of panning: a constant fraction of the zoom.
pub fn pan_speed_for_zoom(zoom: f32) -> f32 {
    zoom * 0.9
}

#[derive(Component, Clone, Copy, Debug)]
pub struct CameraRig {
    pub focus_east: f64,
    pub focus_north: f64,
    pub zoom: f32,
}

impl CameraRig {
    pub fn zoom_by(&mut self, delta: f32) {
        // Multiplicative, so a wheel click feels the same at every distance.
        self.zoom = (self.zoom * (1.0 + delta * 0.1)).clamp(MIN_ZOOM, MAX_ZOOM);
    }

    pub fn pan(&mut self, east: f64, north: f64, cfg: &WorldConfig) {
        let (e, n) = (self.focus_east + east, self.focus_north + north);
        // Stay in the world: keep the focus inside the outermost ri.
        let ri = hexworld::plane::round_at(e, n, Level::Ri);
        if hexworld::world::in_world(ri, cfg) {
            self.focus_east = e;
            self.focus_north = n;
        }
    }
}

/// Focus and camera positions in Bevy space. The camera sits south of the focus,
/// so the view looks north; height comes from the tilt.
pub fn rig_transforms(rig: &CameraRig) -> (Vec3, Vec3) {
    let focus = to_bevy(rig.focus_east, rig.focus_north, 0.0);
    let tilt = tilt_for_zoom(rig.zoom);
    let back = rig.zoom * tilt.cos();
    let up = rig.zoom * tilt.sin();
    (focus, focus + Vec3::new(0.0, up, back))
}
```

Systems: read input (WASD/arrows, middle-drag, wheel), update the rig, set the focus entity's transform with its ground height from `world.store().surface_height_m(east, north)`, and set the camera's transform with `looking_at(focus, Vec3::Y)`.

- [ ] **Step 3: `main.rs` — the app**

```rust
fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Murabito exp-05 — hex world".into(),
                resolution: (1600.0, 900.0).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(HexWorldPlugin::default())
        .add_plugins(bevy_egui::EguiPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(Update, (camera::handle_input, camera::follow_ground).chain())
        .add_systems(bevy_egui::EguiPrimaryContextPass, panel::draw)
        .run();
}
```

`setup` spawns: the focus entity with `CameraRig`, `Loader::default()` and a `Transform`; the camera as its child with `Camera3d`, `AmbientLight { brightness: 240.0, ..default() }` and a perspective projection; a `DirectionalLight` with `shadow_maps_enabled: true`; and `ClearColor` for the sky.

- [ ] **Step 4: Run it and look**

```bash
cd Experiments/exp-05 && cargo run -p viewer --release
```

Expected: ground appears around the focus, panning and zooming work, and the detail levels are visible if you turn the tint on. Watch the terminal for shader or asset errors. **This is the first point where a person should look at it**; show the user, and ask before carrying on if anything looks wrong.

- [ ] **Step 5: Commit**

```bash
cd /home/lexa/DevProjects/_GameDev/Murabito/Experiments/exp-05
cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test
git -C /home/lexa/DevProjects/_GameDev/Murabito add Experiments/exp-05
git -C /home/lexa/DevProjects/_GameDev/Murabito commit -m "exp-05: viewer with an overhead camera rig

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 15: The panel

**Files:**
- Create: `crates/viewer/src/panel.rs`
- Modify: `crates/viewer/src/main.rs`

**Interfaces:**
- Consumes: `HexWorld`, `Shown`, `LineMode`, `TintByLevel`, `CameraRig`.
- Produces: `panel::draw` (an egui system in `EguiPrimaryContextPass`); `HoveredCell` resource holding `Option<(Hex, i32)>`.

Contents, as the spec lists them:

- **Address:** the focus's `ri / cho / ken / shaku / layer` (from `Address`'s `Display`), its position in metres, and the same for the cell under the mouse.
- **Levels:** a row per level with chunks, columns, lingering, loads and unloads in the last second, and triangles.
- **Tuning:** rings per level (sliders, 0 to 24), unload delay (0 to 30 s), layer thickness in sun (1 to 30), seed. Changing the thickness or the seed calls `store.clear()` and despawns every chunk entity.
- **Display:** line mode (three radio buttons), tint by level (a checkbox).
- **FPS and frame time**, from `bevy::diagnostic::FrameTimeDiagnosticsPlugin`.
- **Loaders:** a row per loader entity, with its address; buttons to add a loader at the mouse and to remove the last one.

- [ ] **Step 1: Write the panel**

```rust
pub fn draw(
    mut contexts: EguiContexts,
    mut world: ResMut<HexWorld>,
    mut line_mode: ResMut<LineMode>,
    mut tint: ResMut<TintByLevel>,
    mut rigs: Query<(&CameraRig, &mut Loader)>,
    hovered: Res<HoveredCell>,
    diagnostics: Res<DiagnosticsStore>,
) -> Result {
    let ctx = contexts.ctx_mut()?;
    let mut viewport_ui = egui::Ui::new(
        ctx.clone(),
        "viewport".into(),
        egui::UiBuilder::new()
            .layer_id(egui::LayerId::background())
            .max_rect(ctx.viewport_rect()),
    );
    egui::Panel::right("hexworld_panel")
        .default_size(340.0)
        .show(&mut viewport_ui, |ui| {
            // ... sections as listed above
        });
    Ok(())
}
```

- [ ] **Step 2: Write the hover system**

```rust
/// The cell under the mouse, found by casting a ray onto the plane at the focus height.
pub fn update_hover(
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    focus: Query<&GlobalTransform, With<CameraRig>>,
    mut hovered: ResMut<HoveredCell>,
    world: Res<HexWorld>,
) {
    hovered.0 = None;
    let (Ok(window), Ok((camera, camera_transform)), Ok(focus)) =
        (windows.single(), cameras.single(), focus.single())
    else {
        return;
    };
    let Some(cursor) = window.cursor_position() else { return };
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor) else { return };
    let ground = focus.translation();
    let Some(distance) = ray.intersect_plane(ground, InfinitePlane3d::new(Vec3::Y)) else {
        return;
    };
    let point = ray.get_point(distance);
    let (east, north, _) = from_bevy(point);
    let shaku = round_at(east, north, Level::Shaku);
    let layer = world
        .store()
        .surface_height_m(east, north)
        .map(|h| world.store().config().layer_of_height(h))
        .unwrap_or(0);
    hovered.0 = Some((shaku, layer));
}
```

- [ ] **Step 2b: Add the config-change API to the plugin**

The panel changes the seed, the layer thickness and the ring counts. Rings are just the
`Loader` component, but seed and thickness change the world itself, so they belong in
`hexworld_bevy` (with the tests), not in the viewer's binary:

```rust
// in hexworld_bevy/src/lib.rs
impl HexWorld {
    /// Replace the world's settings and drop everything generated from the old ones.
    pub fn set_config(&mut self, config: WorldConfig) {
        let settings = *self.store.settings();
        self.store = ChunkStore::new(config, settings);
        self.generation += 1;
    }

    /// Bumped whenever the world is replaced, so chunk entities from the old one can go.
    pub fn generation(&self) -> u64 {
        self.generation
    }
}

/// Despawn every chunk entity left over from a previous world.
pub fn despawn_stale_chunks(
    mut commands: Commands,
    world: Res<HexWorld>,
    mut shown: ResMut<Shown>,
    views: Query<(Entity, &ChunkView)>,
    mut last_generation: Local<u64>,
) {
    if world.generation() == *last_generation {
        return;
    }
    *last_generation = world.generation();
    for (entity, _) in &views {
        commands.entity(entity).despawn();
    }
    *shown = Shown::default();
}
```

Add `despawn_stale_chunks` to the plugin's systems, before `drive_store`.

- [ ] **Step 3: Write the tests for the parts that are not drawing**

These go in `crates/hexworld_bevy/tests/config.rs`, because the behaviour lives in the
plugin; the viewer is a binary and is checked by eye and by screenshots.

```rust
#[test]
fn changing_the_seed_clears_the_world() {
    let mut app = headless_app();
    app.world_mut().spawn((Transform::default(), Loader::default()));
    for _ in 0..200 { app.update(); }
    assert!(!app.world().resource::<HexWorld>().store().loaded_keys().is_empty());
    app.world_mut()
        .resource_mut::<HexWorld>()
        .set_config(WorldConfig { seed: 7, ..WorldConfig::default() });
    app.update();
    let world = app.world().resource::<HexWorld>();
    assert!(world.store().loaded_keys().is_empty(), "the old world should be cleared");
    let mut query = app.world_mut().query::<&ChunkView>();
    assert_eq!(query.iter(app.world()).count(), 0, "old chunk entities should be gone");
}

#[test]
fn ring_changes_reach_the_store() {
    let mut app = headless_app();
    let entity = app.world_mut().spawn((Transform::default(), Loader::default())).id();
    for _ in 0..200 { app.update(); }
    let before = app.world().resource::<HexWorld>().store().loaded_keys().len();
    app.world_mut().entity_mut(entity).insert(Loader { rings: Rings { shaku: 6, ken: 3, cho: 3 } });
    for _ in 0..300 { app.update(); }
    let after = app.world().resource::<HexWorld>().store().loaded_keys().len();
    assert!(after > before, "widening the shaku window should load more: {before} -> {after}");
}
```

- [ ] **Step 4: Run it, look at it, and commit**

```bash
cd Experiments/exp-05 && cargo run -p viewer --release
```

Check by hand: the address changes as you pan; hovering reports a cell; sliders change what loads; parking on a cell border reloads nothing (watch "loads in the last second" stay at 0).

```bash
cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test
git -C /home/lexa/DevProjects/_GameDev/Murabito add Experiments/exp-05
git -C /home/lexa/DevProjects/_GameDev/Murabito commit -m "exp-05: viewer panel — address, level stats, tuning, display toggles

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 16: Headless screenshots, docs, and the whole-branch check

**Files:**
- Create: `crates/viewer/src/headless.rs`, `Experiments/exp-05/README.md`
- Modify: `crates/viewer/src/main.rs`, `Experiments/manifest.md`, `AGENTS.md`

**Interfaces:**
- Produces: CLI flags `--frames N`, `--screenshot-dir DIR`, `--screenshot-every M`, `--seed N`, `--start-east M --start-north M`, `--rings-shaku N`.

- [ ] **Step 1: Add the flags and the screenshot path**

Parse the arguments by hand from `std::env::args()` — no `clap`, to keep the dependency list short. With `--frames N` the app exits after N frames; with `--screenshot-dir` it saves a PNG every `--screenshot-every` frames:

```rust
fn shoot(mut commands: Commands, frames: Res<FrameCount>, options: Res<Headless>) {
    let Some(dir) = options.screenshot_dir.as_ref() else { return };
    if frames.0 > 0 && frames.0 % options.screenshot_every == 0 {
        let path = dir.join(format!("frame-{:05}.png", frames.0));
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
    }
}

fn quit_after(frames: Res<FrameCount>, options: Res<Headless>, mut exit: MessageWriter<AppExit>) {
    if let Some(limit) = options.frames {
        if frames.0 >= limit {
            exit.write(AppExit::Success);
        }
    }
}
```

The window still opens; these runs are for taking pictures without a person watching, not for running with no display.

- [ ] **Step 2: Take the spec's screenshots and check them**

```bash
cd Experiments/exp-05
cargo run -p viewer --release -- --frames 240 --screenshot-every 60 --screenshot-dir screenshots
```

Look at each one and check, against the spec's success criteria:
1. ground around the focus with shaku borders drawn;
2. a handover from shaku to ken detail with no hole and no crack;
3. the nested line mode showing ken, cho and ri borders;
4. the tint mode showing the windows;
5. the panel.

Attach them when reporting to the user.

- [ ] **Step 3: Write `Experiments/exp-05/README.md`**

Short: what exp-05 is, how to build (`cargo build --release`), run (`cargo run -p viewer --release`), test (`cargo test`), the controls, the headless flags, and a pointer to the spec and this plan. Note that the crates are `hexworld` (engine-free), `hexworld_bevy` and `viewer`.

- [ ] **Step 4: Update `Experiments/manifest.md`**

Fill the exp-05 entry's **Run**, **Test** and **Controls** lines in, matching the other entries' shape:

```markdown
- **Run:** `cd Experiments/exp-05 && cargo run -p viewer --release`
- **Test:** `cd Experiments/exp-05 && cargo test`
```

Add the controls line (WASD/arrows or middle-drag pan · wheel zooms · L cycles line modes · T tint by level · F2 adds a loader at the mouse) and the headless line.

- [ ] **Step 5: Update `AGENTS.md`**

Under "Repository Organization", add the third kind of experiment next to the Python and UE ones:

```
   └─ exp-NN/              a Rust + Bevy experiment
      ├─ Cargo.toml        workspace
      ├─ crates/           an engine-free core crate, a Bevy plugin, the viewer app
      └─ docs/             specs/ and plans/, as below
```

And a short section saying: build and test with `cargo build --release` and `cargo test` from the experiment directory; the core crate has no engine dependency and is the part meant to outlive the engine choice; Rust build directories must not go under `/tmp`, which is a RAM-backed tmpfs on this machine.

- [ ] **Step 6: The whole-branch check**

```bash
cd /home/lexa/DevProjects/_GameDev/Murabito/Experiments/exp-05
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo test --release -- --ignored --nocapture   # the measurements
git -C /home/lexa/DevProjects/_GameDev/Murabito status -s
git -C /home/lexa/DevProjects/_GameDev/Murabito log --oneline origin/main..HEAD
```

Confirm, with output in hand, that:
- every success criterion in the spec is met, or is reported to the user as not met;
- the shaku window holds **1,332 columns in 37 ken chunks** with the queue idle;
- `hexworld`'s `Cargo.toml` still lists **no dependencies**;
- exp-03 and exp-04 are untouched (`git -C ... status` shows nothing outside `Experiments/exp-05`, `Experiments/manifest.md` and `AGENTS.md`).

- [ ] **Step 7: Commit, then stop**

```bash
git -C /home/lexa/DevProjects/_GameDev/Murabito add Experiments/exp-05 Experiments/manifest.md AGENTS.md
git -C /home/lexa/DevProjects/_GameDev/Murabito commit -m "exp-05: headless screenshots, README, manifest and AGENTS entries

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

**Do not push and do not open a pull request.** Report to the user what was built, what the measurements were, and which success criteria are met, and wait for them to decide.

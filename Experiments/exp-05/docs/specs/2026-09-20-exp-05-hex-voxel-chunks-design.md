# exp-05 — Hex coordinates, addresses and chunks: design

**Date:** 2026-09-20
**Location:** `Experiments/exp-05/`
**Stack:** Rust 1.95 (stable), Bevy 0.19, bevy_egui 0.42; a Cargo workspace. One small Python script (run with `uv run`) writes test fixtures from exp-03.
**Based on:** exp-03's coordinate system (`Experiments/exp-03/src/hexaddr.py` and its spec). No code is copied from another experiment; the address maths is ported to Rust and tested against exp-03's.

## Purpose

exp-05 builds one uniform hex coordinate system: ri, cho and ken are nested scales of chunks that are hot-loaded, the shaku is the primary unit, and the terrain lives inside that system as voxels. The drawn hexes come from the same system.

exp-05 covers **the coordinates, the address system and the chunks**. The terrain in it is a placeholder: procedural, but not the real terrain generator, which is expected to be heavy and is left for later.

All terrain, colour and line-style parameters are **placeholders**, to be tuned by eye.

## Decisions from brainstorming (2026-09-19 and 2026-09-20)

In the order they were made; each was the user's pick or in the user's words.

1. **One uniform hex coordinate system**, with ri, cho and ken as nested scales of hot-loadable chunks and the shaku as the primary unit. Terrain lives in it, and the drawn hexes reflect it. (From the handoff.)
2. **A fresh start, not in Unreal Engine.** First planned as plain C++ with a light viewer; then changed to **Rust and Bevy** for this experiment (the user: "i like that its FOSS and rust. lets try it for this experiment").
3. **The coordinates are for more than terrain.** What the user expects to live in them later: trees and large rocks generated with the world; player-built structures that snap to shaku; large structures such as Japanese castles that span many shaku. A building is not one object: it is made of **parts (walls, floors, doors) defined by a start and an end shaku**. None of this is built in exp-05.
4. **Terrain can be altered**: digging, flattening, moats, earthworks. Not built in exp-05.
5. **Blockiness fits** ("a vaguely rimworld like vibe"). On a pointy-top hex lattice, a rectangle's two side lengths come in different units (whole shaku one way, multiples of √3 shaku the other); that is accepted.
6. **Terrain is a voxel system of "pancake" hexes**: hex prisms one shaku across, stacked in layers.
7. **A pancake is 5 sun thick** (half a shaku, about 15 cm), and the thickness must be **tunable**.
8. **Scope of exp-05: coordinates, the hex address system, and chunks.** Placeholder terrain: "it can be procedural, but its not the real deal".
9. **Carried over from exp-03 unchanged:** the ratios (6 shaku per ken, 60 ken per cho, 36 cho per ri), packing B, one owner per split cell (battlement borders), and the world as a hexagon of radius 12 ri.
10. **For now the world is 1 ri**, keeping the 12-ri radius open for later.
11. **Distant detail uses columns.** A chunk holds a column per child cell; coarser cells get wider, but every column measures height in the same fine layers.
12. **One run list per cell, over the full height** (option A). No vertical sections for now; the layer is part of every address so that sections can be added later.
13. **Camera:** a RimWorld-style overhead camera. **Loading follows the camera only, but anything must be easy to declare as a loader.**
14. **Windows:** a fixed, tunable number of rings per level and per loader. The default is exp-03's tight 3 rings, so the system is easy to see working.
15. **Hex lines:** either exp-03's scheme (every level, nested, owned borders) or the borders of loaded shaku only, toggleable in the viewer. **The default is loaded shaku only.**
16. **Structure:** an engine-free core crate, a Bevy plugin crate, and a viewer app (approach 1).
17. **Coarse columns are sampled** from the height function, not aggregated from their children, **for now**; reconsider later (see "Open questions").
18. **Unloading is delayed** (the user's idea): a chunk that is no longer requested stays loaded, and drawn, for a delay before it unloads.

## Layout

```
Experiments/exp-05/
├─ Cargo.toml               workspace
├─ crates/
│  ├─ hexworld/             core: no Bevy, no graphics
│  ├─ hexworld_bevy/        plugin: Loader component, chunk tasks, chunk meshes, line material
│  └─ viewer/               the app
├─ tools/make_fixtures.py   writes reference cases from exp-03's hexaddr.py
├─ .gitignore               target/, screenshots/
└─ docs/specs/, docs/plans/
```

- `hexworld` depends on nothing from Bevy. It is what a later experiment, or another engine, would reuse.
- `tools/make_fixtures.py` imports exp-03's `hexaddr.py` (read only; exp-03 is not changed) and writes a fixture file into `crates/hexworld/tests/fixtures/`. The fixture is committed, so `cargo test` needs no Python.
- `AGENTS.md` gains a third kind of experiment (Rust + Bevy), with this layout and its build and test commands. The manifest entry is filled in from this design.

## Coordinates and addresses (`hexworld`)

### Units

| Unit | Contains | Flat-to-flat width |
|---|---|---|
| shaku | — | 10/33 m ≈ 0.303 m |
| ken | 36 shaku | 6 shaku ≈ 1.818 m |
| cho | 3,600 ken | 60 ken ≈ 109.09 m |
| ri | 1,296 cho | 36 cho ≈ 3,927.27 m |

One sun is a tenth of a shaku (1/33 m).

### Horizontal

Exactly exp-03's system:

- Every hex at every level is **pointy-top with the same orientation**, in axial `(q, r)`. The plane position of axial `(q, r)`, in units of the cell's width, is `(q + r/2, r·√3/2)` as `(east, north)`.
- **Packing B.** With `N` = 6, 60 or 36, parent `(a, b)` is centred on child `(N·a, N·b)`. Each parent has a true centre child; the children along its ideal hexagon's edges are halved, and its corner children are split three ways.
- **Ownership.** `owner(c, N)`: among the parent candidates near `(q/N, r/N)`, the one with the smallest `d2 = dq² + dq·dr + dr²`, where `(dq, dr) = (q − N·a, r − N·b)`; **ties go to the lexicographically greatest `(a, b)`**. All integer arithmetic. Every parent owns exactly `N²` children, and its owned territory is its ideal hexagon with battlement edges.
- **One global shaku grid.** Its origin is the world centre, which is the centre of a ri, a cho, a ken and a shaku.
- Cells are `i32` (the 12-ri world is about ±170,000 shaku across). `d2` is computed in `i64`, because the squares approach the `i32` limit at ri scale.

### Vertical

- A signed integer **layer**. Layer `k` spans heights `k·t` to `(k+1)·t`; layer 0 starts at height 0.
- `t` is the layer thickness, **5 sun by default** (5/33 m ≈ 0.1515 m). It is a field of `WorldConfig`, not a compile-time constant.
- The layer unit is the same at every level: a cho-wide column measures its height in the same layers as a shaku-wide one.

### `WorldConfig`

`seed`, `world_radius_ri` (default **0**: only ri (0, 0); 12 gives exp-03's world), `layer_thickness_sun` (default 5), `bottom_layer` (the lowest layer any column reaches). Everything generated is a pure function of the config.

### Addresses

`ri (q,r) / cho (q,r) / ken (q,r) / shaku (q,r) / layer k`. The ri part is the ri's position in the world; the cho, ken and shaku parts are offsets from the parent's centre in the child lattice, as in exp-03. A voxel is `(global shaku, layer)`; a coarse voxel is `(level, cell, layer)`.

### Functions

- Both ways: address ↔ global shaku (+ layer) ↔ plane metres `(east, north)` + height. Metres are `f64`.
- Ported from `hexaddr.py`: `owner`, `parent`, `up`, `centre_child`, `local`, `children` (from a per-level template of owned offsets, built once), `neighbours`, `window(cell, rings)`, `round_at`, `in_world`, `world_ri`, `format_address`.
- The core has no sub-shaku positions. A continuous position (the camera's focus) is converted to a shaku at the boundary.

### Axes

The core knows only `(east, north, height)`. Bevy is right-handed with Y up, so the plugin maps **east → +X, height → +Y, north → −Z**, in one function used everywhere. (North on +Z would mirror the world.)

### World bounds

The world is every ri within hex distance `world_radius_ri` of `(0, 0)`. With the default, that is ri (0, 0) alone: about 3.9 km across, its edge the ri's owned border. Nothing outside the world is generated.

At ±2 km, Bevy's `f32` positions are accurate to under a millimetre, so exp-05 has no floating origin. Chunk meshes are built relative to their chunk's centre, so one can be added later without changing them.

## Chunks, columns and placeholder terrain (`hexworld`)

### Chunks

A chunk is **one parent cell's owned children**: the unit of generation, meshing, loading and unloading. Its key is `ChunkKey { level, cell }`, where the level is the parent's (ken, cho, ri, or world). It holds one column per child, in the order of the level's template.

| Chunk | Columns | Column width |
|---|---|---|
| world | 1 per ri in the world (1 for now) | ri |
| a ri | 1,296 | cho |
| a cho | 3,600 | ken |
| a ken | 36 | shaku |

### Columns

A column is a sorted list of non-overlapping runs, `Run { bottom, top, material }`, in layers (both ends inclusive). Gaps between runs are air, so the format can already hold overhangs and caves. No run goes below `bottom_layer`. Materials are a placeholder enum: rock, dirt, grass.

### Placeholder terrain

- A seeded, point-evaluable height function: hashed-lattice gradient noise written in the crate (no dependency), a few octaves, hills of about 30 m across the ri. Its module and names say "placeholder".
- A column is rock from `bottom_layer` up to near the surface, a few layers of dirt, and one layer of grass on top.
- A chunk's contents are a pure function of `(config, key)`. Load order, timing and which loader asked never change what is generated, and generation is safe on any thread.

### Coarse columns

A coarse column **samples** the height function at its cell's centre and drops the octaves whose wavelength is under twice its cell width, so distant terrain does not alias. It is an approximation of its children, not a summary of them. See "Open questions".

## Loaders, windows and the chunk store (`hexworld`)

### Loader

A focus (a global shaku) and a ring count per detail level, default 3 / 3 / 3. A loader requests:

| Detail | Chunks requested |
|---|---|
| shaku columns | the ken chunks of its ken + N rings |
| ken columns | the cho chunks of its cho + N rings |
| cho columns | the ri chunks of its ri + N rings, clipped to the world |
| ri columns | the world chunk, always |

Windows are clipped to the world at every level: a chunk whose parent cell's ri is outside the world is never requested.

### Ancestors

The request always includes every requested chunk's ancestors. With tunable rings a fine window can reach past a coarse one; this rule means a loaded chunk's parent column always exists, whatever the ring settings.

### `ChunkStore`

No threads, no clock and no engine.

- `update(loaders, now)`:
  1. takes the union of the loaders' requests, with ancestors;
  2. returns the chunks to load: requested, not loaded and not already handed out. They are ordered **coarsest level first, then by distance from the nearest loader focus to the parent cell's centre**, ties by key;
  3. marks loaded chunks that are no longer requested as **lingering**, stamped with `now`; clears the stamp of any lingering chunk that is requested again;
  4. returns the chunks to unload: lingering chunks whose delay has run out, and which are not the ancestor of a chunk that stays.
- **Delayed unloading.** The delay is a store setting, default **5 s**; 0 gives exp-03's immediate unloading. A lingering chunk is an ordinary loaded chunk until it expires: it is drawn, it counts as loaded for the handover, and its ancestors stay with it.
- `generate(config, key) -> Chunk` is a pure function, separate from the store. `insert(chunk)` stores a finished chunk, or drops it if it is no longer requested.
- **Lookups:** the column at `(level, cell)`; the finest detail loaded at a shaku; whether a parent cell's child chunk is loaded.
- **Stats per level:** chunks and columns loaded, lingering, loads and unloads in the last second, and the number waiting.

Tests drive the store synchronously; the plugin drives it with background tasks. The logic is the same.

## Meshing (`hexworld`)

Pure geometry with no Bevy types: `mesh_chunk(...) -> MeshData` (positions, normals, colours, per-vertex line data, indices), relative to the chunk's centre, in `(east, north, height)`.

- **Prisms.** Each run of each column is a hex prism of the column's width (an ideal hexagon): a top face where there is air above it, a bottom face where there is air below it (never at `bottom_layer`), and side faces only over the layers where the neighbouring column is air.
- **Neighbours across the chunk's border** come from the generator (it is pure), so a mesh job needs nothing but the config and the key. See "Open questions" for what edits change here.
- **Omitted columns.** `mesh_chunk` takes the set of child cells whose own chunk is shown, and leaves those columns out.
- **Guests.** A chunk also draws the neighbour-owned children that overlap its parent's ideal hexagon (the halved edge children on 3 edges, and the corner children it does not own). Coarse columns are drawn as ideal hexagons, while a fine chunk's owned territory is battlement-shaped; without guests there would be half-cell holes along a handover. Where the neighbouring chunk is loaded too, the guest is an identical copy of its cell.
- **The world's edge.** Cells outside the world count as air and are never drawn, as neighbours or as guests, so the world ends in a wall.
- **Full-depth outer walls.** Side faces toward cells outside the chunk (and all of a guest's outward faces) run down to `bottom_layer`. A wall is two triangles whatever its height, and columns are solid beneath, so steps between detail levels cannot crack.
- **Line data.** For each top-face edge, the mesher computes the **border level**: the highest level at which the cells on the two sides have different owners (none, ken, cho, ri, or the world's edge), using the core's integer maths. The mesh carries it, with what the shader needs to measure a pixel's distance from the edge.
- **Winding** follows Bevy's convention (counter-clockwise front faces, after the axis mapping); a test checks that top faces point up.
- **Colours:** a base colour per material with slight per-cell hash shading.

## The Bevy plugin (`hexworld_bevy`)

- **`Loader { rings }` component.** Any entity with a transform and a `Loader` is a loader. One system per frame converts loader positions to shaku (through the axis mapping), calls `store.update(loaders, now)` with Bevy's elapsed time, and acts on the result.
- **Tasks.** Each chunk to load becomes a task on Bevy's async compute pool: generate, then mesh. The number in flight is capped. Finished chunks are inserted into the store; results the store drops are discarded.
- **Entities.** One mesh entity per shown chunk, placed at the chunk's centre, carrying its `ChunkKey`.
- **Handover.** A parent's column is left out of its chunk's mesh while the child chunk for that cell is shown. When a child chunk finishes, the plugin re-meshes the parent without that column (also as a task) and then shows the child and swaps the parent's mesh **in the same frame**; unloading does the reverse. There is never a hole or a doubled surface. The store loads coarsest first, so a parent is always there to hand over from.
- **Line material.** A material extension (a WGSL fragment on Bevy's standard material) draws lines at a constant pixel width from the mesh's line data. Modes:
  - **shaku only** (default): every edge of shaku-detail columns, nothing on coarser detail;
  - **nested:** every detail level's edges, styled by border level as in exp-03 (shaku thin and dark, ken light, cho yellow, ri red and heaviest), so a cho border follows ken hexagon edges far away and shaku edges close up;
  - **off.**

  The shader contains no hex maths; the drawn borders are owned borders by construction.
- **Tint by detail level:** an option of the same material that tints the ground by the detail level drawn there.
- **Config changes.** Changing the seed or the layer thickness clears the store and regenerates.

## The viewer (`viewer`)

- **Camera.** An overhead rig: a focus point on the ground, with the camera up and behind it, looking down. WASD, the arrow keys or middle-drag pan the focus; the wheel zooms from a few metres up to far enough to see the whole ri. Pan speed scales with zoom. The tilt goes from about 45° close in to about 75° far out. The focus follows the ground: its height is the top of the finest column loaded under it. The focus is clamped to the world. No rotation, and no screen-edge panning.
- **The focus entity carries the `Loader`.** A debug key spawns a second `Loader` entity at the mouse position, and another removes it.
- **Panel** (egui, at the side):
  - **Address:** the focus's `ri / cho / ken / shaku / layer`, its position in metres, and the same for the cell under the mouse.
  - **Levels:** a row per detail level: chunks and columns loaded, lingering, loads and unloads in the last second, generation and meshing time, triangles, and the number waiting.
  - **Tuning:** rings per level, the unload delay, the layer thickness, the seed.
  - **Display:** line mode; tint by detail level.
  - FPS and frame time.
- **Headless runs:** `--frames N --screenshot-dir DIR --screenshot-every M`, plus `--seed` and a start position, for checking renders without a person at the screen.
- One directional light plus ambient; a plain sky colour.

## Testing

**Core** (`cargo test`, no GPU):

- **Addresses:** address ↔ shaku ↔ metres round trips; every parent owns exactly 36, 3,600 or 1,296 children; each child has exactly one owner, matching the rule; neighbours across ken, cho and ri borders; unit lengths; agreement with exp-03's fixture cases (`owner`, `address`, `from_address`, `round_at`, including negative coordinates and ties); layer ↔ height for several thicknesses; `world_ri` gives 1 ri at radius 0 and 469 at radius 12.
- **Columns and generation:** runs sorted, non-overlapping and above `bottom_layer`; determinism; a chunk is identical whatever the load order; a coarse sample equals the full sample minus exactly the dropped octaves.
- **Store:** window sizes, and clipping at the world's edge; ancestors always included, also with odd ring settings; the union of two loaders; load order; delayed unloading (a chunk requested again in time is kept and not reloaded; otherwise it expires after the delay; a delay of 0 unloads at once); ancestors of lingering chunks stay.
- **Mesher:** top faces point up; face counts for small hand-made chunks; no duplicate and no missing faces inside a chunk; guests present; outer walls reach `bottom_layer`; omitted columns are absent; every edge's border level matches the ownership maths.

**Plugin:** a headless Bevy app (no window) checks that a `Loader` entity drives loading, that a second one adds its windows, and that the child's appearance and the parent's re-mesh land in the same frame.

**Visual checks** (headless screenshots): the detail levels around the focus with the tint on; a handover close up; both line modes; the panel.

## Success criteria

1. Pan and zoom around one ri: shaku detail under the focus, ken and cho detail beyond it, and **no holes or cracks** at the handovers or while chunks load.
2. The panel's address matches the core. With the queue idle and the focus well inside the ri, the panel counts **1,332 shaku columns in 37 ken chunks**.
3. Parking the camera on a border, or jittering across it, causes **no reloads** within the delay.
4. Spawning a second entity with `Loader` loads its windows **with no other code changes**.
5. Frames stay steady while panning. The number is agreed with the user after the first measurement, not set here.
6. `cargo test` passes.

## Out of scope

- The real terrain generator (erosion and the rest).
- Terrain edits; saving and loading.
- Trees, rocks and other world objects; building parts.
- Vertical sections.
- A world larger than 1 ri (the radius setting exists and is covered by core tests only); a floating origin.
- Pathfinding and AI; joining the exp-00 to exp-02 line.
- Whether the result moves into another engine or into the root project.

## Open questions

Recorded for later experiments; none blocks exp-05.

- **TODO: reconsider sampled coarse columns** (decision 17). Once terrain can be edited, a coarse column sampled from the generator no longer shows what its children hold (a moat, a flattened castle site). Options then: aggregate coarse columns from stored fine data, or keep sampling and store corrections per coarse cell.
- **Guests and neighbour columns after edits.** The mesher takes border neighbours and guests from the generator because generation is pure. With edits they must come from stored data, which may not be loaded.
- **Parts that cross chunk borders.** A wall from one shaku to another can cross ken and cho borders: which chunk stores it, and which chunks load it?
- **Full-detail chunk size.** A ken chunk is 36 columns and a cho chunk 129,600 shaku; the hierarchy has nothing between. If ken chunks prove too small to be efficient, full detail may need a grouping of its own.
- **Window sizes for an overhead camera.** The default 3 rings give full detail only about 6 m around the focus. Rings that follow the zoom were discussed and deferred.
- **Vertical sections** (decision 12) for caves, deep mines and tall structures.
- **Triangle budget.** Default rings give roughly 2 to 3 million triangles at ken detail. The plan measures this early.

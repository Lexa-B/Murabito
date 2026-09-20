# Experiments manifest

One entry per experiment: what it is, why it exists, how to run it.

---

## exp-00 — StateTree hello world

- **Started:** 2026-09-17
- **Stack:** Python 3.13 + pygame, managed with uv
- **Run:** `cd Experiments/exp-00 && uv run src/main.py`
- **Test:** `cd Experiments/exp-00 && uv run pytest`
- **Design:** [`exp-00/docs/specs/2026-09-17-exp-00-statetree-visualizer-design.md`](exp-00/docs/specs/2026-09-17-exp-00-statetree-visualizer-design.md)

### Why

A quick mockup, built outside Unreal Engine because UE5 is a very large download.
It's a hello world of pathing and object use that shows what the AI systems will
be like once there's UE to write AI for.

### What

An isometric pygame window showing a hexagon-shaped hex-tile world (25 tiles
across, with a camera that follows the actor) with three placeholder
objects (A, B, C) and a single actor. The objects are modeled on UE5's **Smart
Objects**: Sims-style interactables that advertise interactions and offer slots
the actor claims, walks to, uses, and releases. The actor's behavior is driven by a
small **StateTree-style** engine modeled on UE5's StateTree: hierarchical states,
enter conditions, tasks, transitions, and evaluators writing to a shared context.
The actor paths to object slots with **A\*** on the hex grid.

Which object the actor goes to next depends on **where it is** (West or East zone)
and **which object it used last**. These rules are placeholders, kept in one data
file (`src/ai/tree_def.py`) so they're easy to change.

A side panel shows what's going on in the actor's "brain" while the sim runs: the
state tree with the active branch highlighted and pass/fail marks on each enter
condition, the current context values, and a running transition log.

### Layout

- `src/` — game loop, hex grid, world model, Smart Objects, camera, rendering, starting layout (`layout.py`), sim wiring (`sim.py`)
- `src/ai/` — StateTree engine, tasks/evaluators, the example tree, A* pathing
- `tests/` — pytest for the hex grid, world zones, pathing, Smart Objects, the StateTree engine, the tasks, the starting layout, the tree definition, the sim cycle, and the headless main loop

### Controls

Space pause/resume · N step one tick while paused · +/- sim speed (0.25x-8x) · R reset · Esc quit.

### Headless

`cd Experiments/exp-00 && SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy uv run src/main.py --frames N --speed S --screenshot-dir screenshots --screenshot-every K` (screenshots/ is gitignored).

---

## exp-01 — Wandering Smart Objects

- **Started:** 2026-09-17
- **Stack:** Python 3.13 + pygame, managed with uv
- **Run:** `cd Experiments/exp-01 && uv run src/main.py`
- **Test:** `cd Experiments/exp-01 && uv run pytest`
- **Design:** [`exp-01/docs/specs/2026-09-17-exp-01-wandering-objects-design.md`](exp-01/docs/specs/2026-09-17-exp-01-wandering-objects-design.md)

### Why

Built on exp-00. It's almost the same, but the targets slowly wander around.

### What

Starts as a copy of exp-00: same hex map, camera, StateTree engine, Smart Objects, A\* pathing and brain panel.

**What changes:**
- **Three zones.** The map is split into three 120° sectors (NE, S, NW).
- **Wandering objects.** Each Smart Object wanders inside its own zone, with stochastic momentum. It usually keeps its heading, and sharper turns are less likely. Slots ride along with their object and can poke across a zone border.
- **Chasing.** The actor re-plans when its claimed slot moves or its path gets blocked.
- **Pausing.** A and B hold still while in use. C keeps moving, and the interaction fails if its slot drifts away.
- **Rules.** The placeholder rules still key on the last object used and the actor's zone. Finishing in that object's home zone moves forward (A→B→C), and finishing across a border moves backward.

### Layout

- `src/`: same as exp-00, plus `wander.py`, the stochastic object mover.
- `src/ai/`: same as exp-00. `tasks.py` gains chasing, re-planning and in-use tracking.
- `tests/`: same as exp-00, plus `test_wander.py`. The sim tests check invariants over seeded runs instead of a fixed cycle: claims, in-use, zones, overlaps and step order.

### Open questions

Three design and tuning questions from the final review are listed in the spec's "Open questions" section: the slot marker during a mid-step pause, claims abandoned on brief blocks and C's drift-failure rate, and wall placement.

### Controls

Same as exp-00: Space pause/resume · N step one tick while paused · +/- sim speed (0.25x-8x) · R reset · Esc quit.

### Headless

`cd Experiments/exp-01 && SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy uv run src/main.py --frames N --speed S --screenshot-dir screenshots --screenshot-every K` (screenshots/ is gitignored).

---

## exp-02 — Fog of war and the believed world

- **Started:** 2026-09-18
- **Stack:** Python 3.13 + pygame, managed with uv
- **Run:** `cd Experiments/exp-02 && uv run src/main.py`
- **Test:** `cd Experiments/exp-02 && uv run pytest`
- **Design:** [`exp-02/docs/specs/2026-09-18-exp-02-fog-of-war-design.md`](exp-02/docs/specs/2026-09-18-exp-02-fog-of-war-design.md)

### Why

In exp-00 and exp-01 the actor is omniscient: it finds and chases objects using their true positions. exp-02 makes the actor earn its knowledge, to show that partial observability slots in underneath the decision layer: the rule table stays the same, and only what the tree reads changes.

### What

Starts as a copy of exp-01: same hex map, three zones, wandering Smart Objects, StateTree engine, A\* and brain panel.

**What changes:**
- **Random maps.** Walls, object starts and the actor's start are random, seeded from `--seed`. Nothing is in view at the start.
- **Sight.** The actor sees a 120° cone out to 6 tiles, plus its neighboring tiles. Walls, and objects set to cast shadows, block sight.
- **Beliefs.** The actor keeps a private belief store. It learns walls on sight and paths on a believed map. It dead-reckons objects it has lost sight of, with an uncertainty radius that grows with each object's observed speed and sets how sure it is.
- **Behaviour.** The actor chooses targets from beliefs, claims a slot only once the object is in view, searches when a belief turns out wrong, and explores when it knows nothing.
- **Rendering.** Fog of war, ghost markers for believed positions, a sense cone, and a truth overlay (`T`). The brain panel gains a belief table.

### Layout

- `src/`: same as exp-01, plus `zones.py` (zone geometry), `mapgen.py` (random walls and starts), `body.py` (the truth-side actuator).
- `src/ai/`: same as exp-01, plus `vision.py` (sense cone and shadowcasting), `beliefs.py` (belief store, believed map, search area, frontier), `perception.py` (the only AI code that reads the world). `tasks.py` gains `ChooseTarget`, `Search` and `Explore` and reads beliefs.
- `tests/`: same as exp-01 minus `test_layout.py`, plus `test_vision.py`, `test_mapgen.py`, `test_beliefs.py`, `test_believed_map.py`, `test_perception.py`, `test_body.py`, `test_boundary.py`.

### Open questions

Tuning and polish left open after the final review (2026-09-18); none is a bug.

- **Explore churn.** `EXPLORE_MIN_DISTANCE = 3` often picks a frontier tile the actor sees within a fraction of a second, so Explore re-enters and logs a lot in the opening: about 35 `explore ->` lines per 120 s, against about 20 at a value of 9, with no measured change in how fast objects are found.
- **Believed-map cost.** `BeliefStore.is_walkable` recomputes `object_blocked()` on every call, so a failed A\* costs about 10 ms. Fine at 1x; a per-tick cache would remove possible hitches at 8x.
- **Stale target in the panel.** After Search gives up, `Target` keeps the abandoned name while Explore runs, and the CONTEXT panel shows it. Display only: no condition reads it there.

### Controls

Space pause/resume · N step one tick while paused · +/- sim speed (0.25x-8x) · R reset (same seed) · Shift+R reset with a new seed · T truth overlay · Esc quit.

### Headless

`cd Experiments/exp-02 && SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy uv run src/main.py --frames N --speed S --seed K --truth --screenshot-dir screenshots --screenshot-every M` (screenshots/ is gitignored; `--truth` is optional).

---

## exp-03 — Tiered hex world

- **Started:** 2026-09-18
- **Stack:** Python 3.13 + pygame + moderngl (OpenGL 3.3) + numpy, managed with uv
- **Run:** `cd Experiments/exp-03 && uv run src/main.py`
- **Test:** `cd Experiments/exp-03 && uv run pytest`
- **Design:** [`exp-03/docs/specs/2026-09-18-exp-03-tiered-hex-world-design.md`](exp-03/docs/specs/2026-09-18-exp-03-tiered-hex-world-design.md)

### Why

Groundwork for tiered pathfinding: fine A\* only close in, coarser regions further out. exp-03 builds the hierarchical coordinate system at historical Japanese scale, and a procedural world that loads in tiers of detail around whoever needs it. There is no pathfinding yet. exp-05 will merge this into the exp-00 to exp-02 line.

### What

A new line. From exp-02 it keeps only the scaffolding: the uv project, the shape of the main loop, and the axial hex maths.

- **Units.** shaku (10/33 m, flat to flat), ken = 6 shaku, cho = 60 ken, ri = 36 cho. Every level is a hex grid with the same orientation. A parent owns the children nearest its centre, and split children go to exactly one owner, so every ken owns 36 shaku, every cho 3,600 ken and every ri 1,296 cho. Every position has a `ri / cho / ken / shaku` address.
- **World.** A hexagon of radius 12 ri: about 98 km across and 6,270 km², the size of a typical prefecture. Gentle country with mountains around the rim; grass, dirt, bare rock and snow; woods and clearings; trees, rocks, grass tufts and pebbles.
- **Tiers.** Any loader (for now only the camera) gets shaku detail in its ken plus 3 rings, ken detail in its cho plus 3 rings, cho detail in its ri plus 3 rings, and ri detail across the whole world. A queue loads the chunks nearest first, within a time budget per frame. Each finer tier blends into the coarser one before its window edge, so tiers meet without cracks.
- **View.** A 3D camera on a rail: a circle of radius 4 ri round the world centre at 2.5 m/s, about 11 h per lap. Every tier's cell borders are drawn on the ground. A side panel shows the camera's address, per-tier stats and the load windows.

### Layout

- `src/`: `hexgrid.py` (kept), `hexaddr.py` (units, ownership, addresses), `noise.py`, `terrain.py` (height, ground type, forests), `chunkgeom.py` (chunk templates and meshes), `chunkgen.py` (chunk contents and props), `loading.py` (loaders, windows, the load queue), `rail.py`, `main.py`.
- `src/gfx/`: the moderngl context, the renderer, the GLSL shaders, prop meshes and the panel.
- `tests/`: one file per module. `test_gfx.py` checks the shader's hex maths against `hexaddr` on real renders; it skips itself when no GL context is available.

### Controls

Space pause/resume · Esc quit. The camera cannot be steered.

### Headless

`cd Experiments/exp-03 && SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy uv run src/main.py --frames N --seed K --start S --screenshot-dir screenshots --screenshot-every M`. `--no-preload` shows the world loading in; `screenshots/` is gitignored. Headless runs need a GPU driver with EGL. `test_gfx.py` and `test_main.py`'s headless run skip themselves without a GL context; set `MURABITO_REQUIRE_GL=1` to make that a failure instead (for a machine, like the development one, where a skip would hide a real regression).

### Open questions

Raised in the final whole-branch review; the fixes are in, but these are spec questions for the user to decide.

- **Load hysteresis.** With no hysteresis, the rail's default start makes the ken window toggle along a cho edge as well as the shaku window along a ken edge (see the spec's "Amendments from review"): the loader's cho flips between two candidates every so often, reloading 7 ken chunks each way. The user saw the flapping too. Deferred on 2026-09-19, to revisit once the project is in UE5. Two ways to fix it, both small (one change to `loading.py` plus tests):
  - **Sticky window centre** (fixes the flapping itself). A loader keeps its cell at each level until the focus is clearly inside the new one, e.g. nearer the new cell's centre than the old one's by about 0.1 of a cell width (about 18 cm for the ken the shaku window follows, about 11 m for the cho). Nothing unloads while the focus zig-zags along a border, and it needs no memory. The windows lag the focus by at most the margin, which must stay under about 0.26 of a cell width so the tier blend (ending at 2.0 parent widths) still finishes inside the nearest window edge (2.26).
  - **Cache of recently unloaded chunks** (makes the flapping cheap). A small least-recently-used cache keeps unloaded chunks, so a chunk requested again reloads in about 1 ms of GPU upload instead of about 10 ms of generation for a ken chunk. Chunks still unload and reload on every flap, and the cache costs a few MB per cached ken chunk.

  The sticky centre targets the flapping directly; the cache helps any pattern that revisits areas. They combine.
- **What "60 fps" means.** After the rail fix, a 400-frame run from `--start 1200` s (which includes one cho crossing) measured: total frame time median 3.08 ms, max 14.96 ms, 0 of 400 frames over 16.7 ms, including the 40 frames right after the crossing (max there 15.0 ms). The spec's success criterion ("60 fps ... no frame over 33 ms at a cho crossing") reads as a worst-case bound, which this now clears comfortably; is the criterion meant as an average, or as a bound on every frame?
- **Distant ri borders** still alias into faint red blotches near the horizon (placeholder line tuning, not addressed in this wave).

---

## exp-04 — UE5 hex terrain and camera

- **Started:** 2026-09-18
- **Stack:** Unreal Engine 5.8.2, C++ (Linux). Set `UE_ROOT` to use another engine install.
- **Build:** `Experiments/exp-04/scripts/build.sh`
- **Run:** `Experiments/exp-04/scripts/game.sh [-Seed=N]`, or `Experiments/exp-04/scripts/editor.sh` and press Play
- **Test:** `Experiments/exp-04/scripts/test.sh [TestPathPrefix]` (headless UE automation tests)
- **Design:** [`exp-04/docs/specs/2026-09-18-exp-04-ue5-hex-terrain-design.md`](exp-04/docs/specs/2026-09-18-exp-04-ue5-hex-terrain-design.md)
- Works side by side with an open editor. With exp-04 open, `build.sh` does a hot-reload build that the editor loads by itself (after Play stops, if it's running). Restart the editor after header or class-layout changes. The scripts never stop an editor.
- `test.sh` builds and tests a private copy of the project (`~/.cache/murabito/exp-04-test-mirror`, set `MURABITO_TEST_MIRROR` to move it), so it runs fine with the editor open.
- `test.sh` needs `rg` (ripgrep) on PATH to parse the automation log.

### Why

A separate line from exp-00–03. Instead of mocking AI in pygame, this gets a basic Unreal Engine 5 world running.

### What

A C++ UE5 project with no binary assets. At startup the game mode builds everything into the engine's empty `/Engine/Maps/Entry` map:

- **Terrain.** Hilly ground made from seeded layered noise (`-Seed=N`).
- **Hex grid.** A pointy-top hex grid, radius 12, defined on a flat 2D plane and draped onto the terrain as thin lines. A 2D hex coordinate stands for a spot in the 3D world.
- **Camera.** An overhead camera that pans (WASD/arrows, screen edges, middle-drag) and zooms (wheel). It tilts from about 75° down far out to about 45° close in.
- **Hover.** An outline on the tile under the mouse.

The repo holds only what's needed to build and run: no engine code, and no `.uasset`/`.umap` files. Don't save the Entry map from the editor.

### Layout

- `MuraBito.uproject`, `Config/`: project file and settings (default map, game mode, Enhanced Input)
- `Source/MuraBito/`: `HexGrid`, `TerrainHeight`, `MeshBuilders`, `CameraMath` (pure math); `Terrain`, `HexOverlay`, `CameraRig`, `InputController`, `WorldBuilder` (actors); `Materials` (runtime materials)
- `Source/MuraBito/Private/Tests/`: automation tests for the hex grid, height function, mesh builders and camera math
- `scripts/`: build, editor, game and test wrappers around the local engine install

### Controls

WASD/arrows or screen edges pan · middle-drag pans · wheel zooms · mouse hover highlights a tile.

---

## exp-05 — Hex coordinates, addresses and chunks (Rust + Bevy)

- **Started:** 2026-09-19 (handoff), designed 2026-09-20
- **Stack:** Rust 1.95, Bevy 0.19, bevy_egui; a Cargo workspace
- **Run:** `cd Experiments/exp-05 && cargo run -p viewer --release`
- **Test:** `cd Experiments/exp-05 && cargo test`
- **Design:** [`exp-05/docs/specs/2026-09-20-exp-05-hex-voxel-chunks-design.md`](exp-05/docs/specs/2026-09-20-exp-05-hex-voxel-chunks-design.md). The earlier [`exp-05/docs/HANDOFF.md`](exp-05/docs/HANDOFF.md) has the background.

### Why

The user wants one uniform hex coordinate system instead of terrain, hex grids and everything else being separate elements: ri, cho and ken as nested scales of chunks that can be hot-loaded, with the shaku as the primary unit. Terrain lives in that system, and the drawn hexes reflect it. The coordinates are meant for more than terrain: later, world objects, and buildings made of parts that snap to shaku.

First planned in Unreal Engine C++; the user chose a fresh start outside UE, and then Rust and Bevy for this experiment.

### What

Coordinates, the address system and chunks, with placeholder terrain.

- **Coordinates.** exp-03's system unchanged (the ratios, packing B, one owner per split cell), plus a vertical **layer**, 5 sun thick by default and tunable. Addresses are `ri / cho / ken / shaku / layer`. The world is 1 ri for now; the radius is a setting.
- **Voxels.** Terrain is hex prisms ("pancakes") one shaku across. A chunk is one parent cell's children and holds a column per child: a list of runs of material over the full height. Coarser chunks hold wider columns measured in the same layers.
- **Loading.** Anything can be a loader: a focus and a number of rings per level (default 3). Ancestors are always loaded, and unloading is delayed (default 5 s), so sitting on a border reloads nothing.
- **Structure.** `hexworld` (engine-free core: addresses, chunks, the store, the mesher), `hexworld_bevy` (a `Loader` component, background generation, chunk meshes, the hex-line material), and `viewer` (a RimWorld-style overhead camera and a tuning panel).
- **Hex lines.** The borders of loaded shaku by default, or every level nested as in exp-03; toggleable.

### Controls

WASD/arrows or middle-drag pan · wheel zooms · the panel's Display section picks the line mode (off / shaku only / nested) and toggles tint by level · the panel's Loaders section adds or removes a loader at the hovered cell. No keyboard shortcuts for line mode, tint or loaders — only the panel.

### Headless

`cd Experiments/exp-05 && cargo run -p viewer --release -- --frames N --screenshot-dir DIR --screenshot-every M --seed N --start-east M --start-north M --rings-shaku N`, plus `--auto-pan` (sweep the focus for a sustained-panning measurement), `--zoom M`, `--line-mode off|shaku|nested` and `--tint` (start in a chosen camera/display state — there is no keyboard or CLI-free way to reach nested lines or tint headlessly, since those are panel-only). `screenshots/` is gitignored.

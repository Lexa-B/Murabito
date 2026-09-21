# exp-05 handoff

> **Update 2026-09-20:** the design is written: [`specs/2026-09-20-exp-05-hex-voxel-chunks-design.md`](specs/2026-09-20-exp-05-hex-voxel-chunks-design.md). In brainstorming the user moved exp-05 out of Unreal Engine and C++, to Rust and Bevy. Where this handoff and the spec differ, the spec is right. The Unreal notes below still apply to the root project and exp-04.

Written 2026-09-19, at the end of the session that set up the root Murabito project (camera and terrain). This file holds the context a fresh session needs that **isn't** already in `AGENTS.md`, `Experiments/manifest.md`, or the experiment specs. Read those first.

## What exp-05 is

What the user said, in their words (2026-09-19):

> "i actually decided i dont like how the terrain, hex grids, and anything else are distict elements... i actually want a uniform hex coordinate system whith each ri/cho/ken being different nested scales of chunks that we can hot load and shaku beiing the primary unit... i want terrain to live in that system and for the drawn hexes to reflect that."

> "lets make this the new exp-05. lets work in c++ for it too"

So, as stated:

- **One uniform hex coordinate system.** ri, cho and ken are nested scales of **chunks** that can be **hot-loaded**; the **shaku is the primary unit**.
- **Terrain lives in that system**, not as a separate element.
- **The drawn hexes reflect that system.**
- **It's an experiment, in C++** (Unreal Engine 5.8).

Nothing else is decided. Start with brainstorming and ask. Where this handoff lists options or facts, they are inputs to that conversation, not decisions.

**Superseded:** exp-03's spec (decision 3, and "Out of scope") says "exp-05 merges this into the exp-00 to exp-02 line". The user has since defined exp-05 as above. Whether the AI line (exp-00 to exp-02) joins later hasn't been said.

## Read first

1. **`AGENTS.md`**: repo rules, the Git workflow (PR-only `main`; wait for the user's approval before pushing or opening a PR), the Unreal rules.
2. **exp-03**, the design this builds on:
   - spec `Experiments/exp-03/docs/specs/2026-09-18-exp-03-tiered-hex-world-design.md`: the units, packing B, ownership (one owner per split cell, battlement borders), addresses `ri / cho / ken / shaku`, tier windows (own cell + 3 rings), the time-budgeted load queue, geomorphing between tiers, and hex lines drawn in the fragment shader from the same integer ownership maths. Read its two amendment sections too: the last one records a **limitation for exp-05** (rendering and geomorphing are centred on the single camera loader).
   - code `Experiments/exp-03/src/hexaddr.py` (pure, tested address maths; the obvious thing to port first and test the C++ against), `loading.py`, `chunkgeom.py`, `chunkgen.py`, `terrain.py`, `gfx/shaders/`.
   - manifest entry: its open questions (load hysteresis, what "60 fps" means, distant ri borders).
3. **exp-04** (`Experiments/exp-04/`): the first UE experiment. Procedural terrain and a draped hex overlay built with `UProceduralMeshComponent`, pure-math classes kept apart from actors and covered by automation tests, and the triangle-winding convention (up-facing means `CrossProduct(B - A, C - A).Z < 0`). Its spec and plan are good worked examples.
4. **The root project** (repo root): the current C++ patterns and tooling.
   - `ACameraRig` / `AInputController` / `CameraMath`: the overhead camera with ground following (a downward trace), and input assets wired through Blueprint children.
   - `Tools/`: `build.sh`, `test.sh` (a private test mirror, a completion-count check), `editor.sh` / `game.sh` (desktop display, X11 by default).
   - `Tools/make_heightmap.py`: the terrain generator with the erosion filter (see below).

## Facts from this session that cost time or are easy to miss

### Unreal Engine on this machine

- **Never kill, signal or otherwise touch a process you didn't start**, above all the user's Unreal Editor. The user works side by side with the editor open. (An implementer subagent once SIGTERMed it; that's why this rule exists.)
- **Hot reload:** with an editor open, `build.sh` writes numbered modules (`-0001.so`, ...) that the open editor loads itself (after Play stops). **But `UnrealEditor.modules` isn't always updated.** A restarted editor once loaded a stale module and "couldn't find" new classes. The fix: close the editor, run a normal build (no editor running), and reopen. New classes and header changes need an editor restart anyway.
- **Tests run on a private mirror** (`~/.cache/murabito/<name>-test-mirror`, built with `-NoHotReloadFromIDE`), so they never touch the editor's modules. The editor's exit code is always 1 under `-TestExit`, so pass/fail is read from the log, and the "N tests performed" count must equal the passed count.
- **Display:** the user's WezTerm tabs have no `DISPLAY`/`WAYLAND_DISPLAY` (the mux server is a systemd user service). The scripts take them from `systemctl --user show-environment`.
- **Wayland vs X11:** on SDL's Wayland backend, editor pop-ups (the Pick Parent Class tree) stop taking clicks. The root project's scripts default to `SDL_VIDEODRIVER=x11`, and so do the user's desktop shortcuts. exp-04's scripts don't.
- **Engine plugin `AndroidFileServer`** rewrites `Config/DefaultEngine.ini` with a fresh token on every run on Linux. Disable it in the `.uproject`.
- **Material node names in 5.8:** "World Position" (its setting is Absolute World Position), "ComponentMask" (which shows as "Mask (R)" and so on once a channel is ticked). Check names in the engine source (`Engine/Source/Runtime/Engine/Public/Materials/MaterialExpression*.h`) before giving the user click-by-click steps.
- **Landscape import can shift the Location** the user typed. After an import, check the actor's Location in Details.

### Coordinates and scale

- exp-03 uses metres with x east, z north and y up. UE uses centimetres with X forward, Y right and Z up, and is left-handed. Choose the mapping deliberately, or the world comes out mirrored.
- One ri is 129600/33 m (about 3,927.27 m) flat to flat. exp-03's world is radius 12 ri, about 98 km across. UE5 uses double-precision world positions (Large World Coordinates), but nothing here has tested precision at that range yet.
- exp-03 hexes are pointy-top, with the same orientation at every level. The root project's current terrain hexagon is pointy-top, flat sides facing +X/−X, centred on the origin.

### Terrain and the erosion filter

- `Tools/make_heightmap.py` implements an erosion filter from runevision's video "Fast & Gorgeous Erosion Filter Explained" (after Clay John's and Fewes' Shadertoys): downhill-aligned stripes with a pivot per cell, blended across neighbours and normalized; octaves that follow earlier slopes; fading toward an altitude-based target on flat ground.
- **Licensing:** runevision's code is MPL and the Shadertoys default to a non-commercial licence, so ours was written from the video's description only. Keep it that way in the MIT repo.
- **Not chunk-ready as written:** it takes slopes from neighbouring pixels (`np.gradient`) on one finished grid, and each octave uses the whole grid's slopes. The technique itself is point-evaluable (the video's main selling point) if the height function has analytic derivatives. That is what chunked generation would need. The video's Part II covers keeping derivatives consistent (the amplitude and frequency rules).
- Tuning lessons: a smaller octave must be gentler (depth scaled by a gain per octave), or slopes pile up into "crumpled paper". Pivots near cell centres plus a matching blend radius avoid seams. An ease-in on flat ground avoids starbursts at peaks and saddles.

### What the root project has right now (as of `main` at 7d47f4b)

- `Content/Maps/Testing.umap`: a UE Landscape imported once from `make_heightmap.py` (seed 1), 2017×2017 at 2.26 m, cut to a 1-ri hexagon by `M_Terrain`'s opacity mask. Collision remains outside the hexagon. The user now wants terrain inside the hex system instead, so this is likely to be replaced; how and when is for the user to decide.
- Git LFS is set up (not lockable); assets are CC BY-SA 4.0 and code is MIT (`LICENSING.md`).

## Questions to ask, not assume

- **Where exp-05 lives and how it starts:** its own UE project in `Experiments/exp-05/` (a copy of exp-04, of the root project's skeleton, or fresh)? Its own module name?
- **"Hot load" concretely:** exp-03's tier windows (own cell + 3 rings per level, time-budgeted queue)? Streaming around which loaders (the camera only, or actors later)?
- **What a chunk is at each level**, and what the shaku being "the primary unit" means for data: per-shaku data stored, or computed on demand?
- **Terrain in the system:** height per shaku from a point-evaluable function (so chunks agree at their edges)? Is the erosion filter wanted from the start?
- **Drawn hexes:** exp-03's per-tier border lines in a shader (owned, battlement borders), geometry like exp-04, or something else? Which levels, when?
- **Camera:** the root project's overhead rig, exp-03's rail, or something else?
- **World size:** exp-03's radius 12 ri, or smaller to start?
- **Rendering:** `UProceduralMeshComponent` (unknown performance at exp-03's cell counts; exp-03 measured ~180k cells loaded), dynamic mesh components, instancing, or something else? A spike might be worth it.
- **Does the result later move into the root project**, and does the AI line (exp-00 to exp-02) join?
- **Workflow:** full spec, plan and subagent-driven development as in earlier experiments, or lighter? (The root project uses no spec/plan files; experiments may. Ask.)

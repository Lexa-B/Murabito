# exp-06 handoff: worldgen

Written 2026-09-29 at the end of the first design session, for a fresh context to pick up mid-conversation. Everything here was agreed in chat; there is no spec yet. Read this, open the viz page, and continue at "Where we stopped".

## What exp-06 is

The worldgen for Murabito, built as a test world for exp-07 (the GPU hex-voxel raycaster) and as the framework the full world grows from. Towns, flora, fiefdoms and the like are later layers on top, not part of this. Lexa's words: "1 + 2-lite: set up the framework we can play off of for the full Murabito world eventually."

It is a design walkthrough first: every step is thought through and drawn as an interactive page before any code is written. The page is `Experiments/exp-06/viz/banana.html` (standalone HTML, opens from disk; also published as a private artifact at https://claude.ai/artifact/FmofqfQz8ZuP3LTM6UVpcJ). Code, when it comes, is a Rust + Bevy experiment in its own Cargo workspace here; the core crate must be engine-free (AGENTS.md).

## Where the work lives

- **This half:** worktree `.claude/worktrees/exp-06-worldgen`, branch `exp-06-worldgen`, cut from `origin/main` at `c3781c3`. Commits so far are all to `Experiments/exp-06/viz/banana.html` (last: `ac8e905`).
- **The other half:** the raycasting conversation, exp-07, works in `.claude/worktrees/exp-06-voxel-raycast` on branch `exp-06-voxel-raycast`. Its files (`goals.md` for raycasting, `viz/hex-ray-walk.html`, `viz/kanto-scale.html`) still sit under `Experiments/exp-06/` there and are to be moved to `exp-07/` by that session. Never commit into that tree from here, and expect a rename on its side.
- Memory notes (Claude's auto-memory) that matter: `exp-06-decisions` (decisions shared by both halves), `steps-are-actions` (process rule from this session).

## Decisions carried over from the shared conversation

- **Everything in cube coordinates `(q, r, s)`**, main-branch directionality: `+q` east, anticlockwise positive, pointy-top, `Direction::ALL` order (`E, NNE, NNW, W, SSW, SSE` are the edge directions; `Direction::N` is the corner neighbour `(1, −2, 1)`).
- **The unit ladder:** `shaku ×6 → ken ×6 → decichō ×10 → chō ×36 → ri ×36 → world`, every rung packing B (exp-03's `owner()`: parent `(a, b)` centred on child `(N·a, N·b)`, nearest centre by `d² = dq² + dq·dr + dr²`, ties to the greatest `(a, b)`). The decichō is invented, chosen so cells grow as they recede. Packing B holds for any `N`; corners are three-way only when `3 | N`.
- **The world is a packing cell of `N = 36` ri:** 1,296 ri, 36 ri = 141 km flat to flat, about 17,300 km² (the Kantō plain), **centred on the origin, which the Kantō scale check placed at 日本橋 Nihonbashi (the page calls this point "the origin", never by the place name: Lexa, 2026-09-29)** (35.6841 N, 139.7744 E). In shaku the ladder is 1 · 6 · 36 · 360 · 12,960 · 466,560. The world is the top cell.
- **A ri is 3.927 km** (12,960 shaku of 10/33 m). A layer is 5 sun = 0.5 shaku ≈ 0.1515 m, so about 6.6 layers per metre.
- Columns are run lists at every rung; the raycaster's detail rule (level-`k` detail across the camera's level-`k+2` cell and its six neighbours) is exp-07's business, but the worldgen must be a pure function of (seed, position) evaluable at any rung, so any rung can be produced on demand.

## The napkin plan, as it now stands

Lexa's original steps, renumbered after climate was added and the shoreline step was dissolved:

1. **Place the world on the banana.** Done.
2. **Tectonics: the height surface, above and below the sea.** Done.
3. **Climate per ri.** Done.
4. **Structure: fractal noise on the step-2 surface.** Done.
5. **Erosion: the gully filter on the step-4 surface.** Next, see below. Then the rest of the erosion approach from *Fast & Gorgeous Erosion Filter Explained* (Rune Skovbo Johansen's refinement of Clay John's / Fewes' Shadertoy technique; transcript at `~/Downloads/Fast & Gorgeous Erosion Filter Explained.txt`), set up so only the ri scale is computed up front and finer rungs are evaluated as needed.

Two ideas were tried for a shoreline and rejected; don't reintroduce them: a line displaced/warped by noise (loops and impossible geometry), and composed SDF primitives (bays/peninsulas placed by hand: "hacky"). **The coast is the zero set of the height surface.** It is never generated as a curve.

## Step 1: the banana (built)

- A slot bent along a circular arc, sized like Honshu at its Kantō width: arc `A = 80°`, centreline radius `R = 237` ri (≈ 1,300 km along the axis), width `w = 60` ri (≈ 236 km). Fixed, not dialled.
- Turned by `φ = −A/2` so the top tip of the centreline heads due north; the outer apex then faces ESE (the Pacific side).
- The outline is walked anticlockwise from the outer apex; `θ ∈ [0, 360)` is the fraction of its length walked, in degrees (≈ 8.3 ri per degree). The world (36 ri, north-up, unrotated) is centred on the outline point at `θ` moved inland by the inset `d` (ri, positive inland). Defaults `θ = 12`, `d = 6`.
- Each world edge reads sea, land or shore (crossed). Napkin rule "no more than four of either" is checked live; it bounds `|d|` to about 15 ri.
- The inset is to be tuned after step 4 so the world is about **70% land** (69.5% before any filter).

## Step 2: the height surface (built)

`h(p) = H(s) · F(a)` in metres, positive on land, negative at sea, zero exactly on the outline.

- `bandCoords(p)`: unrotate by `−φ`; on the arc, `s = (ang + A/2) / A` along the centreline (0 bottom tip, 1 top tip) and `a = (r − R) / (w/2)` across (coasts at `±1`); beyond a tip, polar about the cap centre with `e` along and `a` across.
- **`F(a)`** is one monotone cubic (Fritsch–Carlson) through a table, in units of the spine height, `a` from −3 (basin) to +3 (trench):
  `(−3, −1.0) (−2.2, −0.9) (−1.5, −0.35) (−1.15, −0.05) (−1, 0) (−0.6, 0.35) (−0.1, 1.0) (0.2, 0.45) (0.5, 0.05) (1, 0) (1.3, −0.03) (1.7, −0.1) (2.3, −0.8) (3, −2.4)`.
  Outer side: a wide plain then a shelf and the trench; inner side: steep to the coast, narrow shelf, deep basin. Slope continuous through the coasts.
- **`H(s) = 900 + 1600 · sin(π s)`** m: spine 900 m at the tips, 2,500 m at the bend.
- **Caps:** `ρ = √(a² + e²)`, `t = (a/ρ + 1)/2`, `h = H · (t·F(ρ) + (1−t)·F(−ρ))`; continuous with the arc where they join (jump under 2 m).
- Probed values at the default placement, along the normal from the placement point: −18 m at 3 ri out, 16 m at the origin (6 ri in), 2,191 m at the spine (30 ri), −486 m 10 ri into the Japan Sea, −2,109 m at the shelf edge (40 ri out), −5,891 m in the trench (60 ri out). Height sampled all round the outline peaks at 0.19 m.
- **Slope just inland of the coast at Kantō: 3.0 m per ri**, so 100 m of height moves the coast about 33 ri there; on a mountain front (500 m per ri) the same 100 m moves it 0.2 ri. This is the lever the filter works through.
- The page draws: the cross-section at the world's `s` (a from −2.5 to 2.5, sea level as a line), the world's 1,296 ri hypsometric (the ri set is `owner(q, r, 36) == (0,0)` over `|q|, |r| ≤ 30`), and the six edge profiles (−1,000 to 3,000 m, zero crossings dotted).

## Step 3: climate per ri (built)

Four numbers per land ri, stylised from Honshu's stations; every term a coefficient at the top of the step-3 block:

- Sea-level annual mean `15.5 °C − 0.036 °C/ri × (northing from the outer apex)`, minus `6.5 °C per 1,000 m`.
- Coldest and warmest month = mean ∓ half-range, `10.5 °C` at the coast widening to `13.0 °C` at 20 ri inland (inlandness = distance from the coast / 20 ri, clamped; distance from `a` via `(1 − |a|) · w/2`).
- Precipitation `2,200 mm` (Japan Sea side) blending to `1,500` (Pacific) by `a`, plus `0.5 mm per m` of height, plus up to `±600 mm` for slopes facing into or away from the winter NW monsoon (weight 0.6) and summer SE rains (0.4); gradient by central differences 0.5 ri apart; floor 400 mm.
- Checks: the origin 3.2 / 14.5 / 25.7 °C, 1,574 mm (Tokyo 5.2 / 15.4 / 26.4, 1,600); inner coast 1.3 / 12.2 / 23.1, 2,345 (Niigata 2.5 / 13.6 / 26.5, 1,800); spine at 2,191 m −13.2 / −0.2 / 12.8, 2,826; northern tip −2.6 / 8.7 / 20.0 (Aomori −1.2 / 10.4 / 23.3). Runs a degree or two cold at the coast, a touch wet on the Japan Sea side.
- Four hex maps on shared ramps (−10 to 30 °C; 600 to 3,400 mm). When the erosion filter arrives, the slope term should read the same analytic gradient it uses.

## Measured facts to design against

Box-counting Natural Earth 1:10m coastlines (public domain; the file was downloaded to a scratchpad and is not in the repo), boxes 2–128 km: Japan 1.15, Honshu 1.13, **Kantō 1.11**, Sanriku 1.18; Britain 1.26, Australia 1.14, Norway 1.35 (literature 1.21–1.25, 1.13, 1.52: the data generalises rough coasts down, so Japan is likely 1.2–1.25 in truth and Kantō about 1.1–1.15). A coast is a level set of a surface, and a surface with Hurst exponent `H` has level sets of dimension `2 − H`, so **the terrain noise wants `H ≈ 0.85–0.9`** for a Kantō-like coast (0.75 would be Britain). The generated coast should be box-counted on the page (a ri up to the world's width) and `H` dialled until the slope reads right.

## Step 4: structure (built)

Seeded fractal noise added to the step-2 height in metres, everywhere, above and below the sea: `h₄ = h₂ + n(p)`.

- **Noise:** Perlin lattice noise (unit gradients from a seeded 32-bit integer hash, quintic fade) scaled by `√2` so one octave runs about −1…1.
- **Octave ladder, separate from the hex rungs:** octave `k` has wavelength `λₖ = 36 / 2ᵏ` ri and amplitude `A₀ · 2^(−kH)`. Each lattice is the coarser one's halved, so a point lands in the same cells whatever rung evaluates it and a finer rung only adds terms. At the ri scale five octaves are evaluated (36, 18, 9, 4.5, 2.25 ri); the page's table lists the ladder to octave 10 (1.27 chō) so the fine amplitudes can be read.
- **Frame:** origin at the world's centre; `u` along the outline's tangent at `θ` (the arc's grain), `w` across it; `u` is divided by the stretch, so features are longer along the arc. The grain is fixed across the world (the tangent turns only ~9° over 36 ri).
- **Lean:** the field's first moment along the inward normal is summed over the world's ri; if it is negative the lattice is turned half a circle about the origin (every node stays a node), so the heavy side is always inland. Lexa's ask, on seeing seed 17 lean seaward.
- **Perlin is zero at every node, and the origin is a node of every octave**, so the noise is exactly 0 at the origin and the height there is step 2's. Flagged to Lexa; accepted as is.
- **Defaults Lexa chose by eye:** seed 1, `A₀ = 250` m, `H = 0.65`, stretch 2×. At those: 663 of 1,296 ri above sea (51%, from 70% in step 2); noise −277…+231 m over the world; 41 m at 2.25 ri, 122 m at a 12-ri wavelength, 8 m ≈ 53 layers at a chō. Level-set dimension before erosion 1.35.
- **Nothing reads the coast here.** Lexa: "coastlines won't be until after erosion". Land share, the box-count against Kantō's 1.1–1.15, and the inset retune to ~70% land all wait for step 5.
- Step 3's climate still reads the step-2 surface; whether it should follow the finished terrain is open.

Drawings: the noise alone over the ri (diverging ramp, ±the sum of the drawn amplitudes, the grain marked); the world recoloured by `h₄` with the step-2 shore dotted; the cross-section along the normal through the origin with step 2 dashed under step 4; the octave table; facts (land ri before/after, height at the origin before/after, noise range, heavy side and the inland/seaward means, the grain's heading and the octave-0 lattice size, the 12-ri amplitude as ri of coast on the plain).

## Where we stopped: step 5, erosion

Step 4 is committed; step 5 is next and nothing is drawn. What is settled: the erosion is the gully filter from the transcript (stripes across the slope at each octave's cell pivots, blended between cells, the gradient fed to the next octave; refinements: fade to a peak/valley target by slope, stacked fading, normalised and straightened gullies), slope-modulated so it bites on the mountains and the shelf edge and leaves the plain alone, on the same halving octave ladder as step 4. Open for step 5: how many octaves at ri scale; the slope power and strength in metres; where it fades below sea level; whether it reads the step-4 gradient analytically or by differences; and the coast readouts that come after it (land share, box-count, inset retune).

## How to work here

- **Steps are discrete actions with results, never recaps.** A step that adds no computation or decision is folded into the step that produced its content or dropped; the next real action takes the number. Process history stays in chat and commits.
- **Draw before coding, one small step at a time; commit only when Lexa says.** Each step gets sliders; the `θ` and `d` sliders are repeated under every step, all bound to one state (`controls-shared` divs, built from `SLIDERS` in the script), so no scrolling to change them. Tables and readouts keep fixed heights so controls never move under a drag.
- **Sizes fixed by Lexa are constants, not sliders** (`R_RI`, `W_RI`, `ARC_DEG`). Tunables Lexa wants to edit are tables (`SECTION`, the climate coefficients).
- **Publishing the page:** the file in `viz/` carries a full HTML skeleton so it opens from disk. The Artifact tool wants the same content without `<!doctype>`, `<html>`, `<head>`, `<meta>`, `<body>` lines; strip them with `sed` into the scratchpad and publish that copy to the existing artifact URL (same file path in the session keeps the URL).
- **Check headlessly before publishing:** `node -e` with `new Function` on each `<script>` block for syntax, and a DOM stub (`getElementById`/`createElement`/`createElementNS`/`querySelectorAll` returning objects with `appendChild`, `replaceChildren`, `append`, `setAttribute`, `addEventListener`, `textContent`) to run the whole render and read the facts back. The whole page renders in about 55 ms.
- Git: `git -C <absolute worktree path>` for everything; never touch the other worktree; `main` is PR-only and the PR waits for Lexa's approval.

## Open questions, in the order they will come up

1. Step 5: the erosion filter (above).
2. Anisotropy: how much to stretch the structure noise along the arc, and whether the grain rotates with `s`.
3. Amplitudes in metres per octave for structure; the erosion strength and slope power; where erosion fades below sea level.
4. The box-count readout on the page, and tuning `H` to Kantō's dimension.
5. Inset tuning to 70% land after step 4.
6. Materials and the vertical: nothing yet says what a column's runs are made of (rock/dirt/grass came from exp-05's placeholder), nor how caves and overhangs arise; the raycasting test world wants both.
7. `goals.md` in Lexa's voice for exp-06 (the one in the other worktree is the raycasting's), and the spec in `docs/specs/YYYY-MM-DD-exp-06-worldgen-design.md` once the walkthrough settles; then the manifest entry.

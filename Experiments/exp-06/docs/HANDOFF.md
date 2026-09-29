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
5. **Erosion: the gully filter on the step-4 surface.** Done, 5a–5d.
6. **Reading the coast.** Done; the inset retune and the optimisation are next, see below. Then the rest of the erosion approach from *Fast & Gorgeous Erosion Filter Explained* (Rune Skovbo Johansen's refinement of Clay John's / Fewes' Shadertoy technique; transcript at `~/Downloads/Fast & Gorgeous Erosion Filter Explained.txt`), set up so only the ri scale is computed up front and finer rungs are evaluated as needed.

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
  `(−3, −1.0) (−2.2, −0.9) (−1.5, −0.35) (−1.15, −0.05) (−1, 0) (−0.6, 0.35) (−0.1, 1.0) (0.2, 0.45) (0.5, 0.12) (0.8, 0.04) (1, 0) (1.3, −0.03) (1.7, −0.1) (2.3, −0.8) (3, −2.4)`.
  Outer side: a plain then a shelf and the trench; inner side: steep to the coast, narrow shelf, deep basin. Slope continuous through the coasts. **The plain was raised on 2026-09-29** (the `(0.5, 0.05)` point became `(0.5, 0.12)` and `(0.8, 0.04)` was added) because the erosion in metres was eating the flat lowland: the coastal slope went from 3.0 to 13.2 m per ri and the origin from 16 to 98 m. Lexa also tried a 1.5× wider banana (90 ri) for the same reason and reverted it: a wider band is a flatter plain in metres per ri, so the same noise moves the coast further.
- **`H(s) = 900 + 1600 · sin(π s)`** m: spine 900 m at the tips, 2,500 m at the bend.
- **Caps:** `ρ = √(a² + e²)`, `t = (a/ρ + 1)/2`, `h = H · (t·F(ρ) + (1−t)·F(−ρ))`; continuous with the arc where they join (jump under 2 m).
- Probed values at the default placement, along the normal from the placement point: −18 m at 3 ri out, 16 m at the origin (6 ri in), 2,191 m at the spine (30 ri), −486 m 10 ri into the Japan Sea, −2,109 m at the shelf edge (40 ri out), −5,891 m in the trench (60 ri out). Height sampled all round the outline peaks at 0.19 m.
- **Slope just inland of the coast at Kantō: 13.2 m per ri** since the plain was raised (3.0 before), so 100 m of height moves the coast about 8 ri there; on a mountain front (500 m per ri) the same 100 m moves it 0.2 ri. This is the lever the filter works through; the page reads it live.
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

- **Noise: gradient noise on a hex lattice** (switched from square-lattice Perlin on 2026-09-29 at Lexa's call, to match the project's geometry and lose the square grid's axis grain). Nodes `(i, j)` at `(i + j/2, −j·√3/2)` cell units, the ri's own axial convention; halving maps `(i, j)` to `(2i, 2j)` so every coarse node stays a node. A point takes the three nodes of its lattice triangle and blends them with the kernel `(¾ − d²)⁴`, normalised; `¾ = (√3/2)²` is derived, not tuned: the nearest node outside a triangle is exactly `√3/2` from an edge's midpoint, so the kernel is zero on every edge for every node but the edge's ends and the blend is continuous. Each node's unit gradient comes from a seeded 32-bit hash. Measured: the raw peak is exactly `1/√3`, so `√3` scales it to −1…1; zero at every node. The same kernel serves the erosion's pivots.
- **Octave ladder, separate from the hex rungs:** octave `k` has wavelength `λₖ = 36 / 2ᵏ` ri and amplitude `A₀ · 2^(−kH)`. Each lattice is the coarser one's halved, so a point lands in the same cells whatever rung evaluates it and a finer rung only adds terms. At the ri scale five octaves are evaluated (36, 18, 9, 4.5, 2.25 ri); the page's table lists the ladder to octave 10 (1.27 chō) so the fine amplitudes can be read.
- **Frame:** origin at the world's centre; `u` along the outline's tangent at `θ` (the arc's grain), `w` across it; `u` is divided by the stretch, so features are longer along the arc. The grain is fixed across the world (the tangent turns only ~9° over 36 ri).
- **Lean:** the field's first moment along the inward normal is summed over the world's ri; if it is negative the lattice is turned half a circle about the origin (every node stays a node), so the heavy side is always inland. Lexa's ask, on seeing seed 17 lean seaward.
- **The noise is zero at every node, and the origin is a node of every octave**, so the noise is exactly 0 at the origin and the height there is step 2's. Flagged to Lexa; accepted as is.
- **Defaults Lexa chose by eye:** seed 1, `A₀ = 250` m, `H = 0.65`, stretch 2×. With the hex lattice and the raised plain: 771 of 1,296 ri above sea after step 4 (59%, from 70% in step 2); 122 m at a 12-ri wavelength, which is 9 ri of coast on the 13.2 m/ri plain; 8 m ≈ 53 layers at a chō. Level-set dimension before erosion 1.35.
- **Nothing reads the coast here.** Lexa: "coastlines won't be until after erosion". Land share, the box-count against Kantō's 1.1–1.15, and the inset retune to ~70% land all wait for step 5.
- Step 3's climate still reads the step-2 surface; whether it should follow the finished terrain is open.

Drawings: the noise alone over the ri (diverging ramp, ±the sum of the drawn amplitudes, the grain marked); the world recoloured by `h₄` with the step-2 shore dotted; the cross-section along the normal through the origin with step 2 dashed under step 4; the octave table; facts (land ri before/after, height at the origin before/after, noise range, heavy side and the inland/seaward means, the grain's heading and the octave-0 lattice size, the 12-ri amplitude as ri of coast on the plain).

## Step 5: erosion, the gully filter (5a and 5b built; 5c, 5d and the coast reading to come)

The transcript's filter (Clay John → Fewes → Rune Skovbo Johansen), in our terms, on the same halving octave ladder as step 4. Lexa's split: 5a faded stripes, 5b the slope fed forward, 5c stacked fading, 5d normalised and straight gullies, then step 6 reads the coast (land share, box-count against Kantō's 1.1–1.15, inset retune to ~70% land).

**5a, faded gullies (built).** At a point, octave `k` uses the step-4 hex lattice **without the stretch**, cells `c = 36/2ᵏ` ri. The three nodes of the point's lattice triangle are pivots; each pivot reads a downhill direction once (cached by octave and node) and lays cos stripes across it, `S` per cell, ridge at the node; the lattice kernel blends the three into `g` (−1 crease … +1 ridge). The slope at the point gives a mask `m = 1 − (1 − clamp(σ/σ_ref))²`, σ made dimensionless by 1 ri = 3,927 m, the video's flip-square-flip so it rises gently from flat. The octave adds `E_k · (m·g + (1 − m)·t)` metres, `E_k = E₀·2^(−kH)` with step 4's `H`, listed beside `A_k` in the ladder table.
- **The target `t` reads the surface's own relief, not altitude** (Lexa: "do the fade target fix"). The video's altitude target (−1 at sea level) dropped the whole plain by `E_k` and ate 150 ri of land. Now `t = clamp((h − mean of the four neighbours a quarter cell away) / (A_k/2), −1, 1)`: a plain gives 0 and stays put, a full ridge of this octave +1, a crease −1. It reuses the four samples the gradient takes.
- **What octave `k` reads is pure:** its pivots, mask and target read `base_k = h₂ + structure octaves 0…k` (the ones at least as wide as its cell), by central differences a quarter cell apart, so a ri and a chō agree on every octave they share and a finer rung only appends. Nothing special below sea level.
- Sliders (once, under 5a, linked to 5b): `E₀` (default 300 m), first octave (2), octaves (**8**, Lexa: "8 octaves looks amazing"), stripes per cell (**1.5**; 2 and 3 were tried, Lexa found fewer stripes looked better on 5b), `σ_ref` (3%: this surface's steepest ri is a 6% slope, so the video-ish 25% masked everything out).
- Drawings: the first octave's gullies alone, its mask, the world after, and the **window**: the picked ri and its six neighbours as their chō (9,072 cells, batched into one path per shade, 64 shades plus sea), before and after, on one shade scale. The window ri is **picked by clicking a ri** on any step-5 map and kept as a world `(q, r)`, so it survives moving θ or d; until a click it is the ri nearest 12 ri inland. The window reads structure octaves to 10 (2.5 chō).
- Readouts: pivots at the first octave and the circular spread of their headings mod 180° (Lexa saw "teeth on a comb": 31 pivots at octave 2 with ±34° spread is eight near-parallel stripes across the mountains; the cure is octaves and 5b).

**5b, the slope fed forward (built).** Octave `k+1`'s pivots and mask read the base's slope **plus the gullies of octaves first…k**. A gully's slope is the video's straight gullies: a triangle wave of the same amplitude and spacing, `±4·E·S/c` across the slope by the sign of the sine, blended by the same kernel. Every pivot is still a pure function of node and seed through the cache of coarser pivots. Drawings: what 5b cuts and builds in metres, the world after, the window with 5a and 5b side by side, and per-octave readouts of how far the fed-forward slope turned the pivots from the base's slope (36° at octave 3, 64° at octave 4 at the defaults: the coarse gullies steer the fine ones hard, since `4·E·S/c` at octave 2 is ~80 m/ri). **The drainage steps read 5b's surface.**

**Speed.** With 8 octaves the page took 8.7 s; now 4.8 s, and the erosion maps show at 2.7 s: the drainage steps evaluate the surface once per point (a memo; 10,561 points), reuse the maps' gradients and rain instead of recomputing them, and render on a timeout after steps 1–5b (a newer render cancels a pending one). A **render-time-by-section row** sits at the bottom of the drainage facts. What is left is honest: each surface point at 8 octaves is ~300 noise evaluations; the windows are 9,072 chō × two eroded surfaces. Next lever if wanted: draw the windows only when their own inputs change.

**At the defaults now** (seed 1, θ 12°, d +6, A₀ 250, H 0.65, E₀ 300, octaves 2–9, 1.5 stripes, σ_ref 3%): 901 ri above sea after step 2 → 771 after step 4 → ~750 after 5b. The drainage's lakes now hold about half the year's rain, up from a tenth, because sine gullies at ri resolution are full of closed hollows: a reading for after erosion, not a bug.

**5c, stacked fading (built).** One running value per point, `R`, the normalised shape the octaves so far produced (−1 crease … +1 ridge, 0 to start). At octave `k`: target `t = R + (1 − |R|)·t_rel` (an existing coarse ridge or crease wins; elsewhere the base's relief at this scale); mask `m = m_slope · (1 − |R|)^p` (no new gullies on an existing ridge or in an existing crease); add `E_k·(m·g + (1 − m)·t)`; then `R ← m·g + (1 − m)·t`, the shape this octave produced. `p` is one slider under 5c, the video's terrain-detail knob, default 1; `p = 0` is 5b exactly. Effect at the defaults: mean |Δ| on land 67 → 98 m and the window's relief 471 → 672 m, because where `R` is near ±1 every finer octave adds its full `E_k` onto the coarse ridge or into the coarse crease instead of scribbling across it; the last octave keeps 51% of its slope mask on sloping land. Lexa: "it looks beautiful". Drawings as 5b's, the window with 5b and 5c side by side; **the drainage steps read 5c's surface.** All step-5 maps take the window click (5c's were missed at first and fixed).

**Speed, the standing decision:** with three erosion substeps at 8 octaves the page takes ~8 s in the headless check (erosion maps ~5.4 s, drainage ~2.8 s behind them). Lexa, 2026-09-29: "the gen time is killing me, but we'll optimize once we get all the way through." So: finish 5d and step 6 first, then optimise. Levers already identified: draw the windows only when their own inputs change (each is 9,072 chō × an eroded surface); each surface point at 8 octaves is ~300 noise evaluations because octave k reads the structure to its own scale at five samples; the 5a and 5b maps and windows are kept only for the comparison and could be skipped once 5c is the surface.

**5d, normalised gullies (built).** In `gullyOctave` each pivot's cos and sin are blended as a pair; the blended pair's length (≤ 1) is how much the three pivots agree. Above the threshold the pair is divided back to length 1, so gullies keep their full swing; below it (where ridges and creases join, where normalising makes spikes) the blend is left alone. One slider, "normalise above", default 0.30; 1 is off and is 5c exactly. At the defaults the first octave's pair length on sloping land averages 0.85, normalised on 679 of 705 ri; mean |Δ| on land 98 → 116 m. Drawings as 5c's, the window with 5c and 5d side by side. **The drainage steps read 5d's surface.** All step-5 maps take the window click.

**The plain, raised a second time (2026-09-29, Lexa: "we're like 50% ocean").** The section table's `(0.5, 0.12) (0.8, 0.04)` became `(0.5, 0.18) (0.8, 0.07)`: origin 172 m, coast slope 21.5 m per ri. Land after 5d at the default placement 54% → 59%. **A steeper plain buys almost nothing more** (0.24/0.10, a 28 m/ri plain, gives 60%): the bare tectonics at d = 6 are already 30% sea, the ±250 m structure noise takes ~8 points and erosion ~3, and no plain is as tall as the noise near the coast. **What reaches 70% is the inset:** with this plain and d = 12 the reading is 88% after step 2, 73% after step 4, 69% after 5d, and the edges read one sea, three land, two shore. That is step 6's retune, left to step 6.

## Step 6: reading the coast (built)

Measures the 5d coast, changes nothing. (1) **Land share against the inset:** for `d` from −10 to +30 in steps of 2 the world is re-placed and its ri counted above sea after step 2 and after step 4 (no erosion, so ~150 ms for the whole chart; the lean/turn is recomputed per placement); the 5d share at the current `d` is marked; since erosion costs a near-constant few points, the `d` for 70% after erosion is read off the step-4 curve at `70 + cost`. (2) **Box count at ri scale:** coast ri = land ri with a sea neighbour among its six; square boxes of 1, 2, 4, 8, 16 ri anchored at the origin; `D` = minus the least-squares slope of log N against log s over 1–8 ri. (3) **Box count at chō scale** inside the window cluster (chō axial keys are `36·(ri offset) + (q, r)`, the packing), boxes 1–32 chō, fit over 1–16, only when ≥ 30 coast chō are in the cluster. Drawings: the share chart, the coast map (land/sea/coast ri with the 4-ri boxes drawn; takes the window click), the log-log box chart with both series and fitted slopes, and the cluster's coast at chō scale. Facts: the shares, erosion's cost, the `d` for 70%, both `D` readings against Kantō's 1.1–1.15 and structure-alone `2 − H`.

**Readings at d = +6:** 70% → 62% → 59%; erosion costs 2.6 points; **d ≈ +11.4 for 70% after erosion**; 132 coast ri; **D at ri scale 1.10**, inside Kantō's range (structure alone would be 1.35, so erosion and the plain's slope smoothed the coast at that scale); **D at chō scale ≈ 0.9**, i.e. the coast runs straight through a cluster: the fine octaves do not roughen the zero set at chō scale. Ria-scale coastal detail, if wanted, would have to come from finer structure at the coast or from the shelf's own erosion, not from the filter as it stands. All world maps from 5a down to the coast map take the window click (the click targets are a hand-kept list of ids in the script; two maps were missed and fixed, so a new map must be added there, or the binding moved to a class).

## Where we stopped: the inset retune, then optimise

Step 6 says d ≈ +11.4 gives 70% land after erosion at seed 1, θ 12°. The retune is setting the inset default there and checking the edge rule (at d = 12: one sea edge, three land, two shore). Then **optimise** (Lexa's standing decision; levers in the Speed paragraph above), then the open questions list.

## Steps n+1a–c: drainage, drawn ahead of their turn (built)

Lexa's detour on 2026-09-29, "while it's fresh": three steps that belong **after erosion** (after step 5 and the coast reading), numbered `n+1a`, `n+1b`, `n+1c` on the page and sitting below step 4. Each **reads whatever surface the last step left**: today that is step 4's `h₄`, and when step 5 exists they must be pointed at the eroded surface (one line in the script: `const surface = height4;`). Nothing in steps 1–5 depends on them. They share the θ/d sliders and, from step 4 on, a **seed slider under every step** (`controls-seed` divs bound to the same state).

- **n+1a, the downhill face of every ri.** Gradient at each ri's centre by central differences ±0.5 ri, in m per ri, reversed to point downhill, and snapped to the nearest of the six faces by rounding the heading to a multiple of 60°: `E, NNE, NNW, W, SSW, SSE`, the order of `Direction::ALL` (E at 0°, anticlockwise). Arrow length is the raw slope capped by its own slider (default 150 m/ri). Facts: slope at the origin and its face, steepest ri, mean slope over land, ri per face. At the defaults the steepest ri is 212 m/ri (3.1°); most ri drain SSE.
- **n+1b, the year's rain on every ri.** Step 3's precipitation model, read on the current surface (the `climate` function now takes the surface as an optional argument, defaulting to step 2's, so step 3's maps are unchanged; `gradientOf(field, p, d)` moved to the top of the script), times the ri's area: `AREA_RI = (√3/2)·3927² = 13.36 km²`, `1 mm = 13,355 m³`. Volume per ri per year in millions of m³, every ri including sea, shaded with step 3's blue ramp rescaled. At the defaults: 27.8 km³ a year on the world, 15.6 on land; 17.5 to 34.2 million m³ per ri.
- **n+1c, the rain settled.** `settle(cells, heightAt, rainAt)`: nodes are the world's ri plus the ring of ri just outside it, read from the same surface. **The ring is the only outlet, in all six directions**; the sea inside the world is low ground whose water runs off the edge like any other (Lexa: "don't just consider hitting ocean to be gone"). A priority flood from the ring fills every pit to its spill height `F ≥ h`; a ri's outlet is its lowest neighbour by `F` if strictly lower, else its flood parent (across a flat lake); rain is routed in reverse flood order so every ri drains after everything above it. **Lakes:** connected ri sharing one `F > h`. Since the coast is the zero set, everything below 0 m is water already, so a pit's **fresh capacity runs from `max(h, 0)` up to the spill**: a basin the noise cut off from the sea becomes a lake whose fresh water sits above 0 m, and a pit whose spill is under the sea has no capacity and passes water on. A lake keeps the year's inflow up to its capacity and spills the rest; a lake can touch its spill through several ri, so each exit fills what capacity is left (a bug found at seed 5, where the balance came out 42 M m³ short). Held water is spread over a lake's ri by capacity share; the surface level is `max(floor, 0)` plus the filled fraction of the way to the spill. Water reaching the ring is counted by the world edge crossed (edge by the ring ri's heading from the origin, same 60° snap). Two maps: lakes by volume held; the flow, an arrow on every ri out of its outlet face, width `0.4 + 5·√(flow/max land flow)` px, sea ri faint. Facts: rain in = off the map + held (checked, "balances"), off the map by edge, pits found / with fresh capacity / holding water, largest lake (held, capacity, ri, floor, surface, spill, spilling), largest flow through a land ri and where it goes, outlet faces used. Defaults: 27,807 M m³ in, 27,521 off (SSW 14,021, SSE 7,685, E 4,106, NNE 1,709, none N or W), 286 held in 7 small lakes. **Seed 5 is the example to look at:** a 15-ri lake on a −7 m floor one ri from the sea, spill at 5 m, holding 683 M m³ and spilling 7,273.

Open for these later: they run at ri scale on a smooth surface, so pits are rare (7–19 per seed) until erosion and finer rungs roughen it; the year-versus-capacity reading of "held" is a single year's rain settled, not a steady state; sea ri that are open sea still get arrows and rain, faint on the map.

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
6. Steps n+1a–c must be pointed at the eroded surface once step 5 exists (`const surface = height4;` in the script), and their pit and lake readings re-checked on it.
7. Materials and the vertical: nothing yet says what a column's runs are made of (rock/dirt/grass came from exp-05's placeholder), nor how caves and overhangs arise; the raycasting test world wants both.
8. `goals.md` in Lexa's voice for exp-06 (the one in the other worktree is the raycasting's), and the spec in `docs/specs/YYYY-MM-DD-exp-06-worldgen-design.md` once the walkthrough settles; then the manifest entry.

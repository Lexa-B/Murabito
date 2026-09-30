# exp-06 worldgen: the port to Rust and the GPU

**Status:** design, agreed in chat with Lexa on 2026-09-30, to be built in a fresh context. The reference implementation is `viz/banana.html` (commit `952747a`); this spec restates its pipeline in the order a port needs it, with every constant and default, so the port can be checked against the page cell by cell.

**Goal, in Lexa's words:** "rewrite all of exp-06 in rust and on-gpu where possible... that way we have a version we can plug in as a basis for a game terrain generator when we're ready. keep an exp-06.html entrypoint/wrapper... this will be a side-by-side of two browsers with the same settings on the old banana and new exp-06 files so i can verify similar results."

## 1. Shape of the port

- **A Rust workspace at `Experiments/exp-06/`** (an experiment's own workspace, per `AGENTS.md`; build and test from that directory, never from the repo root). Crates under `crates/`:
  - **`worldgen`**, engine-free, no dependencies: coordinates and packing, the banana, tectonics, climate, the hex-lattice noise, the erosion, the coast reading, the ri-scale drainage and carve. Two numeric paths: `f64` for the reference (must match the page), `f32` mirroring the shader (so CPU and GPU can be compared). This is the crate the game will depend on.
  - **`worldgen-gpu`**, on plain `wgpu` (Bevy is built on wgpu; a compute pipeline here moves into Bevy's render graph later): WGSL for the per-point pipeline (§4–§6) and the per-octave pivot tables. A CPU fallback is the `worldgen` `f32` path.
  - **`exp06`**, the app: a local HTTP server (`axum` or `tiny_http`; whichever is lighter) that serves `viz/exp-06.html` and answers a render request with the arrays the page draws. Runs the GPU path, falls back to CPU, and can be told which to use, so the page can show both.
- **`viz/exp-06.html`, the wrapper:** `banana.html`'s markup, sliders and drawing code with the computation replaced by one request per render carrying the settings, and the response drawn exactly as the page draws its own. Same sliders, same defaults, same facts rows, so the two pages sit side by side. Both pages should accept the settings in the URL query (`?seed=1&theta=12&inset=10...`) so a side-by-side is one link each; `banana.html` needs that small addition too.
- **Why a server and not WASM + WebGPU in the page:** native wgpu, no WASM toolchain, the crate exercised the way the game will call it, and the page stays a thin viewer. WASM can come later if a standalone page is wanted.
- **What stays in JavaScript:** drawing only. `banana.html` remains the reference and is not changed except for the URL query.

## 2. Coordinates, units, cells

- **Cube coordinates everywhere, `(q, r, s)` with `s` carried**, in calculations and keys (Lexa's rule, no exceptions). Pointy-top, `+r` south, `Direction::ALL` order for the six neighbours: `E (1,0,−1) NNE (1,−1,0) NNW (0,−1,1) W (−1,0,1) SSW (−1,1,0) SSE (0,1,−1)`.
- **Plane:** `(east, north)` in ri. A child's centre is its parent's plus `[(q − s)/2, −r·√3/2] × size`, `size` the child's flat-to-flat in ri. A cell's hexagon is `RI_HEX × size`, `RI_HEX` the six points at angles `30° + 60°·i`, radius `1/√3`.
- **The ladder:** world (one packing cell of 36 ri) → ri (`size 1`) → chō (`1/36`) → decichō (`1/360`) → ken (`1/2160`) → shaku (`1/12960`); packing N of 36, 36, 10, 6, 6. `1 ri = 3,927 m`; a layer is `0.15151 m` (half a shaku). `AREA_RI = (√3/2)·3927²` m².
- **`owner(q, r, s, N)`** (exp-03's packing B in cube): candidates `(a, b, c = −a − b)` for `a` in `round(q/N) ± 1`, `b` in `round(r/N) ± 1`; distance `((q − Na)² + (r − Nb)² + (s − Nc)²)/2`; nearest wins, ties to the greater `(a, b)`. The children of a cell are the `(q, r, s)` with `|q|, |r| ≤ N` whose owner is `(0, 0, 0)`: 1,296 for N = 36, 100 for N = 10, 36 for N = 6.

## 3. Steps 1–3: placement, tectonics, climate (CPU or GPU, all per point)

- **The banana (step 1):** a slot bent on an arc, `R = 237` ri, `w = 60` ri, `A = 80°`, turned by `φ = −A/2` so the top tip of the centreline heads north. The outline is a dense polyline walked anticlockwise from the outer apex (outer arc, north cap, inner arc, south cap, outer arc; the page uses 180 arc samples and 60 per cap); `θ` in degrees is the fraction of its length walked; `at(θ)` gives the point and the outward normal (the tangent turned clockwise). **The origin** (the world's centre) is `at(θ).p − normal·d`. The world hexagon is pointy-top, 36 ri flat to flat, corners at `30° + 60°·i`. Edge k runs between corners k−1 and k; an edge the outline crosses is "shore", else "land" or "sea" by its midpoint. **The tangent `t = (−n_y, n_x)`** at `θ` is the arc's grain for steps 4–5. Defaults `θ = 12°`, `d = +10`.
- **Band coordinates:** unrotate by `−φ`; `r = |p|`, `ang = atan2`; on the arc (`|ang| ≤ A/2`): `s = (ang + A/2)/A`, `a = (r − R)/(w/2)`, `e = 0`; beyond a tip: polar about the cap centre, `a` across, `e` along.
- **Height (step 2):** `h₂ = H(s)·F(a)` on the arc; round a cap `ρ = √(a² + e²)`, `t = (a/ρ + 1)/2`, `h₂ = H·(t·F(ρ) + (1 − t)·F(−ρ))`. `H(s) = 900 + 1600·sin(πs)` m. `F` is a Fritsch–Carlson monotone cubic through `SECTION = (−3, −1.0) (−2.2, −0.9) (−1.5, −0.35) (−1.15, −0.05) (−1, 0) (−0.6, 0.35) (−0.1, 1.0) (0.2, 0.45) (0.5, 0.18) (0.8, 0.07) (1, 0) (1.3, −0.03) (1.7, −0.1) (2.3, −0.8) (3, −2.4)`, clamped outside. The coast is the zero set; it is never a curve.
- **Climate (step 3)**, per point on a surface `h`: `tMean = 15.5 − 0.036·(north − apexNorth) − 0.0065·h`; `half = 10.5 + 2.5·min(1, coastDist/20)` with `coastDist = max(0, 1 − min(1, |aEff|))·(w/2)`, `aEff = a/√(1 − e²)` round a cap; `tMin/tMax = tMean ∓ half`; `precip = max(400, 2200 + (1500 − 2200)·side + 0.5·h + 600·(0.6·slope(NW wind) + 0.4·slope(SE wind)))`, `side = (clamp(aEff, −1, 1) + 1)/2`, `slope(w) = clamp((∇h·w)/100, −1, 1)` with `∇h` by central differences 0.25 ri apart and the winds the unit vectors toward 315° and 135°.

## 4. Step 4: structure, the hex-lattice noise (GPU)

- **Lattice nodes are cube `(q, r, s)`** at `(q + r/2, −r·√3/2) = ((q − s)/2, −r·√3/2)` in cell units. A point `(x, y)`: `j = −y/(√3/2)`, `i = x − j/2`; `i₀ = floor(i)`, `j₀ = floor(j)`; corners `(i₀, j₀), (i₀+1, j₀), (i₀, j₀+1)` if `fi + fj < 1`, else `(i₀+1, j₀), (i₀, j₀+1), (i₀+1, j₀+1)`; each with `s = −q − r` and the offset `d = p − node`.
- **Kernel** `w = (¾ − |d|²)⁴` for `|d|² < ¾`, else 0. `¾ = (√3/2)²` is derived: the nearest node outside a triangle is exactly `√3/2` from an edge's midpoint, so the kernel is zero on every edge for every node but the edge's ends and the blend is continuous.
- **Hash** (32-bit, wrapping multiplies; `Math.imul` in JS is i32 wrapping multiply, so use `u32`/`i32` wrapping arithmetic):
  `h = imul(q, 0x27d4eb2d) ^ imul(r, 0x165667b1) ^ imul(s, 0x7feb352d) ^ imul(seed + 1, 0x9e3779b1); h = imul(h ^ (h >>> 15), 0x85ebca6b); h = imul(h ^ (h >>> 13), 0xc2b2ae35); h = (h ^ (h >>> 16)) >>> 0`. The node's gradient angle is `h · 2π / 2³²`. Negative `q, r, s` go in as two's-complement i32.
- **Value** `n = √3 · Σ w·(g·d) / Σ w` (exactly −1…1 at the extreme; zero at every node). **Exact gradient** `∇n = √3·(∇N·W − N·∇W)/W²` with `N = Σ w·(g·d)`, `W = Σ w`, `∇w = −8(¾ − |d|²)³·d`, `∇(g·d) = g`. Checked against finite differences to 1.4e-8.
- **Octaves:** `λ_k = 36/2^k` ri, `A_k = A₀·2^(−kH)` m; `k ≥ 0`. Halving maps node `(q, r, s)` to `(2q, 2r, 2s)`, so every coarse node stays a node.
- **The world's frame:** `dx, dy = p − origin`; `u = turn·(dx·t_x + dy·t_y)/stretch`, `w = turn·(dx·n_x + dy·n_y)` with `n = (−t_y, t_x)`; octave k evaluates the noise at `(u/λ_k, w/λ_k)` with seed `seed·64 + k`. Value in metres `A_k·n`; gradient in m/ri: `A_k·(∂n/∂U·turn·t/(stretch·λ) + ∂n/∂W·turn·n/λ)`.
- **The turn:** the field's first moment along the inward normal over the world's 1,296 ri (the structure with the ri rung's 5 octaves, `turn = +1`); if negative, `turn = −1` (the lattice turned half a circle, every node still a node), so the heavy side is inland. A constant of the world.
- **`h₄ = h₂ + Σ_{k < fineOctaves} A_k·n_k`**, `fineOctaves` by the rung rule (§7): 5 at the ri rung, 10 at the chō rung.
- Defaults `seed 1`, `A₀ 250` m, `H 0.65`, `stretch 2`.

## 5. Step 5: erosion, the gully filter (GPU per point; pivot tables per octave)

Octaves `k = first … first + count − 1`, defaults `first 2`, `count 8`; cell `c_k = λ_k` ri; `E_k = E₀·2^(−kH)` m, default `E₀ 200`; `S` stripes per cell, default `1.5`; `σ_ref` default `0.03` (a 3% slope); stacking exponent `p` default `1`; normalise-above `norm` default `0.30` (1 = off); base-for-pivots `baseStrength` default `0` (true slope; the video's fixed-strength form otherwise).

- **The erosion lattice** is §4's lattice **without the stretch**, in the same `(u, w)` frame with the same `turn`, cell `c_k`.
- **Pivot table, per octave, per node** (cached; a pure function of node, octave, seed): the unit direction of `slopeAt(node) = tectGrad + Σ_{j ≤ k} ∇n_j + priorGrad(node)`, where `tectGrad` is `h₂`'s central difference 0.05 ri apart, `∇n_j` §4's exact gradients (structure octaves 0…k, the ones at least as wide as the cell), and `priorGrad` the **straight-gully slope** of the coarser erosion octaves (below). With `baseStrength > 0` and a coarser octave present, the base part `tectGrad + Σ∇n_j` is scaled to magnitude `baseStrength·4·E_{k−1}·S/c_{k−1}` before the prior is added. Built coarse to fine, over the region plus a halo of one cell of the coarsest octave in use.
- **At a point, octave k:** the three corners of its lattice triangle, weights `w_v` (§4's kernel), each pivot's direction `d_v`; `across_v = (p − node_v) · d_v⊥` in ri with `d⊥ = (−d_y, d_x)`; phase `φ_v = 2π·S·across_v/c_k`.
  - **Gullies:** `cs = Σ w cos φ / Σ w`, `sn = Σ w sin φ / Σ w`, `len = √(cs² + sn²)`; `g = cs / max(len, norm)` if `norm < 1`, else `cs`. (5d; the `max` joins continuously, a switch at `norm` drew lines.)
  - **Straight slope, for the pivots of finer octaves:** `Σ w·(−sign(sin φ_v))·(4·E_k·S/c_k)·d_v⊥ / Σ w`, m/ri.
  - **Smooth slope, for the mask:** `Σ w·(−sin φ_v)·(E_k·2π·S/c_k)·d_v⊥ / Σ w`. (The straight one flips sign at every ridge and crease, which the mask drew as lines.)
  - **Mask:** `σ = |tectGrad + Σ_{j ≤ k} ∇n_j + priorGradSmooth| / 3927`; `mSlope = 1 − (1 − clamp(σ/σ_ref, 0, 1))²`.
  - **Relief target:** `tRel = clamp(n_k/A_k + (h₂(p) − mean of h₂ at the four points ± c_k/4 along x and y)/(A_k/2), −1, 1)`; 0 if `A_k = 0`. (The octave's own noise in units of its swing, plus the tectonic surface's four-sample relief; not the video's altitude target, which sank the plain.)
  - **Stacking (5c):** running `R` per point, 0 to start. If `p > 0`: `room = (1 − |R|)^p`, `t = R + (1 − |R|)·tRel`, `m = mSlope·room`; else `t = tRel`, `m = mSlope`. `shape = m·g + (1 − m)·t`; add `E_k·shape`; `R ← shape`.
- **The surface:** `h = h₂ + Σ_{k < fineOctaves} A_k n_k + Σ_octaves E_k·shape_k`. Feed-forward (`priorGrad`) on (5b) for everything the page calls 5b, 5c, 5d; the page's 5a is the same with the prior off and `p = 0`, `norm = 1`.
- **Cost per point:** one structure evaluation returning every octave's value and gradient (18 octaves at most), 4 + 4·count cheap tectonic samples, and count × 3 pivot lookups. ~3 s for the whole page in JS at 8 octaves; a chō chunk is nothing on a GPU.

## 6. Step 6: reading the coast (reductions)

Coast ri = land ri (`h > 0`) with a sea ri among its six neighbours. Box count: square boxes of side `s` anchored at the origin, `N(s)` = boxes holding a coast point; `D = −slope` of `log N` on `log s` by least squares. Ri scale: sides 1, 2, 4, 8, 16 ri, fit over 1–8. Chō scale over every coast ri and its sea neighbours (the strip): sides 1…64 chō, fit over 4–64 (the smallest boxes under-read: raster bias and nothing finer than the finest octave). Land share against the inset: re-place the world at each `d` and count `h₂ > 0` and `h₄ > 0`; the `d` for 70% after erosion is read off the step-4 curve at `70 + (share₄ − share_5d)`.

## 7. The rungs (chunk evaluation)

A rung's chunk is one cell of the rung above, evaluated as its children at their centres. **Rule:** a rung evaluates the octaves whose wavelength is at least two of its cells, finest `k = floor(log2(18/size))`: ri 4, chō 9, decichō 12, ken 15, shaku 17; the same bound caps the erosion octaves (`count` clipped to `finest − first + 1`). Everything is a pure function of position, seed and octave, so a chunk evaluated alone matches its neighbours on every octave they share; the step where one rung meets the next is the dropped octaves' amplitude (measured on the page before it was reverted: chō → decichō mean 0.9 m, decichō → ken 0.28 m, ken → shaku half a layer). Structure and erosion are under a layer by the ken.

**What the page does not yet do, and the port should design for:** the ri rung's maps evaluate all 8 erosion octaves rather than the 3 the rule allows (accepted aliasing on the page); the erosion count slider caps at 8, so finer rungs add structure only; and the drainage's tables (§8) are not yet handed to finer rungs.

## 8. Step 7: drainage at ri scale (CPU, global, small)

All on the 5d surface sampled at ri centres (and the ring of ri outside the world, from the same surface).

- **7a** gradient by central differences ±0.5 ri, reversed to downhill, snapped to the nearest of the six faces (round the heading to a multiple of 60°, E at 0°).
- **7b** step 3's precipitation on the surface times `AREA_RI`: a volume per ri per year.
- **7c** `settle`: nodes = the world's ri + the ring; **the ring is the only outlet**, seeded at its own heights; a priority flood inward fills each ri to `F = max(h, F(parent))`; an ri's outlet is its lowest neighbour by `F` if strictly lower, else its flood parent; lakes are connected ri sharing one `F > h`, with **fresh capacity from `max(h, 0)` up to the spill** (everything under 0 m is sea already); rain is routed in reverse flood order, a lake keeping inflow up to its capacity (each exit ri fills what is left) and passing the rest; water reaching the ring is counted by the world edge crossed.
- **7d** `carve`: every pit with `spill > 0` and `spill − max(floor, 0) ≤ keepDeeper` (default `100` m) is breached: from its floor, follow the flood tree outward until a ri with `h < floor` (or a ring ri, taken as `floor − 1`); the ri strictly between are lowered onto a straight ramp from the floor's height to the exit's (`cut` recorded per ri); settle again; repeat until no shallow pit remains or a round cuts nothing. Output: carved heights, `cut` per ri, the settle on the carved surface (the rivers).
- **Rivers as objects (not built):** each channel is a path of ri centres with the ramp's height along it; give it a width and finer rungs evaluate a valley kernel around the path, point-evaluable from a small table. Fine rungs' own small hollows: flood the chunk with a halo of one ri so a hollow smaller than a ri is seen whole; larger hollows are the ri rung's.

## 9. Verification, the side-by-side

1. **Same settings, both pages,** by URL query: `seed, theta, inset, amp, hurst, stretch, erosion, firstOctave, octaves, stripes, sigmaRef, stacking, normAbove, baseStrength, keepDeeper, window (q, r, s)`.
2. **The reference readings** are `docs/reference-readings.md`: every facts row at the defaults, seed 1, from the page's headless harness. The port's `f64` path should reproduce every count exactly and every metre to a fraction of a metre; the `f32`/GPU path may flip a few ri that sit within float noise of a threshold (land counts ±2, a lake more or less), and the maps should be indistinguishable by eye.
3. **The harness:** `viz/tools/check.js` renders `banana.html` headlessly under node with a DOM stub and prints every facts row (`node viz/tools/check.js viz/banana.html`; `STRIP=1` also presses the strip button); `viz/tools/seeds.js` averages the readings over N seeds (`node viz/tools/seeds.js viz/banana.html 50`). Any single value the port needs from the page (a noise value at a point, a pivot direction) can be had by adding a `console.log` in the page and running the harness. The 50-seed averages at the defaults: land after 5d 73.0% (sd 3.8); coast D at ri scale 1.07; rain held after 7d 18% at 100 m.
4. **Order of bites** (Lexa's process: one small step, drawn or printed, checked against the page, then the next): workspace and `worldgen` with coordinates and packing (1,296 / 100 / 36 children, owner ties) → step 1 (facts row 1) → step 2 (facts2, the probes in the handoff) → step 3 (facts3) → the hex noise (values and gradients at a few points from the harness, then facts4) → 5a → 5b → 5c → 5d (facts5, 5b, 5c, 5d, the window's relief numbers) → step 6 → 7a–7d (facts through factsCarve) → the server and `exp-06.html` → the GPU path for §4–§5 with a CPU-vs-GPU diff → the chunk API (§7).

## 10. Open after the port

Rivers as objects and the halo fill (§8); the ri rung obeying the octave rule and the erosion count cap; H toward Kantō's 1.1–1.15 or the look; materials and the vertical (what a column's runs are made of, caves, overhangs); the world's climate on the finished surface; `goals.md` in Lexa's voice and the manifest entry.

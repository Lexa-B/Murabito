# exp-06 worldgen: the port to Rust and the GPU

**Status:** design. Drafted 2026-09-30 from the walkthrough; the shape (§1), the cell rule (§2) and the verification and order of bites (§9) were reviewed and approved section by section by Lexa on 2026-09-30 in the brainstorming for the port. The reference implementation is `viz/banana.html` (commit `952747a`); this spec restates its pipeline in the order a port needs it, with every constant and default, so the port can be checked against the page cell by cell.

**Goal, in Lexa's words:** "rewrite all of exp-06 in rust and on-gpu where possible... that way we have a version we can plug in as a basis for a game terrain generator when we're ready. keep an exp-06.html entrypoint/wrapper... this will be a side-by-side of two browsers with the same settings on the old banana and new exp-06 files so i can verify similar results."

## 1. Shape of the port

Three decisions Lexa made on 2026-09-30, which everything below follows: the page's compute lives in a **native Rust process** the page calls over localhost (not WASM in the browser); the **page exists from the first bite**, so every step is seen side by side as it lands; and the **GPU path arrives with step 4** and every later step lands on CPU and GPU together.

### 1.1 The workspace and its crates

A Cargo workspace at `Experiments/exp-06/` (an experiment's own workspace, per `AGENTS.md`; build and test from that directory, never from the repo root; `target/` stays inside the experiment). Crates under `crates/`:

- **`worldgen`**, engine-free, **no dependencies at all** (like exp-05's `hexworld`): coordinates and packing, the banana, tectonics, climate, the hex-lattice noise, the erosion, the coast reading, the ri-scale drainage and carve. Generic over the float type through a small `Float` trait, so the same code runs in **f64 for the reference** (must match the page) and **f32 mirroring the shader** (same order of operations as the WGSL, so a CPU-versus-GPU diff measures the driver's float behaviour and not ours). Its public surface is small: a `Settings` struct with the page's defaults, a `World` built from settings (origin, tangent, turn), and functions that evaluate a set of cells, a rung's chunk, and the ri-scale tables. This is the crate the game will depend on. Modules, one per step (§1.4).
- **`worldgen-gpu`**, on **wgpu 29**, the version the root project's Bevy 0.19.1 already uses (checked in the root `Cargo.lock`; wgpu 29 is in the cargo cache). WGSL for every per-point stage (§3–§5) and the per-octave pivot tables; the same evaluate-a-set-of-cells calls as `worldgen`, so a caller swaps them. A compute pipeline here moves into Bevy's render graph later without changing the shader. Details in §1.5.
- **`exp06`**, the binary: a local HTTP server on a fixed port (a constant, printed on start) that serves `viz/exp-06.html` from disk and answers render requests. Runs `worldgen-gpu`; a request can ask for `worldgen` f64 or f32 instead, so the page can show any of the three and diff them. The only crate that knows JSON (`serde`) or HTTP (`tiny_http`, synchronous and small; swapped only if it lacks something needed).

Machine facts that shaped this: an RTX 5090 (and the Intel UHD 770) on Vulkan 1.4, rustc 1.95.0 (the root pin), node 24 for the harness.

### 1.2 The page and the wire

- **`viz/exp-06.html`** is `banana.html`'s markup, steps, sliders, defaults, facts rows and drawing code with the computing cut out, plus a selector for the compute path (GPU, CPU f32, CPU f64). The two pages line up tab for tab. It cannot open from disk: `cargo run -p exp06`, then `http://127.0.0.1:<port>/`.
- **Settings in the URL query on both pages** (`seed, theta, inset, amp, hurst, stretch, erosion, firstOctave, octaves, stripes, sigmaRef, stacking, normAbove, baseStrength, keepDeeper, window` as `q,r,s`); sliders write the query back as they move, so a side-by-side is one URL with the filename swapped. **This is the only change `banana.html` receives**; it remains the reference.
- **One render request per settings change:** `POST /render` with the settings as JSON, debounced 150 ms as the page does. The response is one JSON document with everything the page draws and prints: the outline and edge kinds, the 1,296 ri with every per-step height and climate, the window's chō per substep, the coast and box counts, the drainage tables, and the **facts rows as text**: the facts code moves to Rust (`worldgen::facts`), so a row is the same string on both sides and the harness compares them literally.
- **Staging stays:** maps on the first request, windows and drainage on a second, the strip on a third from its button; a newer render cancels a pending one on the page side.
- **Sizes:** ri tables are kilobytes, a window a few hundred kilobytes, the strip a few megabytes once; fine over localhost. If a payload hurts it becomes a typed array over a binary response and nothing else changes.
- **Timing row:** the server reports its time per stage and per path, and the largest CPU-versus-GPU difference per output array, and the page shows those instead of JavaScript's times.

### 1.3 Why a server and not WASM + WebGPU in the page

Native wgpu on the 5090 through Vulkan, no WASM toolchain, no browser WebGPU flags on Linux, f64 available for the reference path (WGSL and WebGPU lack it), and the crates exercised the way the game will call them; the page stays a thin viewer. The cost is that the page needs the process running. WASM can come later if a standalone page is wanted.

### 1.4 How the pipeline lands in Rust

`worldgen` mirrors the page's steps as modules, each with the page's pure functions and unit tests pinned to `docs/reference-readings.md`:

- `hex`: the cube `(q, r, s)` type, the six directions in `Direction::ALL` order, `owner` with the tie rule, the children of a cell, cell centres in the plane, the lattice triangle and the kernel. Tests: 1,296 / 100 / 36 children, owner ties, the origin is a node.
- `banana`: the outline polyline, `at(θ)`, the origin, the tangent, band coordinates, edge kinds. Tests: the step-1 facts row.
- `tectonics`: the section table, the Fritsch–Carlson spline, `H(s)`, the cap blend, `h₂`. Tests: the origin at 269 m, the coast slope, the six edge ranges.
- `climate`: the coefficients as a struct; the four numbers per point on a given surface.
- `noise`: the hash bit for bit with `Math.imul` semantics, the node gradient, value and exact gradient, the octave ladder, the world frame, the turn. Tests: values and gradients at points printed from the page by the harness; a finite-difference check of the gradient.
- `erosion`: pivot tables per octave, the pair blend, straight and smooth slopes, mask, relief target, stacking, the surface. Tests: the pivot heading spread, the window's relief at each substep.
- `coast`: coast ri, box counts, the least-squares slope, the land-share curve and the inset for 70%.
- `drainage`: 7a–7d, `settle` returning the flood tree and `carve` on it. Tests: the balance, the pits, the largest lake and river, the carve counts.
- `facts`: the facts rows as strings, one function per step.

Two rules carry over from the page: everything per point is a pure function of position, seed and settings, so a chunk at any rung matches its neighbours; and nothing in `worldgen` knows about JSON, wgpu or HTTP.

### 1.5 The GPU path

- **One dispatch per cell set.** A cell set (§2) is a buffer of cube `(q, r, s)` plus the rung's size in ri and the world's origin, built on the CPU: the 1,296 ri, a window's 9,072 chō, the strip, or a chunk. One thread per cell computes the cell's centre in the plane, then tectonics, climate, the structure octaves with their exact gradients, and the erosion octaves, and writes a struct of arrays keyed by the cell: `h₂`, `h₄`, the four substep heights, the first-octave readouts the page prints, the surface gradient. 5a–5d are one kernel with four flags, so four dispatches; only the one the drainage reads is mandatory.
- **The pivot tables are their own passes,** one per erosion octave, coarse to fine, since octave `k`'s pivot at a node reads the straight-gully slope of octaves `first…k−1`, which read their own tables. Each table is a storage buffer of unit directions over the lattice nodes covering the region plus a halo of one cell of the coarsest octave in use. A node is keyed by cube `(q, r, s)` in the API; the buffer's layout is a rectangle in `(q, r)` with `s` derived, private to the crate.
- **The turn is computed first,** on the CPU from one structure dispatch at the ri centres, since it is a sum over the world the erosion needs as a constant. Likewise the land counts, the coast, the box counts, the lean and all of step 7 read arrays the GPU wrote and run on the CPU; the strip's coast count over 318,816 chō stays on the CPU until it proves slow.
- **Settings ride in a uniform buffer,** the section table and the climate coefficients too, so the shader has no literal a Rust constant does not also own.
- **f32 only on the GPU.** The hash is native `u32` wrapping arithmetic in WGSL, bit-identical; the angle and the trig are f32 in the same order of operations as the CPU f32 path. Correctness is the CPU f64 path against the readings.
- **The chunk call** is the same dispatch with a chunk's children as the cell set and the rung's octave cap (§7). It is the last bite, and it is what the game calls.

## 2. Coordinates, units, cells

- **Cube coordinates everywhere, `(q, r, s)` with `s` carried**, in calculations and keys (Lexa's rule, no exceptions). Pointy-top, `+r` south, `Direction::ALL` order for the six neighbours: `E (1,0,−1) NNE (1,−1,0) NNW (0,−1,1) W (−1,0,1) SSW (−1,1,0) SSE (0,1,−1)`.
- **What the API names is cells, never plane points.** A cell set is a list of cube `(q, r, s)` at a rung, plus the world's origin; the plane centre is derived inside from the cube coordinate and the rung's size, and every table, key and output is indexed by the cell's `(q, r, s)`, never by an array position the caller maps back. The plane still exists inside the math, because the banana is Euclidean geometry (an arc of radius 237 ri, band coordinates by `atan2`, the tangent frame, distances in metres) and there is no cube form of an arc; the lattice lookup goes straight back to cube nodes. **The one named exception** is `height_at_plane_point`, the probe the page's cross-section chart samples along a line through the origin; nothing else uses it. (Lexa, 2026-09-30: "why is it in XY?")
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

## 9. Verification, the side-by-side, and the order of bites

Four layers of checking, each running on every bite:

1. **Unit tests in `worldgen`,** pinned to `docs/reference-readings.md` (every facts row at the defaults, seed 1, from the page's headless harness): every count exact in f64, every metre within a fraction. Where a test needs a value the readings don't hold (the noise at one point, one pivot's heading), print it from `banana.html` with a `console.log` through `viz/tools/check.js` and pin that number, saying in the test where it came from.
2. **The facts rows, compared literally.** `viz/tools/check.js` renders `banana.html` headlessly under node with a DOM stub and prints every facts row (`node viz/tools/check.js viz/banana.html`; `STRIP=1` also presses the strip button). It grows a second mode that fetches `exp-06.html`'s render from the running server and prints the same rows in the same format, so `diff` between the two outputs is the check for a bite. The f64 path must diff clean; the f32 and GPU paths may flip a few ri that sit within float noise of a threshold (land counts ±2, a lake more or less) and are compared with a tolerance the harness prints; the maps should be indistinguishable by eye. `viz/tools/seeds.js` averages readings over N seeds (`node viz/tools/seeds.js viz/banana.html 50`); at the defaults: land after 5d 73.0% (sd 3.8), coast D at ri scale 1.07, rain held after 7d 18% at 100 m.
3. **CPU f32 versus GPU,** from step 4 on: the same cell set through both, the largest difference per output array printed by the server's timing row and by the harness.
4. **Lexa's eyes:** two tabs at the same URL query, `banana.html` and `exp-06.html`, on every bite.

**Order of bites** (Lexa's process: one small step, seen on the page, checked against the readings, then the next; commit only when Lexa says):

1. The workspace, `worldgen::hex`, `exp06` serving a page that shows step 1 beside banana, and the URL query on both pages.
2. Step 2 tectonics: the section chart, the hypsometric, the edge profiles, facts2.
3. Step 3 climate, facts3.
4. The hex noise and step 4 on CPU f64 (values and gradients at a few points from the harness, then facts4); then the wgpu pipeline with the first kernel and the CPU-versus-GPU diff. Two bites if the second is heavy.
5. 5a, then 5b, then 5c, then 5d, each with its pivot passes and the window (facts5, 5b, 5c, 5d, the window's relief numbers).
6. Step 6, the coast and the strip button (facts6, the strip).
7. 7a–7d on the CPU (factsGradient through factsCarve).
8. The chunk API and the rung rule (§7), printed for one cell of each rung against the numbers in §7.

## 10. Open after the port

Rivers as objects and the halo fill (§8); the ri rung obeying the octave rule and the erosion count cap; H toward Kantō's 1.1–1.15 or the look; materials and the vertical (what a column's runs are made of, caves, overhangs); the world's climate on the finished surface; `goals.md` in Lexa's voice and the manifest entry.

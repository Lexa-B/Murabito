# exp-05 — Converging on the main project's coordinates: design

Amends `2026-09-20-exp-05-hex-voxel-chunks-design.md`. Where the two disagree, this one wins.

## Purpose

The main project settled its coordinate conventions after exp-05 was specced. They are written up in `Docs/hex_units.md` and implemented in `crates/hexcoords/` (`murabito_hexcoords`). This change moves exp-05 onto the same conventions, so that its systems (packing, chunks, the store, the mesher, the handover) can later be lifted into the main project without translating coordinates on the way.

This change is about coordinates only. Ownership, chunking, meshing and the handover keep their design. The relabelling reflects label positions north↔south, but each chunk's drawn set (its owned cells plus its guest ties) is symmetric under that reflection, so the world they produce looks the same; only which parent owns a tied cell changes, described under "Labels".

## Decisions (2026-09-22)

- **Converge on main's conventions.** The user changed their mind about exp-05's zero-dependency core: the core may depend on Bevy through the coordinate crate.
- **Vendor a copy.** Main's `crates/hexcoords/` is copied into exp-05, keeping experiments self-contained, rather than exp-05 depending on the main project's path.
- **The layer is fixed at 5 sun (0.5 shaku)**, as in main. The runtime thickness setting goes.
- **`f32` throughout**, as in main. The core's `f64` goes.

## What main's conventions are

From `Docs/hex_units.md`, as of main at `79335cd`:

- World space is Bevy's `Vec3`, **in shaku**, `f32`, with Y up. **East is +X, north is −Z.**
- Voxels are pointy-top hex prisms, 1 shaku flat to flat and 0.5 shaku tall. Integer voxel space is `VoxelCoord { q, r, layer }`.
- **The API speaks cube, storage stays axial.** `VoxelCoord::new(q, r, s, layer) -> Result<_, NotOnHexPlane>` is the only public constructor. The fields are private, behind `q()`, `r()`, `s()`, `layer()`. On disk, `(q, r, layer)`.
- Voxel to world: `x = q + r/2`, `z = (√3/2)·r`, `y = 0.5·layer`, the centre of the voxel's bottom face. **+q is east; +r is SSE, down the screen.**
- Twelve compass directions, `Direction::{E, ENE, NNE, N, …, ESE}`, indexed anticlockwise from east. Edges are the even indices, corners the odd ones.

exp-05 differs in four ways:

- +r points NNE, not SSE;
- the core works in metres, as `(east, north, height)` in `f64`;
- `Hex` has public axial fields;
- there are six directions.

North is −Z in both, so **the rendered world is not mirrored and the triangle winding does not change.** What changes is which label sits where.

## The vendored crate

- `crates/hexcoords/` is a copy of main's `crates/hexcoords/` at `79335cd`. The package name stays `murabito_hexcoords`, so lifting exp-05 into main only changes a path.
- **`src/lib.rs` is copied byte for byte and not edited in exp-05.** A fix belongs in main first, then gets copied again. The crate's README records the source commit.
- Only its `Cargo.toml` differs, because exp-05's workspace is not main's:
  - `edition = "2024"` is set on the crate itself (exp-05's workspace default is 2021);
  - Bevy is `default-features = false`, since the crate only needs `bevy::math`. The plugin and viewer turn on the features they need, as now.
- `hexworld` depends on `murabito_hexcoords`, and on `bevy` with `default-features = false` for `bevy::math::Vec3`. The vendored crate takes `Vec3` in its API but doesn't re-export it, and its source isn't edited here, so `hexworld` names Bevy itself, the same way main's crate does.
- The rule in `AGENTS.md`, the README and the original spec that `hexworld` has no dependencies becomes: **`hexworld` depends only on `murabito_hexcoords` and on `bevy` without default features, for its math types.** It still holds no rendering, ECS or asset code; those stay in `hexworld_bevy` and `viewer`.

## Labels: `Hex`

`Hex` stays exp-05's type: one 2D label that serves every level (shaku, ken, cho, ri), which main's `VoxelCoord` does not model. It takes main's shape:

- **Private fields, cube-only public constructor:** `Hex::new(q, r, s) -> Result<Hex, NotOnHexPlane>`, using `murabito_hexcoords::NotOnHexPlane` and summing in `i64` as main does. Getters `q()`, `r()`, `s()`.
- **A crate-private axial constructor**, `Hex::axial(q, r)`, for the packing and ownership maths inside `hexworld`, which works in `(q, r)` throughout. Main's rules allow constructing directly inside the crate. It is not `pub`: integration tests, the plugin and the viewer use the cube constructor.
- `ZERO`, `Add`, `Sub`, `Hash`, `Ord` stay. `Ord` is still `(q, r)` lexicographic; ownership's tie-break uses it.
- **`+r` is SSE.** The label-to-position formula is main's, scaled by the level's width (next section).
- `Display` writes the cube triple, `(q, r, s)`, as main's API speaks. Addresses read `ri (q,r,s) / cho (q,r,s) / ken (q,r,s) / shaku (q,r,s) / layer k`.

**The integer maths does not change.** `d2`, packing B, `owner`, the drawn set, windows and the exp-03 fixtures all work on labels and never on positions, so they carry over as they are. The relabelling from exp-05's labels to main's, `(q, r) → (q + r, −r)`, leaves `d2 = dq² + dq·dr + dr²` unchanged.

The ownership tie-break, **greatest `(a, b)`**, is also a rule about labels. Under the new mapping it picks the cell reflected north↔south. Each chunk's drawn set (its owned cells plus its guest ties) is symmetric under that reflection, so **the battlement pattern looks the same on screen as in exp-03 and the current exp-05.** It is still a correct partition, every parent still owns N² children, and the fixtures still pass. Only which parent owns a tied cell changes, visible only in the fine detail of the border lines.

The fixture file stays as it is: axial pairs, which is storage. Its parser builds labels with `Hex::new(q, r, -q - r)`.

### Directions

- `hex::DIRECTIONS` and `neighbours` become edge steps taken from `murabito_hexcoords::Direction`: the six edge directions, `E, NNE, NNW, W, SSW, SSE`, in index order.
- The offset for each is **read from the vendored crate**, as `VoxelCoord::neighbour(d)` of the origin, and not written out again. Main's direction table stays the single source.
- **The mesher's pairing rule is unchanged.** The six edges are still anticlockwise from east at 0°, 60°, …, 300°, so edge `k` still pairs with the cell edge between corners `k−1` and `k`, and `corners` still starts at 30° and runs anticlockwise. Only the labels on each step change.

## Positions: `Vec3` in shaku

The core's metre functions become shaku `Vec3` or `f32` functions. `(east, north, height)` becomes `(x, y, z)` with north at −Z, as in main.

| Now | Becomes |
|---|---|
| `Level::width_m() -> f64` | `Level::width() -> f32`, which is `scale_shaku` |
| `SHAKU_M`, `SUN_M` | removed from positions. Metres appear only where a person reads them: the panel shows both |
| `plane::cell_centre_m(cell, level) -> (f64, f64)` | `plane::cell_centre(cell, level) -> Vec3`, with `y = 0` |
| `plane::metres_to_axial`, `hex_round`, `round_at(e, n, level)` | `plane::round_at(pos: Vec3, level) -> Hex`: `x` and `z` are scaled by `1/width`, and the result is rounded by the vendored crate (`VoxelspacePos::from(Vec3).round()`). exp-05's own `hex_round` goes |
| `plane::corners_m(level) -> [(f64, f64); 6]` | `plane::corners(level) -> [Vec3; 6]`, starting at 30°, anticlockwise on screen |
| `WorldConfig::layer_thickness_m`, `layer_of_height`, `layer_bottom_m`, `layer_top_m` | free functions over a fixed layer height of 0.5 shaku |
| `Column::surface_height_m`, `ChunkStore::surface_height_m(e, n)` | `surface_height() -> f32`, and `surface_height(pos: Vec3)` |
| `placeholder_terrain::height_m(cfg, e, n, level)` | `height(cfg, x, z, level) -> f32` |

- **The layer height.** Main keeps its 0.5 private. `hexworld` names its own `LAYER_HEIGHT: f32 = 0.5`, with a test pinning it to `VoxelCoord::to_world` of layer 1. If main ever changes it, the test fails, rather than the two silently disagreeing.
- **`VoxelCoord` at the shaku level.** Where `hexworld` means "a shaku voxel", in addresses and in `address_of` / `voxel_of`, it uses `murabito_hexcoords::VoxelCoord` instead of a `(Hex, layer)` pair.
- **The placeholder terrain** keeps its shape exactly. Its constants are converted by ×3.3:
  - the base wavelength is 8,192 m = 27,033.6 shaku;
  - the base amplitude is 10 m = 33 shaku.

  Which octaves each level keeps is unchanged. The noise works in `f32`. At the 12-ri edge, about 155,000 shaku out, an `f32` resolves about 0.016 shaku, roughly 1% of the finest octave's wavelength.
- **The mesher** emits chunk-relative positions already on Bevy's axes, in shaku. Relative to the chunk centre, they are today's positions mapped by `(e, h, −n)` and scaled by 3.3. That mapping is a rotation, so the winding and the normals keep their sense.

## `WorldConfig`

`seed`, `world_radius_ri`, `bottom_layer`. **`layer_thickness_sun` goes**, and so does the panel's thickness slider.

## The plugin and the viewer

- **`hexworld_bevy::axes` is deleted.** The core already speaks Bevy's axes, so there is nothing to map. A loader's position is its `Transform` translation, passed straight to the store.
- **The camera** keeps its focus as a `Vec3` on the ground plane. `MIN_ZOOM` and `MAX_ZOOM` convert from metres to shaku: 3 m becomes 9.9, and 4,000 m becomes 13,200.
- **The panel** reads positions in shaku. Where a length is shown to a person, it gives shaku with metres beside it.
- The shader is unchanged: it carries no lengths.

## Testing

- **Orientation tests are rewritten on purpose, against main's table:**
  - `hex.rs`: `directions_run_anticlockwise_from_east` checks the six edges against `Direction`'s even indices;
  - `plane.rs`: `corners_are_a_pointy_top_hexagon`;
  - `mesh.rs`: `top_faces_point_up_and_wind_anticlockwise`;
  - the four `axes` tests in `hexworld_bevy/tests/plugin.rs` go with `axes.rs`. Orientation is pinned instead by `plane.rs`'s `a_step_east_is_one_width_along_x` and `a_plus_r_step_is_half_a_cell_east_and_a_row_down_the_screen`, plus `mesh.rs`'s winding tests.
- **Conversion tests:**
  - at the shaku level, `plane::cell_centre` agrees with `VoxelCoord::to_world`;
  - `round_at` inverts `cell_centre` at every level;
  - the layer height matches the vendored crate.
- **Everything else must pass unchanged in meaning:** the exp-03 fixtures, the handover suite, the store's tests and the mesher's tests. Test literals in metres get converted, and none of their assertions change what they check.
- The success check is the same as the original spec's, plus: the viewer shows the same terrain and the same battlement pattern as before; only which parent owns a tied cell changes, visible only in the border lines' fine detail.

## Out of scope

- Using `VoxelCoord` for coarse voxels. A cho-wide column is still `(level, Hex, layer)`, since main has no levels yet.
- Distance, heuristics and movement from main's `movement.md`.
- Any change to ownership, chunking, meshing or the handover.
- Changes to main's `hexcoords`. If exp-05 needs something the crate lacks, it is raised with the user for main first, not patched into the copy.

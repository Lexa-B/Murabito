# Hex coordinates and units

A brief on the voxel coordinate system for the main project, as agreed in design.
Nothing here is implemented yet. The open questions are listed at the end.

## Voxels

The world is made of hexagonal prisms (voxels): pointy-top hexes stacked in layers.

- **Voxel space** is integer. One voxel is 1 wide (flat to flat) and 1 tall.
- **World space** is `f32`, in **shaku**, with **Y up** to match Bevy.
  One voxel is 1 shaku flat to flat and 5 sun (0.5 shaku) tall.

## Cube and axial coordinates

A hex grid can be addressed with three coordinates `(q, r, s)` that always satisfy
`q + r + s = 0`: the hexes are the diagonal plane of a cube lattice. The constraint makes one of
the three redundant, so storing only `(q, r)`, which is **axial**, loses nothing, and `s` is
`-q - r` whenever it's needed.

Cube form is still the nicer one to work in. Distance, rotation, rounding and the direction
tables are all symmetric in `q`, `r` and `s`. So the API speaks cube and the storage is axial.

## `VoxelCoord`

```rust
pub struct VoxelCoord { q: i32, r: i32, layer: i32 }   // fields private
```

- **Stores axial** `(q, r)` plus `layer`. Since `s` is never stored, the sum can't drift out of
  true.
- **Takes cube input, validated:** `VoxelCoord::new(q, r, s, layer) -> Result<VoxelCoord, NotOnHexPlane>`
  checks `q + r + s == 0`.
  - `Result` makes the caller deal with bad input. They can't quietly ignore it.
  - The sum is taken in `i64`. Adding three large `i32`s can overflow, which panics in a debug
    build, so this way extreme values come back as an `Err` instead of a crash. If the sum is
    valid, `s` fits in an `i32`; but the way there needn't: `-q - r` overflows at `q == i32::MIN`
    and `-(q + r)` when `q + r` passes `i32::MAX`, both of which are valid voxels. `s()` uses
    wrapping arithmetic, which passes through the overflow and, since the true value is in range,
    lands on it.
  - The fields are private, so `new` is the only way to build a `VoxelCoord`. Rust's visibility
    rules enforce the invariant. Nobody has to remember it.
  - **Cube is the only way in.** There is no axial constructor. Decided 2026-09-22: `VoxelCoord`
    is the runtime type and cube is its one face everywhere. The one place that drops `s` is level
    data on disk, which is a separate format: it writes `(q, r, layer)`, and on reading rebuilds
    `s` (summing in `i64`) and goes through `new`, so a corrupt or hand-edited file that doesn't
    sum to zero is refused as `NotOnHexPlane` rather than becoming a bad voxel. Inside the crate,
    `from_world` builds the struct directly from a rounded triple, which the module can do.
- **Gives cube output, reconstructed:** `q()`, `r()`, `s()` (as `-q - r`), `layer()`.
  The getters take `self` by value because the type is `Copy` and only 12 bytes.
- `NotOnHexPlane { q, r, s }` carries the rejected values, so an error message can show them.
- Derives `Clone, Copy, Debug, PartialEq, Eq, Hash`, so it can key a map.

**Names.** It's `VoxelCoord` rather than `Voxel` because it's an address, not the voxel's
contents: that keeps `Voxel` free for a type that holds what's in a cell. And it's `layer`
rather than `y` or `z` so the integer layer index is never confused with the float world axes.

## Orientation

- Pointy-top: rows run along X, corners point along ±Z.
- `+q` points toward +X.
- `+r` points toward +Z. For a camera looking straight down with +X to the right (Bevy is
  right-handed, Y up), +Z is toward the bottom of the screen, so one `+r` step goes down-right
  on screen.
- Rows are √3/2 shaku apart. Two rows down (`q − 1, r + 2`) lands back in the same screen column,
  √3 shaku away.

## `VoxelspacePos`

The fractional twin of `VoxelCoord`: `q`, `r`, `s` and `layer` as `f32`, for a point in voxel
space rather than a voxel's address, such as where a moving entity is between two cells. Decided
2026-09-22. Same rules: stored axial, cube in through `new -> Result<_, NotNearHexPlane>`, cube out.
The on-plane check has a tolerance, `ON_PLANE_TOLERANCE = 1e-4`, since real arithmetic lands near
zero rather than on it.

Conversions follow Rust's `From`/`Into` convention, so callers get `.into()` and `Vec3::from(pos)`
the way they do everywhere in Bevy. `From` is only for the exact ones:

- `Vec3` ↔ `VoxelspacePos`: `From` both ways, lossless.
- `VoxelCoord` → `VoxelspacePos`: `From`; the voxel's centre at its bottom face.
- `VoxelspacePos` → `VoxelCoord`: `pos.round()`, named because it rounds.
- `VoxelCoord::to_world()` and `VoxelCoord::from_world(v)` are the two above composed, and stay
  named methods for the same reason.

## Voxel ↔ world

A voxel's world position is the **centre of its bottom face**.

Voxel → world:

```
x = q + r/2
z = (√3/2) · r
y = layer · 0.5
```

World → voxel:

```
r' = z / (√3/2)
q' = x − r'/2
(q, r, s) = cube_round(q', r', −q' − r')
layer = floor(y / 0.5)
```

- **`cube_round`:** an arbitrary point gives fractional coordinates that don't sit on a hex
  centre. Rounding each one separately can break the sum. So it rounds all three, then
  recomputes the component that moved furthest from its fractional value from the other two,
  which makes the sum 0 again.
- **`floor` for the layer:** the position is the bottom face, so every point from that face up to
  the next layer belongs to that voxel.
- This direction always yields a valid coordinate, so it can't fail and returns a plain
  `VoxelCoord`, not a `Result`.

## Units

- 1 shaku = 10 sun = 10/33 m exactly (the Meiji definition).
- The game's base unit is the shaku. A separate `units` module will convert shaku ↔ metres.

## Neighbours

**Edge neighbours** share a face: six of them, 1 shaku away, 1 step each. They are the
permutations of `(+1, −1, 0)`.

**Corner neighbours** touch at a single point: six of them, √3 shaku away, 2 steps each. They are
the permutations of `(+2, −1, −1)`. Each one is the sum of the two edge directions on either side
of it, so the line to a corner neighbour passes between the two edge neighbours that flank it.

Together they give 12 directions, evenly spaced every 30°, alternating edge and corner.

**Indexing follows Bevy's rotation convention.** Direction `k` points along
`Quat::from_rotation_y(k · 30°)` applied to +X. A positive rotation about Y turns +X toward −Z,
which is anticlockwise when looking straight down with +X to the right and −Z up the screen. So
index 0 is +X, the indices run anticlockwise on screen, edges are the even indices and corners the
odd ones, and facing an entity along direction `k` needs no conversion.

| k | Angle | On screen | Kind | Axial `(q, r)` | Cube `(q, r, s)` | Distance (shaku) | Flanked by |
|---|---|---|---|---|---|---|---|
| 0  | 0°   | right                | edge   | `( 1,  0)` | `( 1,  0, −1)` | 1  | |
| 1  | 30°  | right, a little up   | corner | `( 2, −1)` | `( 2, −1, −1)` | √3 | 0 and 2 |
| 2  | 60°  | up-right             | edge   | `( 1, −1)` | `( 1, −1,  0)` | 1  | |
| 3  | 90°  | straight up          | corner | `( 1, −2)` | `( 1, −2,  1)` | √3 | 2 and 4 |
| 4  | 120° | up-left              | edge   | `( 0, −1)` | `( 0, −1,  1)` | 1  | |
| 5  | 150° | left, a little up    | corner | `(−1, −1)` | `(−1, −1,  2)` | √3 | 4 and 6 |
| 6  | 180° | left                 | edge   | `(−1,  0)` | `(−1,  0,  1)` | 1  | |
| 7  | 210° | left, a little down  | corner | `(−2,  1)` | `(−2,  1,  1)` | √3 | 6 and 8 |
| 8  | 240° | down-left            | edge   | `(−1,  1)` | `(−1,  1,  0)` | 1  | |
| 9  | 270° | straight down        | corner | `(−1,  2)` | `(−1,  2, −1)` | √3 | 8 and 10 |
| 10 | 300° | down-right           | edge   | `( 0,  1)` | `( 0,  1, −1)` | 1  | |
| 11 | 330° | right, a little down | corner | `( 1,  1)` | `( 1,  1, −2)` | √3 | 10 and 0 |

A corner direction `k` is the sum of its flanking edges: `dir[k] = dir[k − 1] + dir[k + 1]`
(indices mod 12).

Bevy's forward, −Z, is direction 3: a corner. An entity walking an edge direction is never facing
Bevy's default forward, which is fine as long as models are rotated to face their heading.

Which of these an entity may actually move to is a movement rule, covered in `movement.md`.

## Build order

1. `VoxelCoord`, `new`, the getters and their tests.
2. Voxel ↔ world conversion and cube rounding.
3. Later: the `units` module, the direction tables, neighbours, distance.

## Open questions

- **World-space type:** Bevy's `Vec3`. Decided 2026-09-22: it is what every caller has in hand.
- **`units` API:** a library of conversions between shaku, ken, cho, ri, metres and centimetres,
  in any direction, so they are written once. Written when something first needs it. Whether it
  is plain `f32` functions or `Shaku` / `Metres` types the compiler won't let you mix up is
  decided then.
- **Where the module lives:** `crates/hexcoords/`, the crate `murabito_hexcoords`.

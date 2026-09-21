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
    valid, `s` fits in an `i32`, so `s()` can't overflow either.
  - The fields are private, so `new` is the only way to build a `VoxelCoord`. Rust's visibility
    rules enforce the invariant. Nobody has to remember it.
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

Together they give 12 directions, evenly spaced every 30°, alternating edge and corner. Looking
straight down, with angles running clockwise on screen from +X:

| Angle | On screen | Kind | Axial `(q, r)` | Cube `(q, r, s)` | Distance (shaku) | Flanked by |
|---|---|---|---|---|---|---|
| 0°   | right                | edge   | `( 1,  0)` | `( 1,  0, −1)` | 1  | |
| 30°  | right, a little down | corner | `( 1,  1)` | `( 1,  1, −2)` | √3 | 0° and 60° |
| 60°  | down-right           | edge   | `( 0,  1)` | `( 0,  1, −1)` | 1  | |
| 90°  | straight down        | corner | `(−1,  2)` | `(−1,  2, −1)` | √3 | 60° and 120° |
| 120° | down-left            | edge   | `(−1,  1)` | `(−1,  1,  0)` | 1  | |
| 150° | left, a little down  | corner | `(−2,  1)` | `(−2,  1,  1)` | √3 | 120° and 180° |
| 180° | left                 | edge   | `(−1,  0)` | `(−1,  0,  1)` | 1  | |
| 210° | left, a little up    | corner | `(−1, −1)` | `(−1, −1,  2)` | √3 | 180° and 240° |
| 240° | up-left              | edge   | `( 0, −1)` | `( 0, −1,  1)` | 1  | |
| 270° | straight up          | corner | `( 1, −2)` | `( 1, −2,  1)` | √3 | 240° and 300° |
| 300° | up-right             | edge   | `( 1, −1)` | `( 1, −1,  0)` | 1  | |
| 330° | right, a little up   | corner | `( 2, −1)` | `( 2, −1, −1)` | √3 | 300° and 0° |

Which of these an entity may actually move to is a movement rule, covered in `movement.md`.

## Build order

1. `VoxelCoord`, `new`, the getters and their tests.
2. Voxel ↔ world conversion and cube rounding.
3. Later: the `units` module, the direction tables, neighbours, distance.

## Open questions

- **Direction indexing:** is there a numbered direction table, and if so where does index 0
  start and which way does it go? The table above is only a listing, not an agreed order.
- **World-space type:** plain `(f32, f32, f32)`, a small struct of our own, or Bevy's `Vec3`? The
  first two keep the module free of Bevy.
- **`units` API:** plain `f32` conversion functions, or `Shaku` / `Metres` types that the compiler
  won't let you mix up?
- **Where the module lives:** this waits on the crate setup.

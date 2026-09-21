# Hex coordinates and units

A brief on the voxel coordinate system for the main project, as agreed in design.
Nothing here is implemented yet.

## Voxels

The world is made of hexagonal prisms (voxels): pointy-top hexes stacked in layers.

- **Voxel space** is integer. One voxel is 1 wide (flat to flat) and 1 tall.
- **World space** is `f32`, in **shaku**, with **Y up**.
  One voxel is 1 shaku flat to flat and 5 sun (0.5 shaku) tall.

## `VoxelCoord`

```rust
pub struct VoxelCoord { q: i32, r: i32, layer: i32 }   // fields private
```

- **Stores axial** `(q, r)` plus `layer`. `s` is never stored.
- **Takes cube input, validated:** `VoxelCoord::new(q, r, s, layer) -> Result<VoxelCoord, NotOnHexPlane>`
  checks `q + r + s == 0`. The sum is taken in `i64`, so extreme values return `Err` instead of
  overflowing. Private fields mean `new` is the only way to build one.
- **Gives cube output, reconstructed:** `q()`, `r()`, `s()` (as `-q - r`), `layer()`.
- `NotOnHexPlane { q, r, s }` carries the rejected values.
- Derives `Clone, Copy, Debug, PartialEq, Eq, Hash`, so it can key a map.

## Orientation

- Pointy-top: rows run along X, corners point along ±Z.
- `+q` points toward +X.
- `+r` points toward +Z, which is toward the bottom of the screen for a camera looking straight
  down with +X to the right. One `+r` step goes down-right on screen.
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

`cube_round` rounds all three components, then recomputes the one that moved furthest from its
fractional value from the other two, so the sum is 0 again. `floor` is used for the layer
because any point inside a prism, not only on its bottom face, belongs to that prism. This
direction always yields a valid coordinate, so it can't fail.

## Units

- 1 shaku = 10 sun = 10/33 m exactly (the Meiji definition).
- The game's base unit is the shaku. A separate `units` module will convert shaku ↔ metres.

## Neighbours and movement

**Edge neighbours** share a face. There are six, at 1 shaku, 1 step each: the permutations of
`(+1, −1, 0)`.

**Corner (diagonal) neighbours** touch at a single corner. There are six, at √3 shaku, 2 steps
each: the permutations of `(+2, −1, −1)`:

```
(2,−1,−1)  (1,1,−2)  (−1,2,−1)  (−2,1,1)  (−1,−1,2)  (1,−2,1)
```

Each diagonal is the sum of two adjacent edge directions: `diagonal_k = dir_k + dir_(k+1)`.

**12-direction movement (under consideration):** an entity may move to a corner neighbour when
both flanking faces are unobstructed, i.e. both `from + dir_k` and `from + dir_(k+1)` are open.
An edge step costs 1 and a diagonal step costs √3.

**A\* heuristic** for 12-direction movement, the hex analogue of octile distance: take the
displacement's cube components, sort their absolute values into `lo ≤ mid ≤ hi`, and

```
cost = √3 · lo + (mid − lo)
```

This is exact on open ground and never overestimates with obstacles, so it's admissible.
(`hi` drops out, since it's always `lo + mid`.) Plain hex step count isn't admissible once
diagonals exist, because two steps can cost √3 < 2.

## Build order

1. `VoxelCoord`, `new`, the getters and their tests.
2. Voxel ↔ world conversion and cube rounding.
3. Later: the `units` module (shaku ↔ metres), `DIRECTIONS` / `DIAGONALS`, neighbours, distance.

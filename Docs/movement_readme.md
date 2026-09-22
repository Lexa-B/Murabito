# Movement

A brief on how entities move across the hex voxel grid, as agreed in design. The coordinates,
directions and neighbours it builds on are in `hex_units_readme.md`. What is implemented is marked;
the rest is design.

**Compass invariant: east is +X, north is −Z, up is +Y.** Directions across the plane are compass
points; up and down are for gravity, so stepping up is a change of layer, never a heading. The same
statement is in `hex_units_readme.md` and `AGENTS.md`.

## Where a body is, and which way it faces

Implemented, in `murabito_movement`.

- `VoxelPosition(VoxelCoord)`: the voxel an entity is in. Integer, and it changes only when a
  step lands: movement is by whole voxels, and an entity is in exactly one cell.
- `Facing(Direction)`: one of the twelve compass directions.
- `place` writes the entity's `Transform` from the two: the centre of the voxel's bottom face,
  turned to `Direction::heading()`. It is the only thing that writes a moving entity's
  `Transform`, and it runs only when the position or facing changed. `VoxelPosition` requires a
  `Transform`, so an entity gets one without asking.

These are plain components any entity can carry: nothing in them says what the entity is.

## 12-direction movement

Entities move in 12 directions: the 6 edge moves plus the 6 corner moves. They are
`murabito_hexcoords::Direction`, named by compass point, anticlockwise from east:

| Edges (1 shaku, cost 1) | `E` | `NNE` | `NNW` | `W` | `SSW` | `SSE` |
|---|---|---|---|---|---|---|
| **Corners (√3 shaku, cost √3)** | `ENE` | `N` | `WNW` | `WSW` | `S` | `ESE` |

Each corner sits between the two edges beside it in the table's order (`N` between `NNE` and
`NNW`); `Direction::flanks()` gives those two, and `is_edge()` / `is_corner()` tell them apart. The
full table, with angles and axial offsets, is in `hex_units_readme.md`.

**A corner move is allowed only when both flanking faces are unobstructed**, i.e. both edge
neighbours it passes between are open. Otherwise an entity could slip diagonally between two
blocked hexes. A corner direction is the sum of its two flanks, so the check is on
`from.neighbour(before)` and `from.neighbour(after)` where `(before, after) = dir.flanks()`.

## Stepping up

How many layers an entity can step up in one move depends on its size:

| Size | Step up | Height (layers are 5 sun) |
|---|---|---|
| Normal | 2 layers | 1 shaku |
| Small  | 1 layer  | 5 sun |

## Cost

| Move | Cost | Why |
|---|---|---|
| Edge   | 1  | 1 shaku to a face neighbour |
| Corner | √3 ≈ 1.732 | √3 shaku to a corner neighbour, cheaper than the 2 it would take in two edge moves |

## A\* heuristic

With corner moves, plain hex step count overestimates: two steps can cost √3 < 2. That breaks
A*'s guarantee of finding the shortest path. Square grids with diagonals have the same problem
and solve it with octile distance. The hex equivalent is:

take the displacement's cube components, sort their absolute values into `lo ≤ mid ≤ hi`, and

```
cost = √3 · lo + (mid − lo)
```

That's `lo` corner moves plus `mid − lo` edge moves. `hi` drops out, since it's always
`lo + mid`. Example: `(3, 1, −4)` costs `√3 · 1 + (3 − 1) ≈ 3.73`.

It's exact on open ground, and obstacles can only lengthen a path, so it never overestimates.
That makes it admissible, and tight enough that A* doesn't waste time exploring off to the sides.

## Open questions

- **Stepping down:** is there a limit on how far an entity can step or drop down, and is it by
  size too?
- **Stepping on corner moves:** can a corner move also step up? If so, which layers must the two
  flanking hexes have open?
- **Sizes:** are normal and small the only sizes, and what decides which one an entity is?
- **Obstruction and entity height:** a corner move needs both flanking faces open, but on which
  layers? Only the entity's own layer, or every layer it occupies if it's taller than one voxel?
- **Cost of climbing:** does stepping up cost extra on top of the move's horizontal cost? The A*
  heuristic stays admissible either way, since it counts only horizontal distance and climbing can
  only add to a path's cost.

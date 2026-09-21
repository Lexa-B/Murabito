# Movement

A brief on how entities move across the hex voxel grid, as agreed in design.
Nothing here is implemented yet. The coordinates, directions and neighbours it builds on are in
`hex_units.md`.

## 12-direction movement

Entities move in 12 directions: the 6 edge moves plus the 6 corner moves (see the direction
table in `hex_units.md`).

**A corner move is allowed only when both flanking faces are unobstructed**, i.e. both edge
neighbours it passes between are open. Otherwise an entity could slip diagonally between two
blocked hexes. A corner direction `k` (odd) is the sum of its flanking edge directions,
`dir[k] = dir[k − 1] + dir[k + 1]`, so the check is on `from + dir[k − 1]` and `from + dir[k + 1]`.

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

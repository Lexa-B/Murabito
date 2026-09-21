# Movement

A brief on how entities move across the hex voxel grid, as agreed in design.
Nothing here is implemented yet. The coordinates, directions and neighbours it builds on are in
`hex_units.md`.

## 12-direction movement

Entities move in 12 directions: the 6 edge moves plus the 6 corner moves (see the direction
table in `hex_units.md`).

**A corner move is allowed only when both flanking faces are unobstructed**, i.e. both edge
neighbours it passes between are open. Otherwise an entity could slip diagonally between two
blocked hexes. Each corner direction is the sum of its two flanking edge directions,
`corner = a + b`, so the check is on `from + a` and `from + b`.

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

- **Movement between layers:** 12-direction movement is defined within one layer. Can entities
  step up or down layers, how far, and can a move change layer and go diagonally at once?
- **Obstruction and entity height:** a corner move needs both flanking faces open, but on which
  layers? Only the entity's own layer, or every layer it occupies if it's taller than one voxel?

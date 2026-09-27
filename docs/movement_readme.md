# Movement

A brief on how entities move across the hex voxel grid, as agreed in design. The coordinates,
directions and neighbours it builds on are in `hex_units_readme.md`. What is implemented is marked;
the rest is design.

**Compass invariant: east is +X, north is −Z, up is +Y.** Directions across the plane are compass
points; up and down are for gravity, so stepping up is a change of layer, never a heading. The same
statement is in `hex_units_readme.md` and `AGENTS.md`.

## Where a body is, and which way it faces

Implemented, in `murabito_placement`: these are a property of being a thing in the world, not of
moving, so they live below the mechanism that changes them, where the senses can read them too.

- `VoxelPosition(VoxelCoord)`: the voxel an entity is in. Integer, and it changes only when a
  step lands: movement is by whole voxels, and an entity is in exactly one cell.
- `Facing(Direction)`: one of the twelve compass directions.
- `place` writes the entity's `Transform` from the two: the centre of the voxel's bottom face,
  turned to `Direction::heading()`. It is the only thing that writes a placed entity's
  `Transform`, and it runs only when the position or facing changed. `VoxelPosition` requires a
  `Transform`, so an entity gets one without asking.

These are plain components any entity can carry: nothing in them says what the entity is.

## Stepping

Implemented, in `murabito_movement`, on the one accumulation bar of `actions_readme.md`.

- `Locomotion { speed, turn_speed }`: shaku per second and degrees per second. A body's physical
  properties, so they live here; it requires a `Progress`.
- `Step { direction, gait, reach }` is the intent; `Step::walk(direction)` is one voxel at
  walking pace. Put it on a body and the `step` system, in `FixedUpdate`, carries it out:
  `Progress::start::<Step>(cost)`, then `advance(speed × gait × tick)` each tick, and on
  reaching the cost the body is `reach` neighbours along the way, faces as the landing rule
  says, and the `Step` is gone. Movement is by whole voxels: mid-step the body is still in the
  voxel it left from, and a lunge (reach 2) is never in the cell between.
- `cost(direction)` is 1 for an edge, √3 for a corner: the shaku walked; `Step::cost` is that
  times the reach.
- `Gait` is a multiple of `Locomotion::speed`: `Sneak` ½, `Walk` 1, `Jog` 2, `Sprint` 3. A
  property of the step, not the body, and seen by nothing above this crate: the words in
  `actions_readme.md` fix it.
- Any of the twelve directions is a legal step. `Way::of(facing, direction)` says which way it
  goes: `Forward` is the way faced or a notch either side, `Rear` the way behind or a notch
  either side, `Lateral` the six between. `landing(facing, direction)` is the facing after: the
  way gone turned back toward the old facing by 0, 3 or 6 notches, so a forward step lands
  facing the way it went, a lateral one orthogonal to it and a rear one opposite, each on the
  side of the old facing. No landing turns the body more than one notch: that notch is the free
  adjustment every step gets. Whether a *word* may go a way is the actions layer's rule, not
  this one's.
- Since progress is in shaku and carries between steps of the same kind, a run of steps lands when
  the total distance says, not each step rounded up to a tick; a tick spent standing still
  forgets the head start.

## Turning

Implemented, in `murabito_movement`, on the same bar.

- `Turn(Direction)` is the intent. The `turn` system carries it out a notch of 30° at a time:
  each notch is its own action on the bar, `start::<Turn>(30)` then `advance(turn_speed × tick)`,
  and on reaching it `Facing` moves one notch toward the target, the short way round, so a wide
  swing passes through every direction between. Exactly opposite is the one tie, and goes
  anticlockwise. The `Turn` is gone once the body faces the target; a `Turn` to the way already
  faced is done on its first tick. Turning never moves the body.
- Leftover degrees carry from one notch to the next, and from one `Turn` straight into another,
  exactly as shaku carry between steps: a wide swing takes the ticks its total angle says.
- At most one notch is taken per tick, so a turn speed above 1920°/s is capped at that.

## 12-direction movement

The directions and their costs are implemented; the corner-obstruction rule at the end of this
section is design, since nothing is yet an obstacle.

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

Design. How many layers an entity can step up in one move depends on its size:

| Size | Step up | Height (layers are 5 sun) |
|---|---|---|
| Normal | 2 layers | 1 shaku |
| Small  | 1 layer  | 5 sun |

## Cost

Implemented: `murabito_movement::cost`.

| Move | Cost | Why |
|---|---|---|
| Edge   | 1  | 1 shaku to a face neighbour |
| Corner | √3 ≈ 1.732 | √3 shaku to a corner neighbour, cheaper than the 2 it would take in two edge moves |

## A\* heuristic

Design. With corner moves, plain hex step count overestimates: two steps can cost √3 < 2. That breaks
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

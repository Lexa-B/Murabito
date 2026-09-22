# Perception

A brief on how a body reads the world, as agreed in design on 2026-09-22 and 2026-09-23. What is
implemented is marked; the rest is design. The archive's `senses/` (`_Archives/Bevy-Try-1/src/senses/`)
is the reference this was rebuilt from, taken apart and put back under the crate-per-module rules.

**Compass invariant: east is +X, north is −Z, up is +Y**, as everywhere; a sense reports where a
thing is relative to the perceiver, in cube coordinates, never a heading.

## The shape of it

Perception is body, not brain. A sense produces its own list, in its own shape, each tick; nothing
in this layer merges the lists, and nothing decides. Merging, if it ever happens, is the AI's call.

```
crates/perception/                 a group directory, not a crate
├─ perception/                     murabito_perception: what every sense shares
└─ senses/
   ├─ vision/                      murabito_vision: sight        implemented
   ├─ hearing/                     design
   └─ smell/                       design
```

Each sense's list has the shape that sense can honestly give:

| Sense | One entry says | Status |
|---|---|---|
| vision | *this thing*, at *this offset* from me, seen *this well* (`Sighting`) | implemented |
| hearing | *something*, from *this bearing*, *this loud*: a direction and an intensity, no position | design |
| smell | *this strong here*, and *stronger that way by this much*: an intensity and a gradient, no bearing | design |
| others (spiritual pressure, …) | whatever that sense gives; each is its own crate and its own list | design |

## What every sense shares: `murabito_perception`

Implemented.

- **`Occupancy`**: which things stand in which voxel, rebuilt whole every tick from every
  `VoxelPosition`, so nothing registers or unregisters. A body is in exactly one voxel; a voxel may
  hold several things. To sight it is the blocker map and the lookup from a seen cell to what was
  seen; to hearing and smell it will be what attenuates. It knows nothing about any sense.
- **`PerceptionSet`**, in `FixedUpdate`: `Gather` rebuilds the map after `MechanismSet`, then `Sense`,
  where every sense's own system runs. A step that lands on a tick is in the map and seen on that
  tick. Whatever acts on what was sensed orders itself after `PerceptionSet::Sense`.
- Perceiving is simulation: it ticks at 64 Hz with the rest and freezes with it. Chosen over a
  per-frame `Update` (the archive's way), which would see while paused and could act on a
  frame-old view.

## Sight: `murabito_vision`

Implemented.

- **`Vision`** is a member of `Sentient`: a cone centred on `Facing`, `arc` degrees wide, with three
  **bands** (`Near`, `Mid`, `Far`), each a `range` in whole shaku and a `sensitivity` for tuning.
  A species puts its own cone in its own `require`, or `Vision::BLIND` if it has none; the tier's
  cone is a placeholder. The fox's is a hunter's (120°, to 12/30/48 shaku), the hare's is prey's
  (240°, to 8/18/26): the first attempt's placeholder tuning.
- **The cast** is shadowcasting, ring by ring outward from the eye's voxel, on the eye's layer.
  Every occupied cell other than the eye's own is a blocker: it casts a shadow across the angle its
  six corners subtend, and shadows raised at one ring darken only the rings beyond, which keeps the
  walk linear in cells. A cell is judged on its centre. A blocker is itself in view; one standing in
  shadow casts nothing. The cells in view are the crate's private intermediate: **the field is not
  public**, and is opened only if debugging comes to need it.
- **`Seen`** is the output, required by `Vision`: the list of `Sighting { entity, offset, acuity }`
  this tick, `offset` being the thing's voxel minus the looker's. Two things in one cell are two
  sightings; the looker never sees itself; a thing sharing its cell is seen `Near` at no offset.
  Replaced whole each tick; empty for a blind body.
- **Everything is opaque.** Any thing in a voxel stops sight through it, a fox as much as a tree.

### Debts, taken knowingly

Facets, what a thing is *like*, are not back yet. When they are, these land in `murabito_perception`,
since they apply to every sense's list alike:

- **Obscuring and heights.** The archive's 視界 (透明 / 半透明 / 不透明) and 高さ (膝丈 / 腰丈 /
  背丈): grass costs a band of acuity instead of stopping sight, and knee-high grass hides a hare and
  not a person. An occluder hides whatever is no taller than itself.
- **Ambiguation.** A poorly seen thing reads as something vaguer: a 妖狐 in human form at `Mid` is a
  humanoid. The percept becomes something short of an `Entity`, in perception, not the AI.
- **Layers.** The cast is planar on the eye's layer; the offset already carries `dlayer`.
- **Gating on change.** The cast runs every tick for every sighted body (about 8,400 cells for the
  fox's 48-shaku cone, 64 times a second). The baseline to beat: recast only when the looker's
  position or facing changed, or when anything's position changed.

## Open questions

- **Hearing's propagation:** loudness falls with the shortest *unobstructed* path (diffraction), which
  needs a path search over `Occupancy`, not a cast.
- **Smell's state** lives in the world, not the perceiver: a field that spreads, drifts and persists
  after its source has gone. The first sense with state of its own.
- **Where the overlay lives.** The cone and sightlines are drawn by `murabito_scene` as a placeholder,
  with the progress bar; a debug module owns them once one exists.

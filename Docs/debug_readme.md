# Debug

A brief on the debug build: how the running world is read from outside the process, as agreed
in design on 2026-09-24. Implemented as described; what is still design is marked.

## The shape of it

The debug build is the everyday build plus one Cargo feature, `debug`, off by default:

```
cargo run -p murabito --features debug     or     scripts/run.sh --debug
```

With it on, the app serves Bevy's remote protocol (JSON-RPC over HTTP) on `127.0.0.1:15702`,
and every module that put its types on the wire is readable by type path. Without it, nothing
exists: no derives, no registry entries, no server, no HTTP stack in the binary. The everyday
build and `cargo test --workspace` never turn it on, so they never rebuild the engine; the
debug build is a second engine build that lives beside the first in `target/`.

The reading is done by whatever is outside: `curl` and `jq`, `scripts/probe.sh`, and later a
page in a browser (the response headers already allow one). The game gains no debug UI.

## Who puts what on the wire

Each crate owns its part, behind its own `debug` feature, which forwards to the features of the
crates whose types it contains (vision's `Sighting` holds an `Offset`, so vision's feature turns
on hexcoords'). The app's `debug` feature turns on all of them and the server. The pattern:

- one attribute line per type, `#[cfg_attr(feature = "debug", derive(Reflect), reflect(Component))]`
  (or `reflect(Resource)`; plain `derive(Reflect)` for a type that is neither);
- the crate's plugin registers its own types, under `#[cfg(feature = "debug")]`. Registering a
  type registers the types of its fields, so a crate with no plugin (hexcoords) registers nothing;
- a feature-on test pins what the type looks like through the serializer the server uses.

| Crate | On the wire |
|---|---|
| `murabito_hexcoords` | `VoxelCoord`, `Offset`, `Direction` |
| `murabito_placement` | `VoxelPosition`, `Facing` |
| `murabito_identity` | `ThingId`, `NextThingId` |
| `murabito_perception` | `Occupancy` |
| `murabito_vision` | `Vision`, `Band`, `Acuity`, `Seen`, `Sighting` |
| `murabito_kinds` | every node of the tree, and `Model` |
| `murabito_debug` | nothing: the server, and no dependency on any module |

Not yet: `Progress`, `ActionQueue`, the camera. Each is the same two lines when wanted.

## The wire is a mirror of memory

Lexa's rule: a type on the wire looks exactly as it does in memory, so that an odd shape in the
data is seen and not smoothed over. So:

- **A voxel is axial on the wire**: `{q, r, layer}`. `s` is not a field, it is `−q − r` computed
  on demand, so it does not appear. Same for `Offset`: `{dq, dr, dlayer}`.
- **A one-field tuple struct is flattened** to its field by Bevy's serializer: `Seen` is the
  list itself, `ThingId` the bare number, `VoxelPosition` the voxel object.
- **An entity is one number**, its bits, the same in a query result's `entity` and inside a
  `Sighting` or `Occupancy`.
- **A kind's name is its full module path**, `murabito_kinds::all_things::tangible::…::fox::Fox`,
  which is its place in the tree.
- **The one exception, `Occupancy`**: JSON can't key an object by a struct, so the map is a list
  of its entries, each `[voxel, things]`, in the map's own order, which is none. A hand-written
  `Serialize` in `murabito_perception`, the only one.

A sighting, as the fox's `Seen` shows it:

```json
{"acuity": "Near", "entity": 4294967279, "id": 2, "offset": {"dlayer": 0, "dq": 2, "dr": 0}}
```

## What is closed

- **The cast's field**, the cells vision finds in view, stays private (Lexa, 2026-09-24). What
  the AI consumes is `Seen`; the field answers "why doesn't the fox see the hare from here",
  a question about vision's correctness that the vision crate's tests answer. If it is ever
  wanted, it is a `Field` component that `look` writes beside `Seen` in debug builds only.
- **The mutating verbs** (`world.insert_components`, `world.mutate_*`, `world.despawn_entity`,
  …) are served too: Bevy's plugin can only add methods, never drop one. The server binds to
  the loopback address only, so only this machine can call them.

## Reading it

The server answers whatever the last completed frame left, whether or not a tick ran in it,
so a paused world (Space) answers the same as a running one, frozen. `scripts/probe.sh` asks
every half second and redraws:

```
scripts/probe.sh              every thing by id and kinds, with place, facing, cone and seen list
scripts/probe.sh occupancy    the map of what stands where
scripts/probe.sh types        every component type the server knows
scripts/probe.sh <method> '<params json>'
```

The `things` view's `kinds` list is the script's own join (a query can't ask for "whatever
kinds it has"), added beside the components as they came; everything else is the wire.

## Design

- **A web page.** The next client: a page that polls or watches (`+watch` methods) and draws
  the field of things, `Seen` as lines, `Occupancy` as cells, with the raw JSON beside it.
  Lexa's stated want; the CORS headers are already on.
- **A tick-by-tick view.** The server sees a frame's end, never the middle of a tick; showing
  every step of a tick in order needs either a step-one-tick control or a per-step trace the
  server can read. Its own design conversation (`TODO.md`).
- **Selection.** Clicking a thing in the window to name it in the probe, once a selection
  module exists.

# Crate manifest

Every crate in the main project's workspace, and what each one is for. One crate per
module: its `pub` items are its whole API and its `[dependencies]` its whole wiring, so
this tree is also the map of what may know about what. `Docs/HANDOFF.md` lists each
crate's public items; a crate's `src/lib.rs` doc comment is the authority on its design.

```
crates/
├─ murabito/              the app
├─ scene/                 murabito_scene
├─ camera/                murabito_camera
├─ keybinds/              murabito_keybinds
├─ hexcoords/             murabito_hexcoords
├─ placement/             murabito_placement
├─ user_data/             murabito_user_data
├─ settings/              murabito_settings
├─ i18n/                  murabito_i18n
├─ app_state/             murabito_app_state
├─ ui/                    the overlays: a group directory, not a crate
│  ├─ kit/                murabito_ui
│  ├─ navigation/         murabito_navigation
│  ├─ menu/               murabito_menu
│  └─ settings_page/      murabito_settings_page
├─ kinds/                 murabito_kinds
├─ action/                the actions layer: a group directory, not a crate
│  ├─ actions/            murabito_actions
│  ├─ progress/           murabito_progress
│  └─ mechanisms/         one crate per kind of thing a body can do
│     └─ movement/        murabito_movement
└─ perception/            the senses: a group directory, not a crate
   ├─ perception/         murabito_perception
   └─ senses/             one crate per sense, each with its own list in its own shape
      └─ vision/          murabito_vision
```

Arrows in the dependency graph all point down, toward whoever owns a type. The app
depends on every plugin crate and is the only thing that depends on `murabito_settings`.

## The app

- **`murabito`** — the one binary. A plugin list, the `.persist::<T>("key")` line per
  settings resource, and nothing else.

## The app's state

- **`murabito_app_state`** — `AppState`: `Playing` or `Paused`, whether the world runs.
  Not which screen is up: that is the UI's own state, so a new screen never touches this
  crate. Pausing is one call on `Time<Virtual>`, which freezes the whole `FixedUpdate`
  simulation while `Time<Real>` keeps running underneath.

## Overlays

- **`murabito_ui`** — the look and the parts every overlay is built from: the dimmed
  frame over the world, a button whose label is a `Localized` key, a heading, a settings
  row, a slider with a readout, the UI font, and the tinting that answers the pointer.
  Knows no screen and no action.
- **`murabito_navigation`** — `Overlay`: what is laid over the paused world (`None`,
  `Menu`, `Settings`), a sub-state of `AppState::Paused` so it exists only while the
  world is held still and vanishes when it resumes. Owns the back key (Escape). The
  overlays set it and never import each other.
- **`murabito_menu`** — the menu over the paused world: Settings, Resume, Quit. Built
  on entering `Overlay::Menu`, torn down on leaving it however it is left.
- **`murabito_settings_page`** — one row per setting and a way back: the language
  picker and the pan-speed slider. Edits go straight into the resources their modules
  own; saving is `murabito_settings`' doing. By nature the crate that knows every
  setting's owner.

## The world and how it is seen

- **`murabito_scene`** — the placeholder world: a ground one cho square, a sun, the sky
  colour and ambient light, a `Fox` from `murabito_kinds` at the origin walking a
  twelve-sided loop, and a `Hare` six shaku east walking a triangle with a rest at each
  corner, with a progress bar drawn in front of whatever is busy (the bar belongs to a UI
  module once one exists). Where a thing stands and which way it starts off facing is the
  scene's business; what it is, is the kind's.
- **`murabito_camera`** — the overhead camera as a rig (focus, direction, zoom) from
  which one system derives the transform; eased pan and zoom, pan speed following zoom.
  Owns `CameraSettings`: the speed multiplier and the four pan binds.
- **`murabito_hexcoords`** — hex voxel coordinates: cube `(q, r, s)` plus a layer,
  `VoxelspacePos`, the twelve compass `Direction`s with their neighbour and rotation
  maths, and `Offset`, one voxel relative to another, with distance in steps, rings
  outward and a cell's corners. Where a cell is, not what is in it. Design:
  `Docs/hex_units_readme.md`.
- **`murabito_placement`** — where a thing stands (`VoxelPosition`) and which way it
  faces (`Facing`), and `place`, the one system that keeps a model where they say. Plain
  components any entity in the world carries: a tree has a position and never moves.
  The movement mechanism writes them; the senses read them; neither knows the other.

## What things are

- **`murabito_kinds`** — the tree of kinds: what a thing *is*. Every tier and every kind
  is a unit component whose `#[require]` is its parent and its members, so spawning a
  kind inserts the whole chain and the node itself says what it has. A species' numbers
  sit in its own `require` and win over its tiers'. One file per node, in folders that
  mirror the tree. A kind that is drawn names its file with `Model`, and `KindsPlugin`'s
  one observer loads it as the thing is spawned. Depends on the mechanism crates whose
  components the tiers require; only what spawns things depends on it.

## Doing things

- **`murabito_actions`** — the `ActionQueue`: what a body has been asked to do, in
  order, with nothing in it saying who asked. The one system that takes the head of the
  queue and issues a mechanism's intent for it, after `AskingSet`, where whatever pushes
  runs. Design: `Docs/actions_readme.md`.
- **`murabito_progress`** — the one accumulation bar per entity that every sustained
  action fills, in the mechanism's own units (shaku, degrees), and the `MechanismSet`
  the mechanisms tick in. Knows no mechanism.
- **`murabito_movement`** — the movement mechanism: how fast a body goes
  (`Locomotion`), and the `Step` and `Turn` intents it carries out on `FixedUpdate`,
  writing `murabito_placement`'s position and facing. Design: `Docs/movement_readme.md`.

## Reading the world

- **`murabito_perception`** — what every sense shares and no sense owns: `Occupancy`,
  which things stand in which voxel, rebuilt every tick; and `PerceptionSet`, when the
  senses run in `FixedUpdate` (`Gather` the map after the mechanisms, then `Sense`).
  Where the facet debts land when facets return: obscuring, heights, ambiguation.
  Design: `Docs/perception_readme.md`.
- **`murabito_vision`** — sight: `Vision`, a cone on `Facing` with acuity in three
  bands, a member of `Sentient`; the cast, shadowcasting over `Occupancy` with
  everything opaque, whose field of cells is private; and `Seen`, the per-tick list of
  `Sighting`s, each a thing, its offset from the looker and how well it was seen.

## Input, settings and text

- **`murabito_keybinds`** — `Binds`: up to three keys or mouse buttons for one action,
  and `Inputs`, the system parameter a system reads them through. Owns the type and
  knows no actions; each module keeps its own `Binds` in its settings. One word per
  bind in a file (`KeyW`, `Mouse7`).
- **`murabito_user_data`** — the player's directory, `~/.config/murabito`, and the only
  place that decides where it is. Every file the player accumulates is a filename
  joined onto it.
- **`murabito_settings`** — `settings.yaml` in that directory, one top-level key per
  resource the app registers. Loaded as the app is built, written on the frame anything
  changes; a file or section that can't be read is backed up then replaced. Names no
  setting.
- **`murabito_i18n`** — `Language` (a setting, persisted) and `Localized`, the catalogue
  key a text entity carries; the compiled-in catalogues `assets/locales/{en,ja}.yaml`,
  and the systems that fill in the string and re-fill every text when the language
  changes.

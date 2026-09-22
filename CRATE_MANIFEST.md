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
├─ user_data/             murabito_user_data
├─ settings/              murabito_settings
├─ i18n/                  murabito_i18n
└─ action/                the actions layer: a group directory, not a crate
   ├─ actions/            murabito_actions
   ├─ progress/           murabito_progress
   └─ mechanisms/         one crate per kind of thing a body can do
      └─ movement/        murabito_movement
```

Arrows in the dependency graph all point down, toward whoever owns a type. The app
depends on every plugin crate and is the only thing that depends on `murabito_settings`.

## The app

- **`murabito`** — the one binary. A plugin list, the `.persist::<T>("key")` line per
  settings resource, and nothing else.

## The world and how it is seen

- **`murabito_scene`** — the placeholder world: a ground one cho square, a sun, the sky
  colour and ambient light, and a fox walking a twelve-sided loop with a progress bar
  drawn in front of it. Content that belongs to world objects and a UI module once those
  exist.
- **`murabito_camera`** — the overhead camera as a rig (focus, direction, zoom) from
  which one system derives the transform; eased pan and zoom, pan speed following zoom.
  Owns `CameraSettings`: the speed multiplier and the four pan binds.
- **`murabito_hexcoords`** — hex voxel coordinates: cube `(q, r, s)` plus a layer,
  `VoxelspacePos`, and the twelve compass `Direction`s with their neighbour and rotation
  maths. Where a cell is, not what is in it. Design: `Docs/hex_units_readme.md`.

## Doing things

- **`murabito_actions`** — the `ActionQueue`: what a body has been asked to do, in
  order, with nothing in it saying who asked. The one system that takes the head of the
  queue and issues a mechanism's intent for it. Design: `Docs/actions_readme.md`.
- **`murabito_progress`** — the one accumulation bar per entity that every sustained
  action fills, in the mechanism's own units (shaku, degrees), and the `MechanismSet`
  the mechanisms tick in. Knows no mechanism.
- **`murabito_movement`** — the movement mechanism: where a body is (`VoxelPosition`),
  which way it faces (`Facing`), how fast it goes (`Locomotion`), and the `Step` and
  `Turn` intents it carries out on `FixedUpdate`. Design: `Docs/movement_readme.md`.

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

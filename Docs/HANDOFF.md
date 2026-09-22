# Handoff — the main project

Written 2026-09-22, after PR #25 (movement) merged; brought up to date the same day with the
settings and i18n crates, then with app state, then with the overlays and the settings page,
then with the kinds. Everything here is on `main`; nothing is outstanding. `AGENTS.md` is the
authority on how to work; this is where things stand, for a session starting cold.

## What runs

`cargo run -p murabito`, or `scripts/run.sh` (what the desktop shortcut points at): a green
ground one cho square under a pale sky, lit by a sun, and a fox at the origin walking a
twelve-sided loop, facing the way it goes, corner steps visibly slower than edge steps, with a
progress bar filling on the ground in front of it once per step. WASD/arrows pan, the wheel
zooms, both eased; the camera's feel numbers are the ones settled by feel-testing in the first
attempt. Six shaku east, a hare walks a triangle: three steps, a second's rest, a third of a
turn, and again. Space pauses: the fox freezes mid-step and the camera stops taking input; Space again
resumes. Escape opens the menu over the paused world (Settings / Resume / Quit, in the UI's
font, in English or Japanese); Escape again, or Resume, resumes. Settings is a page with a
pan-speed slider (a quarter speed to six times, in octaves, with a readout) and a language picker
that relabels everything in place. The camera's settings (speed multiplier, pan binds), the
pause and back keys and the UI language persist to `~/.config/murabito/settings.yaml`, which is
hand-editable.

## The crates

Every arrow in the dependency graph points down; `crates/action/` is a group directory, not a
crate, and reads the same way.

| Crate | Directory | Job | Public |
|---|---|---|---|
| `murabito` | `crates/murabito` | the app: a plugin list | |
| `murabito_scene` | `crates/scene` | ground, sun, sky, ambient light; spawns a `Fox` and a `Hare` from the kinds with a position and a facing; refills the fox's queue with the twelve-direction loop and walks the hare's triangle with a rest at each corner (its own timer); draws the progress bar (placeholder until a UI module owns it) | `ScenePlugin` |
| `murabito_kinds` | `crates/kinds` | the tree of kinds: every tier and kind a unit component whose `#[require]` is its parent and members; one file per node in folders that mirror the tree; `Model`, the glTF path a kind names, loaded by the plugin's one observer; the tiers, seventeen animals and twelve plants | every kind, `Model`, `KindsPlugin` |
| `murabito_camera` | `crates/camera` | the overhead rig: focus, direction, zoom; eased pan and zoom, taking input only in `AppState::Playing`; settings | `CameraPlugin`, `CameraSettings` |
| `murabito_keybinds` | `crates/keybinds` | `Binds`, up to three keys or mouse buttons for one action; `Inputs`, the system parameter; a `Bind` is one word in a file (`KeyW`, `Mouse7`) | `Bind`, `Binds`, `Held`, `Inputs`, `MAX_BINDS`, `UnknownBind` |
| `murabito_user_data` | `crates/user_data` | the player's directory, `~/.config/murabito`; the only place that decides where it is | `UserData`, `UserDataPlugin` |
| `murabito_settings` | `crates/settings` | `settings.yaml` in that directory: the app registers each module's settings resource with `persist::<T>("key")`; loaded as the app is built, written on the frame anything changes; a file or section that can't be read is backed up to `settings-<moment>.bak` then replaced | `SettingsPlugin`, `Persist` |
| `murabito_app_state` | `crates/app_state` | `AppState`: `Playing` or `Paused`, whether the world runs and nothing about screens; pauses `Time<Virtual>` on the frame a transition lands, which freezes the whole `FixedUpdate` simulation; `AppStateSettings`, the pause key (Space) | `AppState`, `AppStateSettings`, `AppStatePlugin` |
| `murabito_ui` | `crates/ui/kit` | the look and the parts every overlay is built from: the dimmed frame, a button carrying whatever action component the screen chose and a `Localized` key, the font (Noto Sans JP, loaded before Startup), tinting that answers the pointer | `UiPlugin`, `UiFont`, `overlay`, `spawn_button`, `spawn_literal_button`, `spawn_heading`, `spawn_row`, `spawn_slider`, `spawn_readout`, `ButtonSelected`, `text_font`, `SCRIM`, `TEXT` |
| `murabito_navigation` | `crates/ui/navigation` | `Overlay` (`None`, `Menu`, `Settings`), a sub-state of `AppState::Paused`; where back leads, as one table; the back key (Escape) in `NavigationSettings` | `Overlay`, `NavigationSettings`, `NavigationPlugin` |
| `murabito_menu` | `crates/ui/menu` | Settings / Resume / Quit, built on entering `Overlay::Menu` and torn down on leaving it however it is left | `MenuPlugin` |
| `murabito_settings_page` | `crates/ui/settings_page` | one row per setting and Back: the language picker (each language named in its own script, the one in force marked) and the pan-speed slider; edits go straight into `Language` and `CameraSettings`, and the file follows | `SettingsPagePlugin` |
| `murabito_i18n` | `crates/i18n` | `Language` (a setting, persisted as `language: en`), `Localized` (the key a text entity carries), the compiled-in catalogues `assets/locales/{en,ja}.yaml`, live re-localising | `Language`, `Localized`, `I18nPlugin` |
| `murabito_hexcoords` | `crates/hexcoords` | `VoxelCoord` (cube in, axial stored, layer), `VoxelspacePos`, `Direction` (twelve, by compass point) with `neighbour`, `rotated`, `notches_to`, `heading` | those, plus the errors `NotOnHexPlane`, `NotNearHexPlane` and `ON_PLANE_TOLERANCE` |
| `murabito_progress` | `crates/action/progress` | `Progress`, the one accumulation bar per entity; `MechanismSet`; the sweep | `Progress`, `MechanismSet`, `ProgressPlugin` |
| `murabito_movement` | `crates/action/mechanisms/movement` | `VoxelPosition`, `Facing`, `Locomotion`; the `Step` and `Turn` intents and their tick systems; `place` | those, plus `cost`, `can_step`, `MovementPlugin` |
| `murabito_actions` | `crates/action/actions` | `ActionQueue` of `Action::{Go, Face}`; `issue`, where turn-then-step lives, after `AskingSet` | `Action`, `ActionQueue`, `AskingSet`, `ActionsPlugin` |

The design briefs in `Docs/*_readme.md` mark, section by section, what is implemented and what
is still design. `TODO.md` is what's queued.

## Decisions worth not relitigating

- **Crates are the module boundary.** Chosen over modules in one crate because `pub` and
  `[dependencies]` are the only walls Rust enforces. Medium granularity: one crate per job,
  tiny things that always travel together share one.
- **The module that uses a setting owns it**, as a `pub` resource; whoever edits or saves it
  depends on that module. Decided so a persistence module never has to know its consumers.
- **Only the app registers settings for persistence** (`.persist::<T>("key")` in `main.rs`),
  chosen over each module's plugin registering itself so that no module depends on
  `murabito_settings` and a headless test of a module needs no settings crate.
- **A settings file the game can't read is backed up, then replaced.** Lexa's call: copy to
  `settings-<moment>.bak` and carry on with what could be read. Sections nothing registered
  ride along untouched, so an old or future key is never dropped.
- **A `Bind` is one word in a file** (`KeyW`, `MouseLeft`, `Mouse7`), by hand-written serde:
  the derived form nests an enum in an enum, which YAML can't hold. Bevy's `serialize`
  feature is on so `KeyCode` spells itself; `FromStr` hands the word to Bevy's own serde.
- **The catalogues start empty.** A key is added when the text that uses it is; a test fails
  if `en.yaml` and `ja.yaml` disagree. A language's own name is `Language::own_name()`, on
  the enum, never translated.
- **Simulation on `FixedUpdate` at 64 Hz**, presentation on `Update`.
- **`AppState` is `Playing` or `Paused`, and knows no screens.** Chosen over the archive's
  Playing/Menu/Settings so a new screen never edits this crate, and moving between screens can't
  unpause on the way through. Pausing is one call on `Time<Virtual>`, made by a system that reads
  the state rather than the transition's edge; `Time<Real>` runs on underneath. The camera gates
  its input on `Playing` and depends on `murabito_app_state`; nothing depends on the camera.
- **Escape is the menu's; Space pauses.** The pause bind lives in `AppStateSettings`, owned by
  the crate that flips the state, and is persisted like every other setting.
- **`Overlay` is a sub-state of `Paused` and the overlays never import each other.** Chosen over
  a second independent state kept in step by hand: leaving `Paused` removes the overlay and runs
  its `OnExit`, so an overlay closed by the world resuming tears down the same as one closed by
  its button. The menu's Settings button and the page's Back button both set `Overlay`; a new
  overlay is a variant there and a crate of its own. Named `Overlay`, not `Screen`, which read as
  hardware. The kit, `murabito_ui`, is our own on Bevy's headless widgets: `bevy_feathers` says
  itself it is for editors, not games.
- **Space with an overlay up resumes and closes it.** Coherent ("Space runs the world"), with one
  known edge, a slider mid-drag; the keyboard gate in `TODO.md` is the fix.
- **The slider's observer rides on the slider entity**, as Bevy's own examples do. A global
  `add_observer` would also see a `ValueChange` (measured, after a false start: see the gotcha
  below); the entity one is scoped to that slider and needs no marker check.
- **The pan-speed slider tops out at 6x**, Lexa's call, over the archive's 4x; the floor stays a
  quarter speed. In octaves, so the default is no longer dead centre, which is fine.
- **Progress is measured in the mechanism's units, not ticks**, and the overshoot carries between
  actions of one kind. A run of steps lands when the total distance says (80 ticks, not 81, in
  the test); a tick at rest forgets the leftover.
- **Jump movement**: a body is in exactly one voxel; a step lands in one go. A turn is a notch at
  a time, the short way round, through every direction between; exactly opposite goes
  anticlockwise.
- **A step within one notch of the facing needs no turn**; the landing takes the notch for free.
  Wider needs a `Turn` first, which `issue` puts in, to one notch short on the near side.
- **`ActionQueue`, not "orders"**: anything may push, nothing in it says who did or why. The
  state machine is implicit: the head of the queue plus what is in flight.
- **Members are listed in the root manifest**, since a glob can't cover a group directory.
- **Assets live in `assets/` at the repo root**; `.cargo/config.toml` sets `BEVY_ASSET_ROOT`.
- **Shaku is the base unit**, one world unit; north is −Z, east +X, up +Y.
- **Kinds are Bevy required components, not data.** Lexa's design: every tier and kind is a
  unit component, `#[require]` is *extends* plus the members, so "open `Animal` and read what
  every animal has" holds and the compiler enforces it; at 2,000 kinds a registry plus audit
  would not. Verified in `bevy_ecs` 0.19.1: a direct `require` wins over an inherited one,
  otherwise depth-first in list order; a cycle panics at registration naming the loop (not a
  stack overflow, whatever the doc comment says). One node, one file, folders mirroring the
  tree: Lexa's call over one file for all. English keys, kanji beside them. `Sentient` carries
  `Facing`, `Locomotion` and `ActionQueue`, Lexa's call ("yokai move too"). The AI's own reading
  of the world is a separate, non-authoritative system and was kept out of every decision here.
- **A require constructor has no world in reach**, so a kind names its model as a path
  (`Model`) and one observer loads it. What is given at spawn wins over a kind's `require`,
  which is how a scene says which way a fox faces and which sakura it wants.
- **`AskingSet`**: pushes onto a queue run before `issue`. Found when a second walker made the
  fox land a tick late: the order had held by the scheduler's whim.

## Bevy 0.19 things that cost time

- `SceneRoot` is `WorldAssetRoot`; a glTF scene is `GltfAssetLabel::Scene(0).from_asset(path)`.
- Events are messages: `MessageReader`, `add_message`, `world.write_message`.
- A sub-state set on the frame its parent comes into being starts at the value asked for (the
  transition table in `bevy_state`'s `state_set.rs`), which is how Escape lands in `Paused` and
  `Overlay::Menu` together. `NextState::set` re-runs `OnEnter`/`OnExit` even for the state already
  in force: set only what changes.
- A float literal in a generic event infers as `f64` when nothing pins it: `ValueChange { value:
  1.0, .. }` is a `ValueChange<f64>` that no `On<ValueChange<f32>>` observer sees, with no error.
  Write `1.0_f32`. This cost a wrong conclusion about global observers before it was found.
- `InputPlugin` clears `just_pressed` in `PreUpdate`, so a test can't fake a tap by writing to
  `ButtonInput`; it sends the `KeyboardInput` / `MouseButtonInput` message the window would.
- A system asking for a missing resource panics; the message names the fix (`init_resource`,
  or `Option<Res<T>>`). With `MinimalPlugins`, add `InputPlugin` for `ButtonInput`,
  `AssetPlugin` + `init_asset::<T>()` per asset type, `init_asset::<GizmoAsset>()` +
  `init_gizmo_group::<DefaultGizmoConfigGroup>()` for `Gizmos`.
- An app's first `update()` runs no `FixedUpdate` tick: the clock only starts. Spend it in the
  test helper, then `TimeUpdateStrategy::FixedTimesteps(1)` makes every update exactly one tick.
- Commands from a system ordered `.before(MechanismSet)` are applied before the mechanisms run
  on the same tick, so an intent issued on a tick starts on that tick (the tests pin it).
- Gizmo lines are depth-tested: two overlapping lines at the same depth show only the first.
  Draw the parts of a bar end to end, not one over the other.
- `AmbientLight` on a camera is a per-view override; the world's is the `GlobalAmbientLight`
  resource.
- `const { assert!(N <= 3) }` in a const-generic fn fails `cargo build`, not `cargo check`.
- `World::entities().len()` counts more than what you spawned; count a marker instead.
- Sampling `Facing` every tick during a `Turn` sees every direction between, by design.
- A query filter that clippy calls too complex reads better as a `type` alias anyway.
- `serde_yaml_ng` refuses nested enums (`serializing nested enums in YAML is not supported
  yet`). A YAML file of only comments parses as `null`, so read it as `Option<Map>`.
- Turning on a Bevy feature (`serialize`) rebuilds most of the engine once: ~4 minutes.

## How the work is done

- One step at a time, agreed in chat first; the user runs the app and reports before the next.
  Tick-exact predictions in tests ("lands on tick 16", "the loop closes on 263") have been the
  check that the explanation matches the code.
- Commit locally with `git -C <abs path>`; push and open a PR only when told. Main is PR-only.
- Work in a worktree under `.claude/worktrees/`; the main checkout is shared with other
  sessions and is not branched or edited.
- `rg`, not `grep`; `uv`, not `pip`.
- A scripted edit that asserts on file text must gate everything after it on its exit code
  (`python … && cargo test && git commit`), never `;`: `cargo fmt` reformats what a script
  expects to find, and one such miss committed a scratch test before the mistake was seen.
- The next work, in the order agreed: the keyboard gate and typed values, the rebind screen,
  then the larger modules; all in `TODO.md`.

## Where things are

| | |
|---|---|
| Repo | `/home/lexa/DevProjects/_GameDev/Murabito`, main checkout on `main` |
| This session's worktree | `.claude/worktrees/cleanup-refactor`, parked at `main` between tasks; holds the warm `target/` (~170 GB, mostly `target/debug`) and is what the desktop shortcut runs |
| `main` at handoff | `43b439a`, the merge of PR #36 (the villagers, an art session) |
| Merged this stretch | #16 (archive the first attempt), #19 (workspace), #20 (scene, camera, keybinds), #23 (hexcoords, another session), #25 (movement), #27 (docs), #28 (user_data, settings, i18n), #29 (crate manifest), #30 (app state), #31 (models reorganised, an art session), #32 (overlays), #33 (settings page), #35 (kinds), #34 (docs), #36 (villager bodies, an art session) |
| Other worktrees | art sessions (`flora-models`, `understory`, `exp-05-main-coords`); `murabito` on `layer-skeleton` is stale |

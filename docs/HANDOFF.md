# Handoff — the main project

Written 2026-09-22, after PR #25 (movement) merged; brought up to date the same day with the
settings and i18n crates, then with app state, then with the overlays and the settings page,
then with the kinds; on 2026-09-23 with placement, the hex additions, and perception and
sight; on 2026-09-24 with identity (PR #42) and the debug feature (PR #43); and on 2026-09-27
with the AI's I/O (branch `ai-io`, the PR this update describes): the kind label and the
ontology, the brainstem, the reflexes, the port, the bridge and the Python midbrain's home.
Everything here is on `main` or on that branch. `AGENTS.md` is the authority on how to work;
this is where things stand, for a session starting cold.

## What runs

`cargo run -p murabito`, or `scripts/run.sh` (what the desktop shortcut points at); add
`--features debug` or `--debug` for the debug build, which also serves the world's data on
`127.0.0.1:15702` for `scripts/probe.sh` to read. The game always listens for a mind on
`127.0.0.1:15703`; from `ai/midbrain/`, `uv run board` shows every body's snapshot live and
`uv run order 3 goto 0 0` tells one what to want. What runs: a green ground one cho square
under a pale sky, lit by a sun; a fox eight cells west of the origin, standing until something
tells it otherwise, and a hare six cells east, which walks to wherever the ground is
left-clicked (a bandaid in the scene, in place of its old triangle). Corner steps are visibly
slower than edge steps, with a progress bar filling on the ground in front of a walker once
per step. When the hare appears close and new in the fox's view, the fox startles and turns to
face it, with no mind at all. WASD/arrows pan, the wheel zooms, both eased; the camera's feel
numbers are the ones settled by feel-testing in the first attempt. A sugi stands north of the
line between fox and hare. Each looker's cone is drawn on the ground in its colour, orange for
the fox and pale blue for the hare, with a line to each thing it sees, solid up close and
fainter with distance: from its start the fox sees the tree near and the hare beyond it less
well. Space pauses: the fox freezes mid-step and the camera stops taking input; Space again
resumes. Escape opens the menu over the paused world (Settings / Resume / Quit, in the UI's
font, in English or Japanese); Escape again, or Resume, resumes. Settings is a page with a
pan-speed slider (a quarter speed to six times, in octaves, with a readout) and a language picker
that relabels everything in place. The camera's settings (speed multiplier, pan binds), the
pause and back keys and the UI language persist to `~/.config/murabito/settings.yaml`, which is
hand-editable.

## The crates

Every arrow in the dependency graph points down; `crates/action/`, `crates/all_things/`,
`crates/perception/` and `crates/ai/` are group directories, not crates, and read the same way.
`ai/` at the repo root holds the Python side.

| Crate | Directory | Job | Public |
|---|---|---|---|
| `murabito` | `crates/murabito` | the app: a plugin list; its `debug` feature turns on every crate's and the server | |
| `murabito_debug` | `crates/debug` | Bevy's remote protocol on the loopback address, behind the `debug` feature; an empty plugin without it; registers nothing, depends on no module | `DebugPlugin`, `PORT` |
| `murabito_scene` | `crates/scene` | ground, sun, sky, ambient light; spawns a `Fox`, a `Hare` and a `Sugi` from the kinds with a position and a facing; a left-click on the ground orders the hare there through the brainstem (a bandaid); draws the progress bars, cones and sightlines (placeholders until a UI module owns them) | `ScenePlugin` |
| `murabito_brainstem` | `crates/ai/brainstem` | the vocabulary (`Short`, `Sustained`, `Intent`, `Outcome`, `Previous`, `Doing`); `Brainstem`, one intent per body, the only thing that pushes onto a queue; `BrainstemSet::{Orders, Reflexes, Drive}` in `AskingSet`; the `Port`: a board of `Snapshot`s posted after the senses, a channel of `Order`s drained once a tick, far ends `Board` and `Orders` | those, plus `InView`, `BrainstemPlugin` |
| `murabito_reflexes` | `crates/ai/reflexes` | the catalogue `Reflex` (so far `StartleFaceApparition { by: Kind }`), `Wired` (a reflex at a priority), `Reflexes` (a body's repertoire, tunable), `LastLook`; one system in the `Reflexes` slot that preempts the brainstem | `Reflex`, `Wired`, `Reflexes`, `LastLook`, `ReflexesPlugin` |
| `murabito_bridge` | `crates/ai/bridge` | the port's far ends on a loopback TCP socket, port 15703, length-framed protobuf per `proto/murabito.proto`, compiled at build time by `protox`; `wire.rs` converts at the edge | `BridgePlugin`, `Bridge`, `PORT`, `proto` |
| `murabito_identity` | `crates/all_things/identity` | `ThingId`, a serial number for life, never reused, `restored` from a number off a wire or a file; `NextThingId`, the counter it comes from, the one thing here a save keeps; `Kind`, a thing's node in the tree as its file's module path | `ThingId`, `NextThingId`, `Kind`, `IdentityPlugin` |
| `murabito_kinds` | `crates/all_things/kinds` | the tree of kinds: every tier and kind a unit component whose `#[require]` is its parent and members, each labelling itself `Kind::at(module_path!())`; one file per node in folders that mirror the tree, and the nested roster (`roster.rs`) naming every node once; `Ontology`, the tree as the world has it, built from the roster and checked against the `require` chains; `Model`; three observers: stamp the id, check a tangible thing was given its place and facing, load the model | every kind, `Model`, `Ontology`, `KindNode`, `KindsPlugin`; `examples/tree.rs` |
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
| `murabito_hexcoords` | `crates/hexcoords` | `VoxelCoord` (cube in, axial stored, layer), `VoxelspacePos`, `Direction` (twelve, by compass point) with `neighbour`, `rotated`, `notches_to`, `heading`; `Offset` (one voxel relative to another, `b - a`) with `steps` and `bearing`, the nearest direction it points; `distance` in steps, `ring`, `rings_covering`, `corners` | those, plus the errors `NotOnHexPlane`, `NotNearHexPlane` and `ON_PLANE_TOLERANCE` |
| `murabito_placement` | `crates/placement` | `VoxelPosition` and `Facing`, the plain components any thing in the world carries, and `place`, the one system that writes a `Transform` from them | those, plus `PlacementPlugin` |
| `murabito_progress` | `crates/action/progress` | `Progress`, the one accumulation bar per entity, `abandon`ed when a body is cut short; `MechanismSet`; the sweep | `Progress`, `MechanismSet`, `ProgressPlugin` |
| `murabito_perception` | `crates/perception/perception` | `Occupancy`, which things stand in which voxel, rebuilt each tick; `PerceptionSet::{Gather, Sense}` in `FixedUpdate` after `MechanismSet` | `Occupancy`, `PerceptionSet`, `PerceptionPlugin` |
| `murabito_vision` | `crates/perception/senses/vision` | `Vision` (a cone on `Facing`, three bands, `Vision::BLIND`), a member of `Sentient`; the cast, private; `Seen`, the tick's `Sighting`s (entity, id, offset, acuity) | `Vision`, `Band`, `Acuity`, `Seen`, `Sighting`, `VisionPlugin` |
| `murabito_movement` | `crates/action/mechanisms/movement` | `Locomotion`; the `Step` and `Turn` intents and their tick systems, which write `murabito_placement`'s position and facing | those, plus `cost`, `can_step`, `MovementPlugin` |
| `murabito_actions` | `crates/action/actions` | `ActionQueue` of `Action::{Go, Face}`, readable with `iter`; `issue`, where turn-then-step lives, after `AskingSet`; `CutShort`, the mark that drops what is in flight so the head of the queue is issued this tick | `Action`, `ActionQueue`, `AskingSet`, `CutShort`, `ActionsPlugin` |

The design briefs in `docs/*_readme.md` mark, section by section, what is implemented and what
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
  `Locomotion`, `Vision` and `ActionQueue`, Lexa's call ("yokai move too"); it carried `Facing`
  with a default east until 2026-09-24 (below). The AI's own reading
  of the world is a separate, non-authoritative system and was kept out of every decision here.
- **A require constructor has no world in reach**, so a kind names its model as a path
  (`Model`) and one observer loads it. What is given at spawn wins over a kind's `require`,
  which is how a scene says which way a fox faces and which sakura it wants.
- **`AskingSet`**: pushes onto a queue run before `issue`. Found when a second walker made the
  fox land a tick late: the order had held by the scheduler's whim.
- **Position and facing are `murabito_placement`'s**, below the movement mechanism that writes
  them: a tree has a position and never moves, and the senses read where things are without
  depending on what moves them.
- **A sense is its own crate with its own list in its own shape; nothing merges them.** Lexa's
  call: vision gives `(entity, id, offset, acuity)`, hearing will give a bearing and an intensity,
  smell an intensity and a gradient. Merging across senses, if ever, is the AI's. The field of
  cells the cast computes stays private, opened only if debugging needs it. Everything is
  opaque for now; obscuring, heights and ambiguation (a 妖狐 in human form at `Mid` reads as a
  humanoid) are facet debt, to land in `murabito_perception`, not the AI.
- **Every thing has a `ThingId` for life**, Lexa's call on 2026-09-24, needed for a save and
  wanted by the AI: a counter (`NextThingId`) over a UUID, since one world mints in one place
  and a small number reads well; stamped at the root of the kind tree by an observer, since
  a `require` constructor can't reach a counter; what is given at spawn wins, for a loader.
  A `Sighting` carries both the `Entity` and the `ThingId` (chosen over either alone): the
  engine acts through the handle, memory keys on the number. A positioned entity without an
  id makes `look` panic, agreed over leaving it silently unseen. `murabito_identity` and
  `murabito_kinds` sit together under `crates/all_things/`, a group directory.
- **A tangible thing's place and facing are the instance's, and both are checked at spawn.**
  Lexa, 2026-09-24: the tree can't require `VoxelPosition` (a `require` carries a value, and
  no kind can say where an instance stands), so an observer on `Tangible` panics if a thing
  is spawned without one. `Facing` went the same way, off `Sentient` and its default east
  (never liked) and onto `Tangible` beside the place: "if something has a position, it also
  has a facing". Found on the way: `place` needs both, and the sugi had no `Facing`, so its
  model had stood at the world origin since PR #41 while its voxel, the cast and the
  sightline's target all agreed on somewhere else; the scene's test checked the voxel, not
  the screen. Now the tree is given a facing and a test pins its `Transform` to its voxel.
  Lesson: a drawing that only agrees with itself proves nothing; pin the screen to the data.
- **The debug view is Bevy's remote protocol, behind a Cargo feature, and the wire mirrors
  memory.** Lexa, 2026-09-24, choosing an external view over an in-window panel or a dump
  ("we can stand up a web visualizer later"). A feature, not `cfg(debug_assertions)`, so
  that without it nothing exists, not even `bevy_remote`; off by default, since the everyday
  build is the shortcut's. Each crate registers its own types (over one list in the debug
  crate, which would depend on everything). The wire shows a type exactly as it is in memory,
  axial voxels included, over a prettier cube form: "it lets me catch when the agent builds
  things weird". `Occupancy` is the one hand-written shape, pairs over named fields. Every
  kind derives `Reflect` so a thing's chain shows. The cast's field stays closed. Found on
  the way: Bevy's plugin can't be made read-only (its method list only grows), and the
  `http` feature turns on `bevy_tasks/async-io`, so the first debug build is a full engine
  rebuild (3 min 38 s here) that then lives beside the everyday one in `target/`.
- **Perceiving is simulation**: `PerceptionSet` in `FixedUpdate` after `MechanismSet`, every tick
  for now, with gating on change written down as the next step. `Vision` is a member of
  `Sentient` (yokai see too; a species can require `Vision::BLIND`), and the cone follows
  `Facing`, so turning is what points the eyes.
- **Every thing names its own node, and the tree is written once as a tree.** Lexa,
  2026-09-26. `Kind = Kind::at(module_path!())` in every node's `require`, owned by identity so
  the brainstem can read it without depending on the kinds; the whole path, so "some animal"
  is a prefix later. No macro for uniformity ("harder to learn"): a nested roster in one file,
  and tests that hold the `require` chains (via the `Ontology`, built by spawning each node),
  the labels and the folders to it. `module_path!()` in a node's file reports that file.
- **The AI's I/O** (`docs/ai_readme.md`), Lexa's calls of 2026-09-26: define the I/O as plain
  data first, in-process, with the wire a later driver (C over in-process only or wire from
  day one); a brainstem that runs every tick, in the driver's seat but not in charge, dumb
  without a mind; a midbrain that is its own thing on its own clock (~125 ms), outside the
  process, in Python (65 percent sure, so start there), over a plain socket with protobuf
  (schema as the contract) rather than gRPC (a second async runtime in the game); reflexes
  as their own crate, a catalogue with per-reflex code, a kind picking its repertoire and
  dials, priorities per instance ("Bob is a tad jumpy"), answering only a `Short` so a reflex
  never outlives a round; one vocabulary in two tiers (`Short`, `Sustained`), a reflex and a
  mind both speaking it; a reflex interrupts and drops, and the midbrain is told it was
  cancelled (B′ over interrupt-and-resume); a new intent cuts short what is in flight ("a
  reflex that waits half a second is no reflex; the smarter parts must not interrupt
  themselves"); step at a time, no plan, for `GoTo`; a snapshot of flat facts for a utility
  scorer, built at publish after `Sense`, pulled by the mind; `previous` says what was done
  as well as how it ended, so it isn't read as the current intent's; a sighting names the
  seen thing's kind by its whole path (a node, not the leaf, when perception gets fuzzy).
  Found on the way: a wide turn's bar read idle between notches (fixed in movement); a
  step choice by distance alone zig-zags (cost added); the first twitch ran before the first
  look (skip a just-added `Seen`); the fox startled at the tree its own turning revealed
  (a kind dial, and no firing on a tick the body turned).

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
- `add_plugins` takes a tuple of at most sixteen; nest tuples (a nested tuple is a plugin list).
- A workspace `members` glob that matches nothing is a hard error, so `crates/perception/senses/*`
  could only go in with the first sense.
- `Entity::from_bits` panics on bits it considers invalid; a test wanting ids spawns empties in a
  scratch `World`.
- `RemotePlugin::default()` is every method, mutating ones included; the empty constructor is
  private and `with_method` only adds. Bind to loopback and accept it.
- `ReflectSerializer` flattens a one-field tuple struct to its field, and serialises a map
  with `serialize_map`, which `serde_json` refuses for a struct key ("key must be a string"):
  a `HashMap<VoxelCoord, _>` needs `#[reflect(opaque)]` plus a hand-written `Serialize`,
  registered with `reflect(Serialize)`.
- `register_type::<T>()` registers the types of `T`'s fields too, so a crate without a
  plugin needs no registration of its own.
- A `world.query` with a `components` list answers only entities that have all of them;
  `option` lists what to include when present. Nothing asks for "every component": that is
  `world.list_components` per entity.
- `arc_3d(angle, radius, isometry, colour)` sweeps from the isometry's +X about its +Y, the same
  sense as `Direction`'s index, so a cone's arc is `from_rotation_y(bearing − half_arc)` and a
  sweep of the cone's width.
- `module_path!()` inside a `#[require(T = expr)]` expression reports the module of the file
  the attribute is written in, so every node can label itself with one identical line.
- A `require`'s constructor need not be `const`: `Reflexes::new([…])` builds a `Vec` at spawn.
- `Ref<T>` in a query gives `is_added()`, which is how a system tells "this component was just
  spawned" from "it was written this tick"; `Changed<T>` alone can't.
- Commands to insert a marker from a system in `AskingSet` are applied before a system ordered
  `.after(AskingSet)` runs, so `CutShort` set by drive is acted on by actions on the same tick.
- `protox::compile(files, includes)` + `prost_build::Config::new().compile_fds(fds)` in a
  `build.rs` compiles a `.proto` with no `protoc` installed; the module lands in `OUT_DIR` and
  `include!` brings it in. `prost` names a `oneof` field's enum `module::Kind`, and a proto
  enum's variants in `CamelCase` (`Direction::Ene`).
- A test that drives a real `TcpListener` should bind port 0 and read the address back; the
  leaked accept thread is harmless in a test process.

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
- The next work, each its own design talk: the midbrain's first behaviour, a client on its
  own clock that scores and orders (`docs/ai_readme.md`, design); a believed world; then a web
  page on the debug server, Lexa's stated want; hearing (a push from a source, a bearing and
  an intensity, attenuated along the shortest unobstructed path over `Occupancy`); facets,
  which unlock the senses' three debts; a tick-by-tick view; a debug module to take the
  progress bar, cones and sightlines off the scene. Still queued from before: the keyboard
  gate and typed values, the rebind screen. All in `TODO.md`.

## Where things are

| | |
|---|---|
| Repo | `/home/lexa/DevProjects/_GameDev/Murabito`, main checkout on `main` |
| This session's worktree | `.claude/worktrees/ai-io`, on `ai-io`; its `target/` is a symlink to `cleanup-refactor`'s warm one, so the two share an engine build. `cleanup-refactor` (on `debug-feature`, merged) is what the desktop shortcut runs |
| `main` at handoff | `e85e71b`, the merge of PR #43 (the debug feature) |
| Merged this stretch | #16 (archive the first attempt), #19 (workspace), #20 (scene, camera, keybinds), #23 (hexcoords, another session), #25 (movement), #27 (docs), #28 (user_data, settings, i18n), #29 (crate manifest), #30 (app state), #31 (models reorganised, an art session), #32 (overlays), #33 (settings page), #35 (kinds), #34 (docs), #36 (villager bodies, an art session), #37 (docs review), #38 (placement), #39 (hex offsets and rings), #40 (camera zoom-out, another session), #41 (perception and sight), #42 (identity, `crates/all_things/`), #43 (the debug feature) |
| Other worktrees | art sessions (`flora-models`, `understory`, `exp-05-main-coords`); `murabito` on `layer-skeleton` is stale |

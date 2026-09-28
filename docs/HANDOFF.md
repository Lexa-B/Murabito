# Handoff — the main project

Written 2026-09-22, after PR #25 (movement) merged; brought up to date the same day with the
settings and i18n crates, then with app state, then with the overlays and the settings page,
then with the kinds; on 2026-09-23 with placement, the hex additions, and perception and
sight; on 2026-09-24 with identity (PR #42) and the debug feature (PR #43); and on 2026-09-27
with the AI's I/O (PR #44): the kind label and the ontology, the brainstem, the reflexes, the
port, the bridge and the Python midbrain's home; and the same day with the mind's believed
world (PR #46) and the fox's stalk (PR #47), then checked over for a fresh session (PR #48);
and with the action words, the attacks stub (PR #49), the sustained paces (#50) and the stalk
that sneaks, pounces and bites (#51), then checked over for the session that adds ambitions and
wires in the hare. Everything here is on `main`. `AGENTS.md` is the authority on how to
work; this is where things stand, for a session starting cold.

## What runs

`cargo run -p murabito`, or `scripts/run.sh` (what the desktop shortcut points at); add
`--features debug` or `--debug` for the debug build, which also serves the world's data on
`127.0.0.1:15702` for `scripts/probe.sh` to read. The game always listens for a mind on
`127.0.0.1:15703`; from `ai/midbrain/`, `uv run board` shows every body's snapshot live,
`uv run order 3 walkto 0 0` tells one what to want, and `uv run mind --visualize` opens a window
of what one body believes: a flat hex map, north up, the body at its true cell and every thing
it has seen frozen where it last saw it, labelled with how long ago. What runs: a green ground
one cho square under a pale sky, lit by a sun; a fox eight cells west of the origin, standing
until something tells it otherwise, and a hare six cells east, which jogs to wherever the
ground is left-clicked (a bandaid in the scene, in place of its old triangle). With `uv run
mind` beside it, the fox sneaks up behind the hare, lunges the last two cells and bites, and
bites again for as long as the hare stays put, since a bite does nothing yet. Corner steps
are visibly slower than edge steps, with a progress bar filling on the ground in front of a
walker once per step. When the hare appears close and new in the fox's view, the fox startles
and turns to face it, with no mind at all. WASD/arrows pan, the wheel zooms, both eased; the camera's feel
numbers are the ones settled by feel-testing in the first attempt. A sugi stands north of the
line between fox and hare. Each looker's cone is drawn on the ground in its colour, orange for
the fox and pale blue for the hare, with a line to each thing it sees, solid up close and
fainter with distance: from its start the fox sees the tree and the hare beyond it both near,
since 2026-09-27 when both animals' bands were stretched (fox 18/60/120, hare 12/36/65). Space
pauses: the fox freezes mid-step and the camera stops taking input; Space again
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
| `murabito_scene` | `crates/scene` | ground, sun, sky, ambient light; spawns a `Fox`, a `Hare` and a `Sugi` from the kinds with a position and a facing; a left-click on the ground orders the hare to jog there through the brainstem (a bandaid); draws the progress bars, cones and sightlines (placeholders until a UI module owns them) | `ScenePlugin` |
| `murabito_brainstem` | `crates/ai/brainstem` | the vocabulary (`Short`, `Path`, `Intent`, `Outcome`, `Previous`, `Doing`); a path is queued as written and amended in place, never planned here; `Brainstem`, one intent per body, the only thing that pushes onto a queue; `BrainstemSet::{Orders, Reflexes, Drive}` in `AskingSet`; the `Port`: a board of `Snapshot`s posted after the senses (each sighting an `InView` with the seen thing's kind and facing; the action `underway`), a channel of `Order`s drained once a tick, far ends `Board` and `Orders` | those, plus `InView`, `BrainstemPlugin` |
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
| `murabito_hexcoords` | `crates/hexcoords` | `VoxelCoord` (cube in, axial stored, layer), `VoxelspacePos`, `Direction` (twelve, by compass point) with `neighbour`, `rotated`, `notches_to`, `heading`; `Offset` (one voxel relative to another, `b - a`) with `steps` and `bearing`, the nearest direction it points; `distance` in steps, `ring`, `rings_covering`, `corners`; `step_toward` and `straight_to`, the straight walk across an empty plane | those, plus the errors `NotOnHexPlane`, `NotNearHexPlane` and `ON_PLANE_TOLERANCE` |
| `murabito_placement` | `crates/placement` | `VoxelPosition` and `Facing`, the plain components any thing in the world carries, and `place`, the one system that writes a `Transform` from them | those, plus `PlacementPlugin` |
| `murabito_progress` | `crates/action/progress` | `Progress`, the one accumulation bar per entity, `abandon`ed when a body is cut short; `MechanismSet`; the sweep | `Progress`, `MechanismSet`, `ProgressPlugin` |
| `murabito_perception` | `crates/perception/perception` | `Occupancy`, which things stand in which voxel, rebuilt each tick; `PerceptionSet::{Gather, Sense}` in `FixedUpdate` after `MechanismSet` | `Occupancy`, `PerceptionSet`, `PerceptionPlugin` |
| `murabito_vision` | `crates/perception/senses/vision` | `Vision` (a cone on `Facing`, three bands, `Vision::BLIND`), a member of `Sentient`; the cast, private; `Seen`, the tick's `Sighting`s (entity, id, offset, acuity) | `Vision`, `Band`, `Acuity`, `Seen`, `Sighting`, `VisionPlugin` |
| `murabito_movement` | `crates/action/mechanisms/movement` | `Locomotion`; the `Step { direction, gait, reach }` and `Turn` intents and their tick systems, which write `murabito_placement`'s position and facing; `Gait` (½, 1, 2, 3 × speed), `Way` (forward, lateral, rear of the facing) and the landing rule | those, plus `cost`, `landing`, `MovementPlugin` |
| `murabito_attacks` | `crates/action/mechanisms/attacks` | the `Bite` intent, a quarter of a second on the bar that bites nothing yet | `Bite`, `AttacksPlugin` |
| `murabito_actions` | `crates/action/actions` | `ActionQueue` of `Action`, the ten words (`Walk`, `Jog`, `Sprint`, `Sneak`, `Face`, `Sidestep`, `Backstep`, `Recoil`, `Lunge`, `Bite`), readable with `iter`, `truncate`d from outside, keeping the action `underway`; `issue`, where turn-then-step and the refusals live (a wrong-way word, a bite across a corner), after `AskingSet`; `Action::check`, the same refusal askable first; `CutShort`, the mark that drops what is in flight so the head of the queue is issued this tick | `Action`, `Refusal`, `ActionQueue`, `AskingSet`, `CutShort`, `ActionsPlugin` |

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
  Wider needs a `Turn` first, which `issue` puts in for the pace words, to one notch short on
  the near side.
- **Any direction is a legal step, and the way decides the landing** (2026-09-27): forward
  lands facing the way gone, lateral orthogonal, rear opposite, each on the side of the old
  facing, never more than one notch of turn. Gait and reach are knobs on `Step` that nothing
  above the mechanism sees.
- **The words are the vocabulary, all the way up** (Lexa, 2026-09-27): `Walk`, `Jog`,
  `Sprint`, `Sneak`, `Sidestep`, `Backstep`, `Recoil`, `Lunge`, `Bite`, each fixing its own
  pace and reach. The AI is not to choose a gait. A way word asked the wrong way of the facing
  is refused, never bent: "if something sends an action in the wrong direction, we need to know
  it's broken so we can fix it." `issue` drops it with a warning; the brainstem ends it
  `Refused` so the mind hears.
- **A sustained word was a cell at a pace** (Lexa, 2026-09-27): `GoTo` became `WalkTo`, with
  `JogTo`, `SprintTo`, `SneakTo` beside it, each re-aimed every landing in its pace's word.
  Superseded the next day by the path (below); the way words still have no "to" form.
- **Long actions are a path of shorts the mind writes** (Lexa, 2026-09-28). Seen live: a hare
  jogging away moved the stalk's settle cell every couple of seconds, each re-sent `SneakTo`
  cut the step in flight, and the fox stood turning on the spot. Lexa's call, over re-aiming
  only at a landing (mind-side) or the brainstem finishing a step before taking a new pace:
  "make these long actions a queue of shorts in the brainstem. that way as long as the A*
  has it going in the same direction, the immediate step doesn't change. the midbrain will
  also need to be smart enough to only swap out the part of the path that's different."
  So `Sustained` went (A, the click plans its own straight line through
  `VoxelCoord::straight_to`, over keeping it for the click): `Intent::Path { keep, steps }`,
  queued as written, amended in place while a path is in hand (truncate the queue to `keep`,
  append), done when the last step lands, refused whole if a step is not a pace or a turn;
  the step in flight is never cut by a path, only by a short or a stop. The snapshot gained
  `underway`, the action off the queue and not yet landed, so the mind plans from where it
  lands. The mind's A* (`paths.py`) is in shaku round every believed cell; `to_send` keeps
  the front of the wanted path that matches the queue and sends the rest.
- **A bite is across a face, never a corner** (Lexa, 2026-09-28: "i want bite to be faces,
  not corners"), found when the hare stopped facing N and the fox, settled dead behind on
  the corner line, could neither pounce nor bite (beside and pounce were measured in cube
  steps, the settle cell and the lunge in offsets, which disagree by two on a corner line).
  The game refuses a bite while the body faces a corner direction, `Refused("not across a
  face")`, as it refuses a way word the wrong way; the stalk's pounce line is always an edge
  direction, the hare's rear line when that is one, else the edge direction beside its rear
  nearer the fox; beside and pounce are measured along the facing as the bite reaches; and a
  "set up" branch sidesteps onto the line when the hare has shifted a cell (Lexa: "have the
  fox sidestep into position to set up the lunge"), the sidestep word's first use.
- **Bite is a stub in its own crate, `murabito_attacks`**: a quarter of a second on the bar,
  no target, no effect; the cell faced, across a face, is what it will bite.
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
  never outlives a round; one vocabulary in two tiers (`Short`, `Sustained`, now `Path`), a reflex and a
  mind both speaking it; a reflex interrupts and drops, and the midbrain is told it was
  cancelled (B′ over interrupt-and-resume); a new intent cuts short what is in flight ("a
  reflex that waits half a second is no reflex; the smarter parts must not interrupt
  themselves"); step at a time, no plan, for a pace to a cell (superseded 2026-09-28 by the
  mind's path, above); a snapshot of flat facts for a utility
  scorer, built at publish after `Sense`, pulled by the mind; `previous` says what was done
  as well as how it ended, so it isn't read as the current intent's; a sighting names the
  seen thing's kind by its whole path (a node, not the leaf, when perception gets fuzzy).
  Found on the way: a wide turn's bar read idle between notches (fixed in movement); a
  step choice by distance alone zig-zags (cost added); the first twitch ran before the first
  look (skip a just-added `Seen`); the fox startled at the tree its own turning revealed
  (a kind dial, and no firing on a tick the body turned).
- **The believed world lives in the mind, and the game is the truth panel.** Lexa,
  2026-09-27, opening the midbrain: before any behaviour, "a layer that simulates the world in
  the fox's brain between what it's given from the brainstem and what it decides to act on",
  after exp-02's fog of war. In Python, in the mind (A), over a Rust crate below the reflexes
  (B) or both layered (C): fast to iterate on a model that will be tuned by watching it; the
  game-side memory for the reflexes stays a TODO. No truth recreated in Python: "the truth
  panel is the game", run side by side, so the bridge did not grow a listing of every thing
  (the sugi is not on the board, only in someone's view). Start small: a thing frozen where
  it was last seen, refreshed each round; no dead reckoning, fading or growing radius until
  this is validated. The window stays flat and top-down over exp-02's squashed isometric
  columns ("Doom-style for the time being"); layers are tracked, not drawn. On the way, the
  re-send trap was settled: the mind sends only when its want differs from the snapshot's
  `doing` (over the brainstem swallowing an identical intent, which would hide semantics in
  the part that is meant to be dumb), and the mind drives the fox first, the hare on the
  click.
- **The wander, and the body's scratch** (Lexa, 2026-09-28). The fox's second ambition: with
  no hare believed it walks a leg of 4 to 12 shaku on a bearing within 90° of its facing,
  rests 2 to 8 s, and walks another; no leash (A, free drift forward-biased, over a home
  leash or a fully random direction). Lexa knows it also fires when a hare is *lost*: "once
  we add a more fleshed out internal ontology with entity persistence, there will be a second
  searching ambition". The rest's end is the first thing the board cannot show, so a body has
  a **scratch**: a dict per body on the `Mind`, handed to every ambition on its `Context`,
  keys of the ambition's own name. Lexa asked whether Halo Infinite uses a blackboard (it
  does: the `BotManager`, shared between the Lua game mode and the bots, storing ambitions
  and bot state) and chose the opposite: "i want an internal state, not a shared one... lets
  call it scratch instead of board, so its clear. and for a shared one, i'll implement that
  very different from halo."

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
- The next work, each its own design talk: the mind's queue ("Picking up: the midbrain"
  below and `TODO.md`: the hare's ambitions, a search, the Sims' half, `previous` feedback,
  the believed world's next rule); then a web page on the debug server, Lexa's stated want;
  hearing (a push from a source, a bearing and an intensity, attenuated along the shortest
  unobstructed path over `Occupancy`); facets,
  which unlock the senses' three debts; a tick-by-tick view; a debug module to take the
  progress bar, cones and sightlines off the scene. Still queued from before: the keyboard
  gate and typed values, the rebind screen. All in `TODO.md`.

## Picking up: the midbrain

Written 2026-09-27 for the session that starts the midbrain's first behaviour cold, brought
current the same day as the believed world (PR #46), the stalk (#47), the action words
(#49), the sustained paces (#50) and the pounce (#51) landed, and again on 2026-09-28 for
the wander and the scratch (PR #53, merged) and the paths and the face-only bite (PR #54,
this one, on the branch `paths`). **Lexa's stated aim: more ambitions for the fox (the
wander was the first; a second was announced and not yet described), then wiring in the
hare.** Once #54 is on `main`, branch afresh from `origin/main` in the `ai-io` worktree (`git -C <wt> fetch --prune
&& git -C <wt> switch -C <branch> --no-track origin/main`; its `target/` is a symlink to
`cleanup-refactor`'s and stays). A plain-language walkthrough of the I/O, with diagrams, is a
private page Lexa has the link to (claude.ai artifact `Ff85UQifrkfqY9TKUhboVv`; it stops at
the port and knows nothing of the mind); `docs/ai_readme.md` is the brief in the repo,
believed world, stalk, wander and paths included; `docs/actions_readme.md` has the words.

**What is settled, and is not up for redesign:** everything in `docs/ai_readme.md` above its
Design section, and every call listed under "The AI's I/O", "The believed world lives in the
mind", "The stalk", "The words are the vocabulary", "Long actions are a path of shorts", "A
bite is across a face" and "The wander, and the body's scratch" in the decisions above. In
one breath: the midbrain runs outside the game, in Python under `ai/midbrain/`, on its own
clock (~125 ms, one round), pulls every body's `Snapshot` from the bridge on
`127.0.0.1:15703`, keeps a believed world and a scratch per body, lets the body's ambitions
bid, ticks the winner's behaviour tree, plans any walk it wants as a path with A* round what
it believes, and sends what differs from what the body is doing, a path as an amendment of
the one in hand; the brainstem carries it out, plans nothing, and reports back through
`previous`. The mind speaks only in words (`Sneak`, `Lunge`, `Bite`, `Sidestep`, a `Path` of
pace words); it never chooses a gait, and a word the body cannot be asked the way it faces,
or a bite across a corner, comes back `Refused`. All of the mind is **proof of concept** by
Lexa's word, to be fleshed out later: the shape is meant to stay (Halo's ambitions and
trees, the Sims' advertisements off the believed world), the numbers and the two ambitions
are placeholders.

**What a mind has to work with**, all already there:

- `ai/midbrain/src/midbrain/client.py`: `Bridge(host, port).connect()`, `.snapshots()` (a
  list of `pb.Snapshot`), `.order(id, pb.Intent)`. `murabito_pb2` (`pb`) is the contract as
  Python; `pb.Direction.Name(n)`, `snapshot.HasField("doing")`, `outcome.WhichOneof("kind")`.
- `order.py`'s `parse(words, here) -> pb.Intent` shows how each intent is built as a message
  (a pace to a cell needs to know where the body stands, since it is planned as a path); the
  mind builds the same messages through `ambitions.py`'s helpers (`face`, `face_thing`,
  `lunge`, `sidestep`, `BITE`, `STOP`, `path(word, steps, keep)`) and `Context.walk_to` /
  `jog_to` / `sprint_to` / `sneak_to(cell)`, which plan from `ctx.origin` round `ctx.blocked`;
  `route_of(intent, snapshot)` and `bound_for(intent, snapshot)` read a wanted path back out
  as cells.
- `board.py`'s `render` is the live view; run `uv run board` beside the game while working.
- `beliefs.py`: `BelievedWorld(body).observe(snapshot)`, then iterate `Belief`s (`id`, `kind`,
  `cell: Cell(q, r, layer)`, `facing`, `seen_at`, `acuity`, `walked_since`, `age(now)`);
  `forget(id)`. `hexes.py`: the plane's geometry (`OFFSETS`, `bearing`, `steps`, `along`,
  `rotated(pitch, floor)`, `from_world`). `behaviour.py`: `Condition`, `Act`, `Selector`,
  `Sequence`, `Result(ok, intent, path)`. `paths.py`: `plan(start, goal, blocked, limit)`,
  `cells_along`, `landing`, `length`, `neighbour`. `ambitions.py`: `Context(world, snapshot,
  scratch)` (`here`, `facing`, `tick`, `in_hand`, `seen_now`, `origin`, `blocked`), `Idle`,
  `Wander(...)`, `Stalk(...)` with their dials, `REPERTOIRE` keyed on kind path,
  `repertoire_for`, `choose`, `to_send`, `revise`. `mind.py`: `Mind.round(snapshots) ->
  [(body, intent)]` (observe, revise, choose, want, to_send), `scratches[body]`,
  `decisions[body]` (ambition, path, want, sent); `uv run mind` sends and shows,
  `--visualize` draws (`visualize.py`, the view fitting itself round the body, its beliefs
  and its route; `--scale` is the closest zoom). An ambition reads the believed world, its
  scratch, and the snapshot only for what is in view *now* and the body's own place, facing,
  `doing`, `queue` and `underway`.
- A snapshot's facts (`docs/ai_readme.md`, table): id, kind (path), tick, position (axial),
  facing (index), in_view (id, kind or none, offset, distance in steps, acuity, the seen
  thing's facing or none), doing (intent + since), queue, in_flight (0..1 or none), underway
  (the action off the queue and not yet landed, or none), previous (intent + outcome or
  none).
- The vocabulary a mind may send: `Stop`, `Face(dir)`, `FaceThing(id)`, the action words
  `Walk`, `Jog`, `Sprint`, `Sneak`, `Sidestep`, `Backstep`, `Recoil`, `Lunge` (each a dir) and
  `Bite`, and a `Path { keep, steps }` of pace words and faces, walked as written; sent while
  on a path it amends it, the first `keep` waiting actions kept, the rest replaced, the step
  in flight untouched. The paces turn first if need be; a way word (sidestep: lateral,
  backstep and recoil: rear, lunge: forward) is only that way of the body's facing and ends
  `Refused("not lateral")` and the like otherwise; a bite facing a corner ends `Refused("not
  across a face")`; a path with any other word in it is refused whole. New words are
  variants added in the brainstem when the mind needs them; a way word has no "to" form.
- Numbers: 64 ticks a second; at 4 shaku/s an edge step is 16 ticks, a corner step 28, a
  quarter turn 32; a sneak is twice that, a jog half, a sprint a third (an edge sprint lands
  on tick 6, a lunge on 11, a recoil on 8); a bite is 16; a round of 125 ms is 8 ticks. The
  hare is 2 shaku/s, 90°/s. The fox starts at (-8, 0) facing ESE, the hare at (6, 0) facing
  E, the sugi at (5, -4); the fox's bands are 18/60/120 cells and the hare's 12/36/65
  (stretched 1.5/2/2.5 from the first attempt's on 2026-09-27).

**One trap, settled twice.** Every new short cuts what is in flight (Lexa's call: "the
smarter parts must not interrupt themselves"), so a mind that re-sent its want every round
would cut its own steps and the body would never arrive. First (2026-09-27) the mind sent
only when its want differed from the snapshot's `doing`, with two cells of slack on a moved
target; that still cut every step behind a running hare. Now (2026-09-28) a walk is a path
the mind plans from where the step underway lands, and `ambitions.to_send` sends a hold
never, a `Stop` only when something is in hand, a short only when it is not exactly what is
in hand, and a path as an amendment: the steps matching the queue from the front are kept
and the rest sent, nothing when the whole matches. No memory needed, the board is the truth;
a finished or cancelled intent empties `doing` and the whole path is sent afresh. Also
settled: the mind drives the fox first, the hare stays on the scene's click until it has
ambitions of its own; `uv run order 3 …` does the same from a terminal.

**Where the midbrain stands.** `uv run mind` decides and sends for every body on the board;
the hare's repertoire is `Idle` alone, so it does nothing but what the click gives it. The
shape is Halo's, from the two transcripts Lexa gave (AI and Games #71 on Halo Infinite's
bots; Game Maker's Toolkit on The Sims): ambitions bid utilities, the winner runs a behaviour
tree, the one in hand is boosted against thrashing; the Sims' advertisements-and-motives are
the hook for where utilities will come from, off the believed world rather than the world.
The fox has `Idle` (0.1), `Wander` (0.3, when it believes in no hare: a leg, a rest, a leg)
and `Stalk` (1 while it believes in a hare), Lexa's spec "[stay behind target] -> [move to
be x voxels from the target] -> [stay facing the target]" grown into this tree, first branch
to succeed wins: freeze (the hare is looking, a dial, front 180°) → bite (the cell faced,
across an edge, is its) → pounce (facing along an edge, its cell the third that way) → set
up (one sidestep would line a lunge up) → check (three cells walked unseen: pivot to face
where it is believed; `revise` forgets it if the fox looks straight at the cell within its
near reach and sees nothing; else the count starts afresh and the fox goes on) → circle (off
the pounce line: a notch round toward it, spiralling in by 15°) → approach (sneak a path to
three along the line) → watch (face it). The pounce line is always an edge direction, since
a bite is across a face. Seen live on 2026-09-28, end to end with paths: the hare jogged
thirty cells north-east and stopped facing N; the fox sneaked after it in one unbroken walk,
settled on the SSW edge line, faced it, lunged on tick 1326, landed beside it and bit every
18 ticks. `docs/ai_readme.md`, "Ambitions, the stalk, and the wander", has the trees with
their dials.

Known wrinkles, left on purpose: "three along the line" is three offsets, 3 shaku on an
edge line; the spiral's floor is 3 shaku; `revise`'s reach (18) is the fox's near band copied
into Python, so it is wrong for any other kind and whenever the bands change; the boost is a
flat 0.15, `Idle` a flat 0.1, `Wander` a flat 0.3; a bite does nothing and repeats; freeze
comes before bite, so a hare that turns to look at a fox beside it freezes it (Lexa has not
said otherwise), and a ghost last seen looking at the fox that then leaves its cone sideways
keeps it frozen until something moves (told to Lexa, not fixed: freeze holds while the only
thing that could change the state is the fox moving); A* ties between equal walks flip as
the origin moves, so amendments sometimes only reorder equivalent steps (a turn cost would
settle it); a wander leg that lands on a believed cell plans nothing and the fox rests again.

**Adding an ambition**, the pattern as it stands: a frozen dataclass in `ambitions.py` with a
`name`, `utility(ctx) -> float` in 0..1, and `want(ctx) -> Result` (a `Result(False, None,
(name, "why"))` when it has nothing to want, else `self.tree.tick(ctx)`); the tree is a
`Selector` of `Sequence(Condition, Act)` branches ticked afresh each round, each test and want
a method that reads the target afresh from the believed world; its dials are dataclass fields
with docstrings; it goes in `REPERTOIRE[kind path]` beside `Idle`. What it must remember
between rounds that the board cannot show goes in `ctx.scratch`, the body's own dict, under
keys of its own name (`Wander` and its `wander.rest_until` are the pattern; the mind's tests
for it are in `tests/test_wander.py`, which imports the helpers from `test_ambitions`). Tests in
`tests/test_ambitions.py` build a `Context` with `fox_believing(...)` (a belief from one
sighting, then a snapshot a round later) and assert `result.path` and `result.intent`; compute
the hex geometry before asserting, since most of this stretch's test failures were the
tester's arithmetic (a "flank" cell 60° off the hare's nose was inside its arc; a ring of six
edge neighbours walls nothing in, a corner step jumps it). A want that walks is
`ctx.sneak_to(cell)` and the like, a planned path or None; in a test the origin is `here`
unless the snapshot carries `underway`. `Mind` needs nothing when an ambition is added;
`visualize.py` draws whatever route `route_of` finds in the want.

**Wiring in the hare**: `REPERTOIRE[HARE]` gets its ambitions (flee was the one named) and
`Mind` already drives every body on the board, so nothing else in the mind changes; the
scene's click bandaid (`command_the_hare` in `crates/scene/src/lib.rs`; "The scene's
click-to-command is a bandaid" in `TODO.md`) is retired when the hare no longer needs
provoking by hand, or kept beside its ambitions while they are tried. The hare sees
240° at 12/36/65 and its `Seen` carries the fox's facing, so "the fox is looking at me" is
the same `apart(angle_of(facing), bearing)` the stalk's freeze uses. The hare's startle
reflex (`StartleFaceApparition`) still fires in the brainstem under whatever the mind sends,
and `previous` reports `Cancelled` by its name when it does; the mind does not yet read
`previous`.

What the next session decides, one question at a time:

1. The second new ambition for the fox, which Lexa announced ("i wanna make two more
   ambitions for the fox now") and has not yet described; a search waits on entity
   persistence by Lexa's word. Backstep and recoil are the words no tree uses yet.
2. The hare's ambitions (flee, at least), which retire the scene's click bandaid.
3. Where utilities come from: the Sims' half, advertisements off believed things weighted by
   motives; the body's state on the snapshot is the other input Lexa named.
4. What a bite does once there are jaws, and what the fox does after one.
5. The believed world's next rule, still set aside by Lexa's choice ("keep this object
   permanence layer where it's at"): forgetting on an empty view in general (needs the cone,
   which the wire does not carry), ageing, dead reckoning, cells seen and never seen.
6. How `previous` feeds back, and whether the mind defers to a reflex in hand (`Doing` would
   need a "by"). Lexa's remark to keep: the startle reflex "already requires more
   intelligence than it should have"; the brainstem "shouldn't be smart enough to have
   awareness of other entities" (why `Follow` was declined, and why the mind now plans its
   own paths and the brainstem walks them as written).
7. The planner's next rule: a turn cost in A*, believed things blocking the corner between
   two cells, and whether the freeze deadlock above is handled.

**How to test against the game.** From the `ai-io` worktree, `scripts/run.sh` (the desktop
shortcut runs the other worktree, still on a branch from before the bridge); the log says "the
bridge listens on 127.0.0.1:15703". A scripted run is `timeout 60 scripts/run.sh` in the
background, then `uv run order 3 walkto 0 0` (planned as a path from where the hare stands,
so the body must be on the board) or a Python snippet with `Bridge()`. `uv run pytest` in
`ai/midbrain/` (101 tests; a fake bridge thread in `tests/test_client.py` is the pattern for
testing a client without the game). `RUST_LOG=murabito_brainstem=debug` traces
every aim and step in `~/.cache/murabito/run.log`; reflex fires and refused words are always
logged. For the mind: the game, `uv run board`, and `uv run mind --visualize` in three
terminals, then `uv run order 3 face W` to make the hare look at the fox (freeze), `face N`
to put the fox abeam (circle), or a click to jog the hare off; `uv run mind` sends, so
`board` is the passive view. A headless check is a script that opens a `Bridge`, loops
`Mind.round(bridge.snapshots())`, sends each order and prints `mind.decisions[1]` whenever it
changes, with `SDL_VIDEODRIVER=dummy` and `pygame.image.save` for frames; the pattern is in
this stretch's transcripts, not in the repo. A run of the game opens a window on the desktop
even when started from a script, and a click there jogs the hare: twice a check saw the hare
wander because of it. To provoke the wander, the fox must believe in no hare, and it sees
120 cells: `uv run order 3 sprintto 0 -140` and a 30 s wait before starting the mind. To
watch a chase, jog the hare off with a click or `uv run order 3 jogto 30 -30` while the
mind runs: the fox should follow in one unbroken sneak, the board showing amendments with
`keep` above zero.

**How Lexa works, for this in particular:** design in chat first, one question at a time,
options with costs and a recommendation, then wait for the yes; one small step per turn,
explained before it is written; Lexa runs the game and the board and reports; commit locally,
push and PR only on "send it". Python is `uv` only; Python 3.13 with the cached PyTorch 2.13
if a model ever enters (RTX 5090, CUDA 12.9). Lexa asks conceptual questions mid-build and
expects a plain-language answer before the go is re-asked. Lexa's corrections this stretch,
to keep: the AI's vocabulary is words, never knobs ("i explicitly dont want the ai to have
that freedom"); a wrong action is refused, not bent ("we need to know its broken so we can
fix it"); a crate is named for the family (`attacks`), not the first member (`bite`).

## Where things are

| | |
|---|---|
| Repo | `/home/lexa/DevProjects/_GameDev/Murabito`, main checkout on `main` |
| This session's worktree | `.claude/worktrees/ai-io`, on `paths` (PR #54: the fitted window, the two ghost fixes, the paths, the face-only bite, these docs; PR #53 before it was the wander and the scratch); its `target/` is a symlink to `cleanup-refactor`'s warm one, so the two share an engine build. `cleanup-refactor` (on `debug-feature`, merged) is what the desktop shortcut runs |
| `main` at handoff | `4741cab`, the merge of PR #53 (the wander and the scratch) |
| Merged this stretch | #16 (archive the first attempt), #19 (workspace), #20 (scene, camera, keybinds), #23 (hexcoords, another session), #25 (movement), #27 (docs), #28 (user_data, settings, i18n), #29 (crate manifest), #30 (app state), #31 (models reorganised, an art session), #32 (overlays), #33 (settings page), #35 (kinds), #34 (docs), #36 (villager bodies, an art session), #37 (docs review), #38 (placement), #39 (hex offsets and rings), #40 (camera zoom-out, another session), #41 (perception and sight), #42 (identity, `crates/all_things/`), #43 (the debug feature), #44 (the AI's I/O), #45 (handoff), #46 (the believed world), #47 (the stalk, the seen thing's facing, vision stretched), #48 (handoff review), #49 (the action words, the attacks stub), #50 (the sustained paces; the stalk sneaks), #51 (the stalk pounces and bites), #52 (handoff), #53 (the wander, the scratch) |
| Other worktrees | `cleanup-refactor` (on `debug-feature`, merged long ago: the desktop shortcut runs it, so the shortcut's game has no bridge and no stalk until that worktree moves to `main`); `exp-05-main-coords`, `yokai-models` (art sessions) |

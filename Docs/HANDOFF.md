# Handoff — the main project

Written 2026-09-22, after PR #25 (movement) merged. Everything here is on `main`; nothing is
outstanding except the `todo` branch carrying `TODO.md` and this file. `AGENTS.md` is the
authority on how to work; this is where things stand, for a session starting cold.

## What runs

`cargo run -p murabito`, or `scripts/run.sh` (what the desktop shortcut points at): a green
ground one cho square under a pale sky, lit by a sun, and a fox at the origin walking a
twelve-sided loop, facing the way it goes, corner steps visibly slower than edge steps, with a
progress bar filling on the ground in front of it once per step. WASD/arrows pan, the wheel
zooms, both eased; the camera's feel numbers are the ones settled by feel-testing in the first
attempt.

## The crates

Every arrow in the dependency graph points down; `crates/action/` is a group directory, not a
crate, and reads the same way.

| Crate | Directory | Job | Public |
|---|---|---|---|
| `murabito` | `crates/murabito` | the app: a plugin list | |
| `murabito_scene` | `crates/scene` | ground, sun, sky, ambient light; spawns the fox with its position, facing, locomotion and action queue; refills its queue with the twelve-direction loop; draws the progress bar (placeholder until a UI module owns it) | `ScenePlugin` |
| `murabito_camera` | `crates/camera` | the overhead rig: focus, direction, zoom; eased pan and zoom; settings | `CameraPlugin`, `CameraSettings` |
| `murabito_keybinds` | `crates/keybinds` | `Binds`, up to three keys or mouse buttons for one action; `Inputs`, the system parameter | `Bind`, `Binds`, `Held`, `Inputs`, `MAX_BINDS` |
| `murabito_hexcoords` | `crates/hexcoords` | `VoxelCoord` (cube in, axial stored, layer), `VoxelspacePos`, `Direction` (twelve, by compass point) with `neighbour`, `rotated`, `notches_to`, `heading` | those |
| `murabito_progress` | `crates/action/progress` | `Progress`, the one accumulation bar per entity; `MechanismSet`; the sweep | `Progress`, `MechanismSet`, `ProgressPlugin` |
| `murabito_movement` | `crates/action/mechanisms/movement` | `VoxelPosition`, `Facing`, `Locomotion`; the `Step` and `Turn` intents and their tick systems; `place` | those, plus `cost`, `can_step`, `MovementPlugin` |
| `murabito_actions` | `crates/action/actions` | `ActionQueue` of `Action::{Go, Face}`; `issue`, where turn-then-step lives | `Action`, `ActionQueue`, `ActionsPlugin` |

The design briefs in `Docs/*_readme.md` mark, section by section, what is implemented and what
is still design. `TODO.md` is what's queued.

## Decisions worth not relitigating

- **Crates are the module boundary.** Chosen over modules in one crate because `pub` and
  `[dependencies]` are the only walls Rust enforces. Medium granularity: one crate per job,
  tiny things that always travel together share one.
- **The module that uses a setting owns it**, as a `pub` resource; whoever edits or saves it
  depends on that module. Decided so a persistence module never has to know its consumers.
- **Simulation on `FixedUpdate` at 64 Hz**, presentation on `Update`.
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
- **World objects and the ontology (諸法) are deferred** until something needs to ask "what is
  this"; the fox is placeholder content in `scene` until then.

## Bevy 0.19 things that cost time

- `SceneRoot` is `WorldAssetRoot`; a glTF scene is `GltfAssetLabel::Scene(0).from_asset(path)`.
- Events are messages: `MessageReader`, `add_message`, `world.write_message`.
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
- A query filter that clippy calls too complex reads better as a `type` alias anyway.

## How the work is done

- One step at a time, agreed in chat first; the user runs the app and reports before the next.
  Tick-exact predictions in tests ("lands on tick 16", "the loop closes on 263") have been the
  check that the explanation matches the code.
- Commit locally with `git -C <abs path>`; push and open a PR only when told. Main is PR-only.
- Work in a worktree under `.claude/worktrees/`; the main checkout is shared with other
  sessions and is not branched or edited.
- `rg`, not `grep`; `uv`, not `pip`.

## Where things are

| | |
|---|---|
| Repo | `/home/lexa/DevProjects/_GameDev/Murabito`, main checkout on `main` |
| This session's worktree | `.claude/worktrees/cleanup-refactor`, on `todo`; holds the warm `target/` (~34 GB) and is what the desktop shortcut runs |
| `main` at handoff | `5f5600f`, the merge of PR #26 (understory art), on top of #25 |
| Merged this stretch | #16 (archive the first attempt), #19 (workspace), #20 (scene, camera, keybinds), #23 (hexcoords, another session), #25 (movement) |
| Other worktrees | art sessions (`flora-models`, `understory`, `exp-05-main-coords`); `murabito` on `layer-skeleton` is stale |

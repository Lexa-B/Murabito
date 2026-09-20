# Handoff — root project, Rust + Bevy

Written 2026-09-20, at the end of the session that replaced the Unreal root project
with a Bevy one and added i18n. Everything here is merged to `main`; nothing is
outstanding.

## Where things are

| | |
|---|---|
| Repo | `/home/lexa/DevProjects/_GameDev/Murabito` (main checkout, usually on another session's branch) |
| Our worktree | `.claude/worktrees/murabito`, parked on `main` |
| `main` at handoff | `6be7a22` (merge of PR #11) |
| Merged this session | PR #10 (Bevy project becomes the root), PR #11 (English and Japanese UI text) |

One worktree is the agreed arrangement: cut a branch inside it for a task, open the PR,
and it returns to `main` after the merge. Spin up a second worktree only if work splits
across parallel root-project tasks. The 12 GB `target/` has to be moved by hand when a
worktree is retired, or the next build is a full 4½-minute recompile.

## What the project is now

The repo root is a single Rust crate, `murabito`, on Bevy 0.19.1 (Rust 1.95). The
Unreal Engine 5 attempt it started as is archived, unbuilt, in `_Archives/UE-Try/`.
`Experiments/` (exp-00 … exp-05) is untouched and still live; exp-04 is Unreal.

`AGENTS.md` was rewritten for all of this and is the authority — read it first. This
file is the session's context, not a second copy of the rules.

### What runs

A window with placeholder scene (ground plane, a slowly spinning cube, a sun) and:

- An overhead camera: WASD/arrows pan, wheel zooms, both eased.
- Escape opens a menu (設定 / 再開 / 終了, or Settings / Resume / Quit) and pauses.
- A settings page: pan-speed slider, language picker, Back.
- Settings persist to `~/.config/murabito/settings.yaml`.
- F12, or `--shot`, saves a screenshot.

### Modules

`main.rs` is a plugin list and nothing else. Each module registers what it needs through
its own `Plugin`: `camera`, `scene`, `settings`, `settings_page`, `menu`, `state`, `ui`,
`i18n`, `screenshot`.

## Build, run, verify

```sh
cargo build                                    # ~3-8s incremental; first build ~4.5 min
cargo run                                      # or the binary, with BEVY_ASSET_ROOT set
cargo clippy && cargo fmt --check              # both clean at handoff
cargo run -- --shot shots/x.png --screen settings   # capture a screen and exit
```

Tests arrived just after this was written: `cargo test` covers the pan-speed curve as
unit tests and the pause-on-state-change behaviour headlessly in `tests/`. The settings
file's load/save behaviour is the next obvious candidate.

Running the binary directly (rather than `cargo run`) needs two things:

```sh
# the display, since terminal tabs have neither DISPLAY nor WAYLAND_DISPLAY
while IFS= read -r line; do case "$line" in DISPLAY=*|WAYLAND_DISPLAY=*|XAUTHORITY=*) export "$line";; esac
done < <(systemctl --user show-environment)
BEVY_ASSET_ROOT="$PWD" ./target/debug/murabito      # else assets/ resolves next to the exe
```

System libraries `libwayland-dev`, `libxkbcommon-dev` and `libudev-dev` were installed
this session; without them the build dies in `wayland-sys`'s build script.

## Decisions worth not relitigating

- **Pausing is `Time<Virtual>`**, not a flag. Input is gated separately by state. They
  solve different problems: what the world does, versus where clicks go.
- **Settings are resources; `settings.rs` alone writes the file**, on any change to one.
  A page, a keybind or a console command all persist for free.
- **A broken settings file is never overwritten.** Missing means defaults and is created;
  unparseable means warn, use defaults, and leave the file alone to be fixed. The first
  version overwrote it, because `insert_resource` counts as a change on frame 1 — that is
  what `is_added()` guards against now.
- **YAML for everything hand-written** (settings, locales), on Lexa's call: one markup.
  `serde_yaml` is deprecated; `serde_yaml_ng` is the fork in use.
- **Catalogues are compiled in** with `include_str!`. Editing a translation needs a
  rebuild; that was accepted deliberately.
- **Pan speed follows zoom as a power curve**, exponent 0.75, `PAN_SPEED = 13.5` at the
  13 m reference zoom, `PAN_RESPONSE = 13.0`. Four rounds of feel-testing landed there:
  0 (fixed) felt sluggish close in, 1.0 (proportional) frantic far out. Only the overall
  multiplier is exposed as a setting — the curve's shape is a design decision.
- **The zoom slider runs in octaves** (log2 of the multiplier) so the default sits centre
  and every step is the same proportional change.
- **Both languages are shown as buttons**, each named in its own script, never translated.
  A single button naming the other language was tried and read as a label, not a choice.

## Bevy 0.19 gotchas, all of which cost time

- `EventReader`/`EventWriter` are **`MessageReader`/`MessageWriter`**; events are
  registered with `add_message`. Observers take `On<E>`.
- `TextFont::font` is a **`FontSource`**, so `FontSource::Handle(handle)`.
- `TextFont::font_size` is a **`FontSize`** enum: `FontSize::Px(20.0)`.
- **`BorderRadius` is a field of `Node`**, not a component.
- `DirectionalLight` has **`shadow_maps_enabled`**, not `shadows_enabled`.
- **`AmbientLight` is a component on the camera**, not a resource.
- `ChildSpawnerCommands` takes no generic parameter.
- **`UiWidgetsPlugins` is already in `DefaultPlugins`** — adding it again panics. The
  `ui` feature pulls in `bevy_ui_widgets` and picking.
- The slider is **headless**: it handles drag and range, you draw it and place the thumb.
  Thumb travel is the track width *less the thumb width*, or it drifts from the cursor.
- **A fixed-width text node wraps**, possibly onto an invisible whitespace line, which
  doubles the node's height and pushes glyphs off the row's centre. `LineBreak::NoWrap`.
  This was diagnosed by dumping `ComputedNode` sizes; all three items were correctly
  centred and the *boxes* were wrong.
- Bevy's built-in font has **no CJK glyphs**. Noto Sans JP (OFL, LFS) is the UI font.
- A screen's `OnEnter` needs `PreStartup`/`Startup` resources (the font), so the `--shot`
  flag asks for its state transition on frame 2, not frame 1.
- **Capture late.** Render pipelines compile on first use; an early screenshot is a bare
  clear colour. `--settle` defaults to 150 frames.

## How Lexa wants the work done

- **Small, visible steps.** Smallest thing that puts something on screen, then check in.
  This project exists because a previous session disappeared into a 4,000-line build-out.
- **One fix at a time** when something is broken: propose it, wait for confirmation, then
  move on. Don't chain "and once that works…".
- **Never presuppose project goals.** Ask; an inference is not a fact.
- **Commit locally and report. Push or open a PR only when told.** `main` is PR-only.
- Screenshots, not questions, for anything visual.
- `rg`, not `grep`. `uv` for any Python.

## Loose ends, in the order I'd raise them

1. **The Japanese is mine, not a translator's.** `カメラ移動速度` for pan speed is the
   one I'd question; it describes the camera, where the setting is a multiplier.
2. **The settings label column is a fixed 230 px stopgap.** Fine for two rows; a grid
   that sizes to its widest label is the real fix when the page grows.
3. **No `scripts/run.sh`.** The display and `BEVY_ASSET_ROOT` dance is re-typed each
   time; a script would bundle it.
4. **`sccache`** would end the target-dir shuffling between worktrees without
   serialising builds the way a shared `CARGO_TARGET_DIR` would.
5. **An orphaned `~/.config/murabito/exp-06/settings.yaml`** is left over from before the
   rename. Nothing reads it.

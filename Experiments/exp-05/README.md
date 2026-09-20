# exp-05 — Hex coordinates, addresses and chunks

A uniform hex coordinate system for Murabito: ri, cho and ken are nested scales of
hot-loadable chunks, the shaku is the primary unit, and placeholder voxel terrain lives
in that system. Built in Rust and Bevy, as a fresh start outside Unreal Engine.

See the design spec ([`docs/specs/2026-09-20-exp-05-hex-voxel-chunks-design.md`](docs/specs/2026-09-20-exp-05-hex-voxel-chunks-design.md))
and the implementation plan ([`docs/plans/2026-09-20-exp-05-hex-voxel-chunks-plan.md`](docs/plans/2026-09-20-exp-05-hex-voxel-chunks-plan.md))
for the full picture; [`docs/HANDOFF.md`](docs/HANDOFF.md) has the background this was
designed from.

## Crates

A Cargo workspace with three members:

- **`hexworld`** — the engine-free core. No Bevy, no graphics, **no dependencies at all**.
  Addresses (ri/cho/ken/shaku + a vertical layer), packing-B ownership, run-list voxel
  columns, chunks, a `ChunkStore` with delayed unloading, and a pure mesher. This is the
  part meant to outlive the engine choice.
- **`hexworld_bevy`** — the Bevy plugin: a `Loader` component, background chunk
  generation and meshing, one entity per shown chunk, a same-frame handover between
  detail levels, and the ground material (hex lines, tint by level).
- **`viewer`** — the app: an overhead camera rig, an egui side panel, and the headless
  CLI flags below.

## Build, run, test

From this directory (`Experiments/exp-05`):

```
cargo build --release
cargo run -p viewer --release
cargo test
```

`cargo fmt --all` and `cargo clippy --workspace --all-targets -- -D warnings` should
stay clean. Windowed runs need the desktop display; if the shell has none (`echo
$DISPLAY` empty), take it from the systemd user session:

```
export $(systemctl --user show-environment | rg '^(DISPLAY|XAUTHORITY)=' | xargs)
```

Don't point `CARGO_TARGET_DIR` or a temp working directory at `/tmp` — it's a
RAM-backed tmpfs on this machine.

## Controls

WASD/arrows or middle-drag pan the focus · the wheel zooms · the panel's **Display**
section picks the line mode (off / shaku only / nested) and toggles tint by level · the
panel's **Loaders** section adds or removes a loader at the hovered cell. There are no
keyboard shortcuts for line mode, tint or loaders — only the panel.

## Headless flags

For screenshots and measurements without a person at the screen, parsed by hand from
`std::env::args()` (no `clap`):

| Flag | Meaning |
|---|---|
| `--frames N` | Exit on our own after N frames. |
| `--screenshot-dir DIR` | Save a PNG under `DIR` every `--screenshot-every` frames. |
| `--screenshot-every M` | How often to save, in frames (default 30; never 0). |
| `--seed N` | Overrides the world's seed. |
| `--start-east M --start-north M` | The focus's starting position, in metres (both required together). |
| `--rings-shaku N` | Overrides the focus loader's shaku ring count. |
| `--auto-pan` | Sweep the focus east at a steady pace, for a sustained-panning frame-time measurement. |
| `--zoom N` | The camera's starting distance from the focus, in metres. |
| `--line-mode off\|shaku\|nested` | The starting line mode. |
| `--tint` | Start with tint-by-level on. |

The last three (`--zoom`, `--line-mode`, `--tint`) aren't in the original task brief's
flag list — they were added because there is no keyboard shortcut for the camera's
starting distance, the line mode or the tint toggle (only the panel's mouse controls
reach them), so a headless run had no other way to reach the spec's close-up, nested-line
or tint screenshots. Every flag falls back to a default rather than panicking on a
missing or malformed value.

Example: settle the default view and take a screenshot every 60 frames over 240 frames
(a few seconds):

```
cargo run -p viewer --release -- --frames 240 --screenshot-every 60 --screenshot-dir screenshots
```

`screenshots/` is gitignored.

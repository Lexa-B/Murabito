# Handoff — root project, Rust + Bevy

Rewritten 2026-09-21, at the end of the session that built the senses: the hex grid,
the pattern-filled overlay, the F3 screen, occluders, line of sight and height.

Session context only. **`AGENTS.md` is the rules and `Docs/brain-hitlist.md` is the
roadmap** — read both before this, and don't expect either to be repeated here.

## State, and the one thing to check first

| | |
|---|---|
| Repo | `/home/lexa/DevProjects/_GameDev/Murabito` (main checkout, often on another session's branch) |
| Our worktree | `.claude/worktrees/murabito` |
| Worktree branch | **`line-of-sight`**, pushed |
| **PR #14** | **OPEN, not merged** — <https://github.com/Lexa-B/Murabito/pull/14> |
| Merged so far | #10 Bevy at the root, #11 i18n, #12 test harness, #13 ontology + senses |

**Check PR #14 before doing anything.** If Lexa merged it, `git checkout main && git
pull && git branch -d line-of-sight` and branch fresh. If not, either keep working on
that branch or branch from `origin/main` and accept that #14's work is missing.

Git rules that have bitten before: `main` is PR-only, every git command takes
`git -C <absolute path>`, and the shell's cwd drifts between calls — it has landed in
the main checkout (on a *different* branch) several times this session, which silently
reads the wrong files.

## What the project is

One Rust crate, `murabito`, Bevy 0.19.1. A fox and a rabbit stand in a field with grass,
thickets and trees, and you can see what each of them can sense. Nothing decides
anything yet: there are bodies and no brains.

`Experiments/` is untouched and still live. The Unreal attempt is archived in
`_Archives/UE-Try/`.

### Modules

`main.rs` is a plugin list. `being`, `camera`, `debug_screen`, `hex`, `i18n`, `menu`,
`scene`, `screenshot`, `senses/{mod,vision,hearing,occlusion}`, `settings`,
`settings_page`, `state`, `ui`, `user_data`, `諸法`.

### Tests

**109** — 100 in the lib, 4 in `tests/pause.rs`, 5 in `tests/settings_persistence.rs`.
`cargo clippy --all-targets` and `cargo fmt --check` are clean.

## Running it

```sh
scripts/run.sh                                   # handles display env and asset root
cargo run -- --shot shots/x.png --zoom 45        # capture and exit
cargo run -- --shot x.png --screen menu --debug  # a screen, with F3 open
```

`--zoom` is in **shaku** and matters: at the starting zoom every sense field runs off
the edge. 45–70 frames the whole tableau. `--settle <frames>` if 150 isn't enough.

Screenshots at these zooms compress a 1-shaku cell to ~10 px, and **I twice misread one
and concluded working code was broken.** Crop before judging:

```sh
uv run --with pillow python3 -c "
from PIL import Image; im = Image.open('screenshots/x.png'); w,h = im.size
box = (int(w*0.3), int(h*0.25), int(w*0.8), int(h*0.7))
im.crop(box).resize(((box[2]-box[0])*2, (box[3]-box[1])*2), Image.LANCZOS).save('screenshots/crop.png')"
```

Better still, write the test first. Both times, a test settled in a minute what the
picture had made ambiguous for several.

## The senses, as built

The shape and the reasoning are in the hitlist (settled items 24–31). What the code
does, briefly:

- **`hex.rs`** — 1 shaku flat-to-flat, pointy-up, axial. `ring` (ordered walk),
  `within` (a bag), `edge`, `steps_covering`. Provisional; exp-05's unit system replaces
  it.
- **`senses/occlusion.rs`** — the only part of `senses` that knows a taxonomy exists.
  Turns a 種 into `(Opacity, Height)` per cell, and stamps `Height` onto anything with a
  種 so the rest can ask how tall something is without learning what a 種 is.
- **`senses/vision.rs`** — `band_at` is pure cone geometry; `cast` is the shadowcast;
  `SeenCells` is the result, per height class, as a component.
- **`senses/hearing.rs`** — a *receiver* polar pattern only. Propagation does not exist.

### Gotchas, each of which cost real time

- **A radius in shaku is not a radius in steps.** Use `hex::steps_covering`. Coverage
  otherwise stops early *in the diagonal directions only*, which reads as a rendering
  artefact rather than a logic error. Sound and smell will both need it.
- **Test cells must be exactly collinear.** `Hex::new(0, -r)` is; `Hex::from_world(dir *
  n)` is not — it snaps to whichever cell contains the point, and the drift walks a far
  cell out of a near cell's shadow. Two tests failed for a reason unrelated to the code.
- **`ButtonInput::press` only registers `just_pressed` if the key was up**, and `clear()`
  leaves it held. Simulated input needs `reset(key)`.
- **Japanese paths typed into a shell heredoc don't always match what's on disk.** Use
  `find … -exec`, or read the bytes `os.walk` hands back. `sort` is locale-aware and
  will reorder paths misleadingly.
- **`cargo fmt` reformats between edits**, so a string an earlier edit matched may no
  longer exist. Assert the match before replacing.
- `gizmos.linestrip` is fine; line *width* is a property of the config group, not the
  call, hence `BoldStroke` / `MidStroke` / `FaintStroke`.

## Next: 0g, sound that bends

Everything needed is decided; none of it is built.

- A noise is an **event cast from its maker**, not a field. Propagation attenuates it by
  shortest *unobstructed* path length, so a wall muffles rather than silences — that is
  what diffraction is.
- It delivers an **intensity and a bearing**. The listener's existing polar pattern turns
  that into what it actually hears. The cardioid is a receiver gain, not a propagation
  shape.
- Occluders already carry a 音 facet — `遮音` blocks, `吸音` muffles, `透音` transmits —
  and a 高さ. `occlusion.rs` currently exposes sight only, and its header says why: sound
  wants an **attenuation cost**, not a three-way verdict.
- Smell (0h) is deliberately *not* the same primitive. Lexa's call: smell emanates from a
  物 and drifts, sound is cast instantly and normalised at the ear. Share pieces, keep
  them separate.
- Falloff is drawn with **size**, not weight or opacity — see settled item 28, which was
  arrived at by getting it wrong twice.

0f (a legible overlay) is still open but may already be answered by the pattern fills and
the F3 toggles; worth re-reading against what's on screen before doing anything to it.

## How Lexa wants this done

- **Discuss, then implement.** Especially for anything with a design choice in it. This
  session's best work came out of dialogue, not from me guessing well.
- **One change at a time**, visible before the next is agreed. I bundled three things
  twice and was told to stop, twice.
- **One fix at a time** when something is broken: propose it, wait, then move on.
- **Never presuppose project goals.** Ask; an inference is not a fact.
- **Commit locally and report. Push or open a PR only when told.**
- Lexa's professional interest was world-model emergence in embodied AI. He is new to
  game-AI *convention*, not to the subject. Don't explain world models to him; do explain
  Bevy and Rust. Building cheaply is not a reason to build something stupid — "it must
  have been the wind" is the explicit anti-goal.

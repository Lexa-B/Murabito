# AGENTS.md

This file provides guidance to AI coding agents when working with code in this repository. `CLAUDE.md` at the repo root is a one-line import shim (`@AGENTS.md`) kept for Claude Code's file-discovery convention.

## Project Overview

Murabito is a new project, described by its author as halfway between a game and a fun AI experiment / population-dynamics simulator. The main project at the repo root is **Rust + Bevy**.

It was first built in Unreal Engine 5. That attempt is kept, unbuilt, in `_Archives/UE-Try/`; see "The archived Unreal attempt" below.

Ideas are tried out as small, self-contained **experiments** under `Experiments/`. They come in three kinds:

- **Rust + Bevy**: what the main project is now. exp-05 is the first: hex coordinates, addresses and chunks.
- **Unreal Engine 5 (C++)**: exp-04, a UE 5.8 project with procedural hilly terrain, a hex grid draped over it, an overhead camera, and hover highlighting.
- **Python + pygame**: quick testing of ideas.
  - **exp-00** is a "hello world" of pathing and object use.
  - **exp-01** builds on it with objects that wander.
  - **exp-02** adds fog of war and a believed world.
  - **exp-03** is a tiered hex world: hierarchical hex addresses at historical Japanese scale, loaded in tiers of detail (moderngl).

Don't assume goals beyond what `Experiments/manifest.md` and each experiment's spec state.


## Repository Organization

```
Murabito/
├─ AGENTS.md, CLAUDE.md, LICENSE, LICENSE-ASSETS, LICENSING.md, .gitignore, .gitattributes
├─ Cargo.toml, Cargo.lock, rust-toolchain.toml   the main project: one crate, `murabito`
├─ src/                    its code (see "Main project" below)
├─ Docs/                   project docs (no specs or plans; see below)
├─ _Archives/
│  └─ UE-Try/              the Unreal Engine 5 attempt, kept for reference, not built
└─ Experiments/
   ├─ manifest.md          one entry per experiment: what, why, how to run
   ├─ exp-NN/              a Python + pygame experiment
   │  ├─ pyproject.toml    its own uv project (Python deps managed with `uv add`)
   │  ├─ src/              code, with `src/ai/` for decision logic, pathing, etc.
   │  ├─ tests/            pytest; run with `uv run pytest` from the experiment dir
   │  └─ docs/             specs/ and plans/, as below
   ├─ exp-NN/              a Rust + Bevy experiment: its own Cargo workspace, crates/, docs/
   └─ exp-NN/              an Unreal Engine 5 (C++) experiment
      ├─ MuraBito.uproject, Config/       project file and text config
      ├─ Source/MuraBito/                 the C++ module; Private/Tests/ holds automation tests
      ├─ scripts/                         build.sh, editor.sh, game.sh, test.sh
      └─ docs/specs/, docs/plans/
```

- **Experiments are self-contained.** Code, tests, dependencies and docs live inside `Experiments/exp-NN/`. Nothing experiment-specific goes at the repo root.
- **A new experiment can start as a copy of an earlier one.** The copy gets its own project name, and the earlier experiment is left unchanged.
- **Every new experiment gets an entry in `Experiments/manifest.md`.** Update the entry when the experiment's design changes.
- **An experiment's specs go in its `docs/specs/` and plans in its `docs/plans/`.** This replaces the superpowers default of `docs/superpowers/specs/` and `docs/superpowers/plans/`. Don't create a `superpowers/` folder. Name specs `YYYY-MM-DD-<topic>-design.md` and plans `YYYY-MM-DD-<topic>-plan.md`.


## Main project

The repo root is the Murabito game: a single Rust crate, `murabito`, built on Bevy 0.19. The user drives it; AI assists.

- **No spec or plan files for the main project.** Agree the design with the user in chat, wait for a yes, then implement it and open a PR. Specs and plans are only for experiments.
- **Work in small, visible steps.** Build the smallest thing that puts something on screen, let the user see it, then agree the next step. Don't disappear into a long build-out.
- **Build and run:** `cargo build`, `cargo run`, `cargo clippy`, `cargo fmt`. There are no tests yet.
- **Bevy compiles into the binary.** There is no engine install: `bevy = "0.19"` in `Cargo.toml` is the whole thing. The first build takes about 4.5 minutes (551 crates, dependencies at `opt-level = 3`); after that, a change to `src/` rebuilds in about 3 seconds. `target/` runs to roughly 10 GB.
- **Linux system libraries.** Bevy's default features build winit with the Wayland backend, which needs `libwayland-dev`, `libxkbcommon-dev` and `libudev-dev`. Without them the build fails in `wayland-sys`'s build script with a pkg-config error.

### Layout

- `src/main.rs` — the window and the plugin list, and nothing else
- `src/camera.rs` — `CameraRig` (a ground focus, a zoom distance, a pan velocity) and the pan/zoom/apply systems
- `src/scene.rs` — the placeholder world: ground, a cube, a sun
- `src/settings.rs` — `CameraSettings` and the settings file
- `src/settings_page.rs` — the settings screen
- `src/menu.rs` — the escape menu
- `src/state.rs` — `AppState` (Playing / Menu / Settings), and pausing
- `src/ui.rs` — what the screens share: scrim, buttons, slider visuals
- `src/i18n.rs` — the string catalogues, `Language`, and the `Localized` component
- `src/screenshot.rs` — F12, and the `--shot` command-line capture

### Conventions

- **Each module registers itself through a `Plugin`.** `main.rs` adds plugins and knows nothing about what they need; a module's resources, systems and spawns are its own business.
- **Pausing is `Time<Virtual>`, not a flag.** Pausing the clock stops everything driven by elapsed time, so no system has to know a menu exists. `Time<Real>` keeps running, which keeps the UI responsive. Input is gated separately, with `run_if(in_state(AppState::Playing))`.
- **Settings are resources; `settings.rs` persists them.** Anything that edits a settings resource gets saved automatically. Nothing else writes the file.
- **Settings live at `~/.config/murabito/settings.yaml`** (YAML, one top-level key per group). A missing file means defaults and is created; a file that fails to parse warns and is left alone so the user can fix it.
- **Windowed runs need the desktop display.** Shells in the user's terminal may have no `DISPLAY`/`WAYLAND_DISPLAY`; take them from the systemd user session (`systemctl --user show-environment`), as the experiments' scripts do.
- **Check visual work with a screenshot, not by asking.** F12 saves one to `screenshots/` (gitignored) while playing, and `cargo run -- --shot <path> --screen playing|menu|settings` drives the app to a screen, captures it and exits — no keyboard needed. Add `--settle <frames>` if 150 isn't long enough; capture too early and the PNG is a bare clear colour, because render pipelines compile on first use.
- **Running the binary directly needs `BEVY_ASSET_ROOT`.** Bevy resolves `assets/` relative to the executable unless `cargo run` sets it, so a direct `./target/debug/murabito` can't find the font or the locales.
- **UI text is never a literal.** Every string comes from `assets/locales/{en,ja}.yaml` through a `Localized` key, and both catalogues must carry the same keys — a gap warns at startup. Text that is the same in every language (a language's own name, a number) uses `ui::spawn_literal_button` instead.
- **A fixed-width text node wraps.** Bevy UI text in a node with an explicit width will line-break, possibly onto an invisible whitespace line, which doubles the node's height and pushes the glyphs off centre. Use `LineBreak::NoWrap` on labels with a fixed width.
- **Never kill, signal or otherwise touch a process you didn't start.**


## Git workflow

`main` is protected on GitHub by the ruleset "Protect main": no direct pushes (for anyone, admins included), no force-pushes, and no deleting it. Every change reaches `main` through a pull request that the user reviews and merges.

- **Work on a branch.** Branch from an up-to-date `main` (`git fetch`, then branch from `origin/main`). Name it after the change, e.g. `agents-pr-workflow`.
- **Don't push or open a pull request until the user approves the work.** When a change is ready, commit it locally, tell the user what's there and how to check it, and wait. Approval to design or build something is not approval to open its PR.
- **Open the pull request** once approved, with `gh pr create` against `main` and a short summary of what changed and why. Keep each PR small enough to review in one sitting.
- **Never merge into `main` locally or push `main`.** The server rejects it. The user merges the PR on GitHub, then runs `git pull` on `main`.
- **Check the PR is still open before pushing follow-up commits** (`gh pr view <n> --json state,mergedAt`). If it was merged or closed in the meantime, start a fresh branch from `origin/main` and cherry-pick onto it.
- **Use `git -C <absolute path>` for every git command.** Worktrees and a shell working directory that persists between commands make relative paths easy to get wrong.


## Licensing and binary files

- **Public MIT repo: never copy engine code or assets in.** Reading engine headers and source to check an API is fine; pasting them, or Epic copyright headers, into the repo is not.
- **Two licences: code is MIT, assets are CC BY-SA 4.0.** See `LICENSING.md` for what counts as an asset. Third-party files (fonts, Epic/Fab content, anything not made for this repo) keep their own licences: add their licence file next to them and list them in `LICENSING.md`, and never commit anything whose licence doesn't allow redistribution.
- **Binary assets go through Git LFS.** `.gitattributes` at the repo root routes Unreal assets, source art, textures, audio, video, fonts, raw data, third-party binaries and PDFs to LFS (not lockable yet). Check a new binary type is covered before committing it; adding it afterwards means rewriting history. The main project has no binary assets: its world is built in code at runtime.
- **Naming:** code and docs don't name games that inspired the project.


## The archived Unreal attempt

`_Archives/UE-Try/` holds the Unreal Engine 5.8 project the repo started as: `Murabito.uproject`, `Config/`, `Source/Murabito/`, `Content/` and `Tools/`. It is kept for reference and is **not** built or maintained. Its `Tools/` scripts expect it at the repo root and won't work where it now sits.

The rules below still apply to the UE experiment, exp-04, which is live:

- **Use the local engine install by path** (`UE_ROOT`, default `/home/lexa/DevProjects/_GameDev/_GameEngines/UnrealEngine/5.8.2`).
- **Build and test with the experiment's `scripts/`:** `build.sh`, `test.sh [TestPathPrefix]` (headless automation tests; pass/fail comes from the log, since the editor's exit code is always 1 under `-TestExit`).
- **Keep UE's type prefixes** (`A`, `F`, `U`), with plain names after them that say what the class does (`ACameraRig`, not a "…Pawn").
- **The user may have the editor open on the same checkout.** `build.sh` then does a hot-reload build. Changes to class layout don't hot-reload reliably: tell the user to restart the editor after those. `test.sh` copies the project into a private mirror under `~/.cache/murabito/`, so it is safe with an editor open.

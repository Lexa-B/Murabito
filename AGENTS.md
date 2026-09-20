# AGENTS.md

This file provides guidance to AI coding agents when working with code in this repository. `CLAUDE.md` at the repo root is a one-line import shim (`@AGENTS.md`) kept for Claude Code's file-discovery convention.

## Project Overview

Murabito is a new project, described by its author as halfway between a game and a fun AI experiment / population-dynamics simulator. It's being built in Unreal Engine 5, starting with exp-04.

Ideas are tried out as small, self-contained **experiments**. They come in three kinds:

- **Unreal Engine 5 (C++)**: where most of the code is headed. exp-04 is the first: a UE 5.8 project with procedural hilly terrain, a hex grid draped over it, an overhead camera, and hover highlighting.
- **Python + pygame**: mostly for quick testing of ideas.
  - **exp-00** is a "hello world" of pathing and object use, meant to show what the AI systems will be like in UE.
  - **exp-01** builds on it with objects that wander.
  - **exp-02** adds fog of war and a believed world.
  - **exp-03** is a tiered hex world: hierarchical hex addresses at historical Japanese scale, loaded in tiers of detail (moderngl).
- **Rust + Bevy**: a second scaffolding line, tried when the user wanted something FOSS to build the coordinate system in. **exp-05** is the first: ri/cho/ken/shaku hex addresses with a vertical layer, hot-loaded chunks holding voxel columns, and an overhead viewer, in an engine-free core crate plus a Bevy plugin and app.

Don't assume goals beyond what `Experiments/manifest.md` and each experiment's spec state.


## Repository Organization

```
Murabito/
├─ AGENTS.md, CLAUDE.md, LICENSE, LICENSE-ASSETS, LICENSING.md, .gitignore, .gitattributes
├─ Murabito.uproject       the main project (see "Main project" below)
├─ Config/                 its text config
├─ Source/Murabito/        its C++ module; Private/Tests/ holds automation tests
├─ Content/                its assets (.uasset/.umap, through Git LFS)
├─ Plugins/                project plugins (none yet)
├─ Tools/                  build.sh, editor.sh, game.sh, test.sh for the main project
├─ Docs/                   project docs (no specs or plans; see below)
└─ Experiments/
   ├─ manifest.md          one entry per experiment: what, why, how to run
   ├─ exp-NN/              a Python + pygame experiment
   │  ├─ pyproject.toml    its own uv project (Python deps managed with `uv add`)
   │  ├─ src/              code
   │  │  └─ ai/            AI code (decision logic, pathing, etc.)
   │  ├─ tests/            pytest; run with `uv run pytest` from the experiment dir
   │  └─ docs/             specs/ and plans/, as below
   ├─ exp-NN/              an Unreal Engine 5 (C++) experiment
   │  ├─ MuraBito.uproject, Config/       project file and text config
   │  ├─ Source/MuraBito/                 the C++ module; Private/Tests/ holds automation tests
   │  ├─ scripts/                         build.sh, editor.sh, game.sh, test.sh (wrap the local engine)
   │  ├─ .gitignore                       Binaries/ Intermediate/ Saved/ DerivedDataCache/ …
   │  └─ docs/
   │     ├─ specs/         design specs    (YYYY-MM-DD-<topic>-design.md)
   │     └─ plans/         implementation plans (YYYY-MM-DD-<topic>-plan.md)
   └─ exp-NN/              a Rust + Bevy experiment
      ├─ Cargo.toml        workspace
      ├─ crates/           an engine-free core crate, a Bevy plugin, the viewer app
      └─ docs/             specs/ and plans/, as below
```

- **Experiments are self-contained.** Code, tests, dependencies and docs live inside `Experiments/exp-NN/`. Nothing experiment-specific goes at the repo root.
- **A new experiment can start as a copy of an earlier one.** exp-01 started as a copy of exp-00. The copy gets its own project name (uv project, or UE project/module), and the earlier experiment is left unchanged.
- **Every new experiment gets an entry in `Experiments/manifest.md`.** Update the entry when the experiment's design changes.
- **An experiment's specs go in its `docs/specs/` and plans in its `docs/plans/`.** This replaces the superpowers default of `docs/superpowers/specs/` and `docs/superpowers/plans/`. Don't create a `superpowers/` folder. Name specs `YYYY-MM-DD-<topic>-design.md` and plans `YYYY-MM-DD-<topic>-plan.md`.


## Main project

The repo root is the Murabito Unreal Engine 5.8 project. The user drives it; AI assists.

- **No spec or plan files for the main project.** Agree the design with the user in chat, wait for a yes, then implement it and open a PR. Specs and plans are only for experiments.
- **Build, test and run with `Tools/`:** `Tools/build.sh`, `Tools/test.sh [TestPathPrefix]` (default prefix `Murabito`), `Tools/editor.sh`, `Tools/game.sh`. The test mirror is `~/.cache/murabito/main-test-mirror`.
- **`AGameRules`** is the project's game mode (`/Script/Murabito.GameRules`).
- The Unreal Engine rules below apply to the main project as well as the UE experiments.


## Git workflow

`main` is protected on GitHub by the ruleset "Protect main": no direct pushes (for anyone, admins included), no force-pushes, and no deleting it. Every change reaches `main` through a pull request that the user reviews and merges.

- **Work on a branch.** Branch from an up-to-date `main` (`git fetch`, then branch from `origin/main`). Name it after the change, e.g. `agents-pr-workflow`.
- **Don't push or open a pull request until the user approves the work.** When a change is ready, commit it locally, tell the user what's there and how to check it, and wait. Approval to design or build something is not approval to open its PR.
- **Open the pull request** once approved, with `gh pr create` against `main` and a short summary of what changed and why. Keep each PR small enough to review in one sitting.
- **Never merge into `main` locally or push `main`.** The server rejects it. The user merges the PR on GitHub, then runs `git pull` on `main`.
- **Check the PR is still open before pushing follow-up commits** (`gh pr view <n> --json state,mergedAt`). If it was merged or closed in the meantime, start a fresh branch from `origin/main` and cherry-pick onto it.
- **Use `git -C <absolute path>` for every git command.** Worktrees and a shell working directory that persists between commands make relative paths easy to get wrong.


## Unreal Engine (main project and experiments)

- **Public MIT repo: never copy engine code or assets in.** Projects use the local engine install by path (`UE_ROOT`, default `/home/lexa/DevProjects/_GameDev/_GameEngines/UnrealEngine/5.8.2`). Reading engine headers and source to check an API is fine; pasting them, or Epic copyright headers, into the repo is not.
- **Two licences: code is MIT, assets are CC BY-SA 4.0.** See `LICENSING.md` for what counts as an asset. Third-party files (fonts, Epic/Fab content, anything not made for this repo) keep their own licences: add their licence file next to them and list them in `LICENSING.md`, and never commit anything whose licence doesn't allow redistribution.
- **Binary assets go through Git LFS.** `.gitattributes` at the repo root routes Unreal assets, source art, textures, audio, video, fonts, raw data, third-party binaries and PDFs to LFS (not lockable yet). Check a new binary type is covered before committing it; adding it afterwards means rewriting history. exp-04 itself has no binary assets: its world is built in C++ at runtime.
- **Naming:** keep UE's type prefixes (`A`, `F`, `U`), but the names after them are plain and say what the class does (`ACameraRig`, not a "…Pawn"). Code and docs don't name games that inspired the project.
- **Build and test with the scripts** (`Tools/` for the main project, `scripts/` in a UE experiment): `build.sh`, `test.sh [TestPathPrefix]` (headless automation tests; pass/fail comes from the log, since the editor's exit code is always 1 under `-TestExit`).
- **Never kill, signal or otherwise touch an Unreal Editor (or any process) you didn't start.**
- **The user may have the editor open on the same checkout while you work.** `build.sh` then does a hot-reload build, and the open editor loads it by itself within about a second. While the user is in Play, the reload waits until they stop. Changes to class layout (new or changed `UPROPERTY`/`UCLASS`, header changes) don't hot-reload reliably: tell the user to restart the editor after those.
- **`test.sh` is safe with an editor open.** It copies the project into a private mirror under `~/.cache/murabito/` and builds that with `-NoHotReloadFromIDE`, so the tests run the code on disk and never touch the editor's modules.
- **Windowed runs need the desktop display.** Shells in the user's terminal may have no `DISPLAY`/`WAYLAND_DISPLAY`; `game.sh` and `editor.sh` take them from the systemd user session. The main project's `Tools/` scripts run the editor on X11 (`SDL_VIDEODRIVER=x11`), because on SDL's Wayland backend editor pop-ups such as the Pick Parent Class tree stop taking clicks; set `SDL_VIDEODRIVER=wayland` to override.


## Rust + Bevy experiments

- **Build and test from the experiment directory:** `cargo build --release` and `cargo test`, e.g. `cd Experiments/exp-05 && cargo test`. A workspace member's own commands (`cargo run -p viewer --release`, `cargo test -p hexworld`) also work from there.
- **The core crate is engine-free** (no Bevy, no graphics, no dependencies at all in exp-05's case) and is the part meant to outlive whatever engine or graphics stack sits on top of it. Keep it that way: engine types and rendering code belong in the plugin crate or the app, not the core.
- **No Rust build directory under `/tmp`.** It is a RAM-backed tmpfs on this machine, and a `target/` there can silently blow the build past available memory. `cargo`'s default `target/` inside the experiment directory is fine; don't set `CARGO_TARGET_DIR` (or a temp working directory) to anywhere under `/tmp`.
- **Windowed runs need the desktop display**, same as the Unreal Engine experiments: take `DISPLAY`/`XAUTHORITY` from the systemd user session when the shell doesn't have them (`export $(systemctl --user show-environment | rg '^(DISPLAY|XAUTHORITY)=' | xargs)`).

# AGENTS.md

This file provides guidance to AI coding agents when working with code in this repository. `CLAUDE.md` at the repo root is a one-line import shim (`@AGENTS.md`) kept for Claude Code's file-discovery convention.

## Project Overview

Murabito is a new project, described by its author as halfway between a game and a fun AI experiment / population-dynamics simulator. The main project at the repo root is **Rust + Bevy**, and is being rebuilt from an empty root, one module at a time.

Two earlier attempts are kept, unbuilt, under `_Archives/`: the Unreal Engine 5 project the repo started as (`UE-Try/`), and the first Bevy crate (`Bevy-Try-1/`). See "The archived attempts" below.

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
├─ Cargo.toml, Cargo.lock, rust-toolchain.toml   the main project: a Cargo workspace
├─ crates/                 its members, one directory each (see "Main project" below)
│  ├─ murabito/            the app: the one binary, a plugin list and nothing else
│  ├─ scene/               `murabito_scene`: the placeholder world (ground, sun, sky colour)
│  ├─ camera/              `murabito_camera`: the overhead camera
│  ├─ keybinds/            `murabito_keybinds`: `Binds`, up to three keys or mouse buttons for one action
│  ├─ hexcoords/           `murabito_hexcoords`: hex voxel coordinates (`VoxelCoord`), per `Docs/hex_units_readme.md`
│  └─ action/              the actions layer, per `Docs/actions_readme.md`: a group of crates, not one
│     ├─ actions/          `murabito_actions`: `ActionQueue`, what a body has been asked to do, and the one system that issues intents for it
│     ├─ mechanisms/       one crate per kind of thing a body can do
│     │  └─ movement/      `murabito_movement`: where a body is and which way it faces, and the mechanics of stepping and turning, per `Docs/movement_readme.md`
│     └─ progress/         `murabito_progress`: the one accumulation bar every sustained action fills
├─ scripts/run.sh          launches the app; what a desktop entry points at. Finds the display itself, logs to ~/.cache/murabito/run.log
├─ Docs/                   project docs (no specs or plans; see below)
├─ _Archives/
│  ├─ UE-Try/              the Unreal Engine 5 attempt, kept for reference, not built
│  └─ Bevy-Try-1/          the first Bevy attempt, kept for reference, not maintained
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

The repo root is where the Murabito game lives, in Rust on Bevy: a Cargo workspace whose members sit under `crates/`. It is nearly empty: the first Bevy attempt grew tangled and was archived to `_Archives/Bevy-Try-1/`, and the project is being rebuilt from nothing. The user drives it; AI assists.

- **Build, run and test from the repo root:** `cargo run -p murabito`, `cargo test --workspace`, `cargo clippy --workspace --all-targets`, `cargo fmt --all`. The toolchain is pinned in `rust-toolchain.toml`; changing it is a deliberate edit, since a new compiler rebuilds the whole engine.
- **Each module is its own library crate under `crates/`.** Its `pub` items are its whole API and its `[dependencies]` are its whole wiring: a crate can't reach what another doesn't export, and Cargo refuses dependency cycles. `murabito` is the app and the only binary. Members are listed in the root manifest (`cargo new` adds a new one itself; a glob can't cover a group directory such as `crates/action/`), inherit `version`, `edition` and `publish = false` from `[workspace.package]` in the root manifest, and are never published.
- **A module exports its `Plugin`, and as little else as it can.** Components, systems and helpers stay private; unit tests sit in the same file, where they can see them. Widening a crate's `pub` surface is a decision, agreed with the user, not a convenience.
- **The module that uses a setting owns it.** A module's tunables are a `pub` `Resource` with `pub` fields in that module's crate (`murabito_camera::CameraSettings`), inserted by its plugin with `init_resource`, so a value already there when the plugin is added is kept. Whoever edits or saves it depends on that crate; the module depends on none of them. It makes a bad value safe at the one place it uses it, since a setter can't guard what arrives from a file.
- **Actions are bound through `murabito_keybinds::Binds`**: up to three keys or mouse buttons. The crate owns the type and knows no actions; each module keeps its own `Binds` in its settings, and its systems ask for the `Inputs` parameter rather than naming a device. There is no central list of actions.
- **Logic goes in pure functions, and systems stay thin.** A system gathers what Bevy hands it and calls a function that takes plain values (`pan_direction`, `pan_speed`, `zoomed`, `eased_velocity`), which tests exercise without an app.
- **Tests run headless, and a test app has only what it is given.** `MinimalPlugins` plus exactly what the systems ask for: `InputPlugin` for input, `AssetPlugin` and an `init_asset` per asset type for assets. A system asking for a resource that isn't there is a panic, in a test and in the app alike. Drive frames with `app.update()`; fix the frame length with `TimeUpdateStrategy::ManualDuration` when a test is about time. No test opens a window or needs a GPU.
- **Check an engine API in the source before writing against it.** Bevy 0.19 renamed things the archive and older examples still use (`SceneRoot` is now `WorldAssetRoot`; events are messages, read with `MessageReader`). The registry source is under `~/.cargo/registry/src/`.
- **Compass invariant: east is +X, north is −Z, up is +Y.** Directions across the ground are compass points (`murabito_hexcoords::Direction`), never up or down, which are for gravity. With the overhead camera as it stands, north is up the screen. Stated in `Docs/hex_units_readme.md` and `Docs/movement_readme.md` too.
- **Assets live in `assets/` at the repo root**, where the art pipeline writes. `.cargo/config.toml` sets `BEVY_ASSET_ROOT` there, since Bevy would otherwise look beside the app's crate.

- **One module at a time, at a pace the user can learn from.** The user is learning Rust and Bevy through this rebuild. Explain what a piece does and why before writing it, do one small piece, let the user look at it, then agree the next. Show the API and the reasoning, not only the finished diff.
- **Modules are self-contained, with a clear job and a clear API.** Nothing reaches into another module's internals. Code follows Clean Code: small functions, names that say what things do, one level of abstraction at a time.
- **The archive is a reference, not a source.** `_Archives/Bevy-Try-1/` records what was tried and what was learned (its `README.md` holds the old layout and conventions, its `Docs/HANDOFF.md` the decisions and the Bevy 0.19 gotchas). Read it to avoid relearning something; don't copy a module back wholesale, and don't treat its conventions as settled: each is re-decided with the user as its module comes back.
- **No spec or plan files for the main project.** Agree the design with the user in chat, wait for a yes, then implement it and open a PR. Specs and plans are only for experiments.
- **Work in small, visible steps.** Build the smallest thing that puts something on screen, let the user see it, then agree the next step. Don't disappear into a long build-out.
- **Bevy compiles into the binary.** There is no engine install: the `bevy` dependency in `Cargo.toml` is the whole thing. A first build takes about 4.5 minutes (some 550 crates); after that, a change to the project's own code rebuilds in seconds. `target/` runs to roughly 10 GB.
- **Linux system libraries.** Bevy's default features build winit with the Wayland backend, which needs `libwayland-dev`, `libxkbcommon-dev` and `libudev-dev`. Without them the build fails in `wayland-sys`'s build script with a pkg-config error.
- **Windowed runs need the desktop display.** Shells in the user's terminal may have no `DISPLAY`/`WAYLAND_DISPLAY`; take them from the systemd user session (`systemctl --user show-environment`), as the experiments' scripts do.
- **Never kill, signal or otherwise touch a process you didn't start.**


## Rust + Bevy experiments

Each Rust + Bevy experiment is its own Cargo workspace, separate from the main project at the repo root.

- **Build and test from the experiment's directory**, e.g. `cd Experiments/exp-05 && cargo test`. Run from the repo root, `cargo` builds the main project's workspace instead. An experiment's workspace members also work from there (`cargo run -p viewer --release`, `cargo test -p hexworld`).
- **Keep an experiment's core crate engine-free.** exp-05's `hexworld` has no dependencies at all, not even dev-dependencies: it is the part meant to outlive whatever engine or graphics stack sits on top. Engine types and rendering code belong in the plugin crate or the app.
- **No Rust build directory under `/tmp`.** It is a RAM-backed tmpfs on this machine, and a Bevy `target/` there fills it. Keep `cargo`'s default `target/` inside the experiment, and don't point `CARGO_TARGET_DIR` under `/tmp`.
- **Windowed runs need the desktop display**, as for the main project.

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
- **Binary assets go through Git LFS.** `.gitattributes` at the repo root routes Unreal assets, source art, textures, audio, video, fonts, raw data, third-party binaries and PDFs to LFS (not lockable yet). Check a new binary type is covered before committing it; adding it afterwards means rewriting history. The main project has no binary assets yet; the archived Bevy attempt's one is its UI font.
- **Naming:** code and docs don't name games that inspired the project.


## The archived attempts

`_Archives/Bevy-Try-1/` holds the first Bevy attempt: a single crate, `murabito`, on Bevy 0.19, with its `src/`, `tests/`, `assets/`, `scripts/` and `Docs/`. It is kept for reference and is **not** maintained. It moved intact and its paths are relative to itself, so it should still build from its own directory; start from its `README.md`.

`_Archives/UE-Try/` holds the Unreal Engine 5.8 project the repo started as: `Murabito.uproject`, `Config/`, `Source/Murabito/`, `Content/` and `Tools/`. It is kept for reference and is **not** built or maintained. Its `Tools/` scripts expect it at the repo root and won't work where it now sits.

The rules below still apply to the UE experiment, exp-04, which is live:

- **Use the local engine install by path** (`UE_ROOT`, default `/home/lexa/DevProjects/_GameDev/_GameEngines/UnrealEngine/5.8.2`).
- **Build and test with the experiment's `scripts/`:** `build.sh`, `test.sh [TestPathPrefix]` (headless automation tests; pass/fail comes from the log, since the editor's exit code is always 1 under `-TestExit`).
- **Keep UE's type prefixes** (`A`, `F`, `U`), with plain names after them that say what the class does (`ACameraRig`, not a "…Pawn").
- **The user may have the editor open on the same checkout.** `build.sh` then does a hot-reload build. Changes to class layout don't hot-reload reliably: tell the user to restart the editor after those. `test.sh` copies the project into a private mirror under `~/.cache/murabito/`, so it is safe with an editor open.

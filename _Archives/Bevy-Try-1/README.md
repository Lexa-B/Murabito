# Bevy-Try-1 — the first Bevy attempt

The Rust + Bevy 0.19 crate that was the repo's root project from 2026-09-20 to
2026-09-21, archived when the root project was restarted to be rebuilt one module at a
time. It is kept for reference and is **not** maintained.

It moved here intact: `Cargo.toml`, `src/`, `tests/`, `assets/` and `scripts/` sit as
they did at the root, and every compile-time path is relative to the crate, so
`cargo run` from this directory is expected to work. That has not been tried since the
move; a first build is about 4.5 minutes and a 10 GB `target/` (gitignored).

- `Docs/HANDOFF.md` — what the project was at the end, the decisions behind it, and the
  Bevy 0.19 gotchas that cost time.
- `Docs/brain-hitlist.md` — a first draft of the agent architecture.

What follows is the layout and the conventions as `AGENTS.md` recorded them while this
was the main project. The layout list was already behind the code: it never gained
`being.rs`, `hex.rs`, `senses/` or `諸法.rs`; `Docs/HANDOFF.md` covers those.

### Layout

- `src/lib.rs` — the module list; everything lives in the library so the tests can reach it
- `src/main.rs` — the window and the plugin list, and nothing else
- `tests/` — integration tests, each file its own binary against the library
- `src/camera.rs` — `CameraRig` (a ground focus, a zoom distance, a pan velocity) and the pan/zoom/apply systems
- `src/scene.rs` — the placeholder world: ground, a cube, a sun
- `src/settings.rs` — `CameraSettings` and the settings file
- `src/settings_page.rs` — the settings screen
- `src/menu.rs` — the escape menu
- `src/state.rs` — `AppState` (Playing / Menu / Settings), and pausing
- `src/ui.rs` — what the screens share: scrim, buttons, slider visuals
- `src/i18n.rs` — the string catalogues, `Language`, and the `Localized` component
- `src/user_data.rs` — where the player's files live; the only place that decides that
- `src/debug_screen.rs` — the F3 screen: terse clickable lines over the live world
- `src/screenshot.rs` — F12, and the `--shot` command-line capture

### Conventions

- **The base unit is the shaku**, not the metre. One world unit is one 尺 (10/33 m, ~30.3 cm); every length in the crate is shaku unless it says otherwise, and metres appear only in comments to give a familiar sense of scale. Round numbers tend to land on 間 (6 shaku) and 町 (360 shaku). The sense grid is one shaku flat-to-flat, so a sense range in shaku is also a count of cells — which is why those ranges are integers rather than floats. Provisional: exp-05 is building the real ri/cho/ken/shaku system.
- **Each module registers itself through a `Plugin`.** `main.rs` adds plugins and knows nothing about what they need; a module's resources, systems and spawns are its own business.
- **Pausing is `Time<Virtual>`, not a flag.** Pausing the clock stops everything driven by elapsed time, so no system has to know a menu exists. `Time<Real>` keeps running, which keeps the UI responsive. Input is gated separately, with `run_if(in_state(AppState::Playing))`.
- **Settings are resources; `settings.rs` persists them.** Anything that edits a settings resource gets saved automatically. Nothing else writes the file.
- **One directory holds everything belonging to the player**, `~/.config/murabito/`, and `user_data.rs` is the only place that works out where it is. A module joins its own filename onto `UserData::root()` rather than resolving a path itself, so saves and anything else later land beside the settings. `UserDataPlugin` goes first in the plugin list: plugins read the resource while the app is being built, not once it runs.
- **Settings live at `settings.yaml` in that directory** (YAML, one top-level key per group). A missing file means defaults and is created; a file that fails to parse warns and is left alone so the user can fix it.
- **Windowed runs need the desktop display.** Shells in the user's terminal may have no `DISPLAY`/`WAYLAND_DISPLAY`; take them from the systemd user session (`systemctl --user show-environment`), as the experiments' scripts do.
- **Check visual work with a screenshot, not by asking.** F12 saves one to `screenshots/` (gitignored) while playing, and `cargo run -- --shot <path> --screen playing|menu|settings` drives the app to a screen, captures it and exits — no keyboard needed. Add `--settle <frames>` if 150 isn't long enough; capture too early and the PNG is a bare clear colour, because render pipelines compile on first use. Add `--zoom <shaku>` to frame anything drawn in the world — at the starting zoom, world-space overlays run off the edge — and `--debug` to open the F3 screen first.
- **Running the binary directly needs `BEVY_ASSET_ROOT`.** Bevy resolves `assets/` relative to the executable unless `cargo run` sets it, so a direct `./target/debug/murabito` can't find the font or the locales.
- **UI text is never a literal.** Every string comes from `assets/locales/{en,ja}.yaml` through a `Localized` key, and both catalogues must carry the same keys — a gap warns at startup. Text that is the same in every language (a language's own name, a number) uses `ui::spawn_literal_button` instead.
- **A fixed-width text node wraps.** Bevy UI text in a node with an explicit width will line-break, possibly onto an invisible whitespace line, which doubles the node's height and pushes the glyphs off centre. Use `LineBreak::NoWrap` on labels with a fixed width.
- **Tests run headless, and must stay that way.** `MinimalPlugins` brings the schedules
  and `Time`; add `bevy::state::app::StatesPlugin` for anything touching `AppState`, as it
  arrives with `DefaultPlugins` in the real app but not in `MinimalPlugins`. Drive frames
  with `app.update()`, never `app.run()`. No test may open a window or need a GPU.
- **Tests must not touch the player's real files.** Point `UserDataPlugin::at` at a
  temporary directory; never let a test fall back on the real one.
- **The F3 debug screen is not an `AppState`.** Every state but `Playing` pauses `Time<Virtual>`, and the point of that screen is watching the world carry on while you change what is drawn over it — so it spawns and despawns a UI root instead, and does its own hover tinting because `ui::button_visuals` runs only while a menu is up. Debug labels are localised like any other text; taxonomy keys are namespaced by their axis (`分類.狐`), and a key with no entry renders as itself, which on a debug screen is a fine answer.

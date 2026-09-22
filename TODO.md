# TODO

What's queued for the root project, in no particular order unless it says so. Each item is
agreed in chat before it's built (`AGENTS.md`); this is the list, not the design.

## Modules still to come back

- **World objects and the ontology (諸法).** What a thing *is*: the 諸法 tree, with the
  ontology split from the facets, as a crate of its own. The first consumer decides its API;
  spawning the fox and the flora then moves out of `murabito_scene`. Deferred 2026-09-22 until
  something needs to ask "what is this".
- **Obstruction and stepping up.** The design-only parts of `Docs/movement_readme.md`: a corner
  move only when both flanking faces are open, step-up by size, cost of climbing, and the A\*
  heuristic. Needs something in the world to be an obstacle, so it follows world objects.
- **Selection and an overlay.** A selection module that turns clicks into a `Selected` marker,
  and an overlay module that draws progress bars for selected bodies from `Progress` alone.
  Replaces `draw_progress_bars` in `murabito_scene`, which draws one under every busy body.
- **The settings page.** `murabito_settings_page` under `crates/ui/`, answering `Overlay::Settings`
  (today a bare dimmed overlay): a language picker (the first thing to write `Language`) and the
  pan-speed slider in octaves, which brings `spawn_slider` and thumb placement into the kit. Agreed
  2026-09-22 as the PR after the kit.
- **The rebind screen.** The first thing to edit `Binds`: capture the next key or mouse button,
  three slots per action, show conflicts. Its own design conversation.
- **The screenshot tool and the debug screen.** From the archive.
- **Hex: distance and the `units` module.** `Docs/hex_units_readme.md`'s build order, item 4.

## Loose ends in what's there

- `Action::Face` is in the enum and tested, but nothing uses it since the spin placeholder
  went. The first thing that wants to look somewhere without moving takes it up.
- `Progress` could record the intent's type name alongside its `TypeId`, exposed as
  `doing() -> Option<&'static str>`, when a UI wants to label the bar.
- `CameraRig` has pairs of fields that must agree at rest (`zoom` with `zoom_target`,
  `velocity` with zero); a `CameraRig::at_rest(focus, zoom)` constructor would make an
  inconsistent rig unbuildable, once more code constructs rigs.
- Pan speed is constant per zoom; nothing stops the camera panning off the edge of the ground.
- The fox's `FOX_LOCOMOTION` (4 shaku/s, 180°/s) is a placeholder until species decide it.
- `Language::ALL` has no consumer until the language picker.
- Space with an overlay up resumes the world and closes the overlay, which will also close the
  settings page mid-drag on a slider. Accepted for now; the fix is a "the UI has the keyboard"
  gate, decided the day a text field exists.
- A pause with nothing over it (`Overlay::None`, what Space gives) shows no sign of being paused.
  A small PAUSED indicator is the missing piece.
- The real `~/.config/murabito/settings.yaml` still carries a `ui: {language: en}` section from
  the first attempt, which nothing registers; it rides along harmlessly and can be deleted.

## Housekeeping

- The desktop shortcut and the warm `target/` (~89 GB) live in the `cleanup-refactor` worktree;
  when the rebuild gets a permanent home, repoint `scripts/run.sh`'s two paths in the `.desktop`
  entries and move `target/`.
- Stale worktree: `.claude/worktrees/murabito` on `layer-skeleton`, whose one commit merged in
  PR #16; it holds a rebuilt `target/` of the old game.
- Merged branches to prune: local `cleanup-refactor`, `rebuild-workspace`, `scene-crate`,
  `hex-coords`, `movement`; remote `origin/movement`, `origin/cleanup-refactor` if still there.
- `art/__pycache__/` in the main checkout is gitignored clutter from the art scripts.
- `/tmp` scratchpads from older sessions (`/tmp/claude-1000/-home-lexa`, ~16 GB) are on a
  RAM-backed tmpfs.

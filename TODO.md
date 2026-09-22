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
- **The keyboard gate, then typing a number into the settings page.** Agreed 2026-09-22 as its own
  PR, in this order. (1) While Bevy's `InputFocus` holds an entity, game keys don't fire: a run
  condition on `app_state`'s pause toggle and `navigation`'s back, an engine resource so neither
  crate gains a dependency; it also ends the slider-mid-drag edge below if the slider takes focus.
  (2) Double-click on the pan-speed readout (picking's `Click { count }`) swaps it for an
  `EditableText` field from `bevy_ui_widgets`; Enter parses, clamps to the slider's range, writes
  `CameraSettings` and replaces `SliderValue`; Escape or losing focus cancels. `bevy_feathers`'
  `number_input` is the crib. Parse and clamp are pure functions in the kit.
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
- Space with an overlay up resumes the world and closes the overlay, which also closes the
  settings page mid-drag on a slider. Accepted for now; the keyboard gate above is the fix.
- The Japanese for "Pan speed" is `視点移動速度`, the common phrasing on Japanese settings screens,
  chosen by the agent, not a translator.
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

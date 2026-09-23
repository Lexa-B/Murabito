# TODO

What's queued for the root project, in no particular order unless it says so. Each item is
agreed in chat before it's built (`AGENTS.md`); this is the list, not the design.

## Modules still to come back

- **The rest of the kinds.** `murabito_kinds` has the tiers, the seventeen animals and the
  twelve plants; the spiritual leaves (tsukumogami, yoko, bakedanuki, ningyo, kaika, hitodama),
  the humans, and the objects arrive as models and needs do. Facets (what a thing is *like*:
  age, trophic role, height, what it does to each sense) are the archive's second axis and are
  not designed yet; they'd be members on tiers, or their own components. Sight is back and
  runs without them (everything is opaque); what they unlock is listed under the senses' facet
  debt below and in `Docs/perception_readme.md`.
- **The scene's walks are placeholder AI.** The fox's loop and the hare's triangle, with the
  hare's rest between sides, are what an AI layer will do: decide, and push onto the queue, or
  not. A rest is not an action (Lexa, 2026-09-22): an idle body is an empty queue by the AI's
  choice, so there is no `Action::Wait`; the walks and the `TriangleWalk` timer go when the AI
  layer arrives.
- **Picking a plant's version, colour and season.** A plant kind names its first version in
  summer; a spawner gives its own `Model` to say otherwise. Nothing yet picks at random, or
  reads a calendar.
- **Obstruction and stepping up.** The design-only parts of `Docs/movement_readme.md`: a corner
  move only when both flanking faces are open, step-up by size, cost of climbing, and the A\*
  heuristic. `murabito_perception::Occupancy` is already the map of what stands where; the
  corner rule is a read of it.
- **The senses' facet debt.** `Docs/perception_readme.md`: obscuring and heights (grass costs a
  band; knee-high grass hides a hare and not a person), and ambiguation (a 妖狐 in human form at
  `Mid` reads as a humanoid; the percept becomes something short of an `Entity`). Both in
  `murabito_perception`, when facets return, since they apply to every sense's list.
- **Hearing and smell.** Each its own crate under `crates/perception/senses/`, its own list in
  its own shape: a bearing and an intensity; an intensity and a gradient. Design in
  `Docs/perception_readme.md`.
- **Seeing across layers.** The cast is planar on the eye's layer; `Offset` already carries
  `dlayer`.
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
- **The screenshot tool, and a debug overlay in the window.** From the archive. The debug
  feature (`Docs/debug_readme.md`) reads the world from outside instead; what is still queued
  is an in-window home for the sightlines, cones and progress bar `murabito_scene` draws.
- **A web page on the debug server.** Lexa's stated want after `scripts/probe.sh`: a page
  that polls or watches and draws things, `Seen` as lines and `Occupancy` as cells beside the
  raw JSON. The CORS headers are already on. Design in `Docs/debug_readme.md`.
- **`Progress`, `ActionQueue` and the camera on the debug wire.** Two lines each when wanted.
- **A tick-by-tick view.** Lexa, 2026-09-24: after the debug mode, something that shows
  every step of a tick in order (asking, issuing, mechanisms, sweep and gather, sensing) and
  what each wrote, to wrap one's head around the cycle. Design on the debug feature: the
  remote server sees a frame's end, so this needs either a step-one-tick control or a
  per-step trace the server can read. Its own design conversation.
- **Hex: distance and the `units` module.** `Docs/hex_units_readme.md`'s build order, item 4.

## Loose ends in what's there

- `Action::Face` is in the enum and tested, but nothing uses it since the spin placeholder
  went. The first thing that wants to look somewhere without moving takes it up.
- `Sentient` carries `Locomotion` (a placeholder pace), `Vision` and `ActionQueue`; the
  animals' paces are guesses at a walk, marked as such in each file, until someone who
  knows says otherwise. The fox's are the ones settled by eye in the first attempt.
- `Progress` could record the intent's type name alongside its `TypeId`, exposed as
  `doing() -> Option<&'static str>`, when a UI wants to label the bar.
- `CameraRig` has pairs of fields that must agree at rest (`zoom` with `zoom_target`,
  `velocity` with zero); a `CameraRig::at_rest(focus, zoom)` constructor would make an
  inconsistent rig unbuildable, once more code constructs rigs.
- Pan speed is constant per zoom; nothing stops the camera panning off the edge of the ground.
- The cast runs every tick for every sighted body, and `Occupancy` is rebuilt every tick from
  every position: about 8,400 cells per fox per tick. Fine for a few bodies; the agreed next
  step is to recast only when the looker's position or facing changed or anything's position
  changed, and to update the map on `Changed<VoxelPosition>`. The full-recompute version is the
  baseline to beat.
- Space with an overlay up resumes the world and closes the overlay, which also closes the
  settings page mid-drag on a slider. Accepted for now; the keyboard gate above is the fix.
- The Japanese for "Pan speed" is `視点移動速度`, the common phrasing on Japanese settings screens,
  chosen by the agent, not a translator.
- A pause with nothing over it (`Overlay::None`, what Space gives) shows no sign of being paused.
  A small PAUSED indicator is the missing piece.
- The real `~/.config/murabito/settings.yaml` still carries a `ui: {language: en}` section from
  the first attempt, which nothing registers; it rides along harmlessly and can be deleted.

## Housekeeping

- The desktop shortcut and the warm `target/` (~170 GB) live in the `cleanup-refactor` worktree;
  when the rebuild gets a permanent home, repoint `scripts/run.sh`'s two paths in the `.desktop`
  entries and move `target/`.
- Stale worktree: `.claude/worktrees/murabito` on `layer-skeleton`, whose one commit merged in
  PR #16; it holds a rebuilt `target/` of the old game.
- Merged branches to prune: local `app-state`, `cleanup-refactor`, `crate-manifest`,
  `docs-overlay-rule`, `hex-coords`, `kinds-skeleton`, `movement`, `rebuild-workspace`,
  `scene-crate`, `settings-i18n`, `settings-page`, `todo`, `ui-kit`; remote `origin/settings-i18n`.
- `art/__pycache__/` in the main checkout is gitignored clutter from the art scripts.
- `/tmp` scratchpads from older sessions (`/tmp/claude-1000/-home-lexa`, ~16 GB) are on a
  RAM-backed tmpfs.

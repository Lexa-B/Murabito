# exp-06: worldgen

The Murabito world as pure functions of seed and position, in Rust, checked cell by cell against the design
walkthrough on `viz/banana.html`. The spec is `docs/specs/2026-09-30-exp-06-worldgen-port-design.md`; the state
and history are `docs/HANDOFF.md`; the numbers the port must reproduce are `docs/reference-readings.md`.

## Layout

- `crates/worldgen`: the engine-free crate, no dependencies. `hex` (cube coordinates, directions, packing),
  `settings`, `format` are shared. The numeric modules (`plane`, `banana`, `facts`, and the steps to come) are one
  source compiled twice: `worldgen::f64` is the reference and `worldgen::f32` the shader's twin. Each twin fixes
  `type F`, `PI`, `SQRT_3`, `TOL` and `rad`, and includes the files by `#[path]`. Write float literals untyped
  (`2.0`), convert integers with `as F`, and the same line serves both.
- `crates/exp06`: the local server. `cargo run -p exp06`, then open `http://127.0.0.1:8060/`. It serves
  `viz/exp-06.html` from disk (edit the page, reload) and answers `POST /render` with what the page draws.
- `viz/banana.html`: the reference page, opened from disk. `viz/exp-06.html`: the port's page. Both read and
  write the same settings in the URL query, so one link with the other filename is the same world.
- `viz/tools/check.js`: renders `banana.html` headlessly and prints every facts row. `viz/tools/check-port.js`:
  prints the port's facts rows from the running server, in the same format; the `diff` of the two covers steps 1 and 2.

## Build, test, check

From this directory (never from the repo root):

    cargo test --workspace
    cargo clippy --workspace --all-targets
    cargo run -p exp06

    node viz/tools/check.js viz/banana.html | sed -n '/^-- facts$/,/^-- facts3$/p' | sed '$d' > target/banana-facts.txt
    node viz/tools/check-port.js | sed -n '/^-- facts$/,$p' > target/port-facts.txt
    diff target/banana-facts.txt target/port-facts.txt && echo SAME

`FLAGS=1` makes both scripts append ` [bad]` to flagged rows, so the diff compares the flags too.
`check.js` takes `key=value` settings after the file (`node viz/tools/check.js viz/banana.html theta=200 inset=-4`),
the same keys as the URL query.

## Rules

Cube `(q, r, s)` everywhere in runtime code. The API names cells at a rung; the plane is derived inside.
Sizes Lexa fixed are constants. One bite at a time, each checked against the page. Commit only when Lexa says.

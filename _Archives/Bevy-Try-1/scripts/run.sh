#!/usr/bin/env bash
# Launch Murabito.
#
# Exists so a desktop entry has something stable to point at, and so a launch from
# outside a terminal still finds cargo and the project. Locates the checkout from its
# own path, so it works from the main checkout or any worktree.
set -euo pipefail

# A desktop launcher does not read a shell profile, so cargo is not on PATH.
export PATH="$HOME/.cargo/bin:$PATH"

root="$(cd "$(dirname "$(readlink -f "$0")")/.." && pwd)"
cd "$root"

# Nothing is attached to stdout when launched from a desktop entry, so a build error
# would vanish. Keep the last run's output where it can be read.
log_dir="${XDG_CACHE_HOME:-$HOME/.cache}/murabito"
mkdir -p "$log_dir"
exec > >(tee "$log_dir/run.log") 2>&1

echo "murabito: $root"
# Debug, not release: the dev profile already builds dependencies at opt-level 3 and our
# own code at 1, which is plenty, and a release build is a 4.5 minute wait for a launcher.
# `--` so flags reach the game, not cargo: `run.sh --shot x.png` otherwise fails in
# cargo's own argument parser.
exec cargo run -- "$@"

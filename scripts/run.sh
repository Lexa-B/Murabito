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

# A terminal tab may have no display; a desktop launch already does. Take whichever is
# missing from the systemd user session.
while IFS= read -r line; do
    case "$line" in
        DISPLAY=* | WAYLAND_DISPLAY=* | XAUTHORITY=*)
            name="${line%%=*}"
            [[ -n "${!name:-}" ]] || export "$line"
            ;;
    esac
done < <(systemctl --user show-environment 2>/dev/null || true)

# Nothing is attached to stdout when launched from a desktop entry, so a build error
# would vanish. Keep the last run's output where it can be read.
log_dir="${XDG_CACHE_HOME:-$HOME/.cache}/murabito"
mkdir -p "$log_dir"
exec > >(tee "$log_dir/run.log") 2>&1

# `--debug` first is ours: the debug build, which also serves the world's data to
# scripts/probe.sh (Docs/debug_readme.md). Everything else goes to the game.
features=()
if [[ "${1:-}" == "--debug" ]]; then
    features=(--features debug)
    shift
fi

echo "murabito: $root ${features[*]:-}"
# Dev profile, not release: it already builds dependencies at opt-level 3 and our own
# code at 1, which is plenty, and a release build is a long wait for a launcher.
# `--` so flags reach the game, not cargo.
exec cargo run -p murabito "${features[@]}" -- "$@"

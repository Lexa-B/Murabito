#!/usr/bin/env bash
# Rebuild every committed model and compare it face by face with the committed file:
# each face's shape and the colour it gets. Run it after changing shared code or the
# palette, to prove nothing changed that shouldn't have.
#
#   art/tools/check.sh <dir> [--above Z]
#
# Rebuilt models go to <dir>/rebuild, in the same folders as under assets/entity_models/.
# With --above, only faces wholly above height Z are compared (for a change meant to
# touch only what's below, like burying trunks).
# If everything matches and you want the rebuilt files (say, after a palette change,
# so each carries the current palette), copy them in:
#   cp -r <dir>/rebuild/. assets/entity_models/
set -euo pipefail
work=${1:?usage: check.sh <dir> [--above Z]}
shift
tools=$(cd "$(dirname "$0")" && pwd)
root=$(git -C "$tools" rev-parse --show-toplevel)

"$tools/rebuild.sh" "$work/rebuild" > /dev/null
fingerprint() {
  blender -b --python "$tools/fingerprint.py" -- "$@" 2>/dev/null | awk '/^FINGERPRINT/ {print $2, $3, $4}' | sort
}
fingerprint "$@" $(git -C "$root" ls-files 'assets/entity_models/*.glb' | sed "s|^|$root/|") > "$work/committed.txt"
fingerprint "$@" $(find "$work/rebuild" -name "*.glb") > "$work/rebuilt.txt"
echo "$(wc -l < "$work/committed.txt") committed, $(wc -l < "$work/rebuilt.txt") rebuilt"
if diff "$work/committed.txt" "$work/rebuilt.txt"; then
  echo "ALL MATCH"
else
  echo "DIFFERENT (committed <, rebuilt >)"; exit 1
fi

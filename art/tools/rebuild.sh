#!/usr/bin/env bash
# Rebuild every committed model into <dir>, each from its own script with the flags
# its file name implies:
#   animals   <animal>.glb                          art/<animal>.py
#   plants    <plant>-<version>-<colour>-<season>.glb   art/<plant>.py --version --colour --season
#
#   art/tools/rebuild.sh <dir>
set -euo pipefail
out=${1:?usage: rebuild.sh <dir>}
root=$(git -C "$(dirname "$0")" rev-parse --show-toplevel)
mkdir -p "$out"
cd "$root"
for file in $(git ls-files 'assets/models/*.glb'); do
  name=$(basename "$file" .glb)
  if [[ $name != *-* ]]; then
    args=(art/"$name".py --)
  else
    plant=${name%%-*}
    season=${name##*-}
    rest=${name%-*}             # <plant>-<version>-<colour>
    colour=${rest##*-}
    version=${rest%-*}
    version=${version#"$plant"-}  # a version can hold a hyphen, e.g. pruned-00
    args=(art/"$plant".py -- --version "$version" --colour "$colour" --season "$season")
  fi
  if ! log=$(blender -b --python "${args[@]}" --out "$out/$name.glb" 2>&1) || [[ $log == *Traceback* ]]; then
    echo "FAILED $name"; echo "$log" | tail -20; exit 1
  fi
  echo "built $name"
done

#!/usr/bin/env bash
# Rebuild every committed model into <dir>, each from its own script with the flags
# its file name implies. A model's folder under assets/entity_models/ is its script's
# folder under art/entity_models/, and the rebuilt file keeps it:
#   <folder>/<name>.glb                                art/entity_models/<folder>/<name>.py
#   <folder>/<plant>-<version>-<colour>-<season>.glb   art/entity_models/<folder>/<plant>.py
#                                                      --version --colour --season
#
#   art/tools/rebuild.sh <dir>
set -euo pipefail
out=${1:?usage: rebuild.sh <dir>}
root=$(git -C "$(dirname "$0")" rev-parse --show-toplevel)
cd "$root"
for file in $(git ls-files 'assets/entity_models/*.glb'); do
  folder=$(dirname "${file#assets/entity_models/}")
  name=$(basename "$file" .glb)
  if [[ $name != *-* ]]; then
    args=(art/entity_models/"$folder/$name".py --)
  else
    plant=${name%%-*}
    season=${name##*-}
    rest=${name%-*}             # <plant>-<version>-<colour>
    colour=${rest##*-}
    version=${rest%-*}
    version=${version#"$plant"-}  # a version can hold a hyphen, e.g. pruned-00
    args=(art/entity_models/"$folder/$plant".py -- --version "$version" --colour "$colour" --season "$season")
  fi
  mkdir -p "$out/$folder"
  if ! log=$(blender -b --python "${args[@]}" --out "$out/$folder/$name.glb" 2>&1) || [[ $log == *Traceback* ]]; then
    echo "FAILED $folder/$name"; echo "$log" | tail -20; exit 1
  fi
  echo "built $folder/$name"
done

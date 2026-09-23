#!/usr/bin/env bash
# Watches the debug server: asks it the same question every half second and redraws.
#
#   scripts/probe.sh              every thing by id, with place, facing, cone and seen list where it has them
#   scripts/probe.sh occupancy    the map of what stands where
#   scripts/probe.sh types        every component type the server knows
#   scripts/probe.sh <method> '<params json>'   any other question, raw
#
# Needs the debug build running: cargo run -p murabito --features debug
set -euo pipefail

PORT=15702
EVERY=0.5

# Every thing, which is anything with an id, with what each has of the rest: a tree has a
# place and nothing else; an intangible thing has not even that.
things='{"data":{
  "components":["murabito_identity::ThingId"],
  "option":["murabito_placement::VoxelPosition","murabito_placement::Facing",
            "murabito_vision::Vision","murabito_vision::Seen"]
}}'

case "${1:-things}" in
  things)    method=world.query;         params=$things ;;
  occupancy) method=world.get_resources; params='{"resource":"murabito_perception::Occupancy"}' ;;
  types)     method=world.list_components; params='null' ;;
  *)         method=$1;                  params=${2:-null} ;;
esac

# jq -c folds the params onto one line, so they can be written above however reads best.
request=$(jq -cn --arg m "$method" --argjson p "$params" '{jsonrpc:"2.0",id:1,method:$m,params:$p}')

while true; do
  answer=$(curl -s "localhost:$PORT" -H 'Content-Type: application/json' -d "$request" \
    || echo '{"error":"no server on port '"$PORT"' (is the debug build running?)"}')
  clear
  echo "$method  $(date +%T)"
  echo "$answer" | jq .
  sleep "$EVERY"
done

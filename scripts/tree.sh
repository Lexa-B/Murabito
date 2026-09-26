#!/usr/bin/env bash
# Prints the tree of kinds as the running world has it: built from the roster in
# murabito_kinds and each node's `require`, not read from the files.
set -euo pipefail
cd "$(dirname "$0")/.."
exec cargo run -q -p murabito_kinds --example tree

#!/usr/bin/env bash
# Regenerates the Python messages from the contract. Run after the .proto changes; a
# test fails until you do.
set -euo pipefail
cd "$(dirname "$0")"
uv run python -m grpc_tools.protoc -I ../../crates/ai/bridge/proto \
    --python_out=src/midbrain --pyi_out=src/midbrain murabito.proto
echo "regenerated src/midbrain/murabito_pb2.py"

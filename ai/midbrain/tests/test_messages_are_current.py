"""The checked-in messages match the contract: regenerate and compare.

``murabito_pb2.py`` is generated from ``crates/ai/bridge/proto/murabito.proto`` and
checked in, so ``uv run`` needs no generation step. If the contract moves and this is not
regenerated (``./regen.sh``), this fails.
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent.parent
PROTO_DIR = HERE.parents[1] / "crates" / "ai" / "bridge" / "proto"
CHECKED_IN = HERE / "src" / "midbrain" / "murabito_pb2.py"


def test_the_checked_in_messages_are_what_the_proto_generates(tmp_path: Path) -> None:
    subprocess.run(
        [
            sys.executable,
            "-m",
            "grpc_tools.protoc",
            f"-I{PROTO_DIR}",
            f"--python_out={tmp_path}",
            "murabito.proto",
        ],
        check=True,
    )
    fresh = (tmp_path / "murabito_pb2.py").read_text()
    assert CHECKED_IN.read_text() == fresh, "the .proto changed: run ./regen.sh"

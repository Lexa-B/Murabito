"""One order, by hand: ``uv run order <body> <what> ...``.

    uv run order 3 stop
    uv run order 3 face N              a direction: E ENE NNE N NNW WNW W WSW SSW S SSE ESE
    uv run order 3 face-thing 1        a thing, by its number
    uv run order 3 step ESE
    uv run order 3 goto 0 0            a cell, axial q r, and a layer if not 0

Sends it, waits a round, and says what the body is doing then. What a mind would say,
said once; the board (``uv run board``) shows the rest.
"""

from __future__ import annotations

import argparse
import time

from midbrain import murabito_pb2 as pb
from midbrain.board import ROUND, doing, previous
from midbrain.client import HOST, PORT, Bridge

DIRECTIONS = [name for name, _ in sorted(pb.Direction.items(), key=lambda item: item[1])]
"""The twelve, in index order, as the contract spells them."""


TAKES = {
    "stop": "nothing",
    "face": "a direction",
    "face-thing": "a thing's number",
    "step": "a direction",
    "goto": "q r [layer]",
}
"""What each verb wants after it."""


class Unparseable(ValueError):
    """The words don't make an intent; the message says what would."""


def direction(word: str) -> int:
    name = word.upper()
    if name not in DIRECTIONS:
        raise Unparseable(f"no direction {word!r}: one of {' '.join(DIRECTIONS)}")
    return pb.Direction.Value(name)


def number(word: str, what: str) -> int:
    if not word.isdigit():
        raise Unparseable(f"{what} must be a number, not {word!r}")
    return int(word)


def integer(word: str, what: str) -> int:
    try:
        return int(word)
    except ValueError:
        raise Unparseable(f"{what} must be a whole number, not {word!r}") from None


def parse(words: list[str]) -> pb.Intent:
    """The intent the words mean, or ``Unparseable`` saying what they should have been."""
    if not words:
        raise Unparseable("say what: stop, face, face-thing, step or goto")
    verb, rest = words[0].lower(), words[1:]
    match verb, len(rest):
        case "stop", 0:
            return pb.Intent(short=pb.Short(stop=pb.Stop()))
        case "face", 1:
            return pb.Intent(short=pb.Short(face=direction(rest[0])))
        case "face-thing", 1:
            return pb.Intent(short=pb.Short(face_thing=number(rest[0], "a thing")))
        case "step", 1:
            return pb.Intent(short=pb.Short(step=direction(rest[0])))
        case "goto", 2 | 3:
            q, r = integer(rest[0], "q"), integer(rest[1], "r")
            layer = integer(rest[2], "the layer") if len(rest) == 3 else 0
            cell = pb.Voxel(q=q, r=r, layer=layer)
            return pb.Intent(sustained=pb.Sustained(go_to=cell))
    if verb in TAKES:
        raise Unparseable(f"{verb} takes {TAKES[verb]}, not {' '.join(rest) or 'nothing'}")
    raise Unparseable(f"no such order {verb!r}: one of {' '.join(TAKES)}")


def send(host: str, port: int, body: int, intent: pb.Intent) -> str:
    """Sends the order and, a round later, reports what the body is doing."""
    with Bridge(host, port) as bridge:
        bridge.order(body, intent)
        time.sleep(ROUND)
        snapshot = next((s for s in bridge.snapshots() if s.id == body), None)
    if snapshot is None:
        return f"sent to #{body}, but no such body is on the board: the order was dropped"
    return f"#{body}: doing {doing(snapshot)}; last {previous(snapshot)}"


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Tell one body what to want.",
        epilog=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument("--host", default=HOST)
    parser.add_argument("--port", type=int, default=PORT)
    parser.add_argument("body", type=int, help="the body's number, as the board shows it")
    parser.add_argument("what", nargs="+", help="stop | face DIR | face-thing N | step DIR | goto Q R [LAYER]")
    args = parser.parse_args()
    try:
        intent = parse(args.what)
    except Unparseable as why:
        parser.error(str(why))
    try:
        print(send(args.host, args.port, args.body, intent))
    except ConnectionRefusedError:
        raise SystemExit(f"no game listening on {args.host}:{args.port}: is it running?")

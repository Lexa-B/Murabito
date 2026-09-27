"""One order, by hand: ``uv run order <body> <what> ...``.

    uv run order 3 stop
    uv run order 3 face N              a direction: E ENE NNE N NNW WNW W WSW SSW S SSE ESE
    uv run order 3 face-thing 1        a thing, by its number
    uv run order 3 walk ESE            or jog, sprint, sneak: turning first if need be
    uv run order 3 sidestep N          or backstep, recoil, lunge: only that way of the facing
    uv run order 3 bite
    uv run order 3 walkto 0 0          or jogto, sprintto, sneakto: a cell, axial q r, and a layer if not
                                       0, as a path of steps straight there from where the body stands

Sends it, waits a round, and says what the body is doing then. What a mind would say,
said once; the board (``uv run board``) shows the rest.
"""

from __future__ import annotations

import argparse
import time

from midbrain import murabito_pb2 as pb
from midbrain.beliefs import Cell
from midbrain.board import ROUND, doing, previous
from midbrain.client import HOST, PORT, Bridge
from midbrain.paths import plan

DIRECTIONS = [name for name, _ in sorted(pb.Direction.items(), key=lambda item: item[1])]
"""The twelve, in index order, as the contract spells them."""


WORDS = ["walk", "jog", "sprint", "sneak", "sidestep", "backstep", "recoil", "lunge"]
"""The action words that take a direction, as the contract spells them."""

PACES = {"walkto": "walk", "jogto": "jog", "sprintto": "sprint", "sneakto": "sneak"}
"""The verbs that plan a path to a cell, and the word each step is on the wire."""

TAKES = {
    "stop": "nothing",
    "face": "a direction",
    "face-thing": "a thing's number",
    **{word: "a direction" for word in WORDS},
    "bite": "nothing",
    **{pace: "q r [layer]" for pace in PACES},
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


def parse(words: list[str], here: Cell | None = None) -> pb.Intent:
    """The intent the words mean, or ``Unparseable`` saying what they should have been.
    A pace to a cell is planned as a path from ``here``, so it needs to know where the
    body stands."""
    if not words:
        raise Unparseable(f"say what: one of {' '.join(TAKES)}")
    verb, rest = words[0].lower(), words[1:]
    match verb, len(rest):
        case "stop", 0:
            return pb.Intent(short=pb.Short(stop=pb.Stop()))
        case "face", 1:
            return pb.Intent(short=pb.Short(face=direction(rest[0])))
        case "face-thing", 1:
            return pb.Intent(short=pb.Short(face_thing=number(rest[0], "a thing")))
        case word, 1 if word in WORDS:
            return pb.Intent(short=pb.Short(**{word: direction(rest[0])}))
        case "bite", 0:
            return pb.Intent(short=pb.Short(bite=pb.Bite()))
        case pace, 2 | 3 if pace in PACES:
            q, r = integer(rest[0], "q"), integer(rest[1], "r")
            layer = integer(rest[2], "the layer") if len(rest) == 3 else 0
            if here is None:
                raise Unparseable(f"{pace} needs to know where the body stands")
            steps = plan(here, Cell(q, r, layer))
            if steps is None:
                raise Unparseable(f"no way from {here} to ({q}, {r})")
            return pb.Intent(path=pb.Path(steps=[pb.Action(**{PACES[pace]: step}) for step in steps]))
    if verb in TAKES:
        raise Unparseable(f"{verb} takes {TAKES[verb]}, not {' '.join(rest) or 'nothing'}")
    raise Unparseable(f"no such order {verb!r}: one of {' '.join(TAKES)}")


def send(host: str, port: int, body: int, words: list[str]) -> str:
    """Looks the body up, makes the words its intent from where it stands, sends it and,
    a round later, reports what the body is doing."""
    with Bridge(host, port) as bridge:
        standing = next((s for s in bridge.snapshots() if s.id == body), None)
        if standing is None:
            return f"no body #{body} is on the board"
        bridge.order(body, parse(words, Cell.of(standing.position)))
        time.sleep(ROUND)
        snapshot = next((s for s in bridge.snapshots() if s.id == body), None)
    if snapshot is None:
        return f"sent to #{body}, but it left the board: the order was dropped"
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
    parser.add_argument(
        "what",
        nargs="+",
        help="stop | face DIR | face-thing N | WORD DIR | bite | PACEto Q R [LAYER]",
    )
    args = parser.parse_args()
    try:
        parse(args.what, Cell(0, 0))  # the words alone, before anything is sent
        print(send(args.host, args.port, args.body, args.what))
    except Unparseable as why:
        parser.error(str(why))
    except ConnectionRefusedError:
        raise SystemExit(f"no game listening on {args.host}:{args.port}: is it running?")

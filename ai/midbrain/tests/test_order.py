"""The words of an order, and the intent they mean."""

from __future__ import annotations

import re

import pytest

from midbrain import murabito_pb2 as pb
from midbrain.order import DIRECTIONS, Unparseable, parse


def test_the_twelve_directions_in_index_order() -> None:
    assert DIRECTIONS == ["E", "ENE", "NNE", "N", "NNW", "WNW", "W", "WSW", "SSW", "S", "SSE", "ESE"]


def test_each_verb_makes_its_intent() -> None:
    assert parse(["stop"]).short.WhichOneof("kind") == "stop"
    assert parse(["face", "n"]).short.face == pb.Direction.N
    assert parse(["face-thing", "7"]).short.face_thing == 7
    assert parse(["walk", "ESE"]).short.walk == pb.Direction.ESE
    assert parse(["sprint", "e"]).short.sprint == pb.Direction.E
    assert parse(["recoil", "W"]).short.recoil == pb.Direction.W
    assert parse(["bite"]).short.WhichOneof("kind") == "bite"
    cell = parse(["walkto", "3", "-5"]).sustained.walk_to
    assert (cell.q, cell.r, cell.layer) == (3, -5, 0)
    assert parse(["walkto", "0", "0", "2"]).sustained.walk_to.layer == 2
    assert parse(["sneakto", "1", "1"]).sustained.WhichOneof("kind") == "sneak_to"
    assert parse(["jogto", "1", "1"]).sustained.WhichOneof("kind") == "jog_to"
    assert parse(["sprintto", "1", "1"]).sustained.WhichOneof("kind") == "sprint_to"


@pytest.mark.parametrize(
    ("words", "says"),
    [
        ([], "say what"),
        (["dance"], "no such order 'dance'"),
        (["face", "up"], "no direction 'up'"),
        (["face"], "face takes a direction"),
        (["stop", "now"], "stop takes nothing"),
        (["lunge"], "lunge takes a direction"),
        (["bite", "hard"], "bite takes nothing"),
        (["walkto", "1"], "walkto takes q r [layer]"),
        (["sneakto", "one", "2"], "q must be a whole number"),
        (["goto", "1", "2"], "no such order 'goto'"),
        (["face-thing", "hare"], "a thing must be a number"),
    ],
)
def test_what_cannot_be_parsed_says_what_would(words: list[str], says: str) -> None:
    with pytest.raises(Unparseable, match=re.escape(says)):
        parse(words)

"""The words of an order, and the intent they mean."""

from __future__ import annotations

import re

import pytest

from midbrain import murabito_pb2 as pb
from midbrain.beliefs import Cell
from midbrain.order import DIRECTIONS, Unparseable, parse
from midbrain.paths import landing


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
    here = Cell(0, 0)
    route = parse(["walkto", "3", "-5"], here).path
    assert route.keep == 0 and {step.WhichOneof("kind") for step in route.steps} == {"walk"}
    assert landing(here, [step.walk for step in route.steps]) == Cell(3, -5)
    aloft = parse(["walkto", "1", "0", "2"], Cell(0, 0, layer=2)).path
    assert landing(Cell(0, 0, layer=2), [step.walk for step in aloft.steps]) == Cell(1, 0, layer=2)
    assert parse(["sneakto", "1", "1"], here).path.steps[0].WhichOneof("kind") == "sneak"
    assert parse(["jogto", "1", "1"], here).path.steps[0].WhichOneof("kind") == "jog"
    assert parse(["sprintto", "1", "1"], here).path.steps[0].WhichOneof("kind") == "sprint"
    assert len(parse(["walkto", "0", "0"], here).path.steps) == 0, "standing there: a path of no steps"


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
        (["sneakto", "1", "2"], "sneakto needs to know where the body stands"),
        (["goto", "1", "2"], "no such order 'goto'"),
        (["face-thing", "hare"], "a thing must be a number"),
    ],
)
def test_what_cannot_be_parsed_says_what_would(words: list[str], says: str) -> None:
    with pytest.raises(Unparseable, match=re.escape(says)):
        parse(words)

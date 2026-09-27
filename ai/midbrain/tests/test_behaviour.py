"""The tree primitives, on toy trees that read a plain dict."""

from __future__ import annotations

from midbrain import murabito_pb2 as pb
from midbrain.behaviour import FAILURE, Act, Condition, Result, Selector, Sequence

STOP = pb.Intent(short=pb.Short(stop=pb.Stop()))
FACE_N = pb.Intent(short=pb.Short(face=pb.Direction.N))


def test_a_condition_succeeds_or_fails_and_wants_nothing() -> None:
    hungry = Condition("hungry", lambda ctx: ctx["food"] < 1)
    assert hungry.tick({"food": 0}) == Result(True, None, ("hungry",))
    assert hungry.tick({"food": 3}) == FAILURE


def test_an_act_succeeds_with_its_want_or_with_none() -> None:
    stop = Act("stop", lambda ctx: STOP)
    hold = Act("hold", lambda ctx: None)
    assert stop.tick({}) == Result(True, STOP, ("stop",))
    assert hold.tick({}) == Result(True, None, ("hold",))


def test_a_selector_takes_the_first_child_that_succeeds_and_asks_no_further() -> None:
    asked: list[str] = []

    def act(name: str, intent: pb.Intent):
        def want(ctx):
            asked.append(name)
            return intent
        return Act(name, want)

    tree = Selector("pick", (
        Sequence("if hungry", (Condition("hungry", lambda ctx: ctx["food"] < 1), act("eat", STOP))),
        act("look north", FACE_N),
        act("never", STOP),
    ))
    assert tree.tick({"food": 0}) == Result(True, STOP, ("pick", "if hungry", "eat"))
    assert asked == ["eat"]
    assert tree.tick({"food": 5}) == Result(True, FACE_N, ("pick", "look north"))
    assert asked == ["eat", "look north"]


def test_a_sequence_stops_at_the_first_failure_and_wants_what_its_last_child_wants() -> None:
    tree = Sequence("both", (
        Condition("a", lambda ctx: ctx["a"]),
        Condition("b", lambda ctx: ctx["b"]),
        Act("go", lambda ctx: FACE_N),
    ))
    assert tree.tick({"a": True, "b": True}) == Result(True, FACE_N, ("both", "go"))
    assert tree.tick({"a": True, "b": False}) == FAILURE
    assert tree.tick({"a": False, "b": True}) == FAILURE


def test_a_selector_with_nothing_to_offer_fails() -> None:
    tree = Selector("empty", (Condition("no", lambda ctx: False),))
    assert tree.tick({}) == FAILURE
    assert Selector("none", ()).tick({}) == FAILURE


def test_an_empty_sequence_succeeds_wanting_nothing() -> None:
    assert Sequence("nothing", ()).tick({}) == Result(True, None, ("nothing",))

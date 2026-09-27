"""The wander: a leg ahead, a rest, another leg; the rest remembered on the body's scratch."""

from __future__ import annotations

import math
import random

from midbrain import murabito_pb2 as pb
from midbrain.ambitions import Context, Idle, Stalk, Wander, bound_for, choose, walk_to
from midbrain.beliefs import BelievedWorld, Cell
from midbrain.hexes import angle_of, apart, bearing, to_world
from midbrain.mind import Mind
from test_ambitions import HARE, fox_believing, fox_snapshot


def wander(seed: int = 7) -> Wander:
    return Wander(rng=random.Random(seed))


def context(tick: int, scratch: dict | None = None, doing: pb.Intent | None = None, facing: int = pb.Direction.E) -> Context:
    return Context(BelievedWorld(body=1), fox_snapshot((0, 0), facing, tick, doing=doing), scratch if scratch is not None else {})


def shaku_between(a: Cell, b: Cell) -> float:
    (ax, az), (bx, bz) = to_world(a), to_world(b)
    return math.hypot(bx - ax, bz - az)


def test_with_nothing_in_hand_and_no_rest_drawn_it_draws_one_and_holds() -> None:
    ctx = context(tick=64)
    result = wander().want(ctx)
    assert result.path == ("wander", "arrived", "rest") and result.intent is None
    until = ctx.scratch[Wander.REST_UNTIL]
    assert 64 + 2 * 64 <= until <= 64 + 8 * 64, "two to eight seconds at 64 ticks a second"


def test_while_the_rest_runs_it_holds() -> None:
    ctx = context(tick=200, scratch={Wander.REST_UNTIL: 300})
    result = wander().want(ctx)
    assert result.path == ("wander", "resting", "wait") and result.intent is None
    assert ctx.scratch[Wander.REST_UNTIL] == 300, "the rest is left as it was"


def test_once_the_rest_is_over_it_sets_off_on_a_leg_ahead_and_forgets_the_rest() -> None:
    for seed in range(20):
        ctx = context(tick=300, scratch={Wander.REST_UNTIL: 300}, facing=pb.Direction.N)
        result = wander(seed).want(ctx)
        assert result.path == ("wander", "set off")
        assert Wander.REST_UNTIL not in ctx.scratch
        there = bound_for(result.intent)
        assert result.intent.sustained.WhichOneof("kind") == "walk_to"
        assert 3.4 <= shaku_between(ctx.here, there) <= 8.6, "four to eight shaku, give or take the rounding to a cell"
        assert apart(bearing(ctx.here, there), angle_of(pb.Direction.N)) <= 90 + 10, "within the front 180, give or take the rounding"


def test_the_same_seed_draws_the_same_leg() -> None:
    a = wander(3).want(context(tick=300, scratch={Wander.REST_UNTIL: 300}))
    b = wander(3).want(context(tick=300, scratch={Wander.REST_UNTIL: 300}))
    assert a.intent == b.intent


def test_a_pace_in_hand_is_left_to_land() -> None:
    ctx = context(tick=320, doing=walk_to(Cell(5, 0)))
    result = wander().want(ctx)
    assert result.path == ("wander", "walking", "keep on") and result.intent is None
    assert ctx.scratch == {}, "no rest is drawn while walking"


def test_a_leg_then_a_rest_then_a_leg() -> None:
    fox, scratch = wander(1), {}
    assert fox.want(context(64, scratch)).path[1] == "arrived"
    until = scratch[Wander.REST_UNTIL]
    first = fox.want(context(until, scratch))
    assert first.path == ("wander", "set off")
    assert fox.want(context(until + 8, scratch, doing=first.intent)).path[1] == "walking"
    assert fox.want(context(until + 200, scratch)).path[1] == "arrived", "the leg landed: rest again"
    assert scratch[Wander.REST_UNTIL] > until + 200


def test_the_wander_beats_idling_and_loses_to_a_stalk_with_prey() -> None:
    fox = [Idle(), wander(), Stalk(prey=HARE)]
    assert choose(fox, context(64), current=None).name == "wander"
    assert choose(fox, fox_believing(), current="wander").name == "stalk"


def test_the_mind_gives_each_body_its_own_scratch() -> None:
    mind = Mind()
    fox = fox_snapshot(tick=64)
    hare = pb.Snapshot(id=3, kind=HARE, tick=64, position=pb.Voxel(q=6, r=0), facing=pb.Direction.E)
    mind.round([fox, hare])
    assert Wander.REST_UNTIL in mind.scratches[1], "the fox, alone at (-8, 0), wanders: a rest is drawn"
    assert mind.scratches[3] == {}, "the hare only idles"
    assert mind.decisions[1].ambition == "wander"

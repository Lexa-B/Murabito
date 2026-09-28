"""The stalk, on the scene as it starts: fox #1 at (-8, 0), hare #3 at (6, 0)."""

from __future__ import annotations

from midbrain import murabito_pb2 as pb
from midbrain.ambitions import BITE, STOP, Context, Idle, Stalk, bound_for, choose, face, face_thing, is_under, lunge, path, revise, sidestep, to_send
from midbrain.beliefs import BelievedWorld, Cell
from midbrain.hexes import to_world

FOX = "murabito_kinds::all_things::tangible::sentient::living::animal::beast::fox"
HARE = "murabito_kinds::all_things::tangible::sentient::living::animal::beast::hare"
BEAST = "murabito_kinds::all_things::tangible::sentient::living::animal::beast"
SUGI = "murabito_kinds::all_things::tangible::non_sentient::plant::tree::sugi"


def fox_snapshot(at: tuple[int, int] = (-8, 0), facing: int = pb.Direction.ESE, tick: int = 64, in_view=(), doing=None) -> pb.Snapshot:
    q, r = at
    snapshot = pb.Snapshot(id=1, kind=FOX, tick=tick, position=pb.Voxel(q=q, r=r), facing=facing, in_view=in_view)
    if doing is not None:
        snapshot.doing.CopyFrom(pb.Doing(intent=doing, since=tick))
    return snapshot


def hare_seen(from_cell: tuple[int, int], at: tuple[int, int] = (6, 0), facing: int | None = pb.Direction.E) -> pb.InView:
    seen = pb.InView(id=3, kind=HARE, offset=pb.Offset(dq=at[0] - from_cell[0], dr=at[1] - from_cell[1]), distance=1, acuity=pb.Acuity.NEAR)
    if facing is not None:
        seen.facing = facing
    return seen


def fox_believing(hare_at: tuple[int, int] = (6, 0), hare_facing: int | None = pb.Direction.E, fox_at=(-8, 0), fox_facing=pb.Direction.ESE, sees_hare=True) -> Context:
    world = BelievedWorld(body=1)
    world.observe(fox_snapshot(fox_at, fox_facing, 64, [hare_seen(fox_at, hare_at, hare_facing)]))
    in_view = [hare_seen(fox_at, hare_at, hare_facing)] if sees_hare else []
    return Context(world, fox_snapshot(fox_at, fox_facing, 72, in_view))


STALK = Stalk(prey=HARE)


def test_a_kind_is_under_itself_and_its_ancestors_only() -> None:
    assert is_under(HARE, HARE) and is_under(HARE, BEAST)
    assert not is_under(FOX, HARE) and not is_under(None, HARE) and not is_under(SUGI, BEAST)


def test_with_no_prey_believed_the_stalk_wants_nothing_and_idle_wins() -> None:
    ctx = Context(BelievedWorld(body=1), fox_snapshot())
    assert STALK.utility(ctx) == 0.0
    assert choose([Idle(), STALK], ctx, current=None).name == "idle"


def test_a_stalk_in_hand_is_let_go_once_the_prey_is_forgotten() -> None:
    ctx = Context(BelievedWorld(body=1), fox_snapshot())
    assert choose([Idle(), STALK], ctx, current="stalk").name == "idle"
    assert STALK.want(ctx).ok is False and STALK.want(ctx).intent is None


def test_with_prey_believed_the_stalk_wins_and_is_kept_over_a_near_tie() -> None:
    ctx = fox_believing()
    assert STALK.utility(ctx) == 1.0
    assert choose([Idle(), STALK], ctx, current=None).name == "stalk"
    assert choose([Idle(score=0.95), STALK], ctx, current="idle").name == "idle"
    assert choose([Idle(score=0.95), STALK], ctx, current="stalk").name == "stalk"


def test_a_hare_facing_away_on_the_rear_line_is_approached_to_three_behind() -> None:
    ctx = fox_believing(hare_facing=pb.Direction.E)
    result = STALK.want(ctx)
    assert result.path == ("stalk", "approach", "close in")
    assert bound_for(result.intent, ctx.snapshot) == Cell(3, 0)
    assert result.intent == path("sneak", [pb.Direction.E] * 11), "a sneaking path straight there"


def test_a_hare_looking_our_way_freezes_us() -> None:
    result = STALK.want(fox_believing(hare_facing=pb.Direction.W))
    assert result.path == ("stalk", "freeze", "stop")
    assert result.intent == STOP
    # Just past the edge of a 180° front is not looking.
    assert STALK.want(fox_believing(hare_facing=pb.Direction.N)).path[1] != "freeze"
    assert Stalk(prey=HARE, looking_arc=200).want(fox_believing(hare_facing=pb.Direction.N)).path[1] == "freeze"


def test_a_hare_facing_across_us_is_circled_toward_its_rear_spiralling_in() -> None:
    # The hare faces north; its rear is south; we are due west of it, so we swing anticlockwise,
    # and at the default 15 degree pitch each swing brings us about 13% nearer.
    ctx = fox_believing(hare_facing=pb.Direction.N)
    result = STALK.want(ctx)
    assert result.path == ("stalk", "circle", "round")
    target = bound_for(result.intent, ctx.snapshot)
    assert target == Cell(-8, 7)
    assert target.r > 0, "toward the south side"
    (hx, hz), (tx, tz) = to_world(Cell(6, 0)), to_world(target)
    assert 11.5 <= ((tx - hx) ** 2 + (tz - hz) ** 2) ** 0.5 <= 12.5, "nearer than the fourteen we started at"


def test_with_no_pitch_the_circle_keeps_our_distance() -> None:
    ctx = fox_believing(hare_facing=pb.Direction.N)
    arc = Stalk(prey=HARE, spiral=0.0).want(ctx)
    target = bound_for(arc.intent, ctx.snapshot)
    assert target == Cell(-10, 8)
    (hx, hz), (tx, tz) = to_world(Cell(6, 0)), to_world(target)
    assert 13 <= ((tx - hx) ** 2 + (tz - hz) ** 2) ** 0.5 <= 15, "still about fourteen shaku from the hare"


def test_a_hare_whose_facing_was_never_seen_is_approached_straight() -> None:
    ctx = fox_believing(hare_facing=None)
    result = STALK.want(ctx)
    assert result.path == ("stalk", "approach", "close in")
    assert bound_for(result.intent, ctx.snapshot) == Cell(3, 0)


def test_three_behind_and_facing_it_we_pounce_and_off_facing_we_turn() -> None:
    pouncing = STALK.want(fox_believing(fox_at=(3, 0), fox_facing=pb.Direction.E))
    assert pouncing.path == ("stalk", "pounce", "lunge")
    assert pouncing.intent == lunge(pb.Direction.E), "two cells east lands beside the hare at (6, 0)"
    turning = STALK.want(fox_believing(fox_at=(3, 0), fox_facing=pb.Direction.N))
    assert turning.intent == face_thing(3)
    unseen = STALK.want(fox_believing(fox_at=(3, 0), fox_facing=pb.Direction.N, sees_hare=False))
    assert unseen.intent == face(pb.Direction.E)


def test_beside_it_and_facing_it_we_bite_and_a_notch_off_we_turn_first() -> None:
    biting = STALK.want(fox_believing(fox_at=(5, 0), fox_facing=pb.Direction.E))
    assert biting.path == ("stalk", "bite", "bite") and biting.intent == BITE
    turning = STALK.want(fox_believing(fox_at=(5, 0), fox_facing=pb.Direction.ENE))
    assert turning.path == ("stalk", "watch") and turning.intent == face_thing(3)


def test_no_pounce_from_the_side_or_from_too_far() -> None:
    # One cell due north of the hare, facing it: abeam, past its 180° arc, but off its rear
    # line, so we circle rather than lunge onto it.
    flank = STALK.want(fox_believing(fox_at=(7, -2), fox_facing=pb.Direction.S))
    assert flank.path[:2] == ("stalk", "circle")
    # Four behind, facing it: a lunge lands two behind, not beside; keep approaching.
    far = STALK.want(fox_believing(fox_at=(2, 0), fox_facing=pb.Direction.E))
    assert far.path == ("stalk", "approach", "close in")


def test_a_hare_looking_at_us_freezes_us_even_beside_it() -> None:
    frozen = STALK.want(fox_believing(fox_at=(5, 0), fox_facing=pb.Direction.E, hare_facing=pb.Direction.W))
    assert frozen.path == ("stalk", "freeze", "stop")


def test_after_walking_three_cells_unseen_the_fox_pivots_to_check_then_goes_on() -> None:
    world = BelievedWorld(body=1)
    world.observe(fox_snapshot((-8, 0), pb.Direction.ESE, 64, [hare_seen((-8, 0), (6, 0), pb.Direction.N)]))
    world.observe(fox_snapshot((-8, 2), pb.Direction.SSE, 96))
    world.observe(fox_snapshot((-8, 3), pb.Direction.SSE, 112))
    ctx = Context(world, fox_snapshot((-8, 3), pb.Direction.SSE, 112))
    assert world.get(3).walked_since == 3
    result = STALK.want(ctx)
    assert result.path == ("stalk", "check", "look")
    assert result.intent == face(pb.Direction.E), "twelve degrees off the line to the hare rounds to E"
    facing_it = Context(world, fox_snapshot((-8, 3), pb.Direction.E, 120))
    looked = STALK.want(facing_it)
    assert looked.path == ("stalk", "check", "look") and looked.intent is None
    assert world.get(3).walked_since == 0, "having looked, the check is done"
    assert STALK.want(facing_it).path[1] == "circle", "next round the stalk goes on; a ghost beyond reach is walked toward until it can be proven gone"
    two = Stalk(prey=HARE, check_after=4)
    assert two.want(ctx).path[1] == "circle"


def test_at_the_settle_cell_the_fox_pounces_rather_than_sneaking_to_where_it_stands() -> None:
    # The hare faces WNW, a corner direction; the pounce line is an edge line beside its
    # rear, E here, and the settle cell three along it is (3, -140).
    ctx = fox_believing(hare_at=(0, -140), hare_facing=pb.Direction.WNW, fox_at=(3, -137), fox_facing=pb.Direction.WNW)
    assert STALK.pounce_line(ctx, STALK.target(ctx)) in (pb.Direction.E, pb.Direction.SSE)
    at_settle = fox_believing(hare_at=(0, -140), hare_facing=pb.Direction.WNW, fox_at=(3, -140), fox_facing=pb.Direction.W)
    assert STALK.want(at_settle).path == ("stalk", "pounce", "lunge"), "on the line, facing it, three along: pounce"
    assert not STALK.farther_than_distance(at_settle), "never told it is still too far"
    one_short = fox_believing(hare_at=(0, -140), hare_facing=pb.Direction.WNW, fox_at=(4, -140), fox_facing=pb.Direction.W)
    assert STALK.want(one_short).path == ("stalk", "approach", "close in")
    assert bound_for(STALK.want(one_short).intent, one_short.snapshot) == Cell(3, -140)


def test_a_hare_facing_a_corner_direction_is_pounced_along_the_edge_beside_its_rear() -> None:
    # The hare at (30, -30) faces N, a corner direction; a bite reaches only across a
    # face, so the fox lines up 30° off dead behind, on the SSW edge line since it comes
    # from the south-west, and settles three along it at (27, -27).
    hare, fox = (30, -30), (-8, 0)
    ctx = fox_believing(hare_at=hare, hare_facing=pb.Direction.N, fox_at=fox, fox_facing=pb.Direction.ENE)
    assert STALK.pounce_line(ctx, STALK.target(ctx)) == pb.Direction.SSW
    assert STALK.settle_cell(ctx, STALK.target(ctx)) == Cell(27, -27)
    settled = fox_believing(hare_at=hare, hare_facing=pb.Direction.N, fox_at=(27, -27), fox_facing=pb.Direction.NNE)
    pouncing = STALK.want(settled)
    assert pouncing.path == ("stalk", "pounce", "lunge") and pouncing.intent == lunge(pb.Direction.NNE)
    landed = fox_believing(hare_at=hare, hare_facing=pb.Direction.N, fox_at=(29, -29), fox_facing=pb.Direction.NNE)
    assert STALK.want(landed).path == ("stalk", "bite", "bite"), "the cell faced, across a face, is the hare's"
    # From the south-east instead, the other edge line, SSE, is the nearer one.
    other_side = fox_believing(hare_at=hare, hare_facing=pb.Direction.N, fox_at=(40, -10), fox_facing=pb.Direction.N)
    assert STALK.pounce_line(other_side, STALK.target(other_side)) == pb.Direction.SSE


def test_a_bite_is_never_wanted_across_a_corner() -> None:
    # Two steps off along N is the corner neighbour: the hare is in the cell faced, but
    # no bite reaches across a corner, and no lunge lands there.
    corner = fox_believing(hare_at=(6, 0), hare_facing=pb.Direction.E, fox_at=(5, 2), fox_facing=pb.Direction.N)
    result = STALK.want(corner)
    assert result.path[1] not in ("bite", "pounce")


def test_one_sidestep_off_the_pounce_line_the_fox_sidesteps_rather_than_turning() -> None:
    # Settled three west of the hare, facing it; the hare shifted a cell to (6, -1).
    ctx = fox_believing(hare_at=(6, -1), hare_facing=pb.Direction.E, fox_at=(3, 0), fox_facing=pb.Direction.E)
    result = STALK.want(ctx)
    assert result.path == ("stalk", "set up", "sidestep")
    assert result.intent == sidestep(pb.Direction.NNW), "to (3, -1), a lateral cell, no turn"
    lined_up = fox_believing(hare_at=(6, -1), hare_facing=pb.Direction.E, fox_at=(3, -1), fox_facing=pb.Direction.E)
    assert STALK.want(lined_up).path == ("stalk", "pounce", "lunge")


def test_a_ghost_reached_and_not_seen_is_forgotten_and_the_fox_wanders() -> None:
    from midbrain.mind import Mind

    mind = Mind()
    mind.round([fox_snapshot((-8, 0), pb.Direction.ESE, 64, [hare_seen((-8, 0), (0, -140), pb.Direction.WNW)])])
    assert mind.decisions[1].ambition == "stalk"
    # The fox arrives at the settle cell facing the way its last step went, the hare gone.
    mind.round([fox_snapshot((3, -137), pb.Direction.NNW, 6400)])
    assert mind.decisions[1].path[1] in ("check", "watch") and mind.decisions[1].want == face(pb.Direction.WNW), "turn to look"
    mind.round([fox_snapshot((3, -137), pb.Direction.WNW, 6440)])
    assert mind.world(1).get(3) is None, "looked straight at where it should be, saw nothing: forgotten"
    assert mind.decisions[1].ambition == "wander"


def test_looking_straight_at_where_the_hare_should_be_and_seeing_nothing_forgets_it() -> None:
    world = BelievedWorld(body=1)
    world.observe(fox_snapshot((-8, 0), pb.Direction.ESE, 64, [hare_seen((-8, 0), (6, 0), pb.Direction.N)]))
    looking_away = Context(world, fox_snapshot((-5, 0), pb.Direction.S, 96))
    assert revise(looking_away) == []
    looking_at_it = Context(world, fox_snapshot((-5, 0), pb.Direction.E, 104))
    assert revise(looking_at_it) == [3]
    assert world.get(3) is None
    far = BelievedWorld(body=1)
    far.observe(fox_snapshot((-8, 0), pb.Direction.E, 64, [hare_seen((-8, 0), (20, 0), pb.Direction.N)]))
    assert revise(Context(far, fox_snapshot((-8, 0), pb.Direction.E, 72))) == [], "beyond reach, nothing is proven"


def test_standing_where_the_hare_was_believed_and_not_seeing_it_forgets_it() -> None:
    ctx = fox_believing(fox_at=(5, 0), sees_hare=False)
    assert revise(ctx) == [3]
    assert ctx.world.get(3) is None
    still = fox_believing(fox_at=(3, 0), sees_hare=False)
    assert revise(still) == []


def test_what_is_sent_for_a_want() -> None:
    E, N = pb.Direction.E, pb.Direction.N
    idle = fox_snapshot()
    # On a path of three steps east: the first is underway, two still wait on the queue.
    going = fox_snapshot(doing=path("walk", [E, E, E]))
    going.queue.extend([pb.Action(walk=E), pb.Action(walk=E)])
    going.underway.CopyFrom(pb.Action(walk=E))
    assert to_send(None, idle) is None and to_send(None, going) is None
    assert to_send(STOP, idle) is None and to_send(STOP, going) == STOP
    assert to_send(path("walk", [E, E, E]), idle) == path("walk", [E, E, E]), "nothing in hand: the whole path"
    assert to_send(path("walk", [E, E]), going) is None, "what waits already"
    assert to_send(path("walk", [E, E, E]), going) == path("walk", [E], keep=2), "keep what matches, send the rest"
    assert to_send(path("walk", [E, N]), going) == path("walk", [N], keep=1)
    assert to_send(path("sneak", [E, E]), going) == path("sneak", [E, E], keep=0), "another pace: nothing matches"
    assert to_send(path("walk", []), going) == path("walk", [], keep=0), "already there: drop what waits"
    assert to_send(face(N), idle) == face(N) and to_send(face(N), going) == face(N)
    assert to_send(face(N), fox_snapshot(doing=face(N))) is None


def test_a_path_is_planned_from_where_the_step_underway_lands() -> None:
    E = pb.Direction.E
    ctx = fox_believing(hare_facing=E)
    assert ctx.origin == Cell(-8, 0)
    mid_step = fox_believing(hare_facing=E)
    mid_step.snapshot.underway.CopyFrom(pb.Action(sneak=E))
    mid_step.snapshot.in_flight = 0.5
    assert mid_step.origin == Cell(-7, 0)
    assert STALK.want(mid_step).intent == path("sneak", [E] * 10), "ten from where the step lands"
    assert ctx.sneak_to(Cell(6, 0)) is None, "the hare's own cell is blocked"

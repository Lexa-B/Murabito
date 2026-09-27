"""The stalk, on the scene as it starts: fox #1 at (-8, 0), hare #3 at (6, 0)."""

from __future__ import annotations

from midbrain import murabito_pb2 as pb
from midbrain.ambitions import BITE, STOP, Context, Idle, Stalk, choose, differs, face, face_thing, is_under, lunge, revise, sneak_to, walk_to
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
    result = STALK.want(fox_believing(hare_facing=pb.Direction.E))
    assert result.path == ("stalk", "approach", "close in")
    assert result.intent == sneak_to(Cell(3, 0))


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
    result = STALK.want(fox_believing(hare_facing=pb.Direction.N))
    assert result.path == ("stalk", "circle", "round")
    target = Cell.of(result.intent.sustained.sneak_to)
    assert target == Cell(-8, 7)
    assert target.r > 0, "toward the south side"
    (hx, hz), (tx, tz) = to_world(Cell(6, 0)), to_world(target)
    assert 11.5 <= ((tx - hx) ** 2 + (tz - hz) ** 2) ** 0.5 <= 12.5, "nearer than the fourteen we started at"


def test_with_no_pitch_the_circle_keeps_our_distance() -> None:
    arc = Stalk(prey=HARE, spiral=0.0).want(fox_believing(hare_facing=pb.Direction.N))
    target = Cell.of(arc.intent.sustained.sneak_to)
    assert target == Cell(-10, 8)
    (hx, hz), (tx, tz) = to_world(Cell(6, 0)), to_world(target)
    assert 13 <= ((tx - hx) ** 2 + (tz - hz) ** 2) ** 0.5 <= 15, "still about fourteen shaku from the hare"


def test_a_hare_whose_facing_was_never_seen_is_approached_straight() -> None:
    result = STALK.want(fox_believing(hare_facing=None))
    assert result.path == ("stalk", "approach", "close in")
    assert result.intent == sneak_to(Cell(3, 0))


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


def test_after_walking_three_cells_unseen_the_fox_pivots_to_check_then_holds() -> None:
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
    assert STALK.want(facing_it).path == ("stalk", "check", "look") and STALK.want(facing_it).intent is None
    two = Stalk(prey=HARE, check_after=4)
    assert two.want(ctx).path[1] == "circle"


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


def test_what_is_worth_sending() -> None:
    idle = fox_snapshot()
    going = fox_snapshot(doing=walk_to(Cell(3, 0)))
    assert not differs(None, idle) and not differs(None, going)
    assert not differs(STOP, idle) and differs(STOP, going)
    assert differs(walk_to(Cell(3, 0)), idle)
    assert not differs(walk_to(Cell(3, 0)), going) and not differs(walk_to(Cell(4, 0)), going)
    assert differs(walk_to(Cell(5, 0)), going)
    assert differs(sneak_to(Cell(3, 0)), going), "another pace to the same cell is a change"
    assert differs(face(pb.Direction.N), idle) and differs(face(pb.Direction.N), going)
    assert not differs(face(pb.Direction.N), fox_snapshot(doing=face(pb.Direction.N)))

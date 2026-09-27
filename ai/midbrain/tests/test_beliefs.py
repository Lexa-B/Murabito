"""The believed world: things frozen where they were last seen."""

from __future__ import annotations

import pytest

from midbrain import murabito_pb2 as pb
from midbrain.beliefs import BelievedWorld, Cell

FOX = "murabito_kinds::all_things::tangible::sentient::living::animal::beast::fox"
HARE = "murabito_kinds::all_things::tangible::sentient::living::animal::beast::hare"
SUGI = "murabito_kinds::all_things::tangible::non_sentient::plant::tree::sugi"


def sighting(
    id: int, dq: int, dr: int, kind: str | None = None, acuity: int = pb.Acuity.NEAR, facing: int | None = None
) -> pb.InView:
    seen = pb.InView(id=id, offset=pb.Offset(dq=dq, dr=dr), distance=max(abs(dq), abs(dr)), acuity=acuity)
    if kind is not None:
        seen.kind = kind
    if facing is not None:
        seen.facing = facing
    return seen


def fox_sees(tick: int, *in_view: pb.InView, at: tuple[int, int] = (-8, 0)) -> pb.Snapshot:
    q, r = at
    return pb.Snapshot(id=1, kind=FOX, tick=tick, position=pb.Voxel(q=q, r=r), facing=pb.Direction.ESE, in_view=in_view)


def test_a_sighting_becomes_a_belief_at_the_cell_the_thing_stood_in() -> None:
    world = BelievedWorld(body=1)
    world.observe(fox_sees(64, sighting(2, 13, -4, SUGI, pb.Acuity.MID)))
    sugi = world.get(2)
    assert sugi is not None
    assert sugi.cell == Cell(5, -4)
    assert sugi.kind == SUGI
    assert sugi.seen_at == 64
    assert sugi.acuity == pb.Acuity.MID
    assert sugi.facing is None
    assert sugi.age(now=100) == 36


def test_the_way_a_thing_faced_is_remembered_and_refreshed() -> None:
    world = BelievedWorld(body=1)
    world.observe(fox_sees(64, sighting(3, 14, 0, HARE, facing=pb.Direction.E)))
    assert world.get(3).facing == pb.Direction.E
    world.observe(fox_sees(72, sighting(3, 14, 0, HARE, facing=pb.Direction.W)))
    assert world.get(3).facing == pb.Direction.W
    world.observe(fox_sees(80))
    assert world.get(3).facing == pb.Direction.W


def test_a_thing_without_a_label_is_believed_with_no_kind() -> None:
    world = BelievedWorld(body=1)
    world.observe(fox_sees(64, sighting(9, 2, 0)))
    assert world.get(9) is not None
    assert world.get(9).kind is None


def test_seeing_a_thing_again_moves_the_belief_and_refreshes_when() -> None:
    world = BelievedWorld(body=1)
    world.observe(fox_sees(64, sighting(3, 14, 0, HARE)))
    world.observe(fox_sees(72, sighting(3, 13, 1, HARE)))
    hare = world.get(3)
    assert hare.cell == Cell(5, 1)
    assert hare.seen_at == 72
    assert len(world) == 1


def test_a_thing_out_of_view_stays_where_it_was_last_seen() -> None:
    world = BelievedWorld(body=1)
    world.observe(fox_sees(64, sighting(3, 14, 0, HARE), sighting(2, 13, -4, SUGI)))
    world.observe(fox_sees(72, sighting(2, 13, -4, SUGI)))
    world.observe(fox_sees(80))
    hare = world.get(3)
    assert hare.cell == Cell(6, 0)
    assert hare.seen_at == 64
    assert hare.age(now=80) == 16
    assert [belief.id for belief in world] == [2, 3]


def test_the_believed_cell_follows_the_body_not_the_offset() -> None:
    world = BelievedWorld(body=1)
    world.observe(fox_sees(64, sighting(3, 4, 0, HARE), at=(2, 0)))
    assert world.get(3).cell == Cell(6, 0)


def test_two_bodies_keep_separate_worlds() -> None:
    fox, hare = BelievedWorld(body=1), BelievedWorld(body=3)
    fox.observe(fox_sees(64, sighting(3, 14, 0, HARE)))
    hare.observe(pb.Snapshot(id=3, kind=HARE, tick=64, position=pb.Voxel(q=6, r=0), in_view=[sighting(1, -14, 0, FOX)]))
    assert [belief.id for belief in fox] == [3]
    assert [belief.id for belief in hare] == [1]
    assert hare.get(1).cell == Cell(-8, 0)


def test_another_bodys_snapshot_is_refused() -> None:
    world = BelievedWorld(body=3)
    with pytest.raises(ValueError, match="snapshot of #1 given to #3"):
        world.observe(fox_sees(64))


def test_a_cell_reads_as_cube_with_its_layer() -> None:
    assert str(Cell(5, -4)) == "(5, -4, -1) L0"
    assert Cell(0, 0).plus(pb.Offset(dq=1, dr=1, dlayer=2)) == Cell(1, 1, 2)

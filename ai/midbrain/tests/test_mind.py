"""The mind's loop, fed snapshots by hand: worlds appear, fill, and read back."""

from __future__ import annotations

from rich.console import Console

from midbrain import murabito_pb2 as pb
from midbrain.beliefs import Cell
from midbrain.mind import Mind, ago, render

FOX = "murabito_kinds::all_things::tangible::sentient::living::animal::beast::fox"
HARE = "murabito_kinds::all_things::tangible::sentient::living::animal::beast::hare"
SUGI = "murabito_kinds::all_things::tangible::non_sentient::plant::tree::sugi"


def fox(tick: int, *in_view: pb.InView) -> pb.Snapshot:
    return pb.Snapshot(id=1, kind=FOX, tick=tick, position=pb.Voxel(q=-8, r=0), facing=pb.Direction.ESE, in_view=in_view)


def hare(tick: int, *in_view: pb.InView) -> pb.Snapshot:
    return pb.Snapshot(id=3, kind=HARE, tick=tick, position=pb.Voxel(q=6, r=0), facing=pb.Direction.E, in_view=in_view)


def seen(id: int, dq: int, dr: int, kind: str) -> pb.InView:
    return pb.InView(id=id, kind=kind, offset=pb.Offset(dq=dq, dr=dr), distance=max(abs(dq), abs(dr)), acuity=pb.Acuity.MID)


def test_a_round_gives_every_body_a_world_and_fills_it() -> None:
    mind = Mind()
    mind.round([fox(64, seen(2, 13, -4, SUGI)), hare(64, seen(2, -1, -4, SUGI))])
    assert sorted(mind.worlds) == [1, 3]
    assert mind.world(1).get(2).cell == Cell(5, -4)
    assert mind.world(3).get(2).cell == Cell(5, -4)
    assert mind.tick == 64


def test_rounds_accumulate_and_a_body_that_looked_away_keeps_its_beliefs() -> None:
    mind = Mind()
    mind.round([fox(64, seen(3, 14, 0, HARE))])
    mind.round([fox(72)])
    assert mind.world(1).get(3).cell == Cell(6, 0)
    assert mind.world(1).get(3).age(mind.tick) == 8
    assert mind.world(7) is None


def test_the_view_names_each_belief_and_how_long_ago() -> None:
    mind = Mind()
    mind.round([fox(64, seen(2, 13, -4, SUGI)), hare(64)])
    mind.round([fox(80)])
    console = Console(width=100, record=True)
    console.print(render(mind))
    text = console.export_text()
    assert "tick 80   2 minds" in text
    assert "#1 fox" in text and "#3 hare" in text
    assert "sugi" in text and "(5, -4, -1) L0" in text and "16 ticks ago" in text
    assert "nothing yet" in text
    assert ago(0) == "now"

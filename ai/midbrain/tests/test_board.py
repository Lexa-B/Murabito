"""The board's rendering, on snapshots made by hand: what a reader sees."""

from __future__ import annotations

from rich.console import Console

from midbrain import murabito_pb2 as pb
from midbrain.board import doing, in_flight, intent, name_of, outcome, render, voxel

FOX = "murabito_kinds::all_things::tangible::sentient::living::animal::beast::fox"


def snapshot() -> pb.Snapshot:
    return pb.Snapshot(
        id=2,
        kind=FOX,
        tick=64,
        position=pb.Voxel(q=-8, r=0, layer=0),
        facing=pb.Direction.ESE,
        in_view=[
            pb.InView(
                id=3,
                kind="murabito_kinds::all_things::tangible::non_sentient::plant::tree::sugi",
                offset=pb.Offset(dq=13, dr=-4, dlayer=0),
                distance=13,
                acuity=pb.Acuity.MID,
            ),
            pb.InView(id=4, offset=pb.Offset(dq=2, dr=0, dlayer=0), distance=2, acuity=pb.Acuity.NEAR),
        ],
        doing=pb.Doing(
            intent=pb.Intent(sustained=pb.Sustained(go_to=pb.Voxel(q=0, r=0, layer=0))),
            since=40,
        ),
        queue=[pb.Action(go=pb.Direction.E)],
        in_flight=0.5,
        previous_outcome=pb.Outcome(cancelled="startle_face_apparition"),
    )


def test_the_words_for_each_fact() -> None:
    s = snapshot()
    assert name_of(FOX) == "fox"
    assert name_of("") == "?"
    assert voxel(s.position) == "(-8, 0, 8) L0"
    assert intent(s.doing.intent) == "GoTo (0, 0, 0) L0"
    assert doing(s) == "GoTo (0, 0, 0) L0  since tick 40 (24 ticks)"
    assert in_flight(s) == "[##########..........] 50%"
    assert outcome(s.previous_outcome) == "Cancelled by startle_face_apparition"
    assert outcome(pb.Outcome(done=pb.Done())) == "Done"
    assert outcome(pb.Outcome(lost=7)) == "Lost #7"


def test_a_body_with_nothing_going_on_reads_as_such() -> None:
    bare = pb.Snapshot(id=5, kind=FOX, tick=1, previous_outcome=pb.Outcome(idle=pb.Idle()))
    assert doing(bare) == "nothing"
    assert in_flight(bare) == "idle"
    assert outcome(bare.previous_outcome) == "Idle"


def test_the_board_renders_every_body_with_its_facts() -> None:
    console = Console(width=100, record=True, force_terminal=False)
    console.print(render([snapshot()]))
    text = console.export_text()
    assert "tick 64" in text and "1 bodies" in text
    assert "#2 fox" in text
    assert "facing ESE" in text
    assert "Go E" in text
    assert "#4" in text and "near" in text
    assert "sugi" in text and "mid" in text
    assert "Cancelled by startle_face_apparition" in text

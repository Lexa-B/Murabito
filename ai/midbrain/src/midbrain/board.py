"""The board, live in the terminal: ``uv run board``.

Every 125 ms, one round of the slower mind, it asks the game's bridge for every body's
snapshot and redraws one panel per body: where it is, what it sees, what it is doing
and how the last thing ended. Read-only; a way to see what a mind would be told.
"""

from __future__ import annotations

import argparse
import time

from rich.console import Console, Group
from rich.live import Live
from rich.panel import Panel
from rich.table import Table
from rich.text import Text

from midbrain import murabito_pb2 as pb
from midbrain.client import HOST, PORT, Bridge

ROUND = 0.125
"""Seconds between asks: about eight ticks of the game's 64 Hz clock."""


def name_of(kind: str) -> str:
    """The last segment of a kind's path: ``fox`` from ``…::beast::fox``."""
    return kind.rsplit("::", 1)[-1] if kind else "?"


def direction(number: int) -> str:
    return pb.Direction.Name(number)


def voxel(cell: pb.Voxel) -> str:
    return f"({cell.q}, {cell.r}, {-cell.q - cell.r}) L{cell.layer}"


def offset(delta: pb.Offset) -> str:
    return f"({delta.dq:+d}, {delta.dr:+d}, {-delta.dq - delta.dr:+d})"


def short(motion: pb.Short) -> str:
    match motion.WhichOneof("kind"):
        case "stop":
            return "Stop"
        case "face":
            return f"Face {direction(motion.face)}"
        case "face_thing":
            return f"FaceThing #{motion.face_thing}"
        case "step":
            return f"Step {direction(motion.step)}"
    return "?"


def intent(what: pb.Intent) -> str:
    match what.WhichOneof("kind"):
        case "short":
            return short(what.short)
        case "sustained":
            if what.sustained.WhichOneof("kind") == "go_to":
                return f"GoTo {voxel(what.sustained.go_to)}"
    return "?"


def action(what: pb.Action) -> str:
    match what.WhichOneof("kind"):
        case "go":
            return f"Go {direction(what.go)}"
        case "face":
            return f"Face {direction(what.face)}"
    return "?"


def outcome(ended: pb.Outcome) -> str:
    match ended.WhichOneof("kind"):
        case "cancelled":
            return f"Cancelled by {ended.cancelled}"
        case "lost":
            return f"Lost #{ended.lost}"
        case None:
            return "?"
        case kind:
            return kind.capitalize()


def doing(snapshot: pb.Snapshot) -> str:
    if not snapshot.HasField("doing"):
        return "nothing"
    began = snapshot.doing.since
    for_ticks = snapshot.tick - began
    return f"{intent(snapshot.doing.intent)}  since tick {began} ({for_ticks} ticks)"


def in_flight(snapshot: pb.Snapshot) -> str:
    if not snapshot.HasField("in_flight"):
        return "idle"
    filled = round(snapshot.in_flight * 20)
    return f"[{'#' * filled}{'.' * (20 - filled)}] {snapshot.in_flight:.0%}"


def body_panel(snapshot: pb.Snapshot) -> Panel:
    facts = Table.grid(padding=(0, 2))
    facts.add_column(style="dim", justify="right")
    facts.add_column()
    facts.add_row("at", f"{voxel(snapshot.position)}  facing {direction(snapshot.facing)}")
    facts.add_row("doing", doing(snapshot))
    facts.add_row("queue", ", ".join(action(a) for a in snapshot.queue) or "empty")
    facts.add_row("in flight", in_flight(snapshot))
    facts.add_row("last", outcome(snapshot.previous_outcome))

    seen = Table(box=None, pad_edge=False, show_header=True, header_style="dim")
    seen.add_column("in view")
    seen.add_column("kind")
    seen.add_column("offset")
    seen.add_column("steps", justify="right")
    seen.add_column("acuity")
    for thing in sorted(snapshot.in_view, key=lambda t: t.distance):
        seen.add_row(
            f"#{thing.id}",
            name_of(thing.kind) if thing.HasField("kind") else "?",
            offset(thing.offset),
            str(thing.distance),
            pb.Acuity.Name(thing.acuity).lower(),
        )
    if not snapshot.in_view:
        seen.add_row(Text("nothing", style="dim"), "", "", "", "")

    title = f"#{snapshot.id} {name_of(snapshot.kind)}"
    return Panel(Group(facts, Text(""), seen), title=title, title_align="left")


def render(snapshots: list[pb.Snapshot]) -> Group:
    tick = max((s.tick for s in snapshots), default=0)
    header = Text(f"tick {tick}   {len(snapshots)} bodies", style="bold")
    panels = [body_panel(s) for s in sorted(snapshots, key=lambda s: s.id)]
    return Group(header, *panels)


def watch(host: str, port: int, every: float) -> None:
    console = Console()
    with Bridge(host, port) as bridge, Live(console=console, refresh_per_second=8) as live:
        while True:
            live.update(render(bridge.snapshots()))
            time.sleep(every)


def main() -> None:
    parser = argparse.ArgumentParser(description="Watch every body's snapshot, live.")
    parser.add_argument("--host", default=HOST)
    parser.add_argument("--port", type=int, default=PORT)
    parser.add_argument("--every", type=float, default=ROUND, help="seconds between asks")
    args = parser.parse_args()
    try:
        watch(args.host, args.port, args.every)
    except ConnectionRefusedError:
        raise SystemExit(f"no game listening on {args.host}:{args.port}: is it running?")
    except KeyboardInterrupt:
        pass

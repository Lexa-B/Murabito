"""The mind's loop: ``uv run mind``.

Every round, about 125 ms, it pulls every body's snapshot from the game's bridge and feeds
each into that body's believed world. It decides nothing and sends nothing yet. What it
shows is what each body believes: every thing it has ever seen, where it last saw it, and
how long ago. Run it beside the game; the game is the truth, this is the belief.
"""

from __future__ import annotations

import argparse
import time
from dataclasses import dataclass, field

from rich.console import Console, Group
from rich.live import Live
from rich.panel import Panel
from rich.table import Table
from rich.text import Text

from midbrain import murabito_pb2 as pb
from midbrain.beliefs import BelievedWorld
from midbrain.board import ROUND, direction, name_of, voxel
from midbrain.client import HOST, PORT, Bridge


@dataclass
class Mind:
    """Every body's believed world, and the latest snapshot each was fed."""

    worlds: dict[int, BelievedWorld] = field(default_factory=dict)
    latest: dict[int, pb.Snapshot] = field(default_factory=dict)

    def round(self, snapshots: list[pb.Snapshot]) -> None:
        """One round: every snapshot into its body's world, a new world for a new body."""
        for snapshot in snapshots:
            self.worlds.setdefault(snapshot.id, BelievedWorld(snapshot.id)).observe(snapshot)
            self.latest[snapshot.id] = snapshot

    def world(self, body: int) -> BelievedWorld | None:
        return self.worlds.get(body)

    @property
    def tick(self) -> int:
        """The latest tick any body has reported."""
        return max((s.tick for s in self.latest.values()), default=0)


def ago(ticks: int) -> str:
    return "now" if ticks == 0 else f"{ticks} ticks ago"


def body_panel(mind: Mind, body: int) -> Panel:
    snapshot, world = mind.latest[body], mind.worlds[body]
    header = Text(f"at {voxel(snapshot.position)}  facing {direction(snapshot.facing)}", style="dim")
    beliefs = Table(box=None, pad_edge=False, show_header=True, header_style="dim")
    beliefs.add_column("believes")
    beliefs.add_column("kind")
    beliefs.add_column("at")
    beliefs.add_column("seen", justify="right")
    beliefs.add_column("acuity")
    for belief in world:
        beliefs.add_row(
            f"#{belief.id}",
            name_of(belief.kind) if belief.kind else "?",
            str(belief.cell),
            ago(belief.age(snapshot.tick)),
            pb.Acuity.Name(belief.acuity).lower(),
        )
    if not len(world):
        beliefs.add_row(Text("nothing yet", style="dim"), "", "", "", "")
    title = f"#{body} {name_of(snapshot.kind)}"
    return Panel(Group(header, beliefs), title=title, title_align="left")


def render(mind: Mind) -> Group:
    header = Text(f"tick {mind.tick}   {len(mind.worlds)} minds", style="bold")
    return Group(header, *(body_panel(mind, body) for body in sorted(mind.worlds)))


def think(host: str, port: int, every: float) -> None:
    mind = Mind()
    console = Console()
    with Bridge(host, port) as bridge, Live(console=console, refresh_per_second=8) as live:
        while True:
            mind.round(bridge.snapshots())
            live.update(render(mind))
            time.sleep(every)


def main() -> None:
    parser = argparse.ArgumentParser(description="Believe what every body sees, round by round.")
    parser.add_argument("--host", default=HOST)
    parser.add_argument("--port", type=int, default=PORT)
    parser.add_argument("--every", type=float, default=ROUND, help="seconds between rounds")
    parser.add_argument("--visualize", action="store_true", help="draw one body's believed world in a window")
    parser.add_argument("--body", type=int, default=None, help="which body's world to draw (default: the lowest id)")
    parser.add_argument("--scale", type=float, default=28.0, help="pixels per shaku in the window")
    args = parser.parse_args()
    try:
        if args.visualize:
            from midbrain.visualize import show

            show(args.host, args.port, args.every, args.body, args.scale)
        else:
            think(args.host, args.port, args.every)
    except ConnectionRefusedError:
        raise SystemExit(f"no game listening on {args.host}:{args.port}: is it running?")
    except KeyboardInterrupt:
        pass

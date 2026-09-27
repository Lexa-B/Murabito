"""The mind's loop: ``uv run mind``.

Every round, about 125 ms, it pulls every body's snapshot from the game's bridge, feeds
each into that body's believed world, revises what can't be so, lets the body's ambitions
bid, ticks the winner's tree, and sends the intent it wants if that differs from what the
body is doing. What it shows is what each body believes and what it decided. Run it beside
the game; the game is the truth, this is the mind.
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

from collections.abc import Callable
from midbrain import murabito_pb2 as pb
from midbrain.ambitions import Ambition, Context, choose, differs, repertoire_for, revise
from midbrain.beliefs import BelievedWorld
from midbrain.board import ROUND, direction, intent as intent_words, name_of, voxel
from midbrain.client import HOST, PORT, Bridge


@dataclass(frozen=True)
class Decision:
    """What one body decided this round: which ambition, the path through its tree, the
    intent it wanted, and whether that was worth sending."""

    ambition: str
    path: tuple[str, ...]
    want: pb.Intent | None
    sent: bool

    def __str__(self) -> str:
        want = intent_words(self.want) if self.want is not None else "hold"
        return f"{' › '.join(self.path)}  →  {want}" + ("  (sent)" if self.sent else "")


@dataclass
class Mind:
    """Every body's believed world, the latest snapshot each was fed, and what each decided."""

    worlds: dict[int, BelievedWorld] = field(default_factory=dict)
    latest: dict[int, pb.Snapshot] = field(default_factory=dict)
    current: dict[int, str] = field(default_factory=dict)
    """The ambition each body has in hand, boosted when its bids are next compared."""
    scratches: dict[int, dict[str, object]] = field(default_factory=dict)
    """Each body's scratch: what its ambitions remember between rounds."""
    decisions: dict[int, Decision] = field(default_factory=dict)
    repertoire: Callable[[str], list[Ambition]] = repertoire_for

    def round(self, snapshots: list[pb.Snapshot]) -> list[tuple[int, pb.Intent]]:
        """One round: observe, revise, choose, want; the orders worth sending, per body."""
        orders = []
        for snapshot in snapshots:
            world = self.worlds.setdefault(snapshot.id, BelievedWorld(snapshot.id))
            world.observe(snapshot)
            self.latest[snapshot.id] = snapshot
            ctx = Context(world, snapshot, self.scratches.setdefault(snapshot.id, {}))
            revise(ctx)
            ambition = choose(self.repertoire(snapshot.kind), ctx, self.current.get(snapshot.id))
            self.current[snapshot.id] = ambition.name
            result = ambition.want(ctx)
            sent = differs(result.intent, snapshot)
            self.decisions[snapshot.id] = Decision(ambition.name, result.path, result.intent, sent)
            if sent:
                orders.append((snapshot.id, result.intent))
        return orders

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
    decided = Text(f"decided  {mind.decisions[body]}")
    beliefs = Table(box=None, pad_edge=False, show_header=True, header_style="dim")
    beliefs.add_column("believes")
    beliefs.add_column("kind")
    beliefs.add_column("at")
    beliefs.add_column("facing")
    beliefs.add_column("seen", justify="right")
    beliefs.add_column("acuity")
    for belief in world:
        beliefs.add_row(
            f"#{belief.id}",
            name_of(belief.kind) if belief.kind else "?",
            str(belief.cell),
            direction(belief.facing) if belief.facing is not None else "-",
            ago(belief.age(snapshot.tick)),
            pb.Acuity.Name(belief.acuity).lower(),
        )
    if not len(world):
        beliefs.add_row(Text("nothing yet", style="dim"), "", "", "", "", "")
    title = f"#{body} {name_of(snapshot.kind)}"
    return Panel(Group(header, decided, beliefs), title=title, title_align="left")


def render(mind: Mind) -> Group:
    header = Text(f"tick {mind.tick}   {len(mind.worlds)} minds", style="bold")
    return Group(header, *(body_panel(mind, body) for body in sorted(mind.worlds)))


def think(host: str, port: int, every: float) -> None:
    mind = Mind()
    console = Console()
    with Bridge(host, port) as bridge, Live(console=console, refresh_per_second=8) as live:
        while True:
            for body, intent in mind.round(bridge.snapshots()):
                bridge.order(body, intent)
            live.update(render(mind))
            time.sleep(every)


def main() -> None:
    parser = argparse.ArgumentParser(description="Believe what every body sees, round by round.")
    parser.add_argument("--host", default=HOST)
    parser.add_argument("--port", type=int, default=PORT)
    parser.add_argument("--every", type=float, default=ROUND, help="seconds between rounds")
    parser.add_argument("--visualize", action="store_true", help="draw one body's believed world in a window")
    parser.add_argument("--body", type=int, default=None, help="which body's world to draw (default: the lowest id)")
    parser.add_argument("--scale", type=float, default=28.0, help="the window's closest zoom, in pixels per shaku; it zooms out as far as it must to contain everything believed")
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

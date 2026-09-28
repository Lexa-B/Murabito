"""One body's believed world, drawn: ``uv run mind --visualize [--body N]``.

A pygame window beside the game. The game is the truth; this window is what one body
believes: a hex grid, north up, the body at its true cell with a line for its facing, and
every thing it has ever seen as a filled hex where it last saw it, labelled with what it
is and how long ago that was; what it decided this round, and the path it wants to walk
as outlined hexes, the last one bold. Nothing here moves or fades on its own; it redraws each
round from the mind. The view has no fixed centre: each frame it zooms and pans to contain
the body, everything it believes and the cell it wants, up to ``--scale`` pixels per shaku.
Beside it, a pane draws the body's current ambition's tree top-down as it loaded from its
file, the nodes on this round's decision path lit and the one that decided marked, with
what it wants under it: the mind thinking, live, off nothing but the decision it already
records. Tab cycles which body is shown; Escape or closing quits.

Screen geometry is the game's (``crates/hexcoords``): pointy-top cells one shaku flat to
flat, a cell's centre at ``x = q + r/2``, ``z = r·√3/2``, east right, north up.
"""

from __future__ import annotations

import math
import os
import subprocess
import time
from dataclasses import dataclass

import pygame

from midbrain import murabito_pb2 as pb
from midbrain.ambitions import repertoire_for, route_of
from midbrain.behaviour import Act, Condition, Node, Selector, Sequence
from midbrain.beliefs import Cell
from midbrain.board import intent as intent_words, name_of
from midbrain.client import Bridge
from midbrain.mind import Mind, ago

SQRT_3 = math.sqrt(3)
CORNER_RADIUS = 1 / SQRT_3
"""Centre to corner, in cell widths."""

WINDOW = 900
PANE = 440
"""The tree pane's width, to the right of the world."""
ROW = 20
"""Pixels per node row in the tree pane."""
BACKGROUND = (28, 32, 30)
PANE_BACKGROUND = (22, 25, 24)
DIM = (110, 112, 118)
LIT = (240, 200, 90)
DECIDED = (70, 60, 30)
GRID = (52, 58, 54)
SELF = (240, 200, 90)
SEEN_NOW = (235, 235, 245)
LABEL = (200, 200, 210)
KIND_COLOURS = {"fox": (222, 120, 60), "hare": (140, 180, 235), "sugi": (70, 140, 90)}
UNKNOWN_COLOUR = (150, 150, 150)


@dataclass(frozen=True)
class View:
    """Where the plane lands on the window: the origin's pixel and pixels per shaku."""

    scale: float = 28.0
    centre: tuple[float, float] = (WINDOW / 2, WINDOW / 2)

    def pixel(self, cell: Cell) -> tuple[float, float]:
        """A cell's centre on screen. Screen y grows downward, which is south."""
        x = cell.q + cell.r / 2
        z = cell.r * SQRT_3 / 2
        return self.centre[0] + x * self.scale, self.centre[1] + z * self.scale

    def corners(self, cell: Cell) -> list[tuple[float, float]]:
        """The six corners, starting from the one due north, going clockwise on screen."""
        cx, cy = self.pixel(cell)
        radius = CORNER_RADIUS * self.scale
        return [
            (cx + radius * math.cos(math.radians(90 - 60 * k)), cy - radius * math.sin(math.radians(90 - 60 * k)))
            for k in range(6)
        ]

    def facing_end(self, cell: Cell, direction: int, length: float = 0.9) -> tuple[float, float]:
        """The tip of a line from the cell's centre in a direction, ``length`` shaku long."""
        cx, cy = self.pixel(cell)
        angle = math.radians(30 * direction)
        return cx + length * self.scale * math.cos(angle), cy - length * self.scale * math.sin(angle)

    @classmethod
    def fitting(cls, cells: list[Cell], width: int, height: int, max_scale: float, margin: float = 60.0) -> View:
        """The view that contains every cell given, a cell's width to spare round the edge
        and ``margin`` pixels more for labels, no larger than ``max_scale`` pixels per shaku.
        An empty list gives the default view."""
        if not cells:
            return cls(scale=max_scale, centre=(width / 2, height / 2))
        xs = [cell.q + cell.r / 2 for cell in cells]
        zs = [cell.r * SQRT_3 / 2 for cell in cells]
        span_x, span_z = max(xs) - min(xs) + 2, max(zs) - min(zs) + 2
        scale = min(max_scale, (width - 2 * margin) / span_x, (height - 2 * margin) / span_z)
        mid_x, mid_z = (max(xs) + min(xs)) / 2, (max(zs) + min(zs)) / 2
        return cls(scale=scale, centre=(width / 2 - mid_x * scale, height / 2 - mid_z * scale))

    def cells_on_screen(self, width: int, height: int) -> list[Cell]:
        """Every cell whose centre falls inside a window of this size."""
        reach = int(max(width, height) / self.scale) + 2
        cells = []
        for r in range(-reach, reach + 1):
            for q in range(-reach, reach + 1):
                x, y = self.pixel(Cell(q, r))
                if 0 <= x < width and 0 <= y < height:
                    cells.append(Cell(q, r))
        return cells


def colour_of(kind: str | None) -> tuple[int, int, int]:
    return KIND_COLOURS.get(name_of(kind), UNKNOWN_COLOUR) if kind else UNKNOWN_COLOUR


@dataclass(frozen=True)
class Row:
    """One node of a tree laid out top-down: how deep, what kind, its name, whether this
    round's decision passed through it, and whether it is the node that decided."""

    depth: int
    kind: str
    name: str
    on_path: bool
    decided: bool


KINDS = {Selector: "?", Sequence: ">", Condition: "if", Act: "do"}


def rows(tree: Node, path: tuple[str, ...]) -> list[Row]:
    """The tree as rows, depth first, the decision path (node names, root first) lit
    along the one branch whose names match it in order."""
    out: list[Row] = []

    def visit(node: Node, depth: int, along: tuple[str, ...]) -> None:
        here = bool(along) and along[0] == node.name
        rest = along[1:] if here else ()
        out.append(Row(depth, KINDS[type(node)], node.name, here, here and not rest))
        for child in getattr(node, "children", ()):
            visit(child, depth + 1, rest)

    visit(tree, 0, path)
    return out


def tree_of(mind: Mind, body: int) -> Node | None:
    """The tree of the ambition the body has in hand, as it loads now; None if that
    ambition has no tree."""
    snapshot, decision = mind.latest.get(body), mind.decisions.get(body)
    if snapshot is None or decision is None:
        return None
    for ambition in repertoire_for(snapshot.kind):
        if ambition.name == decision.ambition:
            return getattr(ambition, "tree", None)
    return None


def draw_tree(surface: pygame.Surface, mind: Mind, body: int, font: pygame.font.Font) -> None:
    """The tree pane: the ambition in hand, its tree with this round's path lit, and what
    it wants."""
    surface.fill(PANE_BACKGROUND)
    decision = mind.decisions.get(body)
    if decision is None:
        surface.blit(font.render("no decision yet", True, LABEL), (12, 12))
        return
    surface.blit(font.render(f"ambition  {decision.ambition}", True, LABEL), (12, 12))
    tree = tree_of(mind, body)
    top = 40
    if tree is None:
        surface.blit(font.render("(no tree)", True, DIM), (12, top))
    else:
        for index, row in enumerate(rows(tree, decision.path)):
            y = top + index * ROW
            if row.decided:
                pygame.draw.rect(surface, DECIDED, (0, y - 2, surface.get_width(), ROW))
            colour = LIT if row.on_path else DIM
            surface.blit(font.render(f"{row.kind} {row.name}", True, colour), (12 + 18 * row.depth, y))
        top += ROW * (len(rows(tree, decision.path)) + 1)
    want = intent_words(decision.want, show=2) if decision.want is not None else "hold"
    sent = "  (sent)" if decision.sent else ""
    surface.blit(font.render(f"wants  {want}{sent}".replace("→", "->"), True, LABEL), (12, top))


def cells_to_show(mind: Mind, body: int) -> list[Cell]:
    """What the view must contain: the body, everything it believes, and the cell it wants."""
    snapshot, world = mind.latest.get(body), mind.world(body)
    if snapshot is None or world is None:
        return []
    cells = [Cell.of(snapshot.position)] + [belief.cell for belief in world]
    decision = mind.decisions.get(body)
    return cells + (route_of(decision.want, snapshot) if decision is not None else [])


def draw(surface: pygame.Surface, mind: Mind, body: int, view: View | None, font: pygame.font.Font, max_scale: float = 28.0, pane: int = 0) -> None:
    """One frame: the grid, then the body's beliefs, then the body itself on top, and,
    given a ``pane`` width, the tree pane on the right. With no view given, one is fitted
    to contain everything there is to show."""
    if pane:
        width, height = surface.get_size()
        draw_tree(surface.subsurface((width - pane, 0, pane, height)), mind, body, font)
        surface = surface.subsurface((0, 0, width - pane, height))
    surface.fill(BACKGROUND)
    width, height = surface.get_size()
    if view is None:
        view = View.fitting(cells_to_show(mind, body), width, height, max_scale)
    for cell in view.cells_on_screen(width, height):
        pygame.draw.polygon(surface, GRID, view.corners(cell), 1)

    snapshot, world = mind.latest.get(body), mind.world(body)
    if snapshot is None or world is None:
        surface.blit(font.render(f"no body #{body} on the board", True, LABEL), (12, 12))
        return
    seen_now = {sighting.id for sighting in snapshot.in_view}
    for belief in world:
        corners = view.corners(belief.cell)
        pygame.draw.polygon(surface, colour_of(belief.kind), corners)
        if belief.id in seen_now:
            pygame.draw.polygon(surface, SEEN_NOW, corners, 2)
        if belief.facing is not None:
            pygame.draw.line(surface, LABEL, view.pixel(belief.cell), view.facing_end(belief.cell, belief.facing), 2)
        cx, cy = view.pixel(belief.cell)
        name = f"#{belief.id} {name_of(belief.kind) if belief.kind else '?'}"
        surface.blit(font.render(name, True, LABEL), (cx + view.scale * 0.6, cy - 14))
        surface.blit(font.render(ago(belief.age(snapshot.tick)), True, LABEL), (cx + view.scale * 0.6, cy + 1))

    decision = mind.decisions.get(body)
    route = route_of(decision.want, snapshot) if decision is not None else []
    for cell in route[:-1]:
        pygame.draw.polygon(surface, SELF, view.corners(cell), 1)
    if route:
        pygame.draw.polygon(surface, SELF, view.corners(route[-1]), 2)
    here = Cell.of(snapshot.position)
    pygame.draw.polygon(surface, SELF, view.corners(here))
    pygame.draw.line(surface, SELF, view.pixel(here), view.facing_end(here, snapshot.facing), 3)
    title = f"#{body} {name_of(snapshot.kind)} believes   tick {snapshot.tick}   {len(world)} things   {view.scale:.0f} px/shaku   Tab: next body"
    surface.blit(font.render(title, True, LABEL), (12, 12))
    if decision is not None:
        surface.blit(font.render(f"decided  {decision}".replace("→", "->"), True, LABEL), (12, 30))


def find_display() -> None:
    """A terminal tab may have no display; take it from the systemd user session, as run.sh does."""
    if os.environ.get("DISPLAY") or os.environ.get("WAYLAND_DISPLAY"):
        return
    try:
        listing = subprocess.run(["systemctl", "--user", "show-environment"], capture_output=True, text=True, check=False).stdout
    except OSError:
        return
    for line in listing.splitlines():
        name, _, value = line.partition("=")
        if name in ("DISPLAY", "WAYLAND_DISPLAY", "XAUTHORITY") and not os.environ.get(name):
            os.environ[name] = value


def next_body(mind: Mind, body: int) -> int:
    """The next body on the board after this one, wrapping round; this one if it is alone."""
    bodies = sorted(mind.worlds)
    if not bodies:
        return body
    later = [b for b in bodies if b > body]
    return later[0] if later else bodies[0]


def show(host: str, port: int, every: float, body: int | None, scale: float) -> None:
    find_display()
    pygame.init()
    screen = pygame.display.set_mode((WINDOW + PANE, WINDOW))
    pygame.display.set_caption("midbrain: a believed world")
    font = pygame.font.SysFont(None, 18)
    mind = Mind()
    with Bridge(host, port) as bridge:
        running = True
        while running:
            for event in pygame.event.get():
                if event.type == pygame.QUIT or (event.type == pygame.KEYDOWN and event.key == pygame.K_ESCAPE):
                    running = False
                elif event.type == pygame.KEYDOWN and event.key == pygame.K_TAB and body is not None:
                    body = next_body(mind, body)
            for wanted_by, intent in mind.round(bridge.snapshots()):
                bridge.order(wanted_by, intent)
            if body is None and mind.worlds:
                body = min(mind.worlds)
            draw(screen, mind, body if body is not None else 0, None, font, max_scale=scale, pane=PANE)
            pygame.display.flip()
            time.sleep(every)
    pygame.quit()

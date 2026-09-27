"""One body's believed world, drawn: ``uv run mind --visualize [--body N]``.

A pygame window beside the game. The game is the truth; this window is what one body
believes: a hex grid, north up, the body at its true cell with a line for its facing, and
every thing it has ever seen as a filled hex where it last saw it, labelled with what it
is and how long ago that was; what it decided this round, and the cell it wants to reach
as an outlined hex. Nothing here moves or fades on its own; it redraws each
round from the mind. Tab cycles which body's world is shown; Escape or closing quits.

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
from midbrain.ambitions import bound_for
from midbrain.beliefs import Cell
from midbrain.board import name_of
from midbrain.client import Bridge
from midbrain.mind import Mind, ago

SQRT_3 = math.sqrt(3)
CORNER_RADIUS = 1 / SQRT_3
"""Centre to corner, in cell widths."""

WINDOW = 900
BACKGROUND = (28, 32, 30)
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


def draw(surface: pygame.Surface, mind: Mind, body: int, view: View, font: pygame.font.Font) -> None:
    """One frame: the grid, then the body's beliefs, then the body itself on top."""
    surface.fill(BACKGROUND)
    width, height = surface.get_size()
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
    target = bound_for(decision.want) if decision is not None and decision.want is not None else None
    if target is not None:
        pygame.draw.polygon(surface, SELF, view.corners(target), 2)
    here = Cell.of(snapshot.position)
    pygame.draw.polygon(surface, SELF, view.corners(here))
    pygame.draw.line(surface, SELF, view.pixel(here), view.facing_end(here, snapshot.facing), 3)
    title = f"#{body} {name_of(snapshot.kind)} believes   tick {snapshot.tick}   {len(world)} things   Tab: next body"
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
    screen = pygame.display.set_mode((WINDOW, WINDOW))
    pygame.display.set_caption("midbrain: a believed world")
    font = pygame.font.SysFont(None, 18)
    view = View(scale=scale)
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
            draw(screen, mind, body if body is not None else 0, view, font)
            pygame.display.flip()
            time.sleep(every)
    pygame.quit()

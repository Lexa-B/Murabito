"""The window's geometry, pure, and one frame drawn offscreen."""

from __future__ import annotations

import math
import os

os.environ.setdefault("SDL_VIDEODRIVER", "dummy")

import pygame

from midbrain import murabito_pb2 as pb
from midbrain.beliefs import Cell
from midbrain.mind import Mind
from midbrain.ambitions import route_of
from midbrain.visualize import DECIDED, PANE, SELF, Row, View, draw, next_body, rows

FOX = "murabito_kinds::all_things::tangible::sentient::living::animal::beast::fox"
SUGI = "murabito_kinds::all_things::tangible::non_sentient::plant::tree::sugi"


def close(a: tuple[float, float], b: tuple[float, float]) -> bool:
    return math.isclose(a[0], b[0], abs_tol=1e-6) and math.isclose(a[1], b[1], abs_tol=1e-6)


def test_cells_land_where_the_game_puts_them() -> None:
    view = View(scale=10, centre=(100, 100))
    assert close(view.pixel(Cell(0, 0)), (100, 100))
    assert close(view.pixel(Cell(1, 0)), (110, 100))  # east is right
    assert close(view.pixel(Cell(0, 1)), (105, 100 + 5 * math.sqrt(3)))  # south-south-east is down and a little right
    assert close(view.pixel(Cell(1, -2)), (100, 100 - 10 * math.sqrt(3)))  # north is straight up


def test_a_cell_is_pointy_top_with_its_first_corner_due_north() -> None:
    view = View(scale=10, centre=(100, 100))
    corners = view.corners(Cell(0, 0))
    assert len(corners) == 6
    assert close(corners[0], (100, 100 - 10 / math.sqrt(3)))
    assert close(corners[3], (100, 100 + 10 / math.sqrt(3)))


def test_facing_lines_point_the_compass_way() -> None:
    view = View(scale=10, centre=(100, 100))
    assert close(view.facing_end(Cell(0, 0), pb.Direction.E, length=1), (110, 100))
    assert close(view.facing_end(Cell(0, 0), pb.Direction.N, length=1), (100, 90))
    assert close(view.facing_end(Cell(0, 0), pb.Direction.W, length=1), (90, 100))


def test_the_grid_covers_the_window_and_no_more() -> None:
    view = View(scale=10, centre=(50, 50))
    cells = view.cells_on_screen(100, 100)
    assert Cell(0, 0) in cells
    assert Cell(4, 0) in cells and Cell(6, 0) not in cells
    assert all(0 <= view.pixel(c)[0] < 100 and 0 <= view.pixel(c)[1] < 100 for c in cells)


def test_a_frame_paints_the_body_at_its_cell_and_its_beliefs_where_it_saw_them() -> None:
    pygame.init()
    font = pygame.font.SysFont(None, 18)
    mind = Mind()
    mind.round([pb.Snapshot(
        id=1, kind=FOX, tick=64, position=pb.Voxel(q=-8, r=0), facing=pb.Direction.ESE,
        in_view=[pb.InView(id=2, kind=SUGI, offset=pb.Offset(dq=13, dr=-4), distance=13, acuity=pb.Acuity.MID)],
    )])
    view = View(scale=20, centre=(450, 450))
    surface = pygame.Surface((900, 900))
    draw(surface, mind, 1, view, font)
    x, y = view.pixel(Cell(-8, 0))
    assert surface.get_at((int(x), int(y) + 6))[:3] == SELF  # below the facing line, still in the body's hex
    x, y = view.pixel(Cell(5, -4))
    assert surface.get_at((int(x), int(y)))[:3] == (70, 140, 90)  # the sugi's green


def test_a_missing_body_draws_a_note_not_a_crash() -> None:
    pygame.init()
    draw(pygame.Surface((300, 300)), Mind(), 7, View(scale=10, centre=(150, 150)), pygame.font.SysFont(None, 18))


def test_tab_cycles_the_bodies_on_the_board() -> None:
    mind = Mind()
    mind.round([pb.Snapshot(id=1, kind=FOX, tick=1), pb.Snapshot(id=3, kind=FOX, tick=1)])
    assert next_body(mind, 1) == 3
    assert next_body(mind, 3) == 1
    assert next_body(Mind(), 1) == 1


def test_a_fitted_view_contains_every_cell_with_room_to_spare() -> None:
    far_apart = [Cell(-8, 0), Cell(0, -140), Cell(5, -4)]
    view = View.fitting(far_apart, 900, 900, max_scale=28.0)
    assert view.scale < 28.0, "zoomed out to fit"
    for cell in far_apart:
        x, y = view.pixel(cell)
        assert 60 <= x <= 840 and 60 <= y <= 840, f"{cell} inside the margins"


def test_a_fitted_view_never_zooms_closer_than_the_cap() -> None:
    view = View.fitting([Cell(0, 0), Cell(1, 0)], 900, 900, max_scale=28.0)
    assert view.scale == 28.0
    assert close(view.pixel(Cell(0, 0)), (450 - 14, 450)), "the two cells sit either side of the middle"
    assert View.fitting([], 900, 900, max_scale=28.0).scale == 28.0


def test_a_frame_with_no_view_fits_itself_round_the_beliefs_and_the_wanted_cell() -> None:
    pygame.init()
    font = pygame.font.SysFont(None, 18)
    mind = Mind()
    mind.round([pb.Snapshot(
        id=1, kind=FOX, tick=64, position=pb.Voxel(q=-8, r=0), facing=pb.Direction.ESE,
        in_view=[pb.InView(id=2, kind=SUGI, offset=pb.Offset(dq=60, dr=-4), distance=60, acuity=pb.Acuity.FAR)],
    )])
    surface = pygame.Surface((900, 900))
    draw(surface, mind, 1, None, font, max_scale=28.0)
    view = View.fitting([Cell(-8, 0), Cell(52, -4)] + route_of(mind.decisions[1].want, mind.latest[1]), 900, 900, 28.0)
    x, y = view.pixel(Cell(52, -4))
    assert surface.get_at((int(x), int(y)))[:3] == (70, 140, 90), "the sugi, sixty cells off, is on screen"


HARE = "murabito_kinds::all_things::tangible::sentient::living::animal::beast::hare"


def stalking_mind() -> Mind:
    mind = Mind()
    mind.round([pb.Snapshot(
        id=1, kind=FOX, tick=64, position=pb.Voxel(q=-8, r=0), facing=pb.Direction.ESE,
        in_view=[pb.InView(id=3, kind=HARE, offset=pb.Offset(dq=14, dr=0), distance=14, acuity=pb.Acuity.NEAR, facing=pb.Direction.E)],
    )])
    return mind


def test_rows_lay_the_tree_out_top_down_and_light_the_decision_path() -> None:
    from midbrain.ambitions import Stalk

    mind = stalking_mind()
    assert mind.decisions[1].path == ("stalk", "approach", "close in")
    laid = rows(Stalk(prey=HARE).tree, mind.decisions[1].path)
    assert laid[0] == Row(0, "?", "stalk", True, False)
    assert laid[1] == Row(1, ">", "freeze", False, False)
    assert laid[2] == Row(2, "if", "looking at us", False, False)
    assert [row.name for row in laid if row.on_path] == ["stalk", "approach", "close in"]
    assert [row.name for row in laid if row.decided] == ["close in"]
    assert laid[-1] == Row(1, "do", "watch", False, False)
    assert not any(row.on_path for row in rows(Stalk(prey=HARE).tree, ()))


def test_the_pane_marks_the_row_that_decided_and_the_world_keeps_its_own_width() -> None:
    from midbrain.ambitions import Stalk

    pygame.init()
    font = pygame.font.SysFont(None, 18)
    mind = stalking_mind()
    surface = pygame.Surface((900 + PANE, 900))
    draw(surface, mind, 1, None, font, max_scale=28.0, pane=PANE)
    laid = rows(Stalk(prey=HARE).tree, mind.decisions[1].path)
    decided = next(i for i, row in enumerate(laid) if row.decided)
    y = 40 + decided * 20
    assert surface.get_at((900 + PANE - 4, y + 2))[:3] == DECIDED, "the deciding row's band spans the pane"
    assert surface.get_at((900 + PANE - 4, 40 + 2))[:3] != DECIDED, "the root's row is not banded"
    # The world is fitted to the 900 px on the left, not the whole surface.
    view = View.fitting([Cell(-8, 0), Cell(6, 0)] + [Cell(3, 0)], 900, 900, 28.0)
    x, yb = view.pixel(Cell(-8, 0))
    assert surface.get_at((int(x), int(yb) + 6))[:3] == SELF

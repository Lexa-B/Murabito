"""A* across the hex plane: shortest in shaku, round what is believed to stand there."""

from __future__ import annotations

import math
import time

from midbrain import murabito_pb2 as pb
from midbrain.beliefs import Cell
from midbrain.paths import cells_along, landing, length, plan, shaku_between

E, ENE, NNE, N, W = pb.Direction.E, pb.Direction.ENE, pb.Direction.NNE, pb.Direction.N, pb.Direction.W


def test_standing_on_the_goal_is_no_steps() -> None:
    assert plan(Cell(2, 3), Cell(2, 3)) == []


def test_straight_east_is_edge_steps() -> None:
    assert plan(Cell(0, 0), Cell(3, 0)) == [E, E, E]


def test_a_corner_cell_is_one_corner_step_not_two_edge_steps() -> None:
    assert plan(Cell(0, 0), Cell(2, -1)) == [ENE], "√3 across the corner beats 2 along the edges"
    assert plan(Cell(0, 0), Cell(1, -2)) == [N]


def test_the_path_is_the_shortest_in_shaku_and_lands_on_the_goal() -> None:
    start, goal = Cell(-8, 0), Cell(6, 0)
    steps = plan(start, goal)
    assert landing(start, steps) == goal
    assert math.isclose(length(steps), shaku_between(start, goal)), "a straight line east"
    start, goal = Cell(-8, 0), Cell(3, -137)
    steps = plan(start, goal)
    assert landing(start, steps) == goal
    assert length(steps) <= shaku_between(start, goal) * 1.04, "within the twelve notches' worst case of a straight line"


def test_a_blocked_cell_is_walked_round() -> None:
    steps = plan(Cell(0, 0), Cell(2, 0), blocked=[Cell(1, 0)])
    assert steps is not None and landing(Cell(0, 0), steps) == Cell(2, 0)
    assert Cell(1, 0) not in cells_along(Cell(0, 0), steps)
    assert length(steps) > 2.0


def test_a_blocked_goal_or_a_walled_in_goal_is_none() -> None:
    assert plan(Cell(0, 0), Cell(1, 0), blocked=[Cell(1, 0)]) is None
    # The six edge neighbours alone don't wall a cell in: a corner step jumps past them.
    edges = [Cell(1, 0), Cell(0, 1), Cell(-1, 1), Cell(-1, 0), Cell(0, -1), Cell(1, -1)]
    assert plan(Cell(0, 0), Cell(5, 0), blocked=edges, limit=500) is not None
    all_twelve = edges + [Cell(2, -1), Cell(1, -2), Cell(-1, -1), Cell(-2, 1), Cell(-1, 2), Cell(1, 1)]
    assert plan(Cell(0, 0), Cell(5, 0), blocked=all_twelve, limit=500) is None


def test_the_layer_is_kept_and_blocked_cells_match_on_the_plane() -> None:
    steps = plan(Cell(0, 0, layer=2), Cell(2, 0, layer=0), blocked=[Cell(1, 0, layer=7)])
    assert landing(Cell(0, 0, layer=2), steps) == Cell(2, 0, layer=2)
    assert Cell(1, 0, layer=2) not in cells_along(Cell(0, 0, layer=2), steps)


def test_a_long_plan_is_quick() -> None:
    started = time.perf_counter()
    assert plan(Cell(-8, 0), Cell(0, -140)) is not None
    assert time.perf_counter() - started < 0.5


def test_cells_along_walks_the_steps_out() -> None:
    assert cells_along(Cell(0, 0), [E, ENE, W]) == [Cell(1, 0), Cell(3, -1), Cell(2, -1)]
    assert landing(Cell(0, 0), []) == Cell(0, 0)

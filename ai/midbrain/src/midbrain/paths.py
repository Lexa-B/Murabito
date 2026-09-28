"""Paths across the plane: what a mind plans and hands the brainstem as steps.

A* over the hex plane, twelve directions a step: an edge step costs one shaku, a corner
step √3, and the heuristic is the straight-line distance in shaku, so the path found is
the shortest walk. Cells the body believes something stands in are blocked. The plane is
unbounded, so a goal walled in would be searched for forever; ``limit`` caps how many
cells are opened before giving up. The layer is not walked: a path stays on its start's.
"""

from __future__ import annotations

import heapq
import math
from collections.abc import Collection

from midbrain.beliefs import Cell
from midbrain.hexes import OFFSETS, to_world

STEP_COST = [1.0 if direction % 2 == 0 else math.sqrt(3) for direction in range(12)]
"""What each direction's step costs in shaku: edge steps on the even notches, corner
steps, √3 across, on the odd ones."""


def neighbour(cell: Cell, direction: int) -> Cell:
    dq, dr = OFFSETS[direction]
    return Cell(cell.q + dq, cell.r + dr, cell.layer)


def shaku_between(a: Cell, b: Cell) -> float:
    (ax, az), (bx, bz) = to_world(a), to_world(b)
    return math.hypot(bx - ax, bz - az)


def plan(start: Cell, goal: Cell, blocked: Collection[Cell] = (), limit: int = 20_000) -> list[int] | None:
    """The directions of the shortest walk from start to goal, avoiding the blocked cells:
    empty standing on it, None if the goal is blocked or not found within ``limit``."""
    goal = Cell(goal.q, goal.r, start.layer)
    if start == goal:
        return []
    walls = {Cell(cell.q, cell.r, start.layer) for cell in blocked}
    if goal in walls:
        return None
    frontier: list[tuple[float, int, Cell]] = [(shaku_between(start, goal), 0, start)]
    came_from: dict[Cell, tuple[Cell, int]] = {}
    cost_so_far: dict[Cell, float] = {start: 0.0}
    opened = 0
    tie = 0
    while frontier:
        _, _, current = heapq.heappop(frontier)
        if current == goal:
            return _trace(came_from, start, goal)
        opened += 1
        if opened > limit:
            return None
        for direction in range(12):
            next_cell = neighbour(current, direction)
            if next_cell in walls:
                continue
            cost = cost_so_far[current] + STEP_COST[direction]
            if cost < cost_so_far.get(next_cell, math.inf):
                cost_so_far[next_cell] = cost
                came_from[next_cell] = (current, direction)
                tie += 1
                heapq.heappush(frontier, (cost + shaku_between(next_cell, goal), tie, next_cell))
    return None


def _trace(came_from: dict[Cell, tuple[Cell, int]], start: Cell, goal: Cell) -> list[int]:
    steps: list[int] = []
    cell = goal
    while cell != start:
        cell, direction = came_from[cell]
        steps.append(direction)
    steps.reverse()
    return steps


def cells_along(start: Cell, steps: list[int]) -> list[Cell]:
    """Where each step lands, in order; empty for no steps."""
    cells = []
    cell = start
    for direction in steps:
        cell = neighbour(cell, direction)
        cells.append(cell)
    return cells


def landing(start: Cell, steps: list[int]) -> Cell:
    """Where the last step lands; the start if there are none."""
    return cells_along(start, steps)[-1] if steps else start


def length(steps: list[int]) -> float:
    """How long a path is, in shaku."""
    return sum(STEP_COST[direction] for direction in steps)

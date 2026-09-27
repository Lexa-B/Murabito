"""The mind's hex geometry, checked against the game's numbers."""

from __future__ import annotations

import math

import pytest

from midbrain import murabito_pb2 as pb
from midbrain.beliefs import Cell
from midbrain.hexes import OFFSETS, along, apart, bearing, from_world, nearest_direction, opposite, rotated, steps, to_world, turn_between


def test_the_twelve_offsets_point_the_compass_way() -> None:
    for index, (dq, dr) in enumerate(OFFSETS):
        x, z = to_world(Cell(dq, dr))
        angle = math.degrees(math.atan2(-z, x)) % 360
        assert math.isclose(angle, 30 * index, abs_tol=1e-6), pb.Direction.Name(index)
    assert OFFSETS[pb.Direction.N] == (1, -2)
    assert OFFSETS[pb.Direction.W] == (-1, 0)


def test_world_and_back_lands_on_the_same_cell() -> None:
    for q in range(-6, 7):
        for r in range(-6, 7):
            assert from_world(*to_world(Cell(q, r))) == Cell(q, r)
    x, z = to_world(Cell(3, -2))
    assert from_world(x + 0.3, z - 0.3) == Cell(3, -2)


def test_steps_is_the_cube_distance() -> None:
    assert steps(Cell(0, 0), Cell(3, 0)) == 3
    assert steps(Cell(0, 0), Cell(1, -2)) == 2
    assert steps(Cell(-8, 0), Cell(6, 0)) == 14


def test_bearing_reads_the_compass_and_none_for_the_same_cell() -> None:
    assert bearing(Cell(0, 0), Cell(3, 0)) == pytest.approx(0.0)
    assert bearing(Cell(0, 0), Cell(1, -2)) == pytest.approx(90.0)
    assert bearing(Cell(6, 0), Cell(-8, 0)) == pytest.approx(180.0)
    assert bearing(Cell(2, 2), Cell(2, 2)) is None
    assert nearest_direction(bearing(Cell(0, 0), Cell(2, -1))) == pb.Direction.ENE


def test_angles_turn_the_short_way() -> None:
    assert turn_between(350, 10) == pytest.approx(20.0)
    assert turn_between(10, 350) == pytest.approx(-20.0)
    assert apart(0, 180) == pytest.approx(180.0)
    assert apart(90, 180) == pytest.approx(90.0)
    assert opposite(pb.Direction.E) == pb.Direction.W
    assert nearest_direction(44) == pb.Direction.ENE and nearest_direction(46) == pb.Direction.NNE


def test_along_walks_a_direction_and_rotated_swings_about_a_cell() -> None:
    assert along(Cell(6, 0), pb.Direction.W, 3) == Cell(3, 0)
    assert along(Cell(0, 0), pb.Direction.N, 2) == Cell(2, -4)
    # Three east of the origin, swung a quarter turn anticlockwise seen from above, is three
    # shaku north: the point (0, -3), whose nearest cell centre is (2, -4)'s at (0, -3.46).
    assert rotated(Cell(3, 0), Cell(0, 0), 90) == Cell(2, -4)
    assert rotated(Cell(3, 0), Cell(0, 0), 90) == from_world(0, -3)
    assert rotated(Cell(3, 0), Cell(0, 0), 180) == Cell(-3, 0)
    assert rotated(Cell(3, 0), Cell(0, 0), 0) == Cell(3, 0)


def test_a_pitched_swing_spirals_in_and_stops_at_the_floor() -> None:
    far = Cell(-14, 0)  # fourteen west of the origin
    assert rotated(far, Cell(0, 0), 30, pitch=0) == rotated(far, Cell(0, 0), 30)
    swung = rotated(far, Cell(0, 0), 30, pitch=15)
    x, z = to_world(swung)
    assert 11.5 <= math.hypot(x, z) <= 12.5, "about 13% nearer per 30 degrees at a 15 degree pitch"
    assert z > 0, "still swung to the south side"
    near = rotated(Cell(3, 0), Cell(0, 0), 30, pitch=60, floor=3.0)
    x, z = to_world(near)
    assert 2.5 <= math.hypot(x, z) <= 3.5, "never inside the floor"

"""Hex geometry the mind needs, on the game's own plane (``crates/hexcoords``).

Pointy-top cells one shaku flat to flat: a cell's centre is at ``x = q + r/2``,
``z = r·√3/2``; east is +x, north is −z. A bearing is a world angle in degrees,
anticlockwise from east, so north is 90 and a ``Direction``'s angle is 30 times its index.
"""

from __future__ import annotations

import math

from midbrain.beliefs import Cell

SQRT_3 = math.sqrt(3)

OFFSETS = [(1, 0), (2, -1), (1, -1), (1, -2), (0, -1), (-1, -1), (-1, 0), (-2, 1), (-1, 1), (-1, 2), (0, 1), (1, 1)]
"""The twelve directions' axial offsets, in index order: E, ENE, NNE, N, NNW, WNW, W, WSW, SSW, S, SSE, ESE."""


def angle_of(direction: int) -> float:
    return 30.0 * direction


def opposite(direction: int) -> int:
    return (direction + 6) % 12


def nearest_direction(degrees: float) -> int:
    return round(degrees / 30.0) % 12


def to_world(cell: Cell) -> tuple[float, float]:
    return cell.q + cell.r / 2, cell.r * SQRT_3 / 2


def from_world(x: float, z: float, layer: int = 0) -> Cell:
    """The cell a point is in: fractional axial, rounded the cube way."""
    r = z / (SQRT_3 / 2)
    q = x - r / 2
    s = -q - r
    rq, rr, rs = round(q), round(r), round(s)
    dq, dr, ds = abs(rq - q), abs(rr - r), abs(rs - s)
    if dq > dr and dq > ds:
        rq = -rr - rs
    elif dr > ds:
        rr = -rq - rs
    return Cell(rq, rr, layer)


def steps(a: Cell, b: Cell) -> int:
    """Cells apart across faces, layer ignored."""
    dq, dr = b.q - a.q, b.r - a.r
    return max(abs(dq), abs(dr), abs(dq + dr))


def bearing(a: Cell, b: Cell) -> float | None:
    """The world angle from ``a`` to ``b`` in degrees, or None if they share a cell."""
    ax, az = to_world(a)
    bx, bz = to_world(b)
    if a.q == b.q and a.r == b.r:
        return None
    return math.degrees(math.atan2(-(bz - az), bx - ax)) % 360.0


def turn_between(a: float, b: float) -> float:
    """The signed shortest turn from angle ``a`` to angle ``b``, in (-180, 180]."""
    return (b - a + 180.0) % 360.0 - 180.0


def apart(a: float, b: float) -> float:
    """How far apart two angles are, 0 to 180."""
    return abs(turn_between(a, b))


def along(cell: Cell, direction: int, count: int) -> Cell:
    dq, dr = OFFSETS[direction]
    return Cell(cell.q + dq * count, cell.r + dr * count, cell.layer)


def rotated(cell: Cell, around: Cell, degrees: float, pitch: float = 0.0, floor: float = 0.0) -> Cell:
    """The cell reached by swinging ``cell`` about ``around`` by ``degrees``, anticlockwise
    seen from above.

    With ``pitch`` 0 the distance is kept: a circle. A positive pitch tilts the path that
    many degrees inward, off the tangent, so the swing spirals in: the distance shrinks by
    ``exp(-sweep · tan(pitch))`` per swing, never below ``floor`` shaku."""
    ax, az = to_world(around)
    x, z = to_world(cell)
    dx, dz = x - ax, z - az
    angle = math.radians(degrees)
    # World z points south, so an anticlockwise turn seen from above is clockwise in (x, z).
    rx = dx * math.cos(angle) + dz * math.sin(angle)
    rz = -dx * math.sin(angle) + dz * math.cos(angle)
    radius = math.hypot(rx, rz)
    if pitch and radius > 0:
        shrunk = max(radius * math.exp(-abs(angle) * math.tan(math.radians(pitch))), floor)
        rx, rz = rx * shrunk / radius, rz * shrunk / radius
    return from_world(ax + rx, az + rz, cell.layer)

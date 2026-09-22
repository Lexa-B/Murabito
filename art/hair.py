"""Hair for a body: a cap over the scalp and pointed clumps, the anime way.

The cap is a closed shell cast over the head: rows of vertices from the crown
down to a hairline, each set on the scalp where a ray from outside meets it and
lifted off it (thick at the crown, thin at the hairline), with an inner layer
just under the scalp so the shell closes. Clumps are pointed blades of diamond
rings (figure.strip) growing from under the cap out over it: bangs over the
forehead, locks beside the face.

Azimuths are degrees round the head from dead ahead (+y) towards the model's
right (+x); the left side mirrors the right.
"""

import math
from dataclasses import dataclass

from mathutils import Vector

import figure

UP = Vector((0, 0, 1))


@dataclass
class Hair:
    """centre: a point inside the head that the cap is cast from.
    hairline: (azimuth, z) from dead ahead (0) to dead behind (180), right side.
    thickness: how far the cap stands off the scalp, (at the crown, at the hairline).
    volume: more lift at the back, fading to nothing at the front, fullest part
        way down (at the back of the skull) and none at the crown or hairline.
    bangs: clumps over the forehead, (azimuth, tip z, width), azimuths either side.
    locks: clumps beside the face, (azimuth, tip z, width), right side, mirrored.
    fall: rings down the back of the head to where it's tied, (z, half-width,
        depth, off): off is how far the ring's middle stands behind the head.
    tail: rings from the tie down the back, the same way; the last closes to a
        point below it.
    cord: (z, height) of the paper cord (motoyui) binding the hair at the tie.
    swoops: strands falling from near the middle of the hairline out across the
        temple and down beside the face, (root azimuth, tip azimuth, tip z,
        width), right side, mirrored.
    sweeps: shallow ridges lying on the cap from the front hairline back over
        the crown to the knot, (azimuth, width): hair pulled back.
    ahoge: stray strands springing up from the cap, (azimuth, where from the
        crown to the hairline, length), curling over at the tip.
    wisps: loose short strands escaping past the hairline, (azimuth, length),
        hanging down over the skin.
    knot: a topknot at the back of the crown, (where from the crown to the
        nape, 0 to 1, length, radius): a bundle bound with the paper cord.
    tuft: the knot's loose ends flaring out of it, each (round the knot in
        degrees, 0 behind and 90 to the right, out from its axis in degrees,
        length, width), drooping a little at their tips.
    """

    centre: tuple
    hairline: list
    thickness: tuple = (0.045, 0.012)
    volume: float = 0.0
    bangs: list = ()
    locks: list = ()
    fall: list = ()
    tail: list = ()
    cord: tuple = None
    swoops: list = ()
    sweeps: list = ()
    ahoge: list = ()
    wisps: list = ()
    knot: tuple = None
    tuft: list = ()
    rows: int = 7
    sides: int = 24
    colour: str = "hair"


class Scalp:
    """Where rays meet the head: tree is a BVHTree of the head without its
    features, so hair lies on the skin, not on the ears."""

    def __init__(self, tree, centre):
        self.tree, self.centre = tree, Vector(centre)

    def along(self, direction):
        """The scalp straight out from the centre along direction, and its normal."""
        direction = Vector(direction).normalized()
        return self._cast(self.centre + direction * 3, -direction)

    def along_line(self, origin, direction):
        """Where a ray from origin meets the body, and its normal."""
        return self._cast(Vector(origin), Vector(direction).normalized())

    def at(self, azimuth, z):
        """The head's surface at an azimuth and height, met by a level ray."""
        out = _outward(azimuth)
        return self._cast(Vector((self.centre.x, self.centre.y, z)) + out * 3, -out)

    def _cast(self, origin, direction):
        hit, normal, *_ = self.tree.ray_cast(origin, direction)
        assert hit is not None, f"no scalp along {direction}"
        return hit, normal if normal.dot(direction) < 0 else -normal


def _outward(azimuth):
    a = math.radians(azimuth)
    return Vector((math.sin(a), math.cos(a), 0))


def _hairline_z(hair, azimuth):
    """The hairline's height at any azimuth, between the given ones, mirrored."""
    a = abs((azimuth + 180) % 360 - 180)
    line = hair.hairline
    for (a0, z0), (a1, z1) in zip(line, line[1:]):
        if a0 <= a <= a1:
            return z0 + (z1 - z0) * (a - a0) / (a1 - a0)
    return line[-1][1]


def build(b, scalp, hair):
    """The cap, every clump and the tied tail. Returns the vertices riding on
    the head, and the tail as (ring, its middle) pairs from the tie down, for
    bones of its own."""
    before = set(b.bm.verts)
    cap(b, scalp, hair)
    for azimuth, tip_z, width in hair.bangs:
        bang(b, scalp, hair, azimuth, tip_z, width)
    for azimuth, tip_z, width in hair.locks:
        for mirror in (1, -1):
            lock(b, scalp, hair, azimuth * mirror, tip_z, width)
    for root, tip, tip_z, width in hair.swoops:
        for mirror in (1, -1):
            swoop(b, scalp, hair, root * mirror, tip * mirror, tip_z, width)
    for azimuth, width in hair.sweeps:
        sweep(b, scalp, hair, azimuth, width)
    for azimuth, t, length in hair.ahoge:
        spring(b, scalp, hair, azimuth, t, length)
    for azimuth, length in hair.wisps:
        wisp(b, scalp, hair, azimuth, length)
    if hair.knot:
        topknot(b, scalp, hair)
    tail = tied_tail(b, scalp, hair) if hair.tail else []
    in_tail = {v for ring, _ in tail for v in ring}
    return [v for v in b.bm.verts if v not in before and v not in in_tail], tail


def _direction(scalp, hair, azimuth, t):
    """Part way (t, 0 at the crown, 1 at the hairline) from the crown down to the
    hairline at this azimuth, as a direction from the centre."""
    crown, _ = scalp.along(UP)
    line, _ = scalp.at(azimuth, _hairline_z(hair, azimuth))
    return (crown - scalp.centre).normalized().slerp((line - scalp.centre).normalized(), t)


def _on_cap(scalp, hair, azimuth, t, above=0.0):
    """A point on the cap's outer surface (and the scalp's normal there)."""
    point, normal = scalp.along(_direction(scalp, hair, azimuth, t))
    lift = hair.thickness[0] + (hair.thickness[1] - hair.thickness[0]) * t
    fuller = hair.volume * (1 - math.cos(math.radians(azimuth))) / 2 * math.sin(math.pi * t)
    return point + normal * (lift + fuller + above), normal


def cap(b, scalp, hair):
    outer, inner = [], []
    for k in range(1, hair.rows + 1):
        t = k / hair.rows
        lift = hair.thickness[0] + (hair.thickness[1] - hair.thickness[0]) * t
        row_out, row_in = [], []
        for j in range(hair.sides):
            azimuth = 360 * j / hair.sides
            point, normal = scalp.along(_direction(scalp, hair, azimuth, t))
            fuller = hair.volume * (1 - math.cos(math.radians(azimuth))) / 2 * math.sin(math.pi * t)
            row_out.append(b.bm.verts.new(point + normal * (lift + fuller)))
            row_in.append(b.bm.verts.new(point - normal * 0.004))
        outer.append(row_out)
        inner.append(row_in)
    crown, normal = scalp.along(UP)
    figure.cap(b, outer[0], crown + normal * hair.thickness[0], hair.colour)
    figure.cap(b, inner[0], crown - normal * 0.004, hair.colour)
    figure.tube(b, outer, hair.colour)
    figure.tube(b, inner, hair.colour)
    figure.bridge(b, outer[-1], inner[-1], hair.colour)


def bang(b, scalp, hair, azimuth, tip_z, width):
    """A clump from under the cap, out over the hairline and down over the forehead
    to a point."""
    root, root_n = scalp.along(_direction(scalp, hair, azimuth, 0.45))
    edge, edge_n = scalp.at(azimuth, _hairline_z(hair, azimuth))
    tip, tip_n = scalp.at(azimuth * 1.05, tip_z)
    over = edge + edge_n * (hair.thickness[1] + 0.02) + UP * 0.01
    tip = tip + tip_n * 0.012
    bend = over.lerp(tip, 0.5) + edge_n * 0.012
    points = [root + root_n * hair.thickness[0] * 0.5, over, bend, tip]
    normals = [root_n, edge_n, edge_n.lerp(tip_n, 0.5).normalized(), tip_n]
    figure.strip(b, points, normals, [width, width, width * 0.7, width * 0.2], 0.01, hair.colour)


def lock(b, scalp, hair, azimuth, tip_z, width):
    """A clump from under the cap down beside the face, in front of the ear, to a point."""
    root, root_n = scalp.along(_direction(scalp, hair, azimuth, 0.5))
    edge, edge_n = scalp.at(azimuth, _hairline_z(hair, azimuth))
    low = (edge.z + tip_z) / 2
    middle, middle_n = scalp.at(azimuth * 0.95, low)
    tip, tip_n = scalp.at(azimuth * 0.9, tip_z)
    points = [
        root + root_n * hair.thickness[0] * 0.5,
        edge + edge_n * (hair.thickness[1] + 0.02),
        middle + middle_n * 0.03,
        tip + tip_n * 0.02,
    ]
    ahead = Vector((0, 1, 0))
    normals = [root_n] + [(n + ahead * 1.2).normalized() for n in (edge_n, middle_n, tip_n)]  # flat side forward, framing the face
    figure.strip(b, points, normals, [width, width, width * 0.75, width * 0.25], 0.011, hair.colour)


def tied_tail(b, scalp, hair):
    """Hair falling down the back of the head, gathered and tied with a paper
    cord behind the nape, and hanging from there down the back in a flattened
    tail to a point. Each ring stands its off behind the body at its height,
    found by a ray from behind, but never further in than the first: hair
    hangs straight down past the nape. Returns the tail's rings from the tie."""
    def ring_at(z, half_width, depth, off):
        back, _ = scalp.along_line(Vector((0, -3, z)), Vector((0, 1, 0)))
        return Vector((0, back.y - off, z)), half_width, depth

    keys = [ring_at(*r) for r in hair.fall] + [ring_at(*r) for r in hair.tail]
    # Hair hangs: below the back of the head it drops straight down rather than
    # following the nape in, and only the back pushes it out.
    drop = keys[0][0].y
    keys = [(Vector((c.x, min(c.y, drop), c.z)), w, d) for c, w, d in keys]
    dense, where = figure.densify([dict(c=c, w=w, d=d) for c, w, d in keys], [1] * (len(hair.fall) - 1) + [3] * len(hair.tail))
    keys = [(r["c"], r["w"], r["d"]) for r in dense]
    rings = []
    for k, (centre, half_width, depth) in enumerate(keys):
        ahead = keys[min(k + 1, len(keys) - 1)][0] - keys[max(k - 1, 0)][0]
        rings.append(figure.ring(b, centre, ahead, Vector((1, 0, 0)), 8, half_width, depth, depth))
    figure.tube(b, rings, hair.colour)
    top, bottom = keys[0][0], keys[-1][0]
    figure.cap(b, rings[0], top + Vector((0, 0.02, 0.05)), hair.colour)
    end = figure.cap(b, rings[-1], bottom + (bottom - keys[-2][0]).normalized() * 0.12, hair.colour)
    rings[-1] = rings[-1] + [end[0].verts[2]]  # the point closing the tail rides with its last ring
    tie = where.index(float(len(hair.fall) - 1))

    if hair.cord:
        z, height = hair.cord
        centre, half_width, depth = keys[tie]
        axis = keys[tie + 1][0] - keys[tie - 1][0]
        band = [figure.ring(b, centre + axis.normalized() * d, axis, Vector((1, 0, 0)), 8,
                            half_width * 1.15, depth * 1.2, depth * 1.2) for d in (-height / 2, height / 2)]
        figure.tube(b, band, "hair_cord")
        for ring, d in zip(band, (-height, height)):
            figure.cap(b, ring, centre + axis.normalized() * d, "hair_cord")
    return [(rings[k], keys[k][0]) for k in range(tie, len(keys))]


def swoop(b, scalp, hair, root_azimuth, tip_azimuth, tip_z, width):
    """A long strand from under the cap near the middle of the forehead, out over
    the hairline, across the temple and down beside the face to a point, its
    flat side turned forward."""
    root, root_n = scalp.along(_direction(scalp, hair, root_azimuth, 0.5))
    edge_z = _hairline_z(hair, root_azimuth)
    edge, edge_n = scalp.at(root_azimuth, edge_z)
    middle_azimuth = root_azimuth + (tip_azimuth - root_azimuth) * 0.6
    middle, middle_n = scalp.at(middle_azimuth, edge_z - (edge_z - tip_z) * 0.35)
    tip, tip_n = scalp.at(tip_azimuth, tip_z)
    ahead = Vector((0, 1, 0))
    points = [root + root_n * hair.thickness[0] * 0.5, edge + edge_n * (hair.thickness[1] + 0.03) + UP * 0.02,
              middle + middle_n * 0.035, tip + tip_n * 0.02]
    normals = [root_n] + [(n + ahead * 0.8).normalized() for n in (edge_n, middle_n, tip_n)]
    figure.strip(b, points, normals, [width * 0.8, width, width * 0.8, width * 0.2], 0.01, hair.colour)


def spring(b, scalp, hair, azimuth, t, length):
    """An ahoge: a thin strand springing up out of the cap and curling over."""
    root, normal = _on_cap(scalp, hair, azimuth, t, above=-0.02)
    rise = (normal + UP).normalized()
    over = (_outward(azimuth) + UP * 0.2).normalized()
    middle = root + rise * length * 0.6
    tip = middle + (rise * 0.3 + over).normalized() * length * 0.5
    across = rise.cross(over)
    flat = across.normalized() if across.length > 1e-3 else Vector((1, 0, 0))
    figure.strip(b, [root, middle, tip], [flat] * 3, [0.03, 0.025, 0.006], 0.008, hair.colour)


def wisp(b, scalp, hair, azimuth, length):
    """A loose strand escaping from under the cap past the hairline, hanging down
    over the skin to a point."""
    root, normal = _on_cap(scalp, hair, azimuth, 0.85, above=-0.01)
    z = _hairline_z(hair, azimuth)
    edge, edge_n = scalp.at(azimuth, z)
    low, low_n = scalp.at(azimuth, z - length)
    points = [root, edge + edge_n * 0.015, low + low_n * 0.012]
    figure.strip(b, points, [normal, edge_n, low_n], [0.035, 0.028, 0.006], 0.006, hair.colour)


def sweep(b, scalp, hair, azimuth, width):
    """A clump lying on the cap from the front hairline back over the crown
    towards the knot: hair pulled back, with volume."""
    back = hair.knot[0] if hair.knot else 0.4
    path = [(azimuth, t) for t in (0.95, 0.7, 0.45, 0.2)] + [(180 if azimuth >= 0 else -180, back * 0.6)]
    points, normals = zip(*(_on_cap(scalp, hair, a, t) for a, t in path))
    widths = [width * w for w in (0.5, 1, 1, 0.8, 0.35)]
    figure.strip(b, list(points), list(normals), widths, 0.02, hair.colour)


def topknot(b, scalp, hair):
    """A short bulging bun at the back of the crown, bound with the paper cord,
    its loose ends flaring out of it in pointed blades."""
    t, length, radius = hair.knot
    base, normal = _on_cap(scalp, hair, 180, t)
    axis = (normal + UP).normalized()
    side = Vector((1, 0, 0))
    behind = axis.cross(side).normalized()  # square to the axis, towards the back
    if behind.y > 0:
        behind = -behind
    rings = [figure.ring(b, base + axis * d, axis, side, 8, r, r, r)
             for d, r in ((-0.03, radius * 1.1), (length * 0.4, radius * 1.25), (length, radius * 0.9))]
    figure.tube(b, rings, hair.colour)
    figure.cap(b, rings[0], base - axis * 0.06, hair.colour)
    top = base + axis * length
    figure.cap(b, rings[-1], top + axis * 0.015, hair.colour)
    band = [figure.ring(b, base + axis * d, axis, side, 8, radius * 1.3, radius * 1.3, radius * 1.3)
            for d in (length * 0.08, length * 0.3)]
    figure.tube(b, band, "hair_cord")
    figure.cap(b, band[0], base + axis * length * 0.05, "hair_cord")
    figure.cap(b, band[1], base + axis * length * 0.33, "hair_cord")
    for around, out, blade, width in hair.tuft:
        a, e = math.radians(around), math.radians(out)
        direction = axis * math.cos(e) + (behind * math.cos(a) + side * math.sin(a)) * math.sin(e)
        start = top - axis * 0.02
        middle = start + direction * blade * 0.5
        tip = middle + direction * blade * 0.5 - UP * blade * 0.2
        radial = direction - axis * direction.dot(axis)
        normal = radial.normalized() if radial.length > 1e-3 else behind  # broad side out from the bun, like a petal
        figure.strip(b, [start, middle, tip], [normal] * 3, [width, width * 0.8, width * 0.2], width * 0.3, hair.colour)

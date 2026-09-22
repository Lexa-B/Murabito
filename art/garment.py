"""Clothes for a body: shells of cloth measured off it, then given thickness.

A garment is built as an open surface a little off the body (ease), from rings
whose size at each height is measured by rays shot in at the body, and then
solidified inward into a closed shell of cloth. The body passes in trees of
just the parts a garment should measure (a robe measures the torso and
shoulders, not the arms hanging beside it), and the garment's vertices later
ride on whatever bones the nearest skin rides on.

    koshimaki: a wrapped skirt, waist to knee (women, under everything)
    hadagi: a close inner robe to the hips, crossed collar, short sleeves
    fundoshi: a loincloth, belt and a panel from back to front between the legs
"""

import math
from dataclasses import dataclass

from mathutils import Vector
from mathutils.bvhtree import BVHTree

import figure

X, Y, Z = Vector((1, 0, 0)), Vector((0, 1, 0)), Vector((0, 0, 1))
SIDES = 30
OFFSET = 6  # the first column's azimuth, so there's a column dead ahead (90) and dead behind (270)


def angle(j):
    """The azimuth of column j: degrees from +x round towards dead ahead."""
    return OFFSET + 360 * j / SIDES
CLOTH = "cloth_white"
THICKNESS = 0.008


# Wrapped cloth closes left over right (the wearer's left, -x, on top): the
# outer layer's edge sits right of the middle and it covers round to the left.
LAYER = 0.012  # how far the outer layer stands proud of the one beneath


def _overlap(edge, end=150):
    """Extra ease for the outer layer of a wrap, from its edge round across the
    front to end: edge(z) is the edge's azimuth at height z (right of dead ahead)."""
    return lambda j, z: LAYER if edge(z) <= angle(j) <= end else 0.0


@dataclass
class Koshimaki:
    """top, hem: heights of the skirt's waist and hem; ease: how loose it hangs;
    edge: the azimuth of the wrap's outer edge, right of dead ahead."""

    top: float
    hem: float
    ease: float = 0.02
    edge: float = 60
    rows: int = 7


@dataclass
class Hadagi:
    """hem: where the robe ends at the hips; neck: the ring it rises to round
    the base of the neck; v_bottom: where the collar's two sides cross, dead
    ahead; below it the outer (left) front carries on down and across to its
    front edge at azimuth cross, reached drop below the crossing, and on down
    to the hem, while the under (right) front passes beneath it; v_width: how
    far round either side the V opens at the neck, in degrees; sleeve: how far
    down the arm the sleeves reach, from the shoulder joint."""

    hem: float
    neck: float
    v_bottom: float
    v_width: float = 40
    cross: float = 70
    drop: float = 0.35
    sleeve: float = 0.6
    ease: float = 0.03
    rows: int = 14


@dataclass
class Fundoshi:
    """An etchu-fundoshi. belt: the height of the cloth belt round the hips;
    crotch: the height of the lowest point between the legs; apron: where the
    front flap hangs to; back, under, front, flap: the cloth's width over the
    seat, between the legs, up the front and hanging as the apron."""

    belt: float
    crotch: float
    apron: float
    back: float = 0.42
    under: float = 0.12
    front: float = 0.22
    flap: float = 0.3
    ease: float = 0.012
    columns: int = 7


def tree_of(faces):
    """A BVHTree of just these faces, for measuring part of the body."""
    index, verts, polys = {}, [], []
    for f in faces:
        poly = []
        for v in f.verts:
            if v not in index:
                index[v] = len(verts)
                verts.append(v.co.copy())
            poly.append(index[v])
        polys.append(poly)
    return BVHTree.FromPolygons(verts, polys)


def _around(tree, z, ease, layer=None):
    """Points round the body at height z, one per SIDES azimuth (0 at +x, 90
    ahead), each where a ray in towards the middle meets the body, eased out
    along the surface. Where a ray finds nothing (the gap between the legs)
    the point spans straight across from its neighbours, as cloth would."""
    points = []
    for j in range(SIDES):
        a = math.radians(angle(j))
        out = Vector((math.cos(a), math.sin(a), 0))
        hit, normal, *_ = tree.ray_cast(Vector((0, 0, z)) + out * 3, -out)
        if hit is not None and Vector((hit.x, hit.y, 0)).dot(out) <= 0:
            hit = None  # slipped through a gap (between the legs) to the far side
        if hit is None:
            points.append(None)
        else:
            normal = normal if normal.dot(out) > 0 else -normal
            points.append(hit + (normal + out).normalized() * ease)
    known = [j for j, p in enumerate(points) if p is not None]
    for j, p in enumerate(points):
        if p is None:
            before = max((k for k in known if k < j), default=known[-1])
            after = min((k for k in known if k > j), default=known[0])
            span = (after - before) % SIDES
            t = ((j - before) % SIDES) / span
            points[j] = points[before].lerp(points[after], t)
    points = _taut(points, z)
    if layer:  # an outer layer of a wrap stands proud after the cloth is pulled taut, so its edge shows
        points = [p + Vector((p.x, p.y, 0)).normalized() * layer(j, z) for j, p in enumerate(points)]
    return points


def _taut(points, z):
    """Cloth bridges hollows: push each point out onto the convex outline of
    them all (seen from above), along its own direction from the middle."""
    flat = sorted({(round(p.x, 6), round(p.y, 6)) for p in points})

    def cross(o, a, b):
        return (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])

    lower, upper = [], []
    for p in flat:
        while len(lower) >= 2 and cross(lower[-2], lower[-1], p) <= 0:
            lower.pop()
        lower.append(p)
    for p in reversed(flat):
        while len(upper) >= 2 and cross(upper[-2], upper[-1], p) <= 0:
            upper.pop()
        upper.append(p)
    hull = lower[:-1] + upper[:-1]
    cx = sum(p[0] for p in hull) / len(hull)
    cy = sum(p[1] for p in hull) / len(hull)
    taut = []
    for p in points:
        d = Vector((p.x - cx, p.y - cy))
        if d.length < 1e-6:
            taut.append(p)
            continue
        d.normalize()
        reach = d.length and max(_ray_to_edge((cx, cy), d, a, b) for a, b in zip(hull, hull[1:] + hull[:1]))
        now = Vector((p.x - cx, p.y - cy)).length
        if reach > now:
            taut.append(Vector((cx + d.x * reach, cy + d.y * reach, p.z)))
        else:
            taut.append(p)
    return taut


def _ray_to_edge(origin, direction, a, b):
    """How far along direction from origin a ray meets the segment a-b (0 if it misses)."""
    ex, ey = b[0] - a[0], b[1] - a[1]
    denom = direction.x * ey - direction.y * ex
    if abs(denom) < 1e-9:
        return 0.0
    ax, ay = a[0] - origin[0], a[1] - origin[1]
    t = (ax * ey - ay * ex) / denom
    u = (ax * direction.y - ay * direction.x) / denom
    return t if t > 0 and -1e-6 <= u <= 1 + 1e-6 else 0.0


def _wrap(b, tree, heights, ease, layer=None):
    """A band of cloth round the body: rows of points from _around, each row
    at heights(azimuth index, row), bridged in step. Returns rows of vertices
    (top row first) and the faces."""
    rows = []
    for height in heights:
        row = []
        for j in range(SIDES):
            row.append(b.bm.verts.new(_around_one(tree, height(j), ease, j, layer)))
        rows.append(row)
    faces = []
    for upper, lower in zip(rows, rows[1:]):
        for j in range(SIDES):
            k = (j + 1) % SIDES
            faces.append(b.face((upper[j], upper[k], lower[k], lower[j]), CLOTH))
    return rows, faces


def _around_one(tree, z, ease, j, layer=None):
    return _around(tree, z, ease, layer)[j]


def solidify(b, faces, colour, axis=None, trust=False):
    """Give an open surface of cloth its thickness, inward: every vertex gets a
    twin THICKNESS in from it, every face a reversed twin, and every open edge a
    rim joining the two, so the shell is closed. Faces' outward sides are the
    ones facing away from the axis they wrap (a vertical line through the body,
    or an arm's line, given as (point, direction)), or with trust, the sides
    the faces already face. Built in the faces' own
    order, so a rebuild comes out the same. Returns the shell's vertices."""
    faces = list(faces)
    origin, direction = axis or (Vector((0, 0, 0)), Z)

    def away(p):
        off = p - origin
        return off - direction * off.dot(direction)

    normals = {}
    for f in faces:
        f.normal_update()
        n = f.normal.copy()
        if not trust and n.dot(away(f.calc_center_median())) < 0:
            n = -n
        for v in f.verts:
            normals.setdefault(v, Vector())
            normals[v] += n
    inner = {}
    for f in faces:
        for v in f.verts:
            if v not in inner:
                inner[v] = b.bm.verts.new(v.co - normals[v].normalized() * THICKNESS)
    counts = {}
    for f in faces:
        for e in f.edges:
            counts[e] = counts.get(e, 0) + 1
    shell = list(faces)
    for f in faces:
        shell.append(b.face([inner[v] for v in reversed(f.verts)], colour))
    for f in faces:
        for loop in f.loops:
            if counts[loop.edge] == 1:
                a, c = loop.vert, loop.link_loop_next.vert
                shell.append(b.face((c, a, inner[a], inner[c]), colour))
    b.paint(shell, colour)
    outer = list(dict.fromkeys(v for f in faces for v in f.verts))
    return outer + [inner[v] for v in outer]


def _column(row, edge):
    """The first column of a row lying in the outer layer of a wrap whose edge
    is at azimuth edge(z)."""
    step = 360 / SIDES
    guess = row[round((edge(row[0].co.z) - OFFSET) / step) % SIDES]
    return math.ceil((edge(guess.co.z) - OFFSET) / step - 1e-6) % SIDES


def _band(b, points, widths, lift, thickness):
    """A flat cloth band along points on the garment's surface, standing lift
    off it. Returns its vertices."""
    normals, lifted = [], []
    for p in points:
        out = Vector((p.x, p.y, 0))
        out = out.normalized() if out.length > 1e-4 else Y
        normals.append(out)
        lifted.append(p + out * lift)
    before = set(b.bm.verts)
    figure.strip(b, lifted, normals, widths, thickness, CLOTH)
    return [v for v in b.bm.verts if v not in before]


def edge_hem(b, rows, edge):
    """A thin hemmed edge down the outer layer of a wrap, where it lies over the
    layer beneath, down the given rows. Returns its vertices."""
    points = [row[_column(row, edge)].co.copy() for row in rows]
    return _band(b, points, [0.022] * len(points), 0.004, 0.007)


def koshimaki(b, trees, spec):
    """A cloth wrapped round from the waist to above the knee, hanging straight
    over the seat and across between the legs, closing left over right."""
    zs = [spec.top + (spec.hem - spec.top) * i / spec.rows for i in range(spec.rows + 1)]
    edge = lambda z: spec.edge  # noqa: E731
    rows, faces = _wrap(b, trees["hips"], [lambda j, z=z: z for z in zs], spec.ease, _overlap(edge))
    return solidify(b, faces, CLOTH) + edge_hem(b, rows, edge)


def hadagi(b, trees, spec, arms):
    """A close inner robe from the hips up round the base of the neck, its two
    fronts crossing left over right: a V at the neck to where they cross, the
    outer front carrying on down and across over the under one, each edged with
    its collar band. Short sleeves."""
    def neckline(j):
        a = math.radians(angle(j))
        if math.sin(a) <= 0:
            return spec.neck
        # straight sides as seen from ahead: rising with how far across the chest, not how far round
        return spec.v_bottom + (spec.neck - spec.v_bottom) * min(1.0, abs(math.cos(a)) / math.sin(math.radians(spec.v_width)))

    def edge(z):
        t = min(max((spec.v_bottom - z) / spec.drop, 0.0), 1.0)
        return 90 + (spec.cross - 90) * t

    heights = [lambda j, i=i: spec.hem + (neckline(j) - spec.hem) * (1 - i / spec.rows) for i in range(spec.rows + 1)]
    rows, faces = _wrap(b, trees["chest"], heights, spec.ease, _overlap(edge))
    verts = solidify(b, faces, CLOTH)
    verts += collars(b, rows, spec, edge)
    below = [row for row in rows[1:] if row[_column(row, edge)].co.z < spec.v_bottom - spec.drop * 0.7]  # under the collar's end
    verts += edge_hem(b, below, edge)
    for joint, along, arm in arms:
        verts += sleeve(b, joint, along, arm, spec)
    return verts


def collars(b, rows, spec, edge):
    """The two collar bands (eri). The outer (left) front's runs from the back of
    the neck over the left shoulder, down to the crossing and on down its front
    edge, standing proud; the under (right) front's runs from the back of the
    neck down to the crossing and tucks beneath it."""
    top, n = rows[0], SIDES
    step = 360 / n
    point = round((90 - OFFSET) / step)  # the column dead ahead: the point of the V
    back = round((270 - OFFSET) / step)
    left = [top[j].co.copy() for j in range(point + 1, back + 1)]
    right = [top[j % n].co.copy() for j in range(back, point + n + 1)]
    down = [row[_column(row, edge)].co.copy() for row in rows[1:]
            if row[_column(row, edge)].co.z >= spec.v_bottom - spec.drop]
    outer = down[::-1] + left

    def widths(k):
        return [0.02] + [0.045] * (k - 2) + [0.03]

    return (_band(b, outer, widths(len(outer)), 0.008, 0.012)
            + _band(b, right, widths(len(right))[::-1], 0.003, 0.012))


def sleeve(b, joint, along, arm, spec):
    """A short sleeve round the upper arm from inside the robe's shoulder to
    sleeve along the arm, a little loose."""
    def dims(d):
        for (d0, *a), (d1, *c) in zip(arm, arm[1:]):
            if d0 <= d <= d1:
                t = (d - d0) / (d1 - d0)
                return [x + (y - x) * t for x, y in zip(a, c)]
        return list(arm[-1][1:])

    rings = []
    for d in (0.02, spec.sleeve * 0.5, spec.sleeve):
        half, front, back = dims(max(d, arm[0][0]))
        rings.append(figure.ring(b, joint + along * d, along, Y, 18, half + spec.ease * 1.5,
                                 front + spec.ease * 1.5, back + spec.ease * 1.5))
    rows = figure.tube(b, rings, CLOTH)
    return solidify(b, [f for row in rows for f in row], CLOTH, (joint, along))


def fundoshi(b, trees, spec):
    """An etchu-fundoshi: a narrow cloth belt low round the hips, and a cloth
    from the belt at the back down over the seat, narrowing between the legs,
    up the front and over the belt, hanging down in front as an apron. The
    cloth lies on the body (pulled taut across the cleft of the seat); the
    apron hangs straight down, clear of it."""
    tree = trees["hips"]
    belt_rows, belt_faces = _wrap(b, tree, [lambda j: spec.belt + 0.025, lambda j: spec.belt - 0.025], spec.ease)
    verts = solidify(b, belt_faces, CLOTH)

    back, down, ahead = -Y, -Z, Y
    seat = [(spec.belt + 0.03, spec.back * 0.8), (spec.belt - 0.1, spec.back),
            ((spec.belt + spec.crotch) / 2 - 0.05, spec.back), (spec.crotch + 0.04, spec.back * 0.95),
            (spec.crotch - 0.05, spec.back * 0.8), (spec.crotch - 0.1, spec.back * 0.55)]  # down the seat, which hangs below the crotch
    rows = [_lying(tree, z, back, w, spec) for z, w in seat]
    up = [(spec.crotch + 0.04, spec.under * 1.4), ((spec.belt + spec.crotch) / 2, spec.front * 0.9),
          (spec.belt - 0.06, spec.front), (spec.belt + 0.05, spec.front)]
    front_rows = [_lying(tree, z, ahead, w, spec, extra=0.015 if k == len(up) - 1 else 0.0) for k, (z, w) in enumerate(up)]
    behind, ahead_y = rows[-1][len(rows[-1]) // 2].y, front_rows[0][len(front_rows[0]) // 2].y
    rows += [_under(spec.crotch, behind + (ahead_y - behind) * t, spec) for t in (0.3, 0.7)]
    rows += front_rows
    hang_y = rows[-1][len(rows[-1]) // 2].y + 0.012
    for z in (spec.belt + 0.03, (spec.belt + spec.apron) / 2, spec.apron):
        rows.append([Vector((-spec.flap / 2 + spec.flap * c / (spec.columns - 1), hang_y, z)) for c in range(spec.columns)])

    grid = [[b.bm.verts.new(p) for p in row] for row in rows]
    faces = []
    for upper, lower in zip(grid, grid[1:]):
        for c in range(spec.columns - 1):
            faces.append(b.face((upper[c], upper[c + 1], lower[c + 1], lower[c]), CLOTH))
    first = faces[0]
    first.normal_update()
    if first.normal.dot(back) < 0:  # the cloth's outer side faces away from the body
        for f in faces:
            f.normal_flip()
    return verts + solidify(b, faces, CLOTH, trust=True)


def _lying(tree, z, outward, width, spec, extra=0.0):
    """A row of columns across the cloth at height z, lying on the body's front
    or back (outward), pulled taut across any hollow between them."""
    xs = [-width / 2 + width * c / (spec.columns - 1) for c in range(spec.columns)]
    depth = []
    for x in xs:
        hit, *_ = tree.ray_cast(Vector((x, 0, z)) + outward * 3, -outward)
        depth.append(hit.dot(outward) if hit is not None else None)
    known = [d for d in depth if d is not None]
    depth = [d if d is not None else min(known) for d in depth]
    taut = _upper_hull(xs, depth)
    return [Vector((x, 0, z)) + outward * (d + spec.ease + extra) for x, d in zip(xs, taut)]


def _under(crotch, y, spec):
    """A row of columns across the cloth passing flat under the crotch, at y."""
    xs = [-spec.under / 2 + spec.under * c / (spec.columns - 1) for c in range(spec.columns)]
    return [Vector((x, y, crotch - spec.ease - 0.01 * (1 - (2 * x / spec.under) ** 2))) for x in xs]


def _upper_hull(xs, heights):
    """Heights raised onto the upper convex outline of the points (x, height):
    cloth stretched across a hollow."""
    hull = []
    for p in zip(xs, heights):
        while len(hull) >= 2 and (hull[-1][0] - hull[-2][0]) * (p[1] - hull[-2][1]) - (hull[-1][1] - hull[-2][1]) * (p[0] - hull[-2][0]) >= 0:
            hull.pop()
        hull.append(p)
    out = []
    for x, h in zip(xs, heights):
        for (x0, h0), (x1, h1) in zip(hull, hull[1:]):
            if x0 <= x <= x1:
                out.append(max(h, h0 + (h1 - h0) * (x - x0) / (x1 - x0) if x1 > x0 else h0))
                break
        else:
            out.append(h)
    return out

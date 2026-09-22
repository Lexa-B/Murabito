"""Building people: tubes of rings round a body, bridged where limbs meet it.

An animal is lofted along one spine (loft.py). A person is taller than long and
branches: a torso that runs up into the neck and head, legs that split from its
bottom ring at the crotch, and arms bridged into holes left in its sides. Every
part is a list of rings, each an ellipse of vertices square to the part's axis,
and consecutive rings are bridged with quads.

    b = loft.Builder()
    hips = figure.ring(b, (0, 0, 2.3), Z, X, 18, 0.46, 0.27, 0.30)
    ...
    figure.bridge(b, upper, lower, "skin")

A ring's frame: `axis` runs along the part, `side` is its local +x, and its
local front is axis x side. A torso ring with axis +Z and side +X faces +Y, the
way models face. Its vertices start at angle `offset` from the side and go
round towards the front; `front` and `back` are its half depths either side of
the side axis, so a ring can be egg-shaped.
"""

import math

import bmesh

from mathutils import Vector

X, Y, Z = Vector((1, 0, 0)), Vector((0, 1, 0)), Vector((0, 0, 1))


def frame(axis, side):
    """The ring frame for an axis: side made square to it, and the front."""
    axis = Vector(axis).normalized()
    side = Vector(side)
    side = (side - axis * side.dot(axis)).normalized()
    return axis, side, axis.cross(side)


def ring(b, centre, axis, side, n, half_width, front, back, offset=0.0, pinch=0.0, bumps=()):
    """n new vertices round centre, square to axis. Returns them in order.

    pinch narrows the front half towards its middle (a V-shaped jaw or chin);
    bumps are (angle, spread, amount) in degrees and shaku, pushing the ring out
    round an angle (a bust, the glutes, a calf), fading to nothing at spread.
    """
    axis, side, fwd = frame(axis, side)
    centre = Vector(centre)
    verts = []
    for k in range(n):
        t = math.radians(offset) + 2 * math.pi * k / n
        s, c = math.sin(t), math.cos(t)
        across = half_width * c * (1 - pinch * max(s, 0.0))
        depth = (front if s > 0 else back) * s
        p = side * across + fwd * depth
        for angle, spread, amount in bumps:
            gap = abs((math.degrees(t) - angle + 180) % 360 - 180)
            if gap < spread and p.length > 0:
                p += p.normalized() * amount * 0.5 * (1 + math.cos(math.pi * gap / spread))
        verts.append(b.bm.verts.new(centre + p))
    return verts


def _normal(loop):
    """A loop's facing (Newell's method): which way it winds."""
    n = Vector()
    for a, c in zip(loop, loop[1:] + loop[:1]):
        a, c = a.co, c.co
        n += Vector(((a.y - c.y) * (a.z + c.z), (a.z - c.z) * (a.x + c.x), (a.x - c.x) * (a.y + c.y)))
    return n


def aligned(upper, lower):
    """lower, turned to wind the same way as upper and to start at the vertex
    nearest upper's first, so two loops needn't be built in step."""
    assert len(upper) == len(lower), (len(upper), len(lower))
    n = len(upper)
    if _normal(upper).dot(_normal(lower)) < 0:
        lower = lower[::-1]
    shift = min(range(n), key=lambda s: sum((upper[i].co - lower[(i + s) % n].co).length for i in range(n)))
    return lower[shift:] + lower[:shift]


def bridge(b, upper, lower, colour):
    """Quads between two loops of the same length (see aligned). Returns the faces."""
    lower = aligned(upper, lower)
    n = len(upper)
    return [b.face((upper[i], upper[(i + 1) % n], lower[(i + 1) % n], lower[i]), colour) for i in range(n)]


def blend(b, upper, lower, t, swell=0.0):
    """A new loop part way (t) from upper to lower, vertex by vertex, pushed out
    from its centre by swell: eases one shape into another, as where a leg's
    share of the hips becomes a round thigh."""
    lower = aligned(upper, lower)
    points = [u.co.lerp(l.co, t) for u, l in zip(upper, lower)]
    centre = sum(points, Vector()) / len(points)
    return [b.bm.verts.new(p + (p - centre) * swell) for p in points]


def bridge_split(b, root, ring, colour):
    """Faces from a loop to one with k times as many vertices, each root edge
    fanning out to k ring edges: how a finger's square share of the knuckles
    becomes a round finger. Returns the faces."""
    n, m = len(root), len(ring)
    assert m % n == 0, (n, m)
    k = m // n
    if _normal(root).dot(_normal(ring)) < 0:
        ring = ring[::-1]
    shift = min(range(m), key=lambda s: sum((root[i].co - ring[(k * i + s) % m].co).length for i in range(n)))
    ring = ring[shift:] + ring[:shift]
    faces = []
    for i in range(n):
        a, c = root[i], root[(i + 1) % n]
        fan = [ring[(k * i + j) % m] for j in range(k + 1)]
        middle = k // 2
        for j in range(k):
            faces.append(b.face((a if j < middle else c, fan[j], fan[j + 1]), colour))
        faces.append(b.face((a, fan[middle], c), colour))
    return faces


def hole(b, faces):
    """Delete these faces (one connected patch) and return the loop of
    vertices round the hole, for a limb to grow from. The loop starts at the
    first face's first boundary corner, so a rebuild comes out the same."""
    patch = set(faces)
    nexts, start = {}, None
    for f in faces:
        for loop in f.loops:
            if sum(g in patch for g in loop.edge.link_faces) == 1:
                nexts[loop.vert] = loop.link_loop_next.vert
                start = start or loop.vert
    loop = [start]
    while nexts[loop[-1]] is not start:
        loop.append(nexts[loop[-1]])
    bmesh.ops.delete(b.bm, geom=list(faces), context="FACES_KEEP_BOUNDARY")
    return loop


def catmull(p0, p1, p2, p3, t):
    """The Catmull-Rom curve through p1 (t = 0) and p2 (t = 1), shaped by its
    neighbours p0 and p3: numbers or Vectors."""
    return 0.5 * (
        p1 * 2
        + (p2 - p0) * t
        + (p0 * 2 - p1 * 5 + p2 * 4 - p3) * t * t
        + (p1 * 3 - p0 - p2 * 3 + p3) * t * t * t
    )


def densify(keys, steps):
    """More rings, on a smooth curve through the key rings: keys are dicts of
    numbers or Vectors, all with the same keys, and steps[i] is how many pieces
    the gap after key i is cut into (1 leaves it alone). Returns the rings and,
    for each, its position counted in keys (key 2 is 2.0, half way to key 3 is
    2.5), for weighing it between bones."""
    rings, where = [], []
    last = len(keys) - 1
    for i, key in enumerate(keys):
        rings.append(dict(key))
        where.append(float(i))
        if i == last:
            break
        p0, p1, p2, p3 = keys[max(i - 1, 0)], key, keys[i + 1], keys[min(i + 2, last)]
        for step in range(1, steps[i]):
            t = step / steps[i]
            rings.append({name: catmull(p0[name], p1[name], p2[name], p3[name], t) for name in key})
            where.append(i + t)
    return rings, where


def strip(b, points, normals, widths, thickness, colour):
    """A thin raised line along a surface, for brows, lash lines and mouths: a
    closed ridge of diamond-shaped rings through points (a little below the
    surface there), standing thickness out along each point's normal, widths
    across it at each point, pointed at both ends."""
    rings = []
    for k, (p, n, w) in enumerate(zip(points, normals, widths)):
        ahead = points[min(k + 1, len(points) - 1)] - points[max(k - 1, 0)]
        rings.append(ring(b, p, ahead, n, 4, thickness, w / 2, w / 2))
    tube(b, rings, colour)
    cap(b, rings[-1], points[-1] + (points[-1] - points[-2]).normalized() * widths[-1] * 0.5, colour)
    cap(b, rings[0], points[0] + (points[0] - points[1]).normalized() * widths[0] * 0.5, colour)


def tube(b, rings, colour):
    """Bridge each ring to the next; returns rows of faces."""
    return [bridge(b, a, c, colour) for a, c in zip(rings, rings[1:])]


def cap(b, loop, tip, colour):
    """Close a loop with a fan of triangles to a point."""
    apex = b.bm.verts.new(Vector(tip))
    n = len(loop)
    return [b.face((loop[i], loop[(i + 1) % n], apex), colour) for i in range(n)]

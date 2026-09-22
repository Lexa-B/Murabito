"""Building plants: branches lofted along 3D polylines, and faceted foliage clumps.

Plants aren't rigged, so unlike the animals a plant is several closed shells
that overlap (trunk, branches, clumps) rather than one connected mesh. They use
loft.Builder for palette colouring and finishing.

Coordinates are Blender's, in shaku: +Z up, the plant's base at the origin.
"""

import math
import random

import bmesh
from mathutils import Vector

BRANCH_RING = 6
ROOT_DEPTH = -1.5  # shaku: how far trunks reach below the ground, so sloping ground shows no gap


def branch(b, points, radii, colour, upright=0):
    """Loft a closed tube through points (3D), with a radius per point.

    colour is one swatch, or a list with one per segment (len(points) - 1).
    The first upright rings are kept level, whichever way the tube leans: for
    a trunk, whose base should sit flat in the ground.
    Returns the side faces, one list per segment, for recolouring.
    """
    points = [Vector(p) for p in points]
    colours = [colour] * (len(points) - 1) if isinstance(colour, str) else list(colour)
    rings = []
    for i, (centre, r) in enumerate(zip(points, radii, strict=True)):
        prev = points[max(i - 1, 0)]
        nxt = points[min(i + 1, len(points) - 1)]
        tangent = Vector((0, 0, 1)) if i < upright else (nxt - prev).normalized()
        ref = Vector((0, 1, 0)) if abs(tangent.y) < 0.9 else Vector((1, 0, 0))
        side = tangent.cross(ref).normalized()
        up = side.cross(tangent)
        rings.append(
            [
                b.bm.verts.new(
                    centre
                    + side * r * math.cos(2 * math.pi * k / BRANCH_RING)
                    + up * r * math.sin(2 * math.pi * k / BRANCH_RING)
                )
                for k in range(BRANCH_RING)
            ]
        )
    n = BRANCH_RING
    segments = []
    for a, c, segment_colour in zip(rings[:-1], rings[1:], colours, strict=True):
        segments.append(
            [b.face((a[k], a[(k + 1) % n], c[(k + 1) % n], c[k]), segment_colour) for k in range(n)]
        )
    b.face(list(reversed(rings[0])), colours[0])
    b.face(rings[-1], colours[-1])
    return segments


def densify(points, radii, step):
    """Put extra points along a polyline, at most step apart, radii following."""
    points = [Vector(p) for p in points]
    out_points, out_radii = [points[0]], [radii[0]]
    for (p0, r0), (p1, r1) in zip(zip(points, radii), zip(points[1:], radii[1:])):
        n = max(1, math.ceil((p1 - p0).length / step))
        for i in range(1, n + 1):
            out_points.append(p0.lerp(p1, i / n))
            out_radii.append(r0 + (r1 - r0) * i / n)
    return out_points, out_radii


def gnarl(points, radii, rng, amount, step):
    """Make a polyline gnarly: points at most step apart, each inner one pushed
    off line by up to amount (less up and down than sideways), and radii
    swelling and pinching a little, for knots. The ends stay where they are,
    so joints still meet."""
    points, radii = densify(points, radii, step)
    out_points, out_radii = [points[0]], [radii[0]]
    for p, r in zip(points[1:-1], radii[1:-1]):
        out_points.append(p + Vector((
            rng.uniform(-amount, amount), rng.uniform(-amount, amount), rng.uniform(-amount, amount) * 0.4,
        )))
        out_radii.append(r * rng.uniform(0.85, 1.2))
    out_points.append(points[-1])
    out_radii.append(radii[-1])
    return out_points, out_radii


def sink(points, radii, depth=ROOT_DEPTH):
    """A trunk polyline with a point added straight below its base at depth,
    to bury it; draw it with branch(..., upright=2) so both rings are level."""
    points = [Vector(p) for p in points]
    return [Vector((points[0].x, points[0].y, depth))] + points, [radii[0] * 1.05] + list(radii)


def point_at_height(points, z):
    """Where a mostly upright polyline passes height z."""
    points = [Vector(p) for p in points]
    for p0, p1 in zip(points, points[1:]):
        if p0.z <= z <= p1.z:
            return p0.lerp(p1, (z - p0.z) / (p1.z - p0.z))
    return points[-1].copy()


def clump(b, centre, radius, up, down, leaves, shades, rng, lumpiness=0.12):
    """A faceted foliage blob: an icosphere reaching up above its centre and down
    below it, so a small down gives a flattish underside like a mushroom cap.
    Its vertices are pushed in and out a little so no two clumps look alike.

    Each face takes a colour at random from leaves, or from shades if it points
    down; repeat a colour in the list to make it more likely.
    """
    made = bmesh.ops.create_icosphere(b.bm, subdivisions=2, radius=1.0)
    verts = made["verts"]
    centre = Vector(centre)
    for v in verts:
        x, y, z = v.co
        local = Vector((x * radius, y * radius, z * (up if z > 0 else down)))
        v.co = centre + local * (1 + rng.uniform(-lumpiness, lumpiness))
    faces = []  # in a fixed order, so a seed always paints the same faces
    for v in verts:
        faces.extend(f for f in v.link_faces if f not in faces)
    for f in faces:
        f.normal_update()
        b.paint([f], rng.choice(shades if f.normal.z < -0.35 else leaves))


def _exit_distance(origin, direction, centre, radius, up, down):
    """How far along a ray from origin it leaves a lump: an ellipsoid reaching
    up above its centre and down below it. None if the ray misses it."""
    for height, upper in ((up, True), (down, False)):
        scale = (1 / radius, 1 / radius, 1 / height)
        o = Vector([(origin[i] - centre[i]) * scale[i] for i in range(3)])
        d = Vector([direction[i] * scale[i] for i in range(3)])
        qa, qb, qc = d.dot(d), 2 * o.dot(d), o.dot(o) - 1
        disc = qb * qb - 4 * qa * qc
        if disc < 0:
            return None
        t = (-qb + math.sqrt(disc)) / (2 * qa)
        if t > 0 and ((origin + direction * t).z >= centre[2]) == upper:
            return t
    return None


GOLDEN_ANGLE = 2.39996  # radians: spreads branches evenly round a trunk
TRI_EDGE = 1.8  # shaku: the triangle edge foliage skins aim for (the maple's is ~1.9)


def _even_sphere(bm, count):
    """A unit sphere of count vertices spread evenly by a golden-angle spiral,
    joined into triangles. Unlike an icosphere, any count will do."""
    verts = []
    for i in range(count):
        z = 1 - 2 * (i + 0.5) / count
        r = math.sqrt(1 - z * z)
        verts.append(bm.verts.new((r * math.cos(i * GOLDEN_ANGLE), r * math.sin(i * GOLDEN_ANGLE), z)))
    bmesh.ops.convex_hull(bm, input=verts)
    return verts


def _reach(co, centre, lumps, shape):
    """The ray a sphere vertex at co casts from centre, and how far along it the
    last lump ends."""
    if shape is None:
        direction = co.normalized()
    else:
        radius, up, down = shape
        x, y, z = co
        direction = Vector((x * radius, y * radius, z * (up if z > 0 else down))).normalized()
    reach = [_exit_distance(centre, direction, Vector(c), r, up, down) for c, r, up, down in lumps]
    return direction, max(t for t in reach if t is not None)


def even_triangles(centre, lumps, shape, edge=TRI_EDGE):
    """How many triangles a skin wrapped over lumps needs for edges of about
    edge: a trial wrap measures its real surface first."""
    bm = bmesh.new()
    verts = _even_sphere(bm, 402)
    centre = Vector(centre)
    for v in verts:
        direction, distance = _reach(v.co, centre, lumps, shape)
        v.co = centre + direction * distance
    area = sum(f.calc_area() for f in bm.faces)
    bm.free()
    return max(20, int(area / (math.sqrt(3) / 4 * edge**2)))


def canopy(b, centre, lumps, leaves, shades, rng, subdivisions=4, lumpiness=0.03, shape=None, triangles=None):
    """One continuous crown shrink-wrapped over lumps, each (centre, radius, up,
    down) as in clump.

    A single icosphere around centre has each vertex pushed out along its ray to
    where it leaves the last lump, so the triangles are an even size all over
    and the lumps blend into one skin instead of cutting through each other.
    The lumps together should be star-shaped from centre, as a crown is.
    Faces are coloured as in clump.

    For a flat shape, give shape = (radius, up, down) roughly matching the
    lumps: the sphere is squashed to it before its rays are cast, so the
    triangles stay even across a flat top instead of bunching at the rim.

    By default the sphere is an icosphere of the given subdivisions, whose
    triangle count only comes in steps of four; give triangles instead for a
    sphere of about that many (see even_triangles). Returns the skin's faces.
    """
    if triangles is None:
        verts = bmesh.ops.create_icosphere(b.bm, subdivisions=subdivisions, radius=1.0)["verts"]
    else:
        verts = _even_sphere(b.bm, triangles // 2 + 2)
    centre = Vector(centre)
    for v in verts:
        direction, distance = _reach(v.co, centre, lumps, shape)
        v.co = centre + direction * distance * (1 + rng.uniform(-lumpiness, lumpiness))
    faces = []  # in a fixed order, so a seed always paints the same faces
    for v in verts:
        faces.extend(f for f in v.link_faces if f not in faces)
    for f in faces:
        f.normal_update()
        b.paint([f], rng.choice(shades if f.normal.z < -0.35 else leaves))
    return faces


def _pad_parts(centre, radius, up, down, lumps, rng):
    """A pad's lumps: a low dome with a few smaller lumps rising from its top."""
    parts = [(centre, radius, up, down)]
    for i in range(lumps):
        angle = 2 * math.pi * i / lumps + rng.uniform(-0.5, 0.5)
        reach = radius * rng.uniform(0.35, 0.6)
        offset = Vector((math.cos(angle) * reach, math.sin(angle) * reach, rng.uniform(0.2, 0.8)))
        parts.append((centre + offset, radius * rng.uniform(0.45, 0.65), up * rng.uniform(0.8, 1.2), down * 0.8))
    return parts


def pad(b, centre, radius, leaves, shades, rng, up=2.2, down=1.0, lumps=4, subdivisions=3):
    """A flat, lumpy pad of foliage, like a pine's tuft of needles: a low dome
    with a few smaller lumps rising from its top at random, wrapped in one skin
    whose triangles match a canopy's."""
    parts = _pad_parts(Vector(centre), radius, up, down, lumps, rng)
    canopy(
        b, centre, parts, leaves, shades, rng,
        subdivisions=subdivisions, lumpiness=0.04, shape=(radius * 1.25, up * 1.3, down),
    )




def _lerp(a, b, t):
    return a + (b - a) * t


def grow_branches(trunk_points, rules, rng):
    """Grow branches up a trunk from rules, spiralling round it by the golden
    angle. Returns (points, radii, pad centre, pad radius) per branch, forks
    included; lengths, thicknesses and pads shrink from the bottom to the top.

    rules: count, from and to (heights of the lowest and highest branch),
    length and pad ((bottom, top) pairs), fork_chance, and optionally rise (a
    range, as a fraction of length), droop (the kink sags by this fraction of
    length instead of rising), wander (sideways kink, in shaku) and thickness
    ((bottom, top) radius at the trunk) and taper (lengths and pads follow
    t ** taper up the crown: above 1 they stay long higher up, then pull in
    quickly near the top, for a rounded crown rather than a cone) and lift
    (how far above a branch's tip, and a fork's, its pad is centred; keep the
    tip inside the pad, so a flat pad wants a small lift).
    """
    taper = rules.get("taper", 1.0)
    lift, fork_lift = rules.get("lift", (1.0, 0.8))
    rise_range = rules.get("rise", (0.2, 0.45))
    droop = rules.get("droop")
    wander = rules.get("wander", 1)
    thickness = rules.get("thickness", (0.55, 0.3))
    out = []
    for i in range(rules["count"]):
        t = i / (rules["count"] - 1)
        z = _lerp(rules["from"], rules["to"], t) + rng.uniform(-0.8, 0.8)
        base = point_at_height(trunk_points, z)
        angle = i * GOLDEN_ANGLE + rng.uniform(-0.3, 0.3)
        out_dir = Vector((math.cos(angle), math.sin(angle), 0))
        across = Vector((-out_dir.y, out_dir.x, 0))
        length = _lerp(*rules["length"], t**taper) * rng.uniform(0.8, 1.2)
        rise = length * rng.uniform(*rise_range)
        kink_z = rise * 0.4 if droop is None else -length * droop
        kink = base + out_dir * length * 0.5 + across * rng.uniform(-wander, wander) + Vector((0, 0, kink_z))
        end = base + out_dir * length + across * rng.uniform(-wander, wander) + Vector((0, 0, rise))
        thick = _lerp(*thickness, t)
        pad_radius = _lerp(*rules["pad"], t**taper) * rng.uniform(0.85, 1.15)
        out.append(([base, kink, end], [thick, thick * 0.65, 0.15], end + Vector((0, 0, lift)), pad_radius))
        if rng.random() < rules["fork_chance"]:
            turn = rng.choice((-1, 1)) * rng.uniform(0.6, 1.0)
            fork_dir = Vector((math.cos(angle + turn), math.sin(angle + turn), 0))
            fork_end = kink + fork_dir * length * 0.45 + Vector((0, 0, rise * 0.5))
            out.append(
                ([kink, fork_end], [thick * 0.45, 0.12], fork_end + Vector((0, 0, fork_lift)), pad_radius * 0.7)
            )
    return out


def conifer(b, spec, needles, shades, bark):
    """A conifer from a version spec: a straight trunk whose faces each pick a
    bark colour, branches grown by grow_branches each carrying a tuft, and a
    tuft on the tip. Tufts and the tip take their up and down from spec["tuft"]
    and spec["tip"]; bark is a list of swatches to pick from.

    With spec["clusters"] (tier, sectors), the tufts aren't separate: each
    tier and sector of the crown is wrapped in one skin, see clustered_crown."""
    rng = rng_for(spec["seed"])
    points, radii = sink(*densify(*spec["trunk"], step=3))
    # the buried segment keeps bark[0], drawing nothing from the random stream
    for segment in branch(b, points, radii, bark[0], upright=2)[1:]:
        for face in segment:
            b.paint([face], rng.choice(bark))

    tuft = spec["tuft"]
    tip = spec["tip"]
    top = Vector(spec["trunk"][0][-1]) + Vector((0, 0, 1))
    if "clusters" in spec:
        sprays = []
        for branch_points, branch_radii, pad_centre, pad_radius in grow_branches(
            spec["trunk"][0], spec["branches"], rng
        ):
            branch(b, branch_points, branch_radii, bark[0])
            sprays.append((pad_centre, pad_radius, tuft["up"], tuft["down"]))
        sprays.append((top, tip["radius"], tip["up"], tip["down"]))
        clustered_crown(b, sprays, needles, shades, rng, **spec["clusters"])
        return

    for branch_points, branch_radii, pad_centre, pad_radius in grow_branches(
        spec["trunk"][0], spec["branches"], rng
    ):
        branch(b, branch_points, branch_radii, bark[0])
        pad(
            b, pad_centre, pad_radius, needles, shades, rng, up=tuft["up"], down=tuft["down"],
            subdivisions=3 if pad_radius >= 4 else 2,  # keeps the triangles one size
        )
    pad(b, top, tip["radius"], needles, shades, rng, up=tip["up"], down=tip["down"], lumps=2, subdivisions=2)


def clustered_crown(b, sprays, leaves, shades, rng, tier, sectors, lumps=2):
    """Wrap sprays of foliage in a few skins rather than one each: the crown is
    cut into tiers tier shaku tall, and each tier into sectors round the trunk,
    and every piece is shrink-wrapped as one lumpy skin with even triangles.

    sprays are (centre, radius, up, down); each piece is wrapped by wrap_sprays."""
    bottom = min(c.z for c, *_ in sprays)
    pieces = {}
    for spray in sprays:
        c = spray[0]
        sector = int((math.atan2(c.y, c.x) % (2 * math.pi)) / (2 * math.pi) * sectors) % sectors
        pieces.setdefault((int((c.z - bottom) // tier), sector), []).append(spray)
    for key in sorted(pieces):
        wrap_sprays(b, pieces[key], leaves, shades, rng, lumps)


def wrap_sprays(b, group, leaves, shades, rng, lumps=2):
    """Wrap a group of sprays, each (centre, radius, up, down), as one lumpy skin
    with even triangles; a core lump at the group's middle fills it, so every
    ray from there meets foliage."""
    middle = sum((Vector(c) for c, *_ in group), Vector()) / len(group)
    parts = []
    for c, r, up, down in group:
        parts += _pad_parts(Vector(c), r, up, down, lumps, rng)
    reach = max((Vector((c.x - middle.x, c.y - middle.y, 0))).length + r * 1.2 for c, r, *_ in group)
    rise = max(c.z + up * 1.3 for c, _, up, _ in group) - middle.z
    sink = middle.z - min(c.z - down for c, _, _, down in group)
    parts.append((middle, max(1.5, reach * 0.45), rise * 0.6, sink * 0.6))  # the core
    shape = (reach, rise, sink)
    canopy(
        b, middle, parts, leaves, shades, rng,
        lumpiness=0.04, shape=shape, triangles=even_triangles(middle, parts, shape),
    )


def blade(b, origin, direction, length, width, leaf, shade, droop=0.25):
    """One long, pointed leaf: a thin closed shape, ridged along its midrib so it
    catches the light in two facets, lying flat-ish along direction with its tip
    drooping by droop of its length. Upper faces take leaf, lower take shade."""
    d = Vector(direction).normalized()
    side = d.cross(Vector((0, 0, 1)))
    side = side.normalized() if side.length > 1e-6 else Vector((1, 0, 0))
    normal = side.cross(d)
    o = Vector(origin)
    verts = [b.bm.verts.new(p) for p in (
        o,  # base
        o + d * length * 0.35 + side * width / 2,  # left
        o + d * length * 0.35 - side * width / 2,  # right
        o + d * length - Vector((0, 0, droop * length)),  # tip
        o + d * length * 0.4 + normal * width * 0.18,  # midrib, above
        o + d * length * 0.4 - normal * width * 0.08,  # midrib, below
    )]
    base, left, right, tip, above, below = verts
    for f in ((base, left, above), (base, above, right), (above, left, tip), (above, tip, right)):
        b.face(f, leaf)
    for f in ((base, below, left), (base, right, below), (below, tip, left), (below, right, tip)):
        b.face(f, shade)


def blade_cluster(b, origin, direction, count, length, width, leaves, shades, rng, spread=0.6, droop=(0.3, 0.8)):
    """A few blades fanning out from one point round direction, each dipping by
    a random angle in droop (radians) and turned within spread either side."""
    d = Vector(direction)
    heading = math.atan2(d.y, d.x)
    for i in range(count):
        turn = heading + (i / max(count - 1, 1) - 0.5) * 2 * spread + rng.uniform(-0.15, 0.15)
        dip = rng.uniform(*droop)
        way = Vector((math.cos(turn) * math.cos(dip), math.sin(turn) * math.cos(dip), -math.sin(dip)))
        blade(
            b, origin, way, length * rng.uniform(0.8, 1.15), width * rng.uniform(0.85, 1.1),
            rng.choice(leaves), rng.choice(shades),
        )


def frond(b, base, heading, length, width, rise, fall, leaves, shades, rng, segments=10, bend=2.0, notch=0.7):
    """A fern frond: a long leaf arching up from base and over, its edges
    zig-zagging to suggest rows of leaflets.

    It sets off at rise radians above level towards heading (radians round the
    vertical) and bends steadily until it points fall radians below level at the
    tip, the bend gathering towards the tip as the bend power rises (1 bends it
    evenly; 2 keeps it rising, then arches it over). width is its full width at
    the widest, a little below the middle; it
    narrows to a stalk at the base and a point at the tip. Built as a thin closed
    shape, ridged along its midrib like blade, so it shows from above and below:
    upper faces pick from leaves, lower from shades. notch is how far out the
    notches between leaflets sit, as a fraction of the width there: nearer 1,
    shallower teeth.
    """
    along_each = length / segments
    point = Vector(base)
    stations = []  # (centre, direction) at the start of each segment, then the tip
    for i in range(segments + 1):
        angle = rise - (rise + fall) * (i / segments) ** bend
        way = Vector((math.cos(heading) * math.cos(angle), math.sin(heading) * math.cos(angle), math.sin(angle)))
        stations.append((point.copy(), way))
        point += way * along_each
    side = Vector((-math.sin(heading), math.cos(heading), 0))

    def half_width(t):
        # a stalk for the first tenth, widest at 0.4, closing to the tip
        return 0 if t < 0.1 else width / 2 * math.sin(math.pi * min(1, (t - 0.1) / 0.9) ** 0.8)

    def verts_at(i):
        centre, way = stations[i]
        normal = side.cross(way).normalized()
        w = max(half_width(i / segments), 0.015) * notch  # the notches between leaflets
        ridge = max(half_width(i / segments), 0.02) * 0.12
        return [b.bm.verts.new(p) for p in (
            centre + normal * ridge, centre - side * w, centre - normal * ridge * 0.5, centre + side * w,
        )]  # above the midrib, left notch, below the midrib, right notch

    rings = [verts_at(i) for i in range(segments)]
    tip = b.bm.verts.new(stations[-1][0])
    start = rings[0]
    b.face([start[0], start[1], start[2], start[3]], rng.choice(shades))  # the cut stalk end
    for i in range(segments):
        above, left, below, right = rings[i]
        last = i == segments - 1
        nxt = [tip] * 4 if last else rings[i + 1]
        n_above, n_left, n_below, n_right = nxt
        centre = stations[i][0].lerp(stations[i + 1][0], 0.5)
        w = half_width((i + 0.5) / segments)
        if last or w < 0.02:
            # no leaflet here: close the segment straight across
            for f in ((above, left, n_left, n_above), (above, n_above, n_right, right)):
                b.face([v for k, v in enumerate(f) if v not in f[:k]], rng.choice(leaves))
            for f in ((below, n_below, n_left, left), (below, right, n_right, n_below)):
                b.face([v for k, v in enumerate(f) if v not in f[:k]], rng.choice(shades))
            continue
        # a leaflet's point sticking out to each side between the notches
        out_left = b.bm.verts.new(centre - side * w)
        out_right = b.bm.verts.new(centre + side * w)
        b.face((above, left, out_left), rng.choice(leaves))
        b.face((above, out_left, n_above), rng.choice(leaves))
        b.face((n_above, out_left, n_left), rng.choice(leaves))
        b.face((above, out_right, right), rng.choice(leaves))
        b.face((above, n_above, out_right), rng.choice(leaves))
        b.face((n_above, n_right, out_right), rng.choice(leaves))
        b.face((below, out_left, left), rng.choice(shades))
        b.face((below, n_below, out_left), rng.choice(shades))
        b.face((n_below, n_left, out_left), rng.choice(shades))
        b.face((below, right, out_right), rng.choice(shades))
        b.face((below, out_right, n_below), rng.choice(shades))
        b.face((n_below, out_right, n_right), rng.choice(shades))


def rng_for(seed):
    return random.Random(seed)

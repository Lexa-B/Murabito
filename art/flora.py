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


def branch(b, points, radii, colour):
    """Loft a closed tube through points (3D), with a radius per point."""
    points = [Vector(p) for p in points]
    rings = []
    for i, (centre, r) in enumerate(zip(points, radii, strict=True)):
        prev = points[max(i - 1, 0)]
        nxt = points[min(i + 1, len(points) - 1)]
        tangent = (nxt - prev).normalized()
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
    for a, c in zip(rings, rings[1:]):
        for k in range(n):
            b.face((a[k], a[(k + 1) % n], c[(k + 1) % n], c[k]), colour)
    b.face(list(reversed(rings[0])), colour)
    b.face(rings[-1], colour)


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


def rng_for(seed):
    return random.Random(seed)

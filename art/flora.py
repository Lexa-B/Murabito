"""Building plants: branches lofted along 3D polylines, and faceted foliage clumps.

Plants aren't rigged, so unlike the animals a plant is several closed shells
that overlap (trunk, branches, clumps) rather than one connected mesh. They use
loft.Builder for palette colouring and finishing.

Coordinates are Blender's, in shaku: +Z up, the plant's base at the origin.
"""

import math
import random

import bmesh
from mathutils import Matrix, Vector

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


def clump(b, centre, radius, height, colour, shade, rng, lumpiness=0.12):
    """A faceted foliage blob: a flattened icosphere, its vertices pushed in and
    out a little so no two clumps look alike. Faces pointing down take shade."""
    matrix = Matrix.Translation(centre) @ Matrix.Diagonal((radius, radius, height, 1))
    made = bmesh.ops.create_icosphere(b.bm, subdivisions=2, radius=1.0, matrix=matrix)
    verts = made["verts"]
    centre = Vector(centre)
    for v in verts:
        v.co = centre + (v.co - centre) * (1 + rng.uniform(-lumpiness, lumpiness))
    faces = {f for v in verts for f in v.link_faces}
    for f in faces:
        f.normal_update()
        b.paint([f], shade if f.normal.z < -0.35 else colour)


def rng_for(seed):
    return random.Random(seed)

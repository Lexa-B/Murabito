"""The cat: a slightly chibi, low-poly mike (三毛) calico with a long tail, standing.

    blender -b --python art/cat.py -- --out assets/models/cat.glb [--renders <dir>]

White with orange and black patches. The patches are whole faces, painted over
the white after the mesh is built, and deliberately lopsided, as calico is.

Built with loft.py: one connected, rig-ready mesh, with a ring of vertices at
every joint and five rings along the tail. About 0.8 shaku (~24 cm) to the
shoulder once scaled and 1.3 shaku long without the tail, feet on z = 0.
"""

import sys
from pathlib import Path

from mathutils import Vector

sys.path.insert(0, str(Path(__file__).resolve().parent))
import loft  # noqa: E402
from loft import LOWER_LEFT, LOWER_RIGHT, UPPER_LEFT, UPPER_RIGHT  # noqa: E402

WHITE, ORANGE, BLACK = "cream", "fox_orange", "soot"

# The spine, tail tip to nose: (y, z, half-width, half-height, colour, underside
# colour). See loft.Builder.spine.
TAIL_TIP = Vector((0, -1.51, 1.45))
SPINE = [
    # tail: long, curving up behind, with a rounded end
    (-1.48, 1.38, 0.07, 0.07, BLACK, BLACK),
    (-1.38, 1.20, 0.075, 0.075, BLACK, BLACK),
    (-1.24, 1.00, 0.08, 0.08, ORANGE, ORANGE),
    (-1.06, 0.86, 0.085, 0.085, ORANGE, ORANGE),
    (-0.90, 0.80, 0.10, 0.10, WHITE, WHITE),  # tail root
    # body: full and even
    (-0.80, 0.76, 0.22, 0.24, WHITE, WHITE),  # rump
    (-0.58, 0.75, 0.28, 0.29, WHITE, WHITE),  # hip
    (-0.30, 0.74, 0.28, 0.28, WHITE, WHITE),
    (0.00, 0.75, 0.28, 0.29, WHITE, WHITE),  # chest
    (0.20, 0.79, 0.26, 0.28, WHITE, WHITE),  # shoulder
    # neck
    (0.34, 0.97, 0.21, 0.21, WHITE, WHITE),
    # head: round, about 1.3x natural, short flat muzzle
    (0.38, 1.14, 0.28, 0.25, WHITE, WHITE),  # back of the skull
    (0.53, 1.20, 0.33, 0.27, WHITE, WHITE),  # cheeks, the widest point
    (0.70, 1.16, 0.25, 0.20, WHITE, WHITE),
    (0.82, 1.10, 0.13, 0.11, WHITE, WHITE),  # muzzle
]
NOSE_TIP = Vector((0, 0.87, 1.09))

# Which spine segment (index of its tail-side ring) and face each limb grows from.
HIND_SEGMENT = 5  # rump -> hip
FRONT_SEGMENT = 8  # chest -> shoulder
EAR_SEGMENT = 11  # back of skull -> cheeks

# Limbs, for the right side: rings of (x, y, z, half-width, half-depth, colour).
# See loft.Builder.limb.
FRONT_LEG = [
    (0.15, 0.10, 0.42, 0.085, 0.09, WHITE),  # shoulder
    (0.14, 0.11, 0.26, 0.065, 0.07, WHITE),  # elbow
    (0.14, 0.12, 0.10, 0.055, 0.06, WHITE),  # wrist
    (0.14, 0.15, 0.05, 0.07, 0.09, WHITE),  # paw
    (0.14, 0.15, 0.00, 0.07, 0.09, WHITE),  # sole
]
HIND_LEG = [
    (0.16, -0.68, 0.42, 0.10, 0.12, WHITE),  # hip
    (0.15, -0.64, 0.26, 0.07, 0.08, WHITE),  # knee
    (0.14, -0.69, 0.11, 0.055, 0.06, WHITE),  # hock
    (0.14, -0.66, 0.05, 0.07, 0.09, WHITE),  # paw
    (0.14, -0.66, 0.00, 0.07, 0.09, WHITE),  # sole
]
EAR = [  # triangular, wide at the base
    (0.17, 0.46, 1.48, 0.11, 0.055, WHITE),
    (0.20, 0.47, 1.60, 0.065, 0.035, WHITE),
]
EAR_TIP = Vector((0.22, 0.47, 1.72))

# Calico patches, painted over the white: (spine segment, faces, colour). Faces
# are numbered as in loft: 0 upper right, 1 top, 2 upper left, 3 left, ... 7 right.
# Each patch is staggered from ring to ring so it doesn't read as a stripe.
PATCHES = [
    (5, (1, 2, 3), ORANGE),  # rump, spilling down the left
    (6, (2, 3), ORANGE),
    (6, (0,), BLACK),  # a black patch over the back, reaching the right flank
    (7, (0, 1, 7), BLACK),
    (8, (1, 2), BLACK),
    (11, (1, 2, 3), ORANGE),  # an orange cap over the left of the head
    (12, (1, 2, 3), ORANGE),
    (12, (0,), BLACK),  # and a black patch over the right eye
    (13, (0, 7), BLACK),
]
EAR_COLOURS = {1: BLACK, -1: ORANGE}  # right ear black, left ear orange


SCALE = 0.8  # built in its own numbers, then sized to a real cat's ~24 cm at the shoulder

def build_cat():
    b = loft.Builder()
    segments = b.spine(TAIL_TIP, SPINE, NOSE_TIP)
    for segment, faces, colour in PATCHES:
        b.paint([segments[segment][k] for k in faces], colour)

    # Collect every base face before growing anything: growing removes faces.
    limbs = []
    for mirror, lower, upper in ((1, LOWER_RIGHT, UPPER_RIGHT), (-1, LOWER_LEFT, UPPER_LEFT)):
        limbs.append((segments[FRONT_SEGMENT][lower], FRONT_LEG, mirror, None))
        limbs.append((segments[HIND_SEGMENT][lower], HIND_LEG, mirror, None))
        limbs.append((segments[EAR_SEGMENT][upper], EAR, mirror, EAR_TIP))
    for face, rings, mirror, tip in limbs:
        rows = b.limb(face, rings, mirror, tip)
        if rings is EAR:
            b.paint([f for row in rows for f in row], EAR_COLOURS[mirror])
    return b.finish("Cat", shaded=True, scale=SCALE)


if __name__ == "__main__":
    loft.run(build_cat, "cat", target=(0.00, -0.28, 0.64), extent=2.08)

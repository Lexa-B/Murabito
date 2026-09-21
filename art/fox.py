"""The fox: a slightly chibi, low-poly red fox in a neutral standing pose.

    blender -b --python art/fox.py -- --out assets/models/fox.glb [--renders <dir>]

Built with loft.py: one connected, rig-ready mesh, with a ring of vertices at
every joint (shoulder, elbow, wrist, hip, knee, hock, neck, tail). Shoulder
height is about 1.35 shaku (~41 cm), feet on z = 0.
"""

import sys
from pathlib import Path

from mathutils import Vector

sys.path.insert(0, str(Path(__file__).resolve().parent))
import loft  # noqa: E402
from loft import LOWER_LEFT, LOWER_RIGHT, UPPER_LEFT, UPPER_RIGHT  # noqa: E402

ORANGE, CREAM, SOOT = "fox_orange", "cream", "soot"

# The spine, tail tip to nose: (y, z, half-width, half-height, colour, underside
# colour). See loft.Builder.spine.
TAIL_TIP = Vector((0, -2.25, 1.25))
SPINE = [
    # tail: big and bushy, low, curling up at the tip
    (-2.10, 1.10, 0.16, 0.15, CREAM, CREAM),
    (-1.85, 0.95, 0.30, 0.28, CREAM, CREAM),
    (-1.50, 0.92, 0.34, 0.31, ORANGE, ORANGE),
    (-1.18, 1.02, 0.26, 0.24, ORANGE, ORANGE),
    (-0.95, 1.13, 0.17, 0.17, ORANGE, ORANGE),  # tail root
    # body: short and even, no waist
    (-0.78, 1.08, 0.28, 0.29, ORANGE, ORANGE),  # rump
    (-0.55, 1.07, 0.31, 0.32, ORANGE, ORANGE),  # hip
    (-0.20, 1.05, 0.31, 0.31, ORANGE, ORANGE),
    (0.10, 1.07, 0.32, 0.33, ORANGE, CREAM),  # chest
    (0.32, 1.15, 0.30, 0.32, ORANGE, CREAM),  # shoulder
    # neck: short and thick
    (0.50, 1.38, 0.27, 0.27, ORANGE, CREAM),
    # head: about 1.3x natural, big cheeks, short sharp snout
    (0.58, 1.62, 0.32, 0.30, ORANGE, CREAM),  # back of the skull
    (0.80, 1.72, 0.42, 0.34, ORANGE, CREAM),  # cheeks, the widest point
    (1.02, 1.68, 0.30, 0.24, ORANGE, CREAM),
    (1.20, 1.60, 0.14, 0.11, ORANGE, CREAM),  # snout
    (1.32, 1.57, 0.07, 0.06, SOOT, SOOT),
]
NOSE_TIP = Vector((0, 1.39, 1.57))

# Which spine segment (index of its tail-side ring) and face each limb grows from.
HIND_SEGMENT = 5  # rump -> hip
FRONT_SEGMENT = 8  # chest -> shoulder
EAR_SEGMENT = 11  # back of skull -> cheeks

# Limbs, for the right side: rings of (x, y, z, half-width, half-depth, colour).
# See loft.Builder.limb.
FRONT_LEG = [
    (0.18, 0.22, 0.72, 0.10, 0.11, ORANGE),  # shoulder
    (0.16, 0.24, 0.45, 0.075, 0.08, ORANGE),  # elbow
    (0.15, 0.26, 0.14, 0.06, 0.065, SOOT),  # wrist
    (0.15, 0.30, 0.07, 0.075, 0.105, SOOT),  # paw
    (0.15, 0.30, 0.00, 0.075, 0.105, SOOT),  # sole
]
HIND_LEG = [
    (0.19, -0.66, 0.72, 0.12, 0.15, ORANGE),  # hip
    (0.17, -0.61, 0.45, 0.08, 0.09, ORANGE),  # knee
    (0.15, -0.67, 0.20, 0.055, 0.065, SOOT),  # hock
    (0.15, -0.63, 0.07, 0.075, 0.105, SOOT),  # paw
    (0.15, -0.63, 0.00, 0.075, 0.105, SOOT),  # sole
]
EAR = [
    (0.22, 0.70, 2.12, 0.15, 0.06, ORANGE),
    (0.26, 0.72, 2.32, 0.08, 0.035, SOOT),
]
EAR_TIP = Vector((0.29, 0.73, 2.50))


def build_fox():
    b = loft.Builder()
    segments = b.spine(TAIL_TIP, SPINE, NOSE_TIP)

    # Collect every base face before growing anything: growing removes faces.
    limbs = []
    for mirror, lower, upper in ((1, LOWER_RIGHT, UPPER_RIGHT), (-1, LOWER_LEFT, UPPER_LEFT)):
        limbs.append((segments[FRONT_SEGMENT][lower], FRONT_LEG, mirror, None))
        limbs.append((segments[HIND_SEGMENT][lower], HIND_LEG, mirror, None))
        limbs.append((segments[EAR_SEGMENT][upper], EAR, mirror, EAR_TIP))
    for face, rings, mirror, tip in limbs:
        b.limb(face, rings, mirror, tip)
    return b.finish("Fox", shaded=True)


if __name__ == "__main__":
    loft.run(build_fox, "fox", target=(0, -0.45, 1.15), extent=3.8)

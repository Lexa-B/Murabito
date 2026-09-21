"""The hare: a slightly chibi, low-poly Japanese hare (野兎) sitting in a loaf.

    blender -b --python art/hare.py -- --out assets/models/hare.glb [--renders <dir>]

A Japanese hare rather than a rabbit: European rabbits reached Japan long after
ca. 1550. Grey-brown with a cream belly and tail and black ear tips.

Built with loft.py: one connected, rig-ready mesh, with rings along the ears so
they can flop and swivel. About 1.7 shaku (~52 cm) long, 0.9 shaku to the top
of the haunches and 1.9 shaku to the ear tips, feet on z = 0.
"""

import sys
from pathlib import Path

from mathutils import Vector

sys.path.insert(0, str(Path(__file__).resolve().parent))
import loft  # noqa: E402
from loft import LOWER_LEFT, LOWER_RIGHT, UPPER_LEFT, UPPER_RIGHT  # noqa: E402

BROWN, CREAM, SOOT = "hare_brown", "cream", "soot"

# The spine, tail tip to nose: (y, z, half-width, half-height, colour, underside
# colour). See loft.Builder.spine.
TAIL_TIP = Vector((0, -0.99, 0.63))
SPINE = [
    # tail: a small cream puff
    (-0.93, 0.62, 0.11, 0.11, CREAM, CREAM),
    (-0.86, 0.60, 0.08, 0.08, CREAM, CREAM),
    # body: round, full haunches tapering to the shoulders
    (-0.80, 0.52, 0.26, 0.30, BROWN, CREAM),  # rump
    (-0.62, 0.52, 0.36, 0.38, BROWN, CREAM),  # haunches, the widest point
    (-0.35, 0.55, 0.36, 0.37, BROWN, CREAM),
    (-0.08, 0.60, 0.32, 0.32, BROWN, CREAM),  # chest
    (0.10, 0.68, 0.27, 0.28, BROWN, CREAM),  # shoulder
    # neck
    (0.22, 0.82, 0.22, 0.22, BROWN, CREAM),
    # head: round, about 1.3x natural, blunt muzzle
    (0.28, 0.98, 0.25, 0.24, BROWN, CREAM),  # back of the skull
    (0.42, 1.04, 0.29, 0.26, BROWN, CREAM),  # cheeks
    (0.58, 1.00, 0.22, 0.20, BROWN, CREAM),
    (0.70, 0.93, 0.13, 0.12, BROWN, CREAM),  # muzzle
    (0.76, 0.90, 0.08, 0.07, CREAM, CREAM),
]
NOSE_TIP = Vector((0, 0.79, 0.89))

# Which spine segment (index of its tail-side ring) and face each limb grows from.
HIND_SEGMENT = 3  # haunches -> behind the chest
FRONT_SEGMENT = 5  # chest -> shoulder
EAR_SEGMENT = 8  # back of skull -> cheeks

# Limbs, for the right side: rings of (x, y, z, half-width, half-depth, colour).
# See loft.Builder.limb.
FRONT_LEG = [
    (0.14, 0.05, 0.26, 0.07, 0.07, BROWN),  # elbow
    (0.13, 0.08, 0.10, 0.055, 0.055, BROWN),  # wrist
    (0.13, 0.12, 0.05, 0.06, 0.08, CREAM),  # paw
    (0.13, 0.12, 0.00, 0.06, 0.08, CREAM),  # sole
]
HIND_LEG = [  # folded under the haunches: only the long foot shows
    (0.24, -0.50, 0.14, 0.09, 0.10, BROWN),  # hock
    (0.24, -0.52, 0.06, 0.08, 0.22, CREAM),  # foot
    (0.24, -0.52, 0.00, 0.08, 0.22, CREAM),  # sole
]
EAR = [  # long and upright, leaning back a little
    (0.12, 0.36, 1.30, 0.08, 0.04, BROWN),
    (0.14, 0.34, 1.55, 0.09, 0.035, BROWN),
    (0.15, 0.32, 1.72, 0.07, 0.03, BROWN),
    (0.16, 0.31, 1.82, 0.045, 0.02, SOOT),
]
EAR_TIP = Vector((0.165, 0.30, 1.90))


def build_hare():
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
    return b.finish("Hare")


if __name__ == "__main__":
    loft.run(build_hare, "hare", target=(0, -0.1, 0.85), extent=2.2)

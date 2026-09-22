"""The rat: a slightly chibi, low-poly black rat (クマネズミ) standing on all fours.

    blender -b --python art/rat.py -- --out assets/models/rat.glb [--renders <dir>]

The black rat is long established in Japan and the classic pest of rice stores
and farmhouses. Dark grey-brown with a cream belly, and pink ears, feet and tail.

Built with loft.py: one connected, rig-ready mesh, with a ring of vertices at
every joint and five rings along the tail. About 0.63 shaku (~19 cm) nose
to rump once scaled, a tail nearly as long again, and 0.23 shaku to the top of the back, feet
on z = 0.
"""

import sys
from pathlib import Path

from mathutils import Vector

sys.path.insert(0, str(Path(__file__).resolve().parent))
import loft  # noqa: E402
from loft import LOWER_LEFT, LOWER_RIGHT, UPPER_LEFT, UPPER_RIGHT  # noqa: E402

GREY, CREAM, PINK = "rat_grey", "cream", "flesh_pink"

# The spine, tail tip to nose: (y, z, half-width, half-height, colour, underside
# colour). See loft.Builder.spine.
TAIL_TIP = Vector((0, -1.02, 0.03))
SPINE = [
    # tail: long and thin, trailing to the ground
    (-0.95, 0.03, 0.012, 0.012, PINK, PINK),
    (-0.80, 0.04, 0.016, 0.016, PINK, PINK),
    (-0.62, 0.07, 0.02, 0.02, PINK, PINK),
    (-0.46, 0.12, 0.025, 0.025, PINK, PINK),
    (-0.34, 0.17, 0.03, 0.03, GREY, GREY),  # tail root
    # body: round and full, the rump rounding off
    (-0.29, 0.19, 0.08, 0.08, GREY, GREY),
    (-0.24, 0.20, 0.12, 0.12, GREY, GREY),
    (-0.15, 0.20, 0.14, 0.13, GREY, CREAM),  # haunches
    (-0.04, 0.20, 0.13, 0.12, GREY, CREAM),
    (0.06, 0.21, 0.11, 0.11, GREY, CREAM),  # chest
    (0.13, 0.23, 0.09, 0.09, GREY, CREAM),  # shoulder
    # head: about 1.3x natural, tapering to a pointed snout
    (0.18, 0.28, 0.115, 0.11, GREY, CREAM),  # back of the skull
    (0.25, 0.30, 0.13, 0.115, GREY, CREAM),  # cheeks, rounder than life
    (0.33, 0.28, 0.08, 0.075, GREY, CREAM),
    (0.39, 0.25, 0.04, 0.04, GREY, GREY),  # snout
]
NOSE_TIP = Vector((0, 0.43, 0.24))

# Which spine segment (index of its tail-side ring) and face each limb grows from.
HIND_SEGMENT = 6  # rump -> haunches
FRONT_SEGMENT = 9  # chest -> shoulder
EAR_SEGMENT = 11  # back of skull -> cheeks

# Limbs, for the right side: rings of (x, y, z, half-width, half-depth, colour).
# See loft.Builder.limb.
FRONT_LEG = [  # short: the body sits low
    (0.06, 0.09, 0.07, 0.025, 0.025, GREY),  # wrist
    (0.06, 0.11, 0.015, 0.025, 0.035, PINK),  # paw
    (0.06, 0.11, 0.00, 0.025, 0.035, PINK),  # sole
]
HIND_LEG = [
    (0.08, -0.19, 0.05, 0.035, 0.04, GREY),  # hock
    (0.08, -0.16, 0.015, 0.03, 0.06, PINK),  # long foot
    (0.08, -0.16, 0.00, 0.03, 0.06, PINK),  # sole
]
EAR = [  # big and round: wide in the middle, thin front to back
    (0.10, 0.21, 0.405, 0.055, 0.022, GREY),  # the base, where the ear meets the head
    (0.10, 0.21, 0.42, 0.06, 0.02, PINK),
    (0.12, 0.21, 0.48, 0.065, 0.018, PINK),
    (0.13, 0.21, 0.53, 0.04, 0.015, PINK),
]
EAR_TIP = Vector((0.135, 0.21, 0.55))


SCALE = 0.7  # built in its own numbers, then sized to a real black rat's ~19 cm, nose to rump

def build_rat():
    b = loft.Builder()
    segments = b.spine(TAIL_TIP, SPINE, NOSE_TIP)

    # Collect every base face before growing anything: growing removes faces.
    limbs = []
    for mirror, lower, upper in ((1, LOWER_RIGHT, UPPER_RIGHT), (-1, LOWER_LEFT, UPPER_LEFT)):
        limbs.append((segments[FRONT_SEGMENT][lower], FRONT_LEG, mirror, None))
        limbs.append((segments[HIND_SEGMENT][lower], HIND_LEG, mirror, None))
        limbs.append((segments[EAR_SEGMENT][upper], EAR, mirror, EAR_TIP))
    for face, rings, mirror, tip in limbs:
        rows = b.limb(face, rings, mirror, tip)
        if rings is EAR:
            # The grey base keeps the pink off the head, but the inside of the ear
            # faces forward and stays pink all the way down.
            front = max(rows[0], key=lambda f: f.calc_center_median().y)
            b.paint([front], PINK)
    return b.finish("Rat", shaded=True, scale=SCALE)


if __name__ == "__main__":
    loft.run(build_rat, "rat", target=(0.00, -0.21, 0.15), extent=1.05)

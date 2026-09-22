"""The hare: a slightly chibi, low-poly Japanese hare (野兎) sitting in a loaf.

    blender -b --python art/entity_models/living/animals/hare.py -- --out assets/entity_models/living/animals/hare.glb [--renders <dir>]

A Japanese hare rather than a rabbit: European rabbits reached Japan long after
ca. 1550. Grey-brown with a cream belly and tail and black ear tips.

Built with loft.py: one connected, rig-ready mesh, with rings along the ears so
they can flop and swivel. About 1.7 shaku (~52 cm) long, 0.9 shaku to the top
of the haunches and 1.8 shaku to the ear tips, feet on z = 0.
"""

import sys
from pathlib import Path

from mathutils import Vector

sys.path.insert(0, str(next(p for p in Path(__file__).resolve().parents if (p / "loft.py").exists())))
import loft  # noqa: E402
from loft import LOWER_LEFT, LOWER_RIGHT, UPPER_LEFT, UPPER_RIGHT  # noqa: E402

BROWN, CREAM, SOOT = "hare_brown", "cream", "soot"

# The spine, tail tip to nose: (y, z, half-width, half-height, colour, underside
# colour). See loft.Builder.spine.
TAIL_TIP = Vector((0, -1.05, 0.69))
SPINE = [
    # tail: a small round cream puff
    (-0.98, 0.67, 0.12, 0.12, CREAM, CREAM),
    (-0.91, 0.63, 0.09, 0.09, CREAM, CREAM),
    # body: a rounded rump whose top curves gently down from the haunches (the
    # rings rise as they shrink), then the back sloping down to low shoulders
    (-0.88, 0.60, 0.17, 0.16, BROWN, BROWN),  # rump, rounding off
    (-0.81, 0.58, 0.27, 0.25, BROWN, BROWN),
    (-0.72, 0.55, 0.33, 0.33, BROWN, BROWN),
    (-0.62, 0.53, 0.35, 0.36, BROWN, BROWN),  # haunches: in line with their neighbours, or it shows as a ridge
    (-0.35, 0.52, 0.35, 0.35, BROWN, CREAM),
    (-0.08, 0.50, 0.30, 0.30, BROWN, CREAM),  # chest
    (0.10, 0.55, 0.26, 0.26, BROWN, CREAM),  # shoulder
    # neck
    (0.22, 0.70, 0.21, 0.21, BROWN, CREAM),
    # head: round, about 1.3x natural, blunt muzzle
    (0.28, 0.86, 0.25, 0.24, BROWN, CREAM),  # back of the skull
    (0.42, 0.92, 0.29, 0.26, BROWN, CREAM),  # cheeks
    (0.58, 0.88, 0.22, 0.20, BROWN, CREAM),
    (0.70, 0.81, 0.13, 0.12, BROWN, CREAM),  # muzzle
    (0.76, 0.78, 0.08, 0.07, CREAM, CREAM),
]
NOSE_TIP = Vector((0, 0.79, 0.77))

# Which spine segment (index of its tail-side ring) and face each limb grows from.
HIND_SEGMENT = 5  # haunches -> behind the chest
FRONT_SEGMENT = 7  # chest -> shoulder
EAR_SEGMENT = 10  # back of skull -> cheeks

# Limbs, for the right side: rings of (x, y, z, half-width, half-depth, colour).
# See loft.Builder.limb.
FRONT_LEG = [  # short and straight
    (0.14, 0.05, 0.18, 0.07, 0.07, BROWN),  # elbow
    (0.13, 0.08, 0.08, 0.055, 0.055, BROWN),  # wrist
    (0.13, 0.12, 0.04, 0.06, 0.08, CREAM),  # paw
    (0.13, 0.12, 0.00, 0.06, 0.08, CREAM),  # sole
]
HIND_LEG = [  # folded under the haunches: only the long foot shows
    (0.25, -0.52, 0.10, 0.10, 0.16, BROWN),  # hock
    (0.25, -0.46, 0.05, 0.09, 0.26, CREAM),  # foot
    (0.25, -0.46, 0.00, 0.09, 0.26, CREAM),  # sole
]
EAR = [  # long and upright, leaning back a little
    (0.12, 0.36, 1.18, 0.08, 0.05, BROWN),
    (0.14, 0.34, 1.43, 0.09, 0.05, BROWN),
    (0.15, 0.32, 1.60, 0.07, 0.04, BROWN),
    (0.16, 0.31, 1.70, 0.045, 0.03, SOOT),
]
EAR_TIP = Vector((0.165, 0.30, 1.78))


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
    return b.finish("Hare", shaded=True)


if __name__ == "__main__":
    loft.run(build_hare, "hare", target=(0, -0.1, 0.8), extent=2.1)

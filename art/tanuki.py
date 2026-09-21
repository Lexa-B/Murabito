"""The tanuki: a slightly chibi, low-poly Japanese raccoon dog (ホンドタヌキ), standing.

    blender -b --python art/tanuki.py -- --out assets/models/tanuki.glb [--renders <dir>]

The real animal, common round every village. Round, stocky and low-slung on short
legs, with a bushy drooping tail. Grizzled grey-brown, with a dark band over the
shoulders running down into dark legs and a dark chest; a dark bandit mask
across the eyes and cheeks, a pale forehead and muzzle, and small rounded,
dark-rimmed ears.

Built with loft.py: one connected, rig-ready mesh, with a ring of vertices at
every joint. About 1.0 shaku (~30 cm) to the shoulder and 1.6 shaku from nose to
rump, feet on z = 0.
"""

import sys
from pathlib import Path

from mathutils import Vector

sys.path.insert(0, str(Path(__file__).resolve().parent))
import loft  # noqa: E402
from loft import LOWER_LEFT, LOWER_RIGHT, UPPER_LEFT, UPPER_RIGHT  # noqa: E402

GREY, DARK, PALE, SOOT = "tanuki_grey", "tanuki_dark", "tanuki_pale", "soot"

# The spine, tail tip to nose: (y, z, half-width, half-height, colour, underside
# colour). See loft.Builder.spine.
TAIL_TIP = Vector((0, -1.00, 0.40))
SPINE = [
    # tail: bushy and drooping, dark at the tip
    (-0.94, 0.46, 0.10, 0.10, DARK, DARK),
    (-0.84, 0.56, 0.14, 0.14, GREY, GREY),
    (-0.72, 0.66, 0.15, 0.15, GREY, GREY),
    (-0.62, 0.72, 0.11, 0.11, GREY, GREY),  # tail root
    # body: short, round and low, a rounded rump, a deep chest
    (-0.56, 0.66, 0.25, 0.24, GREY, GREY),
    (-0.46, 0.63, 0.34, 0.31, GREY, GREY),  # the rump; the hind legs grow from the segment in front
    (-0.14, 0.62, 0.38, 0.34, GREY, GREY),  # a round belly
    (0.12, 0.63, 0.37, 0.33, GREY, DARK),  # a dark chest; the front legs grow from the segment in front
    (0.34, 0.68, 0.30, 0.30, GREY, DARK),  # shoulders
    (0.46, 0.80, 0.22, 0.23, GREY, DARK),  # a short neck
    # head: round and fluffy-cheeked, about 1.3x natural, with a short muzzle
    (0.52, 0.94, 0.28, 0.26, GREY, GREY),  # back of the head
    (0.66, 0.97, 0.31, 0.25, GREY, PALE),  # fluffy cheeks, the widest point
    (0.80, 0.90, 0.17, 0.15, PALE, PALE),
    (0.90, 0.85, 0.10, 0.09, PALE, PALE),  # muzzle
    (0.96, 0.83, 0.06, 0.055, SOOT, SOOT),  # nose
]
NOSE_TIP = Vector((0, 1.00, 0.82))
RUMP_RINGS = (4, 5)  # stand upright, so the rump rounds off under the tail instead of pointing out

# Which spine segment (index of its tail-side ring) and face each limb grows from.
# Each leg's first ring fits inside its base face, front, back and sides.
HIND_SEGMENT = 5  # rump -> belly
FRONT_SEGMENT = 7  # chest -> shoulders
EAR_SEGMENT = 10  # back of the head -> cheeks

# Limbs, for the right side: rings of (x, y, z, half-width, half-depth, colour).
# See loft.Builder.limb. Short and dark.
FRONT_LEG = [
    (0.14, 0.23, 0.27, 0.09, 0.10, DARK),  # shoulder
    (0.13, 0.24, 0.15, 0.065, 0.075, DARK),  # elbow
    (0.13, 0.27, 0.04, 0.075, 0.10, DARK),  # paw
    (0.13, 0.27, 0.00, 0.075, 0.10, DARK),  # sole
]
HIND_LEG = [
    (0.16, -0.29, 0.27, 0.10, 0.14, GREY),  # thigh
    (0.15, -0.29, 0.16, 0.07, 0.08, DARK),  # knee
    (0.15, -0.25, 0.04, 0.075, 0.10, DARK),  # paw
    (0.15, -0.25, 0.00, 0.075, 0.10, DARK),  # sole
]
EAR = [  # small and rounded, rimmed dark
    (0.19, 0.54, 1.23, 0.09, 0.05, GREY),
    (0.20, 0.545, 1.30, 0.065, 0.035, DARK),
]
EAR_TIP = Vector((0.21, 0.55, 1.34))

# (spine segment, faces, colour); faces are numbered as in loft: 0 upper right,
# 1 top, 2 upper left, 3 left, 4 lower left, 6 lower right, 7 right.
MARKINGS = [
    (8, (0, 1, 2, 3, 7), DARK),  # the dark band over the shoulders, down into the legs
    (11, (0, 2, 3, 7), DARK),  # the bandit mask across the eyes and cheeks
    (11, (1,), PALE),  # a pale forehead above it
    (10, (3, 7), DARK),
]


def build_tanuki():
    b = loft.Builder()
    segments = b.spine(TAIL_TIP, SPINE, NOSE_TIP, upright=RUMP_RINGS)
    for segment, faces, colour in MARKINGS:
        b.paint([segments[segment][k] for k in faces], colour)

    # Collect every base face before growing anything: growing removes faces.
    limbs = []
    for mirror, lower, upper in ((1, LOWER_RIGHT, UPPER_RIGHT), (-1, LOWER_LEFT, UPPER_LEFT)):
        limbs.append((segments[FRONT_SEGMENT][lower], FRONT_LEG, mirror, None))
        limbs.append((segments[HIND_SEGMENT][lower], HIND_LEG, mirror, None))
        limbs.append((segments[EAR_SEGMENT][upper], EAR, mirror, EAR_TIP))
    for face, rings, mirror, tip in limbs:
        b.limb(face, rings, mirror, tip)
    return b.finish("Tanuki", shaded=True)


if __name__ == "__main__":
    loft.run(build_tanuki, "tanuki", target=(0, 0.0, 0.62), extent=2.1)

"""The horse: a slightly chibi, low-poly Kiso horse (木曽馬), standing, unsaddled.

    blender -b --python art/horse.py -- --out assets/models/horse.glb [--renders <dir>]

One of Japan's native breeds, and the kind of horse ridden and worked in the
Sengoku period: small and stocky, about 130 cm at the withers, not a tall modern
breed, with a big head and a thick neck. Bay (鹿毛): a brown body with a black
mane, tail and lower legs.

Built with loft.py: one connected, rig-ready mesh, with a ring of vertices at
every joint. About 3.4 shaku to the withers here (short, sturdy legs, for the
chibi hint) and 5.4 shaku from nose to tail, feet on z = 0.
"""

import sys
from pathlib import Path

from mathutils import Vector

sys.path.insert(0, str(Path(__file__).resolve().parent))
import loft  # noqa: E402
from loft import LOWER_LEFT, LOWER_RIGHT, UPPER_LEFT, UPPER_RIGHT  # noqa: E402

BAY, BLACK, HOOF = "horse_bay", "soot", "hoof_grey"

# The spine, tail tip to nose: (y, z, half-width, half-height, colour, underside
# colour). See loft.Builder.spine.
TAIL_TIP = Vector((0, -3.06, 0.72))
SPINE = [
    # tail: long and full, hanging to the hocks
    (-3.02, 0.95, 0.20, 0.18, BLACK, BLACK),
    (-2.96, 1.45, 0.23, 0.20, BLACK, BLACK),
    (-2.86, 2.02, 0.19, 0.17, BLACK, BLACK),
    (-2.74, 2.45, 0.14, 0.14, BLACK, BLACK),  # the dock
    # body: a rump rounded over three rings, then a deep, full barrel
    (-2.65, 2.60, 0.38, 0.36, BAY, BAY),
    (-2.50, 2.57, 0.62, 0.60, BAY, BAY),  # the buttock, over wide hips; the hind legs grow from the long segment in front
    (-1.55, 2.50, 0.69, 0.80, BAY, BAY),  # a round belly, and a long barrel
    (-0.55, 2.52, 0.67, 0.82, BAY, BAY),  # the front legs grow from the long segment in front
    (0.30, 2.70, 0.54, 0.70, BAY, BAY),  # the base of the neck, under the withers
    # neck: thick and short, rising and arching forward
    (0.60, 3.10, 0.42, 0.50, BAY, BAY),
    (0.85, 3.45, 0.37, 0.42, BAY, BAY),
    (1.05, 3.75, 0.33, 0.36, BAY, BAY),
    # head: big, about 1.3x natural, with a short muzzle angled down
    (1.20, 4.05, 0.36, 0.38, BAY, BAY),  # the poll, behind the ears
    (1.48, 3.82, 0.38, 0.50, BAY, BAY),  # cheeks, the widest point: deep, horse-like jowls
    (1.82, 3.58, 0.28, 0.34, BAY, BAY),
    (2.10, 3.36, 0.23, 0.23, BAY, BAY),  # muzzle
    (2.24, 3.25, 0.19, 0.19, BLACK, BLACK),  # nose
]
NOSE_TIP = Vector((0, 2.31, 3.20))

# Which spine segment (index of its tail-side ring) and face each limb grows from.
# Each leg's first ring fits inside its base face, front, back and sides.
HIND_SEGMENT = 5  # buttock -> belly: the back of the thigh continues the buttock's curve
FRONT_SEGMENT = 7  # belly -> base of the neck: long, for a thick forearm and armpit
EAR_SEGMENT = 12  # poll -> cheeks

# Limbs, for the right side: rings of (x, y, z, half-width, half-depth, colour).
# See loft.Builder.limb. Short and sturdy, black below the knees and hocks.
FRONT_LEG = [
    (0.31, -0.05, 1.70, 0.20, 0.30, BAY),  # a thick forearm, deep into the armpit
    (0.30, 0.05, 1.35, 0.15, 0.19, BAY),  # elbow
    (0.29, 0.12, 0.90, 0.12, 0.135, BAY),  # knee
    (0.29, 0.12, 0.45, 0.10, 0.115, BLACK),  # cannon
    (0.29, 0.14, 0.24, 0.11, 0.125, BLACK),  # fetlock
    (0.29, 0.18, 0.10, 0.125, 0.145, HOOF),  # hoof
    (0.29, 0.19, 0.00, 0.135, 0.16, HOOF),  # sole
]
HIND_LEG = [
    (0.37, -2.05, 1.68, 0.24, 0.30, BAY),  # thigh, flush under the buttock
    (0.35, -2.02, 1.30, 0.18, 0.21, BAY),  # gaskin
    (0.32, -2.10, 0.90, 0.13, 0.15, BAY),  # hock
    (0.32, -2.07, 0.45, 0.10, 0.115, BLACK),  # cannon
    (0.32, -2.04, 0.24, 0.11, 0.125, BLACK),  # fetlock
    (0.32, -2.00, 0.10, 0.125, 0.145, HOOF),  # hoof
    (0.32, -1.99, 0.00, 0.135, 0.16, HOOF),  # sole
]
EAR = [  # upright and pointed, pricked forward
    (0.22, 1.28, 4.42, 0.09, 0.07, BAY),
    (0.24, 1.29, 4.60, 0.06, 0.045, BAY),
]
EAR_TIP = Vector((0.25, 1.31, 4.78))

# The mane along the top of the neck and a forelock between the ears, black.
# (spine segment, faces, colour); faces are numbered as in loft: 0 upper right,
# 1 top, 2 upper left.
MARKINGS = [
    (8, (0, 1, 2), BLACK),  # mane
    (9, (0, 1, 2), BLACK),
    (10, (0, 1, 2), BLACK),
    (11, (1,), BLACK),  # forelock
]


def build_horse():
    b = loft.Builder()
    segments = b.spine(TAIL_TIP, SPINE, NOSE_TIP)
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
    return b.finish("Horse", shaded=True)


if __name__ == "__main__":
    loft.run(build_horse, "horse", target=(0, -0.35, 2.3), extent=5.6)

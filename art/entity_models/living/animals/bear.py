"""The bear: a slightly chibi, low-poly Japanese black bear (ツキノワグマ), on all fours.

    blender -b --python art/entity_models/living/animals/bear.py -- --out assets/entity_models/living/animals/bear.glb [--renders <dir>]

The bear of Honshu, Shikoku and Kyushu; the brown bear lives only in Hokkaido.
Black, with the pale crescent on the chest it is named for (月の輪, "moon
ring"), a tan muzzle and big round ears. Stocky, with a big round head and short,
thick legs on long flat feet.

Built with loft.py: one connected, rig-ready mesh, with a ring of vertices at
every joint. About 1.9 shaku (~58 cm) to the shoulder once scaled and 2.6 shaku from nose to
rump, feet on z = 0.
"""

import sys
from pathlib import Path

from mathutils import Vector

sys.path.insert(0, str(next(p for p in Path(__file__).resolve().parents if (p / "loft.py").exists())))
import loft  # noqa: E402
from loft import LOWER_LEFT, LOWER_RIGHT, UPPER_LEFT, UPPER_RIGHT  # noqa: E402

BLACK, CREAM, MUZZLE, SOOT = "bear_black", "cream", "bear_muzzle", "soot"

# The spine, tail tip to nose: (y, z, half-width, half-height, colour, underside
# colour). See loft.Builder.spine.
TAIL_TIP = Vector((0, -1.34, 1.47))
SPINE = [
    (-1.28, 1.47, 0.06, 0.06, BLACK, BLACK),  # a stub of a tail
    # body: a rump rounded over three rings, then full, even and short
    (-1.22, 1.45, 0.24, 0.26, BLACK, BLACK),
    (-1.10, 1.43, 0.38, 0.40, BLACK, BLACK),
    (-0.92, 1.41, 0.48, 0.52, BLACK, BLACK),  # haunches; the hind legs grow from the segment in front
    (-0.45, 1.38, 0.54, 0.63, BLACK, BLACK),  # a round belly, hanging lowest in the middle
    (-0.10, 1.415, 0.55, 0.655, BLACK, BLACK),  # the front legs grow from the segment in front
    (0.32, 1.475, 0.52, 0.635, BLACK, BLACK),  # chest
    (0.55, 1.55, 0.44, 0.50, BLACK, BLACK),  # shoulders
    (0.74, 1.53, 0.36, 0.40, BLACK, BLACK),  # neck, narrower than the head to set it off
    # head: big and round, about 1.3x natural, with a tan muzzle
    (0.90, 1.55, 0.46, 0.46, BLACK, BLACK),  # back of the head
    (1.10, 1.50, 0.54, 0.47, BLACK, BLACK),  # cheeks, the widest point, with full jowls
    (1.32, 1.42, 0.33, 0.30, MUZZLE, MUZZLE),
    (1.48, 1.35, 0.18, 0.17, MUZZLE, MUZZLE),  # muzzle
    (1.58, 1.31, 0.12, 0.11, SOOT, SOOT),  # nose
]
NOSE_TIP = Vector((0, 1.63, 1.31))

# Which spine segment (index of its tail-side ring) and face each limb grows from.
# Each leg's first ring fits inside its base face, front, back and sides.
HIND_SEGMENT = 3  # haunches -> belly
FRONT_SEGMENT = 5  # under the chest
EAR_SEGMENT = 9  # back of the head -> cheeks

# Limbs, for the right side: rings of (x, y, z, half-width, half-depth, colour).
# See loft.Builder.limb.
FRONT_LEG = [  # short, thick columns
    (0.26, 0.11, 0.76, 0.17, 0.20, BLACK),  # shoulder
    (0.25, 0.13, 0.46, 0.15, 0.17, BLACK),  # elbow
    (0.24, 0.15, 0.18, 0.135, 0.145, BLACK),  # wrist
    (0.24, 0.21, 0.08, 0.145, 0.20, BLACK),  # paw
    (0.24, 0.21, 0.00, 0.145, 0.20, BLACK),  # sole
]
HIND_LEG = [
    (0.26, -0.685, 0.76, 0.18, 0.23, BLACK),  # thigh
    (0.25, -0.63, 0.50, 0.16, 0.19, BLACK),  # knee
    (0.24, -0.65, 0.22, 0.145, 0.155, BLACK),  # ankle
    (0.24, -0.59, 0.08, 0.155, 0.22, BLACK),  # a long, flat foot
    (0.24, -0.59, 0.00, 0.155, 0.22, BLACK),  # sole
]
EAR = [  # big and round: wide, with a blunt top
    (0.34, 0.99, 2.02, 0.13, 0.06, BLACK),
    (0.38, 0.98, 2.12, 0.12, 0.055, BLACK),
]
EAR_TIP = Vector((0.39, 0.98, 2.17))  # close above the last ring: a rounded top

# The moon ring: a pale crescent across the chest, below the throat, dipping to a
# point in the middle. (spine segment, faces, colour); faces are numbered as in
# loft: 4 lower left, 5 bottom, 6 lower right.
CRESCENT = [
    (7, (4, 5, 6), CREAM),
    (6, (5,), CREAM),
]


SCALE = 0.9  # built in its own numbers, then sized to a Japanese black bear's ~58 cm at the shoulder

def build_bear():
    b = loft.Builder()
    segments = b.spine(TAIL_TIP, SPINE, NOSE_TIP)
    for segment, faces, colour in CRESCENT:
        b.paint([segments[segment][k] for k in faces], colour)

    # Collect every base face before growing anything: growing removes faces.
    limbs = []
    for mirror, lower, upper in ((1, LOWER_RIGHT, UPPER_RIGHT), (-1, LOWER_LEFT, UPPER_LEFT)):
        limbs.append((segments[FRONT_SEGMENT][lower], FRONT_LEG, mirror, None))
        limbs.append((segments[HIND_SEGMENT][lower], HIND_LEG, mirror, None))
        limbs.append((segments[EAR_SEGMENT][upper], EAR, mirror, EAR_TIP))
    for face, rings, mirror, tip in limbs:
        b.limb(face, rings, mirror, tip)
    return b.finish("Bear", shaded=True, scale=SCALE)


if __name__ == "__main__":
    loft.run(build_bear, "bear", target=(0.00, 0.09, 0.99), extent=2.88)

"""The boar: a slightly chibi, low-poly Japanese wild boar (ニホンイノシシ), standing.

    blender -b --python art/entity_models/living/animals/boar.py -- --out assets/entity_models/living/animals/boar.glb [--renders <dir>]

An adult male of Japan's own subspecies, Sus scrofa leucomystax, named for the
whitish fringe along its jaw. Dark grey-brown with a darker bristly crest down the
back, small upright ears, a long snout ending in a flat disc, and short tusks.
Front-heavy: high shoulders sloping to a rounded rump, on short straight legs.

Built with loft.py: one connected, rig-ready mesh, with a ring of vertices at
every joint. About 2 shaku (~60 cm) to the shoulder and 2.8 shaku from snout to
rump, feet on z = 0.
"""

import sys
from pathlib import Path

from mathutils import Vector

sys.path.insert(0, str(next(p for p in Path(__file__).resolve().parents if (p / "loft.py").exists())))
import loft  # noqa: E402
from loft import LOWER_LEFT, LOWER_RIGHT, UPPER_LEFT, UPPER_RIGHT  # noqa: E402

BROWN, DARK, SNOUT = "boar_brown", "boar_dark", "boar_snout"
CHEEK, TUSK, HOOF = "boar_cheek", "tusk_ivory", "soot"

# The spine, tail tip to snout: (y, z, half-width, half-height, colour, underside
# colour). See loft.Builder.spine.
TAIL_TIP = Vector((0, -1.46, 0.84))
SPINE = [
    # tail: thin and hanging, with a tassel
    (-1.44, 0.91, 0.05, 0.05, DARK, DARK),
    (-1.40, 1.08, 0.035, 0.035, BROWN, BROWN),
    (-1.34, 1.22, 0.05, 0.05, BROWN, BROWN),  # tail root
    # body: a rounded rump, widening and rising to high, heavy shoulders
    (-1.27, 1.20, 0.20, 0.22, BROWN, BROWN),
    (-1.13, 1.17, 0.32, 0.37, BROWN, BROWN),
    (-0.92, 1.16, 0.38, 0.46, BROWN, BROWN),  # haunches
    (-0.55, 1.22, 0.42, 0.55, BROWN, BROWN),  # a deep belly
    (-0.15, 1.30, 0.48, 0.65, BROWN, BROWN),
    (0.20, 1.36, 0.50, 0.67, BROWN, BROWN),  # shoulders, the highest and widest point
    (0.48, 1.38, 0.40, 0.46, BROWN, BROWN),  # a thick neck
    # head: a big wedge sloping down to the snout, about 1.3x natural
    (0.72, 1.32, 0.42, 0.46, BROWN, BROWN),  # back of the head
    (0.95, 1.20, 0.34, 0.34, BROWN, CHEEK),  # cheeks, fringed pale underneath
    (1.18, 1.05, 0.22, 0.22, BROWN, BROWN),
    (1.36, 0.96, 0.14, 0.14, DARK, DARK),  # snout
    (1.47, 0.92, 0.13, 0.13, SNOUT, SNOUT),
]
SNOUT_TIP = Vector((0, 1.51, 0.92))  # close behind the last ring: a flat disc

# Which spine segment (index of its tail-side ring) and face each limb grows from.
HIND_SEGMENT = 5  # haunches
FRONT_SEGMENT = 7  # under the shoulders
EAR_SEGMENT = 10  # back of the head -> cheeks
TUSK_SEGMENT = 12  # along the snout

# Limbs, for the right side: rings of (x, y, z, half-width, half-depth, colour).
# See loft.Builder.limb.
FRONT_LEG = [  # short, and well forward under the shoulders
    (0.22, 0.12, 0.64, 0.12, 0.14, BROWN),  # shoulder
    (0.20, 0.14, 0.40, 0.08, 0.09, BROWN),  # elbow
    (0.19, 0.15, 0.16, 0.06, 0.07, DARK),  # wrist
    (0.19, 0.18, 0.07, 0.065, 0.08, HOOF),  # hoof
    (0.19, 0.19, 0.00, 0.065, 0.085, HOOF),  # sole
]
HIND_LEG = [  # short, and well back under the haunches
    (0.23, -0.86, 0.62, 0.13, 0.16, BROWN),  # thigh
    (0.21, -0.81, 0.40, 0.08, 0.10, BROWN),  # knee
    (0.19, -0.86, 0.16, 0.06, 0.07, DARK),  # hock
    (0.19, -0.82, 0.07, 0.065, 0.08, HOOF),  # hoof
    (0.19, -0.81, 0.00, 0.065, 0.085, HOOF),  # sole
]
EAR = [  # upright and pointed
    (0.22, 0.74, 1.76, 0.11, 0.06, DARK),
]
EAR_TIP = Vector((0.28, 0.68, 2.02))
TUSK = [  # short, curving up out of the lower jaw
    (0.21, 1.26, 0.93, 0.03, 0.03, TUSK),
]
TUSK_TIP = Vector((0.26, 1.30, 1.12))

# The bristly crest: darker top faces from behind the head down the back.
# (spine segment, faces, colour); faces are numbered as in loft: 0 upper right,
# 1 top, 2 upper left.
CREST = [
    (5, (1,), DARK),
    (6, (0, 1, 2), DARK),
    (7, (0, 1, 2), DARK),
    (8, (0, 1, 2), DARK),
    (9, (1,), DARK),
]


def build_boar():
    b = loft.Builder()
    segments = b.spine(TAIL_TIP, SPINE, SNOUT_TIP)
    for segment, faces, colour in CREST:
        b.paint([segments[segment][k] for k in faces], colour)

    # Collect every base face before growing anything: growing removes faces.
    limbs = []
    for mirror, lower, upper in ((1, LOWER_RIGHT, UPPER_RIGHT), (-1, LOWER_LEFT, UPPER_LEFT)):
        limbs.append((segments[FRONT_SEGMENT][lower], FRONT_LEG, mirror, None))
        limbs.append((segments[HIND_SEGMENT][lower], HIND_LEG, mirror, None))
        limbs.append((segments[EAR_SEGMENT][upper], EAR, mirror, EAR_TIP))
        limbs.append((segments[TUSK_SEGMENT][lower], TUSK, mirror, TUSK_TIP))
    for face, rings, mirror, tip in limbs:
        b.limb(face, rings, mirror, tip)
    return b.finish("Boar", shaded=True)


if __name__ == "__main__":
    loft.run(build_boar, "boar", target=(0, 0.0, 1.0), extent=3.2)

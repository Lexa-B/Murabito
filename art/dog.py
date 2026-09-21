"""The dog: a slightly chibi, low-poly native dog of the Shiba type (柴犬), standing.

    blender -b --python art/dog.py -- --out assets/models/dog.glb [--renders <dir>]

Small native dogs of this type kept villages company and hunted with them long
before the modern Shiba breed standard (20th century). Red (赤毛) with the cream
urajiro (裏白) on the cheeks, muzzle, throat, chest, belly and paws; pricked
triangular ears; a fox-like head with broad cheeks; and a tail curled up over the
back, its pale underside showing on the outside of the curl.

Built with loft.py: one connected, rig-ready mesh, with a ring of vertices at
every joint and five rings round the curl of the tail. About 1.15 shaku (~35 cm)
to the withers, feet on z = 0.
"""

import sys
from pathlib import Path

from mathutils import Vector

sys.path.insert(0, str(Path(__file__).resolve().parent))
import loft  # noqa: E402
from loft import LOWER_LEFT, LOWER_RIGHT, UPPER_LEFT, UPPER_RIGHT  # noqa: E402

RED, CREAM, SOOT = "shiba_red", "cream", "soot"

# The spine, tail tip to nose: (y, z, half-width, half-height, colour, underside
# colour). See loft.Builder.spine. The tail starts with its tip resting over the
# back and curls up, back and down into the rump; round the curl, a ring's
# "underside" faces outwards, which is where a curled Shiba tail shows its cream.
TAIL_TIP = Vector((0, -0.32, 1.13))  # tucked into the fur of the back
SPINE = [
    # the curled tail: thick and tight, its tip resting on the back
    (-0.40, 1.24, 0.09, 0.09, RED, CREAM),
    (-0.50, 1.29, 0.11, 0.11, RED, CREAM),  # the top of the curl
    (-0.59, 1.23, 0.11, 0.11, RED, CREAM),
    (-0.61, 1.11, 0.10, 0.10, RED, CREAM),
    (-0.57, 1.00, 0.085, 0.085, RED, RED),  # tail root
    # body: a rump rounded over three rings, then compact, deep and even
    (-0.54, 0.91, 0.15, 0.17, RED, RED),
    (-0.46, 0.89, 0.23, 0.25, RED, RED),
    (-0.34, 0.87, 0.27, 0.29, RED, RED),  # the buttock; the hind legs grow from the long segment in front
    (0.00, 0.86, 0.28, 0.30, RED, CREAM),  # a pale belly; the front legs grow from the segment in front
    (0.26, 0.89, 0.27, 0.31, RED, CREAM),  # chest
    # neck
    (0.40, 1.04, 0.20, 0.21, RED, CREAM),
    (0.48, 1.22, 0.18, 0.18, RED, CREAM),
    # head: fox-like but short-muzzled, about 1.3x natural, with broad cheeks
    (0.52, 1.38, 0.23, 0.21, RED, RED),  # back of the skull
    (0.68, 1.42, 0.27, 0.21, RED, CREAM),  # cheeks, the widest point
    (0.82, 1.36, 0.16, 0.14, RED, CREAM),
    (0.92, 1.31, 0.10, 0.09, RED, CREAM),  # muzzle
    (0.98, 1.29, 0.06, 0.055, SOOT, SOOT),  # nose
]
NOSE_TIP = Vector((0, 1.02, 1.28))

# Which spine segment (index of its tail-side ring) and face each limb grows from.
# Each leg's first ring fits inside its base face, front, back and sides.
HIND_SEGMENT = 7  # buttock -> belly
FRONT_SEGMENT = 8  # belly -> chest
EAR_SEGMENT = 12  # back of the skull -> cheeks

# Limbs, for the right side: rings of (x, y, z, half-width, half-depth, colour).
# See loft.Builder.limb. Sturdy, with cream paws.
FRONT_LEG = [
    (0.13, 0.13, 0.52, 0.09, 0.12, RED),  # shoulder
    (0.12, 0.13, 0.34, 0.07, 0.08, RED),  # elbow
    (0.11, 0.14, 0.16, 0.055, 0.06, RED),  # wrist
    (0.11, 0.16, 0.05, 0.055, 0.06, CREAM),
    (0.11, 0.19, 0.02, 0.065, 0.08, CREAM),  # paw
    (0.11, 0.19, 0.00, 0.065, 0.08, CREAM),  # sole
]
HIND_LEG = [
    (0.14, -0.17, 0.52, 0.10, 0.15, RED),  # thigh, flush under the buttock
    (0.13, -0.15, 0.34, 0.075, 0.09, RED),  # knee
    (0.12, -0.20, 0.17, 0.055, 0.06, RED),  # hock
    (0.12, -0.17, 0.05, 0.055, 0.06, CREAM),
    (0.12, -0.14, 0.02, 0.065, 0.08, CREAM),  # paw
    (0.12, -0.14, 0.00, 0.065, 0.08, CREAM),  # sole
]
EAR = [(0.17, 0.58, 1.65, 0.085, 0.045, RED)]  # pricked triangles, tipped forward
EAR_TIP = Vector((0.20, 0.60, 1.82))

# The urajiro on the face: pale cheeks and sides of the muzzle.
# (spine segment, faces, colour); faces are numbered as in loft: 3 left, 7 right.
URAJIRO = [
    (13, (3, 7), CREAM),
    (14, (3, 7), CREAM),
]


def build_dog():
    b = loft.Builder()
    segments = b.spine(TAIL_TIP, SPINE, NOSE_TIP)
    for segment, faces, colour in URAJIRO:
        b.paint([segments[segment][k] for k in faces], colour)

    # Collect every base face before growing anything: growing removes faces.
    limbs = []
    for mirror, lower, upper in ((1, LOWER_RIGHT, UPPER_RIGHT), (-1, LOWER_LEFT, UPPER_LEFT)):
        limbs.append((segments[FRONT_SEGMENT][lower], FRONT_LEG, mirror, None))
        limbs.append((segments[HIND_SEGMENT][lower], HIND_LEG, mirror, None))
        limbs.append((segments[EAR_SEGMENT][upper], EAR, mirror, EAR_TIP))
    for face, rings, mirror, tip in limbs:
        b.limb(face, rings, mirror, tip)
    return b.finish("Dog", shaded=True)


if __name__ == "__main__":
    loft.run(build_dog, "dog", target=(0, -0.05, 0.85), extent=1.9)

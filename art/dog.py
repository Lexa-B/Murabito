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
TAIL_TIP = Vector((0, -0.54, 1.10))  # tucked into the fur of the back
SPINE = [
    # the curled tail: thick and tight, its tip resting on the back
    (-0.62, 1.24, 0.09, 0.09, RED, CREAM),
    (-0.72, 1.29, 0.11, 0.11, RED, CREAM),  # the top of the curl
    (-0.81, 1.23, 0.11, 0.11, RED, CREAM),
    (-0.83, 1.11, 0.10, 0.10, RED, CREAM),
    (-0.79, 1.00, 0.085, 0.085, RED, RED),  # tail root
    # body: a flat, upright rump under the tail, a tucked-up flank, a deep belly;
    # the back level from the tail, rising a little to the shoulders
    (-0.76, 0.90, 0.15, 0.16, RED, RED),
    (-0.72, 0.86, 0.25, 0.26, RED, RED),  # the rump; the hind legs grow from the segment in front
    (-0.36, 0.845, 0.27, 0.285, RED, CREAM),  # the front of the hips, low to meet the thigh
    (-0.28, 0.92, 0.27, 0.22, RED, CREAM),  # the groin, tucked up just in front of the thigh
    (-0.06, 0.89, 0.28, 0.29, RED, CREAM),  # a pale belly; the front legs grow from the segment in front
    (0.26, 0.90, 0.27, 0.30, RED, CREAM),  # chest, under the withers; it leans with the neck, so it sits a hair higher to stay level with the back behind
    # neck
    (0.40, 1.04, 0.20, 0.21, RED, CREAM),
    (0.48, 1.22, 0.18, 0.18, RED, CREAM),
    # head: fox-like but short-muzzled, about 1.3x natural, with broad cheeks
    (0.52, 1.38, 0.23, 0.21, RED, RED),  # back of the skull
    (0.68, 1.42, 0.27, 0.21, RED, CREAM),  # cheeks, the widest point
    (0.80, 1.35, 0.18, 0.15, RED, CREAM),
    (0.88, 1.31, 0.13, 0.11, RED, CREAM),  # a short, blunt muzzle
    (0.93, 1.30, 0.09, 0.08, SOOT, SOOT),  # nose
]
NOSE_TIP = Vector((0, 0.96, 1.30))  # close behind the last ring: a broad, flat nose

RUMP_RINGS = (5, 6)  # stand upright: squared to the spine diving from the tail, they'd point out under it

# Which spine segment (index of its tail-side ring) and face each limb grows from.
# Each leg's first ring fits inside its base face, front, back and sides.
HIND_SEGMENT = 6  # rump -> flank: the back of the thigh drops straight from under the tail
FRONT_SEGMENT = 9  # belly -> chest
EAR_SEGMENT = 13  # back of the skull -> cheeks

# Limbs, for the right side: rings of (x, y, z, half-width, half-depth, colour).
# See loft.Builder.limb. Sturdy, with cream paws.
FRONT_LEG = [
    (0.13, 0.10, 0.50, 0.09, 0.15, RED),  # shoulder, reaching down and back into the armpit
    (0.12, 0.12, 0.33, 0.07, 0.08, RED),  # elbow
    (0.11, 0.14, 0.16, 0.055, 0.06, RED),  # wrist
    (0.11, 0.16, 0.05, 0.055, 0.06, CREAM),
    (0.11, 0.19, 0.02, 0.065, 0.08, CREAM),  # paw
    (0.11, 0.19, 0.00, 0.065, 0.08, CREAM),  # sole
]
HIND_LEG = [
    (0.14, -0.55, 0.48, 0.10, 0.17, RED),  # the haunch: as long as the body above it
    (0.13, -0.56, 0.30, 0.075, 0.085, RED),  # narrowing to the knee, under the back half
    (0.12, -0.61, 0.15, 0.055, 0.06, RED),  # hock, back under the rump
    (0.12, -0.60, 0.05, 0.055, 0.06, CREAM),
    (0.12, -0.58, 0.02, 0.065, 0.08, CREAM),  # paw
    (0.12, -0.58, 0.00, 0.065, 0.08, CREAM),  # sole
]
EAR = [(0.17, 0.58, 1.65, 0.085, 0.045, RED)]  # pricked triangles, tipped forward
EAR_TIP = Vector((0.20, 0.60, 1.82))

# The urajiro on the face: pale cheeks and sides of the muzzle.
# (spine segment, faces, colour); faces are numbered as in loft: 3 left, 7 right.
URAJIRO = [
    (14, (3, 7), CREAM),
    (15, (3, 7), CREAM),
]


SCALE = 1.1  # built in its own numbers, then sized to about 39 cm at the shoulder, the top of the Shiba range

def build_dog():
    b = loft.Builder()
    segments = b.spine(TAIL_TIP, SPINE, NOSE_TIP, upright=RUMP_RINGS)
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
    return b.finish("Dog", shaded=True, scale=SCALE)


if __name__ == "__main__":
    loft.run(build_dog, "dog", target=(0.00, -0.17, 0.94), extent=2.31)

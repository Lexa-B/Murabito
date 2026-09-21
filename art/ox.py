"""The ox: a slightly chibi, low-poly Japanese native draft ox, standing, unharnessed.

    blender -b --python art/ox.py -- --out assets/models/ox.glb [--renders <dir>]

The small black working cattle that pulled ploughs and carts in Sengoku paddies;
Mishima cattle (見島牛) are a surviving line of them. A deep barrel and wide hips,
a short thick neck with a small dewlap, a broad blocky head with short horns
curving out and up, ears sticking out to the sides, and a long thin tail with a
tassel.

Built with loft.py: one connected, rig-ready body, with a ring of vertices at
every joint; the horns and ears are rigid tubes (flora.branch) rooted in the
head. About 3.1 shaku to the withers (short legs, for the chibi hint) and 4.4
shaku from nose to rump, feet on z = 0.
"""

import sys
from pathlib import Path

from mathutils import Vector

sys.path.insert(0, str(Path(__file__).resolve().parent))
import flora  # noqa: E402
import loft  # noqa: E402
from loft import LOWER_LEFT, LOWER_RIGHT  # noqa: E402

BLACK, MUZZLE, SOOT = "cattle_black", "cattle_muzzle", "soot"
HORN, HOOF = "tusk_ivory", "hoof_grey"

# The spine, tail tip to nose: (y, z, half-width, half-height, colour, underside
# colour). See loft.Builder.spine.
TAIL_TIP = Vector((0, -2.60, 0.70))
SPINE = [
    # tail: long and thin, from high on the rump down to a tassel at the hocks
    (-2.58, 0.85, 0.12, 0.10, BLACK, BLACK),  # tassel
    (-2.55, 1.10, 0.05, 0.05, BLACK, BLACK),
    (-2.50, 1.75, 0.05, 0.05, BLACK, BLACK),
    (-2.44, 2.30, 0.07, 0.07, BLACK, BLACK),  # tail root
    # body: a rump rounded over three rings, wide hips, and a deep barrel
    (-2.38, 2.35, 0.34, 0.40, BLACK, BLACK),
    (-2.22, 2.27, 0.56, 0.66, BLACK, BLACK),  # the buttock; the hind legs grow from the long segment in front
    (-1.40, 2.15, 0.70, 0.86, BLACK, BLACK),  # a deep belly
    (-0.60, 2.17, 0.70, 0.88, BLACK, BLACK),  # the front legs grow from the long segment in front
    (0.15, 2.30, 0.58, 0.80, BLACK, BLACK),  # shoulders, under the withers
    # neck: short and thick, with a dewlap hanging beneath
    (0.50, 2.45, 0.42, 0.62, BLACK, BLACK),
    (0.78, 2.53, 0.38, 0.46, BLACK, BLACK),
    # head: big, broad and blocky, about 1.3x natural, hanging a little
    (0.95, 2.62, 0.46, 0.48, BLACK, BLACK),  # the poll, between the horns
    (1.20, 2.52, 0.46, 0.46, BLACK, BLACK),  # a broad forehead
    (1.48, 2.28, 0.34, 0.34, BLACK, BLACK),
    (1.70, 2.10, 0.30, 0.27, MUZZLE, MUZZLE),  # a broad, pale muzzle
    (1.80, 2.02, 0.27, 0.23, SOOT, SOOT),  # nose
]
NOSE_TIP = Vector((0, 1.85, 1.98))

# Which spine segment (index of its tail-side ring) and face each leg grows from.
# Each leg's first ring fits inside its base face, front, back and sides.
HIND_SEGMENT = 5  # buttock -> belly
FRONT_SEGMENT = 7  # under the barrel -> shoulders

# Legs, for the right side: rings of (x, y, z, half-width, half-depth, colour).
# See loft.Builder.limb. Short and sturdy.
FRONT_LEG = [
    (0.34, -0.20, 1.27, 0.23, 0.30, BLACK),  # a thick forearm, deep into the armpit
    (0.33, -0.12, 0.95, 0.17, 0.20, BLACK),  # elbow
    (0.31, -0.10, 0.62, 0.135, 0.145, BLACK),  # knee
    (0.31, -0.10, 0.35, 0.115, 0.125, BLACK),  # cannon
    (0.31, -0.08, 0.18, 0.125, 0.135, BLACK),  # fetlock
    (0.31, -0.04, 0.08, 0.135, 0.155, HOOF),  # hoof
    (0.31, -0.03, 0.00, 0.145, 0.165, HOOF),  # sole
]
HIND_LEG = [
    (0.36, -1.85, 1.27, 0.25, 0.30, BLACK),  # thigh, flush under the buttock
    (0.34, -1.82, 0.95, 0.19, 0.22, BLACK),  # gaskin
    (0.31, -1.90, 0.65, 0.14, 0.16, BLACK),  # hock
    (0.31, -1.87, 0.35, 0.115, 0.125, BLACK),  # cannon
    (0.31, -1.84, 0.18, 0.125, 0.135, BLACK),  # fetlock
    (0.31, -1.80, 0.08, 0.135, 0.155, HOOF),  # hoof
    (0.31, -1.79, 0.00, 0.145, 0.165, HOOF),  # sole
]

# Horns and ears, for the right side (mirrored for the left): tubes of
# (points, radii, colour per segment), rooted inside the head.
HEAD_TUBES = [
    (  # horn: short, curving out and up, dark at the tip
        [(0.24, 0.98, 2.95), (0.46, 0.98, 3.04), (0.64, 1.02, 3.20), (0.68, 1.08, 3.38)],
        [0.075, 0.065, 0.05, 0.02],
        [HORN, HORN, SOOT],
    ),
    (  # ear: broad, out to the side below the horn
        [(0.34, 0.92, 2.72), (0.58, 0.90, 2.70), (0.76, 0.88, 2.64)],
        [0.07, 0.12, 0.025],
        [BLACK, BLACK],
    ),
]


def build_ox():
    b = loft.Builder()
    segments = b.spine(TAIL_TIP, SPINE, NOSE_TIP)

    # Collect every base face before growing anything: growing removes faces.
    legs = []
    for mirror, lower in ((1, LOWER_RIGHT), (-1, LOWER_LEFT)):
        legs.append((segments[FRONT_SEGMENT][lower], FRONT_LEG, mirror))
        legs.append((segments[HIND_SEGMENT][lower], HIND_LEG, mirror))
    for face, rings, mirror in legs:
        b.limb(face, rings, mirror)

    for mirror in (1, -1):
        for points, radii, colours in HEAD_TUBES:
            flora.branch(b, [(x * mirror, y, z) for x, y, z in points], radii, colours)
    return b.finish("Ox", shaded=True)


if __name__ == "__main__":
    loft.run(build_ox, "ox", target=(0, -0.4, 1.7), extent=4.6)

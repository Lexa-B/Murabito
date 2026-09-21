"""The chicken: a slightly chibi, low-poly native rooster (地鶏, jidori), standing.

    blender -b --python art/chicken.py -- --out assets/models/chicken.glb [--renders <dir>]

Villages kept native chickens more to crow the hours than for eggs, so this is a
rooster, in the wild-type colouring (赤笹): golden neck hackles, a red-brown back
and wings, a black breast, a tall arching black sickle tail, a red comb, face and
wattles, and yellow legs and beak.

Built with loft.py: one connected, rig-ready mesh, with a ring of vertices at
every joint; the comb and wattles grow from the head's top and bottom faces.
About 1.4 shaku (~42 cm) to the top of the comb, feet on z = 0.
"""

import sys
from pathlib import Path

from mathutils import Vector

sys.path.insert(0, str(Path(__file__).resolve().parent))
import loft  # noqa: E402
from loft import BOTTOM, LOWER_LEFT, LOWER_RIGHT  # noqa: E402

RED, GOLD, BLACK = "rooster_red", "rooster_gold", "plumage_black"
COMB, BEAK = "crown_red", "beak_yellow"
TOP = 1  # the top face of a ring segment

# The spine, tail tip to beak: (y, z, half-width, half-height, colour, underside
# colour). See loft.Builder.spine. The tail is a tall, thin fan.
TAIL_TIP = Vector((0, -0.72, 0.54))
SPINE = [
    # sickle tail: arching up and back, drooping at the tip
    (-0.66, 0.74, 0.03, 0.10, BLACK, BLACK),
    (-0.55, 0.94, 0.04, 0.14, BLACK, BLACK),
    (-0.40, 1.00, 0.05, 0.15, BLACK, BLACK),  # the top of the arch
    (-0.30, 0.87, 0.07, 0.13, BLACK, BLACK),  # tail base
    # body: a full egg, red-brown above and black beneath
    (-0.24, 0.72, 0.14, 0.14, RED, BLACK),  # saddle
    (-0.14, 0.64, 0.22, 0.20, RED, BLACK),  # the legs grow from the segment in front
    (0.02, 0.62, 0.24, 0.23, RED, BLACK),
    (0.16, 0.68, 0.20, 0.21, GOLD, BLACK),  # breast
    # neck: golden hackles
    (0.22, 0.84, 0.12, 0.12, GOLD, GOLD),
    (0.24, 1.00, 0.10, 0.10, GOLD, GOLD),
    # head: about 1.3x natural, a red face, and a short beak
    (0.27, 1.12, 0.11, 0.11, GOLD, GOLD),  # back of the head
    (0.33, 1.16, 0.11, 0.10, COMB, COMB),  # face
    (0.40, 1.14, 0.05, 0.045, BEAK, BEAK),  # beak
]
BEAK_TIP = Vector((0, 0.47, 1.11))

LEG_SEGMENT = 5  # under the middle of the body
COMB_SEGMENT = 10  # back of the head -> face, its top face
WATTLE_SEGMENT = 11  # face -> beak, its bottom face

# Legs, for the right side: rings of (x, y, z, half-width, half-depth, colour).
# See loft.Builder.limb. Feathered thighs, then bare yellow shanks.
LEG = [
    (0.10, -0.06, 0.36, 0.07, 0.075, BLACK),  # thigh
    (0.09, -0.05, 0.20, 0.035, 0.04, BEAK),  # hock
    (0.09, -0.04, 0.05, 0.035, 0.04, BEAK),  # ankle
    (0.09, 0.00, 0.02, 0.06, 0.09, BEAK),  # toes, spread flat
    (0.09, 0.00, 0.00, 0.06, 0.09, BEAK),  # sole
]
# The comb and wattles sit on the middle line (x = 0), so they aren't mirrored.
COMB_RINGS = [  # a tall, thin blade
    (0.0, 0.27, 1.28, 0.02, 0.07, COMB),
    (0.0, 0.28, 1.36, 0.015, 0.06, COMB),
]
COMB_TIP = Vector((0.0, 0.29, 1.42))
WATTLE_RINGS = [(0.0, 0.35, 1.02, 0.02, 0.04, COMB)]
WATTLE_TIP = Vector((0.0, 0.35, 0.94))

# Wings folded along the sides: a red-brown bow in front, black flight feathers
# behind. (spine segment, faces, colour); faces are numbered as in loft:
# 3 left, 7 right.
WINGS = [
    (4, (3, 7), BLACK),
    (5, (3, 7), BLACK),
    (6, (3, 7), RED),
]


def build_chicken():
    b = loft.Builder()
    segments = b.spine(TAIL_TIP, SPINE, BEAK_TIP)
    for segment, faces, colour in WINGS:
        b.paint([segments[segment][k] for k in faces], colour)

    # Collect every base face before growing anything: growing removes faces.
    limbs = [
        (segments[LEG_SEGMENT][LOWER_RIGHT], LEG, 1, None),
        (segments[LEG_SEGMENT][LOWER_LEFT], LEG, -1, None),
        (segments[COMB_SEGMENT][TOP], COMB_RINGS, 1, COMB_TIP),
        (segments[WATTLE_SEGMENT][BOTTOM], WATTLE_RINGS, 1, WATTLE_TIP),
    ]
    for face, rings, mirror, tip in limbs:
        b.limb(face, rings, mirror, tip)
    return b.finish("Chicken", shaded=True)


if __name__ == "__main__":
    loft.run(build_chicken, "chicken", target=(0, -0.1, 0.7), extent=1.55)

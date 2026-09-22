"""The crane: a slightly chibi, low-poly red-crowned crane (丹頂, タンチョウ), standing.

    blender -b --python art/entity_models/living/animals/crane.py -- --out assets/entity_models/living/animals/crane.glb [--renders <dir>]

Today the red-crowned crane lives only in Hokkaido, but before its Edo-to-Meiji
decline it lived and wintered across much of Japan, so it belongs in a village
around 1550. White, with a black neck and face, a white band from behind the eye
down the nape, a red crown, and a black "tail" that is really the drooping inner
wing feathers, here a black bustle over the rump.

Built with loft.py: one connected, rig-ready mesh, with plenty of rings along the
neck so it can bend and dip. About 4.5 shaku (~1.4 m) tall, feet on z = 0.
"""

import sys
from pathlib import Path

from mathutils import Vector

sys.path.insert(0, str(next(p for p in Path(__file__).resolve().parents if (p / "loft.py").exists())))
import loft  # noqa: E402
from loft import LOWER_LEFT, LOWER_RIGHT  # noqa: E402

WHITE, BLACK, RED = "feather_white", "feather_black", "crown_red"
BILL, LEG = "bill_olive", "crane_leg"

# The spine, bustle tip to bill tip: (y, z, half-width, half-height, colour,
# underside colour). See loft.Builder.spine. Up the neck, a ring's "top" faces
# point backwards, so they are the nape.
BUSTLE_TIP = Vector((0, -1.36, 2.00))
SPINE = [
    # the black bustle: a full, fluffy droop over the rump
    (-1.24, 2.08, 0.22, 0.16, BLACK, BLACK),
    (-1.02, 2.24, 0.38, 0.28, BLACK, BLACK),
    # body: a full, rounded egg
    (-0.72, 2.42, 0.48, 0.42, BLACK, WHITE),
    (-0.40, 2.52, 0.52, 0.47, WHITE, WHITE),
    (-0.05, 2.58, 0.51, 0.46, WHITE, WHITE),
    (0.30, 2.67, 0.42, 0.39, WHITE, WHITE),  # breast
    (0.55, 2.84, 0.27, 0.27, WHITE, WHITE),  # base of the neck
    # neck: a gentle S, white at the bottom, black above
    (0.70, 3.10, 0.17, 0.17, WHITE, WHITE),
    (0.72, 3.40, 0.15, 0.15, BLACK, BLACK),
    (0.66, 3.70, 0.14, 0.14, BLACK, BLACK),
    (0.64, 3.96, 0.14, 0.14, BLACK, BLACK),
    (0.72, 4.18, 0.16, 0.16, BLACK, BLACK),
    # head: about 1.3x natural, and a long pointed bill
    (0.83, 4.31, 0.21, 0.20, BLACK, BLACK),  # back of the head
    (0.98, 4.38, 0.22, 0.19, BLACK, BLACK),  # crown
    (1.13, 4.35, 0.14, 0.13, BLACK, BLACK),  # face, at the base of the bill
    (1.30, 4.31, 0.06, 0.055, BILL, BILL),
    (1.50, 4.26, 0.04, 0.035, BILL, BILL),
]
BILL_TIP = Vector((0, 1.70, 4.21))

LEG_SEGMENT = 3  # under the middle of the body

# Legs, for the right side: rings of (x, y, z, half-width, half-depth, colour).
# See loft.Builder.limb.
LEG = [
    (0.18, -0.20, 2.00, 0.12, 0.14, WHITE),  # feathered thigh
    (0.16, -0.18, 1.78, 0.06, 0.065, LEG),  # where the feathers end
    (0.16, -0.20, 1.02, 0.055, 0.06, LEG),  # the joint that looks like a backward knee
    (0.16, -0.18, 0.10, 0.05, 0.055, LEG),  # ankle
    (0.16, -0.10, 0.04, 0.09, 0.18, LEG),  # toes, spread flat
    (0.16, -0.10, 0.00, 0.09, 0.18, LEG),  # sole
]

# Markings painted over the ring colours: (spine segment, faces, colour). Faces
# are numbered as in loft: 0 upper right, 1 top, 2 upper left, 3 left, ... 7 right.
MARKINGS = [
    (9, (1,), WHITE),  # the white nape band, widening up to the back of the head
    (10, (0, 1, 2), WHITE),
    (11, (0, 1, 2), WHITE),
    (12, (0, 1, 2, 3, 7), WHITE),  # round the back of the head, behind the eyes
    (13, (1,), RED),  # the red crown
]


SCALE = 0.85  # built in its own numbers, then sized to about 1.17 m standing: a real 1.4 m crane read large beside the others

def build_crane():
    b = loft.Builder()
    segments = b.spine(BUSTLE_TIP, SPINE, BILL_TIP)
    for segment, faces, colour in MARKINGS:
        b.paint([segments[segment][k] for k in faces], colour)

    # Collect both base faces before growing anything: growing removes faces.
    legs = [(segments[LEG_SEGMENT][LOWER_RIGHT], 1), (segments[LEG_SEGMENT][LOWER_LEFT], -1)]
    for face, mirror in legs:
        b.limb(face, LEG, mirror)
    return b.finish("Crane", shaded=True, scale=SCALE)


if __name__ == "__main__":
    loft.run(build_crane, "crane", target=(0.00, 0.13, 1.95), extent=4.08)

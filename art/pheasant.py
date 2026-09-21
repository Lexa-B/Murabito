"""The pheasant: a slightly chibi, low-poly green pheasant cock (キジ), standing.

    blender -b --python art/pheasant.py -- --out assets/models/pheasant.glb [--renders <dir>]

Japan's own pheasant, common in fields and at the edges of villages, and all
through the folktales. The cock, in vivid colours: a royal-blue head with red
face patches round yellow eyes and small ear tufts, a teal-emerald neck, breast
and belly, rusty scaled shoulders over grey-blue wings, a sky-blue rump, a long
pinkish tail held low behind, grey legs and an ivory bill.

Built with loft.py: one connected, rig-ready mesh, with a ring of vertices at
every joint; the ear tufts grow from the head's upper faces, and each eye is a
yellow stud with a black pupil stud on it, on a poked face (a little socket). About 1.3 shaku (~39 cm) to the top of the head and 2.1 shaku from bill
to tail tip, feet on z = 0.
"""

import sys
from pathlib import Path

import bmesh
from mathutils import Vector

sys.path.insert(0, str(Path(__file__).resolve().parent))
import loft  # noqa: E402
from loft import LOWER_LEFT, LOWER_RIGHT, UPPER_LEFT, UPPER_RIGHT  # noqa: E402

GREEN, NECK, BACK, TAIL = "pheasant_green", "pheasant_neck", "pheasant_back", "pheasant_tail"
RUMP, WING, LEG = "pheasant_rump", "pheasant_wing", "pheasant_leg"
FACE, BILL = "pheasant_red", "tusk_ivory"
IRIS, PUPIL = "eye_yellow", "eye_black"
LEFT, RIGHT = 3, 7  # the side faces of a ring segment

# The spine, tail tip to bill: (y, z, half-width, half-height, colour, underside
# colour). See loft.Builder.spine.
TAIL_TIP = Vector((0, -1.55, 0.66))
SPINE = [
    # the tail: long and narrow, held low behind
    (-1.42, 0.66, 0.06, 0.035, TAIL, TAIL),
    (-1.15, 0.67, 0.08, 0.045, TAIL, TAIL),
    (-0.88, 0.68, 0.09, 0.055, TAIL, TAIL),
    (-0.64, 0.69, 0.09, 0.06, TAIL, TAIL),  # tail base
    (-0.52, 0.66, 0.12, 0.10, RUMP, RUMP),  # a grey-blue rump
    # body: a full, round breast, olive-brown above and dark green beneath
    (-0.40, 0.62, 0.20, 0.18, BACK, GREEN),
    (-0.20, 0.58, 0.24, 0.23, BACK, GREEN),  # the legs grow from the segment in front
    (0.00, 0.58, 0.24, 0.24, GREEN, GREEN),  # breast
    (0.14, 0.64, 0.19, 0.20, GREEN, GREEN),
    # neck: thick, teal at the base and royal blue above
    (0.22, 0.80, 0.14, 0.14, GREEN, GREEN),
    (0.25, 0.95, 0.12, 0.12, NECK, NECK),
    # head: about 1.3x natural, red patches round the eyes, a short pale bill
    (0.28, 1.08, 0.13, 0.13, NECK, NECK),  # back of the head
    (0.315, 1.11, 0.135, 0.126, NECK, NECK),  # the face: extra rings, so the red patches
    (0.395, 1.12, 0.122, 0.112, NECK, NECK),  # can spread round the eyes; the eye fits between
    (0.42, 1.105, 0.09, 0.08, NECK, NECK),
    (0.46, 1.085, 0.05, 0.045, BILL, BILL),  # bill
]
BILL_TIP = Vector((0, 0.52, 1.07))

LEG_SEGMENT = 6  # under the middle of the body
TUFT_SEGMENT = 11  # back of the head -> face: ear tufts on its upper faces
EYE_SEGMENT = 12  # the eyes on its side faces, in the middle of the red patch
EYE_SIZE = 0.04

# Limbs, for the right side: rings of (x, y, z, half-width, half-depth, colour).
# See loft.Builder.limb.
LEG = [
    (0.08, -0.10, 0.30, 0.05, 0.07, GREEN),  # feathered thigh, dark like the belly
    (0.07, -0.09, 0.16, 0.03, 0.034, LEG),  # shank
    (0.07, -0.08, 0.04, 0.03, 0.034, LEG),  # ankle
    (0.07, -0.04, 0.015, 0.05, 0.09, LEG),  # toes, spread flat
    (0.07, -0.04, 0.00, 0.05, 0.09, LEG),  # sole
]
TUFT = [(0.08, 0.28, 1.25, 0.03, 0.025, NECK)]  # small ear tufts, up and back
TUFT_TIP = Vector((0.09, 0.24, 1.34))

# The red face patches, round the eyes and down the sides of the face.
# (spine segment, faces, colour).
MARKINGS = [
    # a big red oval on each side of the face, round the eye; the crown and chin stay blue
    (11, (LEFT, RIGHT, 4, 6), FACE),
    (12, (0, 2, LEFT, RIGHT, 4, 6), FACE),
    (13, (0, 2, LEFT, RIGHT, 4, 6), FACE),
    (14, (LEFT, RIGHT), FACE),
    (5, (LEFT, RIGHT), WING),  # grey-blue wings folded along the sides, below the rusty shoulders
    (6, (LEFT, RIGHT), WING),
]


def build_pheasant():
    b = loft.Builder()
    segments = b.spine(TAIL_TIP, SPINE, BILL_TIP)
    for segment, faces, colour in MARKINGS:
        b.paint([segments[segment][k] for k in faces], colour)
    for face, outward in ((segments[EYE_SEGMENT][RIGHT], 1), (segments[EYE_SEGMENT][LEFT], -1)):
        face.normal_update()
        normal = face.normal if face.normal.x * outward > 0 else -face.normal
        # Poke the face: a little socket, and the same split on both sides.
        centre = bmesh.ops.poke(b.bm, faces=[face])["verts"][0].co.copy()
        # a round yellow eye with a black pupil on it, both low, so they sit nearly flush
        b.stud(centre, normal, EYE_SIZE, IRIS, sides=8, height=0.008)
        b.stud(centre + normal * 0.004, normal, EYE_SIZE * 0.45, PUPIL, sides=8, inset=0.004, height=0.008)

    # Collect every base face before growing anything: growing removes faces.
    limbs = []
    for mirror, lower, upper in ((1, LOWER_RIGHT, UPPER_RIGHT), (-1, LOWER_LEFT, UPPER_LEFT)):
        limbs.append((segments[LEG_SEGMENT][lower], LEG, mirror, None))
        limbs.append((segments[TUFT_SEGMENT][upper], TUFT, mirror, TUFT_TIP))
    for face, rings, mirror, tip in limbs:
        b.limb(face, rings, mirror, tip)
    return b.finish("Pheasant", shaded=True)


if __name__ == "__main__":
    loft.run(build_pheasant, "pheasant", target=(0, -0.5, 0.62), extent=2.2)

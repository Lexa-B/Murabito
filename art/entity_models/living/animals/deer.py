"""The deer: a slightly chibi, low-poly sika stag (ニホンジカ) in summer coat, standing.

    blender -b --python art/entity_models/living/animals/deer.py -- --out assets/entity_models/living/animals/deer.glb [--renders <dir>]

The sika is Japan's own deer. In summer it is reddish-brown with rows of white
spots (鹿の子, kanoko) along the back, a pale belly, and a white rump patch edged
in black above a short tail. The stag's antlers are a beam with a brow tine, a
middle tine and a fork at the top.

Built with loft.py: one connected, rig-ready body, with a ring of vertices at
every joint; the antlers are rigid, so they are separate tubes (flora.branch)
rooted in the head, and each spot is a small flat stud on the coat. About 2.75 shaku (~84 cm) to the shoulder once scaled and 5.2 shaku to the
antler tips, feet on z = 0.
"""

import sys
from pathlib import Path

from mathutils import Vector

sys.path.insert(0, str(next(p for p in Path(__file__).resolve().parents if (p / "loft.py").exists())))
import flora  # noqa: E402
import loft  # noqa: E402
from loft import LOWER_LEFT, LOWER_RIGHT, UPPER_LEFT, UPPER_RIGHT  # noqa: E402

BROWN, CREAM, SOOT, ANTLER = "deer_brown", "cream", "soot", "antler"

# The spine, tail tip to nose: (y, z, half-width, half-height, colour, underside
# colour). See loft.Builder.spine.
TAIL_TIP = Vector((0, -1.84, 1.37))
SPINE = [
    # tail: short, black on top and white beneath
    (-1.78, 1.45, 0.07, 0.05, SOOT, CREAM),
    (-1.70, 1.57, 0.08, 0.07, CREAM, CREAM),  # tail root; the rump patch is white
    # body: a rump rounded from above as well as from the side, full and even to the chest
    (-1.64, 1.57, 0.20, 0.23, BROWN, BROWN),
    (-1.53, 1.56, 0.32, 0.34, BROWN, BROWN),  # the buttock; the thighs grow from the segment in front
    (-1.00, 1.57, 0.42, 0.49, BROWN, CREAM),  # haunches; the thighs grow from the long segment behind
    (-0.80, 1.58, 0.42, 0.47, BROWN, CREAM),  # a little belly, sagging in the middle
    (-0.57, 1.56, 0.43, 0.50, BROWN, CREAM),
    (-0.35, 1.57, 0.43, 0.51, BROWN, CREAM),
    (-0.15, 1.60, 0.425, 0.51, BROWN, CREAM),
    (0.05, 1.66, 0.42, 0.49, BROWN, CREAM),  # chest
    (0.30, 1.80, 0.36, 0.40, BROWN, BROWN),  # shoulders
    # neck: short and thick, up and forward
    (0.46, 2.08, 0.26, 0.27, BROWN, BROWN),
    (0.58, 2.35, 0.23, 0.23, BROWN, BROWN),
    (0.66, 2.58, 0.22, 0.22, BROWN, BROWN),
    # head: about 1.3x natural, tapering to a dark nose
    (0.72, 2.78, 0.28, 0.27, BROWN, BROWN),  # back of the head
    (0.90, 2.85, 0.30, 0.27, BROWN, BROWN),  # cheeks
    (1.10, 2.79, 0.21, 0.20, BROWN, BROWN),
    (1.30, 2.69, 0.14, 0.14, BROWN, CREAM),  # muzzle, pale underneath
    (1.42, 2.64, 0.11, 0.11, SOOT, SOOT),  # nose
]
NOSE_TIP = Vector((0, 1.48, 2.63))
BODY_AXIS_Z = 1.65  # roughly the height of the body's spine, for telling outside from in

# Which spine segment (index of its tail-side ring) and face each limb grows from.
HIND_SEGMENT = 3  # buttock -> haunches: as long as the thigh is deep, so it tucks in neither behind nor in front
FRONT_SEGMENT = 9  # chest -> shoulders
EAR_SEGMENT = 14  # back of the head -> cheeks

# Limbs, for the right side: rings of (x, y, z, half-width, half-depth, colour).
# See loft.Builder.limb.
FRONT_LEG = [
    (0.20, 0.18, 1.14, 0.13, 0.15, BROWN),  # shoulder
    (0.18, 0.18, 0.80, 0.09, 0.10, BROWN),  # elbow
    (0.17, 0.20, 0.40, 0.065, 0.07, BROWN),  # knee
    (0.17, 0.22, 0.10, 0.055, 0.06, BROWN),  # fetlock
    (0.17, 0.25, 0.05, 0.06, 0.075, SOOT),  # hoof
    (0.17, 0.26, 0.00, 0.06, 0.08, SOOT),  # sole
]
HIND_LEG = [
    (0.18, -1.21, 1.02, 0.13, 0.22, BROWN),  # a full thigh, inside its base face and the haunch's width
    (0.19, -1.13, 0.88, 0.125, 0.18, BROWN),  # tapering, so the back of the leg curves down to the hock
    (0.19, -0.99, 0.72, 0.10, 0.12, BROWN),  # stifle
    (0.17, -1.03, 0.42, 0.065, 0.075, BROWN),  # hock
    (0.17, -1.00, 0.10, 0.055, 0.06, BROWN),  # fetlock
    (0.17, -0.97, 0.05, 0.06, 0.075, SOOT),  # hoof
    (0.17, -0.96, 0.00, 0.06, 0.08, SOOT),  # sole
]
EAR = [  # big, held out to the side and up
    (0.30, 0.74, 3.08, 0.09, 0.045, BROWN),
    (0.42, 0.72, 3.18, 0.09, 0.035, BROWN),
]
EAR_TIP = Vector((0.54, 0.70, 3.27))

# Antlers, for the right side (mirrored for the left): tubes of (points, radii),
# the beam first, rooted inside the head, spreading up and out in a V.
ANTLER_TUBES = [
    (
        [(0.12, 0.95, 2.98), (0.20, 0.90, 3.30), (0.30, 0.82, 3.62), (0.39, 0.74, 3.92), (0.43, 0.74, 4.15)],
        [0.065, 0.058, 0.05, 0.04, 0.018],
    ),
    ([(0.19, 0.91, 3.26), (0.25, 1.13, 3.40)], [0.04, 0.014]),  # brow tine
    ([(0.30, 0.81, 3.64), (0.38, 1.00, 3.80)], [0.036, 0.013]),  # middle tine
    ([(0.39, 0.75, 3.94), (0.48, 0.60, 4.10)], [0.03, 0.012]),  # the fork at the top
]

# Markings painted over the ring colours: (spine segment, faces, colour). Faces
# are numbered as in loft: 0 upper right, 1 top, 2 upper left, 3 left, ... 7 right.
MARKINGS = [
    (1, (1,), SOOT),  # the black edge above the white rump patch
    (2, (0, 1, 2), SOOT),
    (2, (3, 7), CREAM),
]

# Kanoko spots: (spine segment, face, offset along the body, offset around it).
# Each is a small flat stud on that face, so spots can be smaller than a face.
SPOT_SIZE = 0.055
SPOTS = [
    (segment, face, along, around)
    for segment in range(4, 10)
    for face, along, around in (
        # a row high on each side, two spots per face, staggered
        (0, 0.06 if segment % 2 else -0.05, -0.04),
        (0, -0.07 if segment % 2 else 0.06, 0.05),
        (2, -0.05 if segment % 2 else 0.06, 0.04),
        (2, 0.06 if segment % 2 else -0.07, -0.05),
        # and a sparser row lower on the flank
        (7 if segment % 2 else 3, 0.0, 0.0),
    )
]


SCALE = 1.25  # built in its own numbers, then sized to a Honshu stag's ~84 cm at the shoulder

def spot(b, face, along, around):
    """A small, flat, closed stud of cream on a body face."""
    face.normal_update()
    centre = face.calc_center_median()
    normal = face.normal.copy()
    if normal.dot(centre - Vector((0, centre.y, BODY_AXIS_Z))) < 0:
        normal.negate()
    lengthwise = (Vector((0, 1, 0)) - normal * normal.y).normalized()
    across = normal.cross(lengthwise)
    middle = centre + lengthwise * along + across * around
    s = SPOT_SIZE
    corners = [
        b.bm.verts.new(middle - normal * 0.004 + offset)
        for offset in (lengthwise * s, across * s * 0.8, -lengthwise * s, -across * s * 0.8)
    ]
    apex = b.bm.verts.new(middle + normal * 0.012)
    for k in range(4):
        b.face((apex, corners[k], corners[(k + 1) % 4]), CREAM)
    b.face(list(reversed(corners)), CREAM)  # a base inside the coat, closing the stud


def build_deer():
    b = loft.Builder()
    segments = b.spine(TAIL_TIP, SPINE, NOSE_TIP)
    for segment, faces, colour in MARKINGS:
        b.paint([segments[segment][k] for k in faces], colour)
    for segment, face, along, around in SPOTS:
        spot(b, segments[segment][face], along, around)

    # Collect every base face before growing anything: growing removes faces.
    limbs = []
    for mirror, lower, upper in ((1, LOWER_RIGHT, UPPER_RIGHT), (-1, LOWER_LEFT, UPPER_LEFT)):
        limbs.append((segments[FRONT_SEGMENT][lower], FRONT_LEG, mirror, None))
        limbs.append((segments[HIND_SEGMENT][lower], HIND_LEG, mirror, None))
        limbs.append((segments[EAR_SEGMENT][upper], EAR, mirror, EAR_TIP))
    for face, rings, mirror, tip in limbs:
        b.limb(face, rings, mirror, tip)

    for mirror in (1, -1):
        for points, radii in ANTLER_TUBES:
            flora.branch(b, [(x * mirror, y, z) for x, y, z in points], radii, ANTLER)
    return b.finish("Deer", shaded=True, scale=SCALE)


if __name__ == "__main__":
    loft.run(build_deer, "deer", target=(0.00, -0.12, 2.50), extent=5.38)

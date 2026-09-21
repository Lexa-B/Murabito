"""The macaque: a slightly chibi, low-poly Japanese macaque (ニホンザル), on all fours.

    blender -b --python art/macaque.py -- --out assets/models/macaque.glb [--renders <dir>]

The snow monkey, found through the hills round most villages; stables sometimes
kept one to guard the horses (厩猿). Grey-brown fur, a flat red face framed by a
ruff of fur round a big round head, darker skin on the hands and feet, a short
stubby tail, and a red rump beneath it. Walking on all fours, the neutral pose;
a skeleton can sit it up later.

Built with loft.py: one connected, rig-ready mesh, with a ring of vertices at
every joint; the eyes are small studs on the face. About 1.25 shaku (~38 cm) to
the shoulder, feet on z = 0. The ears are small and hidden in the mane, so it
has none.
"""

import sys
from pathlib import Path

from mathutils import Vector

sys.path.insert(0, str(Path(__file__).resolve().parent))
import loft  # noqa: E402
from loft import LOWER_LEFT, LOWER_RIGHT  # noqa: E402

BROWN, FACE, SKIN, EYE = "macaque_brown", "macaque_face", "macaque_skin", "eye_black"

# The spine, tail tip to face: (y, z, half-width, half-height, colour, underside
# colour). See loft.Builder.spine.
TAIL_TIP = Vector((0, -0.71, 1.02))
SPINE = [
    # a short, stubby tail, up and back
    (-0.67, 0.96, 0.05, 0.05, BROWN, BROWN),
    (-0.61, 0.92, 0.06, 0.06, BROWN, FACE),  # tail root, with the red rump beneath
    # body: a rounded rump, a short full body, the shoulders a little high
    (-0.57, 0.85, 0.18, 0.20, BROWN, FACE),
    (-0.50, 0.82, 0.27, 0.28, BROWN, BROWN),  # the rump; the hind legs grow from the segment in front
    (-0.14, 0.84, 0.30, 0.30, BROWN, BROWN),  # belly
    (0.14, 0.90, 0.30, 0.31, BROWN, BROWN),  # chest; the front legs grow from the segment in front
    (0.36, 0.98, 0.26, 0.28, BROWN, BROWN),  # shoulders
    (0.47, 1.10, 0.23, 0.23, BROWN, BROWN),  # a short, thick neck
    # head: big and round, about 1.3x natural, a poofy mane round a flat red face
    (0.54, 1.28, 0.29, 0.30, BROWN, BROWN),  # the mane, poofed up over the back of the head
    (0.72, 1.321, 0.31, 0.266, BROWN, BROWN),  # the ruff, the widest point; its underside in line with its neighbours', or it hangs as a point
    # the face: upright rings, a tall trapezoid (wide brow, narrow chin), flat
    (0.81, 1.30, 0.21, 0.25, FACE, FACE, 0.3),
    (0.86, 1.30, 0.18, 0.22, FACE, FACE, 0.3),
    (0.91, 1.25, 0.06, 0.05, FACE, FACE),  # a small pink nose
]
NOSE_TIP = Vector((0, 0.94, 1.25))
UPRIGHT = (2, 3, 10, 11, 12)  # the rump rounds off under the tail; the face stands flat and vertical

# Two black eyes on the face: (x, y, z) for the right one (mirrored), each a
# small closed stud facing forward.
EYE_AT = (0.075, 0.89, 1.38)
EYE_SIZE = 0.035

# Which spine segment (index of its tail-side ring) and face each leg grows from.
# Each leg's first ring fits inside its base face, front, back and sides.
HIND_SEGMENT = 3  # rump -> belly
FRONT_SEGMENT = 5  # chest -> shoulders

# Legs, for the right side: rings of (x, y, z, half-width, half-depth, colour).
# See loft.Builder.limb. Sturdy, with dark-skinned hands and long feet.
FRONT_LEG = [
    (0.16, 0.26, 0.54, 0.10, 0.11, BROWN),  # shoulder
    (0.16, 0.24, 0.30, 0.075, 0.085, BROWN),  # elbow
    (0.15, 0.27, 0.08, 0.06, 0.065, BROWN),  # wrist
    (0.15, 0.32, 0.03, 0.07, 0.10, SKIN),  # hand, flat on the ground
    (0.15, 0.32, 0.00, 0.07, 0.10, SKIN),  # palm
]
HIND_LEG = [
    (0.15, -0.32, 0.48, 0.11, 0.16, BROWN),  # thigh
    (0.15, -0.27, 0.30, 0.08, 0.095, BROWN),  # knee
    (0.14, -0.30, 0.08, 0.06, 0.07, BROWN),  # ankle
    (0.14, -0.23, 0.03, 0.075, 0.14, SKIN),  # a long foot
    (0.14, -0.23, 0.00, 0.075, 0.14, SKIN),  # sole
]


def eye(b, centre):
    """A small, closed, faceted stud facing forward (+Y)."""
    forward, across, up = Vector((0, 1, 0)), Vector((1, 0, 0)), Vector((0, 0, 1))
    s = EYE_SIZE
    corners = [
        b.bm.verts.new(centre - forward * 0.006 + offset)
        for offset in (up * s, across * s, -up * s, -across * s)
    ]
    apex = b.bm.verts.new(centre + forward * 0.014)
    for k in range(4):
        b.face((apex, corners[k], corners[(k + 1) % 4]), EYE)
    b.face(list(reversed(corners)), EYE)


def build_macaque():
    b = loft.Builder()
    segments = b.spine(TAIL_TIP, SPINE, NOSE_TIP, upright=UPRIGHT)
    for mirror in (1, -1):
        eye(b, Vector((EYE_AT[0] * mirror, EYE_AT[1], EYE_AT[2])))

    # Collect every base face before growing anything: growing removes faces.
    legs = []
    for mirror, lower in ((1, LOWER_RIGHT), (-1, LOWER_LEFT)):
        legs.append((segments[FRONT_SEGMENT][lower], FRONT_LEG, mirror))
        legs.append((segments[HIND_SEGMENT][lower], HIND_LEG, mirror))
    for face, rings, mirror in legs:
        b.limb(face, rings, mirror)
    return b.finish("Macaque", shaded=True)


if __name__ == "__main__":
    loft.run(build_macaque, "macaque", target=(0, 0.05, 0.7), extent=2.0)

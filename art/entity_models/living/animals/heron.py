"""The heron: a slightly chibi, low-poly grey heron (アオサギ), standing.

    blender -b --python art/entity_models/living/animals/heron.py -- --out assets/entity_models/living/animals/heron.glb [--renders <dir>]

The common heron of paddies, ponds and rivers. Grey back and wings, a white
head, neck and underside, a black stripe from the eye back into a trailing black
plume, black patches at the shoulders, a yellow-orange dagger of a bill, long
yellowish legs, and yellow eyes. Built like the crane, but with the neck in a
relaxed S and a short tail instead of the crane's bustle.

Built with loft.py: one connected, rig-ready body, with plenty of rings up the
neck; the plume is a rigid tube (flora.branch), and each eye is a low yellow stud
with a low black pupil on it, on a poked face, sitting nearly flush (like the
pheasant's). About 3.3 shaku (~1.0 m) to the top of the head, once scaled, feet on z = 0.
"""

import sys
from pathlib import Path

import bmesh
from mathutils import Vector

sys.path.insert(0, str(next(p for p in Path(__file__).resolve().parents if (p / "loft.py").exists())))
import flora  # noqa: E402
import loft  # noqa: E402
from loft import LOWER_LEFT, LOWER_RIGHT  # noqa: E402

GREY, WHITE, BLACK = "heron_grey", "feather_white", "feather_black"
BILL, LEG, IRIS, PUPIL = "heron_bill", "heron_leg", "eye_yellow", "eye_black"
LEFT, RIGHT = 3, 7  # the side faces of a ring segment

# The spine, tail tip to bill tip: (y, z, half-width, half-height, colour,
# underside colour). See loft.Builder.spine. Up the neck, a ring's "underside"
# faces point forward, so they are the front of the neck.
TAIL_TIP = Vector((0, -1.00, 1.76))  # close behind the last ring: a rounded tail
SPINE = [
    # a short tail
    (-0.93, 1.81, 0.15, 0.10, GREY, GREY),
    (-0.74, 1.92, 0.28, 0.21, GREY, GREY),
    # body: a full egg, grey above and white beneath
    (-0.55, 2.02, 0.36, 0.32, GREY, WHITE),
    (-0.25, 2.10, 0.40, 0.36, GREY, WHITE),  # the legs grow from the segment in front
    (0.05, 2.16, 0.38, 0.35, GREY, WHITE),
    (0.30, 2.26, 0.30, 0.30, GREY, WHITE),  # breast
    (0.48, 2.42, 0.20, 0.20, WHITE, WHITE),  # base of the neck
    # neck: white, in a relaxed S
    (0.58, 2.65, 0.15, 0.15, WHITE, WHITE),
    (0.56, 2.90, 0.14, 0.14, WHITE, WHITE),
    (0.62, 3.12, 0.14, 0.14, WHITE, WHITE),
    (0.72, 3.26, 0.15, 0.15, WHITE, WHITE),
    # head: about 1.3x natural, white, a long dagger of a bill
    (0.82, 3.34, 0.19, 0.18, WHITE, WHITE),  # back of the head
    (0.96, 3.36, 0.19, 0.17, WHITE, WHITE),  # the eyes sit on the segment in front
    (1.10, 3.32, 0.12, 0.11, WHITE, WHITE),
    (1.22, 3.27, 0.055, 0.05, BILL, BILL),  # bill
    (1.45, 3.20, 0.035, 0.03, BILL, BILL),
]
BILL_TIP = Vector((0, 1.66, 3.14))

LEG_SEGMENT = 3  # under the middle of the body
EYE_SEGMENT = 12  # the eyes on its side faces
EYE_SIZE = 0.055

# Legs, for the right side: rings of (x, y, z, half-width, half-depth, colour).
# See loft.Builder.limb. Long and fairly straight.
LEG = [
    (0.16, -0.10, 1.70, 0.11, 0.13, GREY),  # feathered thigh
    (0.14, -0.08, 1.50, 0.06, 0.065, LEG),  # where the feathers end
    (0.14, -0.10, 0.85, 0.055, 0.06, LEG),  # the joint that looks like a backward knee
    (0.14, -0.08, 0.10, 0.05, 0.055, LEG),  # ankle
    (0.14, 0.00, 0.04, 0.08, 0.17, LEG),  # toes, spread flat
    (0.14, 0.00, 0.00, 0.08, 0.17, LEG),  # sole
]

# The black plume trailing from the back of the head: a tube of (points, radii).
PLUME = ([(0.0, 0.80, 3.42), (0.0, 0.62, 3.40), (0.0, 0.46, 3.34)], [0.025, 0.02, 0.008])

# (spine segment, faces, colour); faces are numbered as in loft: 0 upper right,
# 2 upper left, 3 left, 5 bottom (the front of the neck), 7 right.
MARKINGS = [
    (11, (0, 2), BLACK),  # the black stripe above the eye, back into the plume
    (12, (0, 2), BLACK),
    (5, (LEFT, RIGHT), BLACK),  # black patches at the shoulders
    (8, (5,), BLACK),  # a dark streak down the front of the neck
]


SCALE = 0.93  # built in its own numbers, then sized to a grey heron's ~100 cm standing

def build_heron():
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

    # Collect both base faces before growing anything: growing removes faces.
    legs = [(segments[LEG_SEGMENT][LOWER_RIGHT], 1), (segments[LEG_SEGMENT][LOWER_LEFT], -1)]
    for face, mirror in legs:
        b.limb(face, LEG, mirror)
    flora.branch(b, *PLUME, BLACK)
    return b.finish("Heron", shaded=True, scale=SCALE)


if __name__ == "__main__":
    loft.run(build_heron, "heron", target=(0.00, 0.28, 1.67), extent=3.72)

"""The wolf: a slightly chibi, low-poly Japanese wolf (ニホンオオカミ), standing.

    blender -b --python art/entity_models/living/animals/wolf.py -- --out assets/entity_models/living/animals/wolf.glb [--renders <dir>]

Still in the mountains round villages in 1550, and revered as a guardian (大口真神);
extinct since about 1905. The smallest of wolves, with shortish legs and small
ears: a tawny, sandy buff with a warmer brown saddle over the back, pale
cream-buff legs, face and underside, black eyes, a long wolfish muzzle, and a
bushy, dusky tail hanging down behind (after a preserved specimen).

Built with loft.py on the dog's layout at one and a half times the size, so it
keeps the dog's lessons: a flat, upright rump under the tail, hind legs dropping
straight from it, a tucked groin, and a level back. About 1.5 shaku (~46 cm, once scaled) to
the shoulder, feet on z = 0.
"""

import sys
from pathlib import Path

import bmesh
from mathutils import Vector

sys.path.insert(0, str(next(p for p in Path(__file__).resolve().parents if (p / "loft.py").exists())))
import loft  # noqa: E402
from loft import LOWER_LEFT, LOWER_RIGHT, UPPER_LEFT, UPPER_RIGHT  # noqa: E402

GREY, SADDLE, PALE, TAIL = "wolf_grey", "wolf_saddle", "wolf_pale", "wolf_tail"
SOOT, EYE = "soot", "eye_black"

# The spine, tail tip to nose: (y, z, half-width, half-height, colour, underside
# colour). See loft.Builder.spine.
TAIL_TIP = Vector((0, -1.35, 0.50))
SPINE = [
    # the tail: bushy, hanging down behind, dark at the tip
    (-1.35, 0.60, 0.12, 0.12, SOOT, SOOT),
    (-1.35, 0.80, 0.17, 0.17, TAIL, TAIL),
    (-1.33, 1.05, 0.19, 0.19, TAIL, TAIL),
    (-1.28, 1.30, 0.17, 0.17, TAIL, TAIL),
    (-1.185, 1.50, 0.13, 0.13, TAIL, TAIL),  # tail root
    # body: a flat, upright rump under the tail, a tucked-up flank, a deep belly;
    # the back level from the tail, rising a little to the shoulders
    (-1.14, 1.35, 0.225, 0.24, GREY, GREY),
    (-1.08, 1.29, 0.375, 0.39, GREY, GREY),  # the rump; the hind legs grow from the segment in front
    (-0.54, 1.2675, 0.405, 0.4275, GREY, PALE),  # the front of the hips, low to meet the thigh
    (-0.42, 1.38, 0.405, 0.33, GREY, PALE),  # the groin, tucked up just in front of the thigh
    (-0.09, 1.335, 0.42, 0.435, GREY, PALE),  # a pale belly; the front legs grow from the segment in front
    (0.39, 1.35, 0.405, 0.45, GREY, PALE),  # chest, under the withers
    # neck: thick, carrying the head low and forward
    (0.62, 1.52, 0.32, 0.33, GREY, PALE),
    (0.74, 1.74, 0.30, 0.30, GREY, PALE),
    # head: about 1.3x natural, with a long wolfish muzzle
    (0.80, 1.92, 0.345, 0.315, GREY, GREY),  # back of the skull
    (1.04, 1.96, 0.38, 0.315, GREY, PALE),  # cheeks, the widest point
    (1.28, 1.86, 0.25, 0.20, GREY, PALE),
    (1.46, 1.78, 0.16, 0.14, GREY, PALE),  # muzzle
    (1.56, 1.75, 0.10, 0.09, SOOT, SOOT),  # nose
]
NOSE_TIP = Vector((0, 1.61, 1.74))

RUMP_RINGS = (5, 6)  # stand upright, so the rump rounds off under the tail

# Which spine segment (index of its tail-side ring) and face each limb grows from.
# Each leg's first ring fits inside its base face, front, back and sides.
HIND_SEGMENT = 6  # rump -> hips: the back of the thigh drops straight from under the tail
FRONT_SEGMENT = 9  # belly -> chest
EAR_SEGMENT = 13  # back of the skull -> cheeks

# Limbs, for the right side: rings of (x, y, z, half-width, half-depth, colour).
# See loft.Builder.limb.
FRONT_LEG = [
    (0.195, 0.15, 0.75, 0.135, 0.225, GREY),  # shoulder, reaching down and back into the armpit
    (0.18, 0.18, 0.495, 0.105, 0.12, GREY),  # elbow
    (0.165, 0.21, 0.24, 0.0825, 0.09, PALE),  # wrist: pale lower legs
    (0.165, 0.24, 0.075, 0.0825, 0.09, PALE),
    (0.165, 0.285, 0.03, 0.0975, 0.12, PALE),  # paw
    (0.165, 0.285, 0.00, 0.0975, 0.12, PALE),  # sole
]
HIND_LEG = [
    (0.21, -0.825, 0.72, 0.15, 0.255, GREY),  # the haunch: as long as the body above it
    (0.195, -0.84, 0.45, 0.1125, 0.1275, GREY),  # narrowing to the knee
    (0.18, -0.915, 0.225, 0.0825, 0.09, PALE),  # hock, back under the rump: pale lower legs
    (0.18, -0.90, 0.075, 0.0825, 0.09, PALE),
    (0.18, -0.87, 0.03, 0.0975, 0.12, PALE),  # paw
    (0.18, -0.87, 0.00, 0.0975, 0.12, PALE),  # sole
]
EAR = [(0.255, 0.86, 2.30, 0.11, 0.06, GREY)]  # small, upright triangles
EAR_TIP = Vector((0.29, 0.88, 2.48))

# (spine segment, faces, colour); faces are numbered as in loft: 0 upper right,
# 1 top, 2 upper left, 3 left, 7 right.
MARKINGS = [
    (6, (1,), SADDLE),  # a darker saddle over the back
    (7, (0, 1, 2), SADDLE),
    (8, (0, 1, 2), SADDLE),
    (9, (0, 1, 2), SADDLE),
    (10, (1,), SADDLE),
    (14, (3, 7), PALE),  # pale cheeks and sides of the muzzle
    (15, (3, 7), PALE),
]
EYE_SEGMENT = 14  # cheeks -> muzzle: an eye on each upper side
EYE_SIZE = 0.065


SCALE = 0.84  # built in its own numbers, then sized to the museum specimens' ~46 cm at the shoulder

def build_wolf():
    b = loft.Builder()
    segments = b.spine(TAIL_TIP, SPINE, NOSE_TIP, upright=RUMP_RINGS)
    for segment, faces, colour in MARKINGS:
        b.paint([segments[segment][k] for k in faces], colour)
    for face, outward in ((segments[EYE_SEGMENT][UPPER_RIGHT], 1), (segments[EYE_SEGMENT][UPPER_LEFT], -1)):
        face.normal_update()
        normal = face.normal if face.normal.x * outward > 0 else -face.normal
        # Poke the face into four triangles round its centre first: the quad isn't
        # flat, and mirrored quads can split along different diagonals, which sinks
        # one eye deeper into the head than the other.
        centre = bmesh.ops.poke(b.bm, faces=[face])["verts"][0].co.copy()
        b.stud(centre, normal, EYE_SIZE, EYE)

    # Collect every base face before growing anything: growing removes faces.
    limbs = []
    for mirror, lower, upper in ((1, LOWER_RIGHT, UPPER_RIGHT), (-1, LOWER_LEFT, UPPER_LEFT)):
        limbs.append((segments[FRONT_SEGMENT][lower], FRONT_LEG, mirror, None))
        limbs.append((segments[HIND_SEGMENT][lower], HIND_LEG, mirror, None))
        limbs.append((segments[EAR_SEGMENT][upper], EAR, mirror, EAR_TIP))
    for face, rings, mirror, tip in limbs:
        b.limb(face, rings, mirror, tip)
    return b.finish("Wolf", shaded=True, scale=SCALE)


if __name__ == "__main__":
    loft.run(build_wolf, "wolf", target=(0.00, -0.04, 1.09), extent=2.86)

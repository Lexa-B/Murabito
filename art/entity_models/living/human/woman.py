"""The baseline village woman: a blockout of the body, for proportions.

    blender -b --python art/entity_models/living/human/woman.py -- \\
        --out assets/entity_models/living/human/woman.glb [--renders <dir>]

A lean, toned village woman of about 1550, 4.8 shaku (145 cm, the period's
average) and 6.5 heads tall, standing in an A-pose facing +Y. Built with
figure.py: a torso running up into the neck and head, legs split from its
bottom ring, arms bridged into holes in its sides, and hands with five fingers.
Faces, hair and undergarments come once the proportions are right.
"""

import math
import sys
from pathlib import Path

from mathutils import Vector

sys.path.insert(0, str(next(p for p in Path(__file__).resolve().parents if (p / "loft.py").exists())))
import figure  # noqa: E402
import loft  # noqa: E402
from figure import X, Y, Z  # noqa: E402

SKIN = "skin_tan"
HEIGHT = 4.8
HEAD = HEIGHT / 6.5

# The torso, crotch to crown: (z, y, half-width, front, back, extras). Rings have
# TORSO_SIDES vertices, starting TORSO_OFFSET degrees round from +X so there is
# a vertex dead ahead (90) and dead behind (270) for the legs to share.
TORSO_SIDES, TORSO_OFFSET = 18, 10
FRONT, BACK = 4, 13  # vertices at 90 and 270 degrees
TORSO = [
    (2.38, 0.00, 0.50, 0.235, 0.33, {}),  # hips: the legs split from here
    (2.49, 0.00, 0.50, 0.25, 0.37, {"bumps": [(240, 45, 0.07), (300, 45, 0.07)]}),  # seat, low
    (2.64, 0.00, 0.47, 0.245, 0.355, {"bumps": [(240, 45, 0.05), (300, 45, 0.05)]}),  # seat, high
    (2.78, 0.00, 0.44, 0.24, 0.30, {}),  # low belly
    (2.96, 0.00, 0.35, 0.24, 0.25, {}),  # waist
    (3.15, 0.00, 0.39, 0.28, 0.27, {}),  # ribs
    (3.34, 0.00, 0.42, 0.31, 0.29, {"bumps": [(65, 30, 0.06), (115, 30, 0.06)]}),  # bust
    (3.53, 0.00, 0.45, 0.29, 0.29, {}),  # armpits
    (3.70, 0.00, 0.44, 0.23, 0.25, {}),  # shoulders
    (3.80, 0.00, 0.32, 0.18, 0.20, {}),  # trapezius
    (3.90, -0.01, 0.17, 0.14, 0.15, {}),  # neck base
    (4.00, -0.02, 0.15, 0.13, 0.145, {}),  # neck
    (4.10, 0.02, 0.20, 0.22, 0.17, {"pinch": 0.55}),  # under the jaw, to the chin
    (4.22, 0.00, 0.27, 0.27, 0.26, {"pinch": 0.3}),  # jaw
    (4.36, 0.00, 0.305, 0.29, 0.30, {"pinch": 0.15}),  # cheeks
    (4.48, 0.00, 0.31, 0.28, 0.325, {"pinch": 0.05}),  # eyes
    (4.58, 0.00, 0.31, 0.27, 0.33, {}),  # brow
    (4.67, 0.00, 0.28, 0.245, 0.305, {}),  # temples
    (4.75, 0.00, 0.215, 0.18, 0.24, {}),  # crown
    (4.79, -0.01, 0.115, 0.095, 0.125, {}),  # top of the head
]
CROWN = (0, -0.02, 4.80)
CROTCH_DROP = 0.12  # the hips ring dips this much towards front and back, like a bikini line
ARMPIT, SHOULDER = 6, 8  # the arm hole runs from ring 6 to ring 8
ARM_FACES = (16, 17, 0)  # faces round +X left open for the right arm

# Legs: (x, y, z, half-width, front, back, extras), hip to ankle, 10 sides, then
# the foot, whose rings turn forward (axis, then front is its top).
LEG_SIDES = 10
LEG = [
    (0.26, -0.01, 2.09, 0.24, 0.245, 0.31, {}),  # upper thigh
    (0.25, 0.00, 1.78, 0.205, 0.235, 0.25, {}),
    (0.235, 0.00, 1.387, 0.135, 0.15, 0.13, {}),  # knee
    (0.23, 0.00, 1.18, 0.135, 0.13, 0.15, {}),
    (0.225, 0.00, 0.952, 0.145, 0.12, 0.15, {"bumps": [(270, 60, 0.03)]}),  # calf
    (0.22, 0.00, 0.60, 0.10, 0.09, 0.10, {}),
    (0.215, 0.00, 0.29, 0.08, 0.075, 0.075, {}),  # ankle
]
GROIN = [(0.35, 0.0), (0.7, 0.01)]  # rings easing the hips' share into the thigh: (part way, swell)
FOOT = [
    ((0.215, 0.00, 0.12), (0, -1, 1), 0.085, 0.11, 0.10),  # heel, turning forward
    ((0.215, 0.18, 0.085), (0, -1, 0), 0.10, 0.06, 0.085),  # instep
    ((0.215, 0.40, 0.05), (0, -1, 0), 0.10, 0.04, 0.05),  # ball of the foot
]
TOE = (0.215, 0.52, 0.04)

# Arms: A-pose, hanging ARM_DROP degrees below level from the shoulder joint.
ARM_DROP = 45
SHOULDER_JOINT = Vector((0.42, 0.0, 3.56))
ARM_SIDES = 10
# (distance along the arm, half-width across (y), front (back of the arm), back)
ARM = [
    (0.40, 0.12, 0.125, 0.125),
    (0.85, 0.09, 0.09, 0.09),  # elbow
    (1.05, 0.10, 0.095, 0.095),
    (1.55, 0.07, 0.045, 0.045),  # wrist
]
DELTOID = ((0.53, 0.0, 3.53), 0.13, 0.15, 0.15)  # first ring, facing out along +X
PALM = [(0.06, 0.085, 0.037, 0.033), (0.12, 0.09, 0.031, 0.027)]  # front is the back of the hand
KNUCKLES = 0.16  # in line with the thumb's middle knuckle
KNUCKLE_ROW = [0.09, 0.045, 0.0, -0.045, -0.09]  # across the knuckles in y, thumb side first
KNUCKLE_DEPTH = (0.026, 0.024)  # back of the hand, palm
FINGERS = [0.21, 0.24, 0.22, 0.17]  # lengths from the knuckles, index to little finger
FINGER_RADII = [0.022, 0.02, 0.018, 0.013]  # at the knuckle, the middle joint, the last joint, rounding the tip
FINGER_SIDES = 8  # twice the four corners of a finger's share of the knuckles
THUMB = [(0.03, 0.03), (0.09, 0.027), (0.16, 0.023), (0.195, 0.017)]  # (distance out, radius): base, first
# knuckle, middle knuckle, rounding the tip
THUMB_TIP = 0.21


def build_torso(b):
    rings = []
    for z, y, half_width, front, back, extras in TORSO:
        rings.append(figure.ring(b, (0, y, z), Z, X, TORSO_SIDES, half_width, front, back, TORSO_OFFSET, **extras))
    for k, vert in enumerate(rings[0]):
        vert.co.z -= CROTCH_DROP * math.sin(math.radians(TORSO_OFFSET) + 2 * math.pi * k / TORSO_SIDES) ** 2
    holes = set(ARM_FACES) | {mirrored(k) for k in ARM_FACES}
    n = TORSO_SIDES
    for i, (lower, upper) in enumerate(zip(rings, rings[1:])):
        for k in range(n):
            if ARMPIT <= i < SHOULDER and k in holes:
                continue
            b.face((lower[k], lower[(k + 1) % n], upper[(k + 1) % n], upper[k]), SKIN)
    figure.cap(b, rings[-1], CROWN, SKIN)
    return rings


def mirrored(face):
    """The torso face across from this one, mirrored in x."""
    return (TORSO_SIDES // 2 - 2 - face) % TORSO_SIDES


def arm_hole(b, rings, mirror):
    """The loop of vertices round one arm's hole, from the armpit ring up. The
    middle ring's vertices inside the hole belong to no face, so they go."""
    first, last = ARM_FACES[0], ARM_FACES[-1] + 1
    if mirror < 0:
        first, last = mirrored(ARM_FACES[-1]), mirrored(ARM_FACES[0]) + 1
    n = TORSO_SIDES
    across = [(first + j) % n for j in range((last - first) % n + 1)]
    low, mid, high = rings[ARMPIT], rings[ARMPIT + 1], rings[SHOULDER]
    for k in across[1:-1]:
        b.bm.verts.remove(mid[k])
    return [low[k] for k in across] + [mid[across[-1]]] + [high[k] for k in reversed(across)] + [mid[across[0]]]


def crotch_loop(hips, mirror):
    """One leg's share of the hips ring: its side, plus the front and back vertices."""
    n = TORSO_SIDES
    if mirror > 0:
        return [hips[(FRONT - j) % n] for j in range((FRONT - BACK) % n + 1)]
    return [hips[k] for k in range(FRONT, BACK + 1)]


def build_leg(b, hips, mirror):
    rings = []
    for x, y, z, half_width, front, back, extras in LEG:
        rings.append(figure.ring(b, (x * mirror, y, z), Z, X, LEG_SIDES, half_width, front, back, **extras))
    groin = crotch_loop(hips, mirror)
    eased = [figure.blend(b, groin, rings[0], t, swell) for t, swell in GROIN]
    rings = [groin] + eased + rings
    for (x, y, z), axis, half_width, top, sole in FOOT:
        rings.append(figure.ring(b, (x * mirror, y, z), axis, X, LEG_SIDES, half_width, top, sole))
    figure.tube(b, rings, SKIN)
    figure.cap(b, rings[-1], (TOE[0] * mirror, TOE[1], TOE[2]), SKIN)


def build_arm(b, torso, mirror):
    drop = math.radians(ARM_DROP)
    along = Vector((math.cos(drop) * mirror, 0, -math.sin(drop)))
    joint = Vector((SHOULDER_JOINT.x * mirror, SHOULDER_JOINT.y, SHOULDER_JOINT.z))
    (x, y, z), half_width, front, back = DELTOID
    rings = [arm_hole(b, torso, mirror)]
    rings.append(figure.ring(b, (x * mirror, y, z), X * mirror, Y * mirror, ARM_SIDES, half_width, front, back))
    for distance, half_width, front, back in ARM:
        rings.append(figure.ring(b, joint + along * distance, along, Y * mirror, ARM_SIDES, half_width, front, back))
    wrist = joint + along * ARM[-1][0]
    for distance, half_width, front, back in PALM:
        rings.append(figure.ring(b, wrist + along * distance, along, Y * mirror, ARM_SIDES, half_width, front, back))
    _, _, back_of_hand = figure.frame(along, Y * mirror)
    top, bottom = knuckle_row(b, wrist + along * KNUCKLES, back_of_hand)
    rings.append(top + bottom[::-1])
    rows = figure.tube(b, rings, SKIN)
    for j, length in enumerate(FINGERS):
        finger(b, [top[j], top[j + 1], bottom[j + 1], bottom[j]], along, back_of_hand, length)
    thumb(b, rows[len(ARM) + 1], wrist, along, back_of_hand)


def knuckle_row(b, centre, back_of_hand):
    """The end of the palm: a row of vertices across the back of the hand and
    one across the palm, evenly spaced, so each finger gets an equal share."""
    top, bottom = [], []
    for y in KNUCKLE_ROW:
        arch = 1 - 0.3 * (y / KNUCKLE_ROW[0]) ** 2
        top.append(b.bm.verts.new(centre + Y * y + back_of_hand * KNUCKLE_DEPTH[0] * arch))
        bottom.append(b.bm.verts.new(centre + Y * y - back_of_hand * KNUCKLE_DEPTH[1] * arch))
    return top, bottom


def finger(b, root, along, back_of_hand, length):
    """A finger grown from its square share of the knuckles: round from the
    first ring, then straight, then curling a little towards the palm."""
    base = sum((v.co for v in root), Vector()) / 4
    curl = (along - back_of_hand * 0.25).normalized()
    middle = base + along * length * 0.5
    last = middle + curl * length * 0.35
    points = [base + along * 0.025, middle, last, last + curl * length * 0.11]
    rings = [figure.ring(b, p, along if k < 2 else curl, Y, FINGER_SIDES, r, r * 1.05, r * 1.05)
             for k, (p, r) in enumerate(zip(points, FINGER_RADII))]
    figure.bridge_split(b, root, rings[0], SKIN)
    figure.tube(b, rings, SKIN)
    figure.cap(b, rings[-1], last + curl * length * 0.15, SKIN)


def thumb(b, row, wrist, along, back_of_hand):
    """The thumb, grown out of a hole opened in the two faces at the base of the
    palm on the thumb side (+y), below the palm's edge."""
    towards = (Y - back_of_hand * 0.7).normalized()
    score = {f: (f.calc_center_median() - wrist).dot(towards) for f in row}
    first = max(row, key=score.get)
    second = max((g for e in first.edges for g in e.link_faces if g in score and g is not first), key=score.get)
    root = figure.hole(b, [first, second])
    base = sum((v.co for v in root), Vector()) / len(root)
    out = (along * 0.8 + Y * 0.5 - back_of_hand * 0.45).normalized()
    rings = [figure.ring(b, base + out * d, out, along, len(root), r, r, r) for d, r in THUMB]
    figure.bridge(b, root, rings[0], SKIN)
    figure.tube(b, rings, SKIN)
    figure.cap(b, rings[-1], base + out * THUMB_TIP, SKIN)


def build_woman():
    b = loft.Builder()
    torso = build_torso(b)
    for mirror in (1, -1):
        build_leg(b, torso[0], mirror)
        build_arm(b, torso, mirror)
    return b.finish("woman")


if __name__ == "__main__":
    loft.run(build_woman, "woman", target=(0.00, 0.00, 2.40), extent=5.2)

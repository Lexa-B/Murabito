"""A human body from a Shape: the recipe every villager (and every humanoid
yokai built on them) shares, with the numbers left to each model.

    import body
    WOMAN = body.Shape(torso=[...], leg=[...], ...)
    obj = body.build(WOMAN, "woman")

One sealed mesh in an A-pose, facing +Y, feet on z = 0: a torso of rings
running up into the neck and head; legs split from its bottom ring along a
dip like a bikini line and eased into round thighs; arms bridged into holes
left in its sides; hands whose fingers grow from an even row of knuckles and
whose thumb grows from a hole at the base of the palm. See figure.py for the
rings and how they join.
"""

import math
from dataclasses import dataclass

from mathutils import Vector

import figure
import loft
from figure import X, Y, Z

# The topology every body shares. Torso rings have TORSO_SIDES vertices,
# starting TORSO_OFFSET degrees round from +X, so there is a vertex dead ahead
# (FRONT, 90) and dead behind (BACK, 270) for the legs to share.
TORSO_SIDES, TORSO_OFFSET = 18, 10
FRONT, BACK = 4, 13
ARM_FACES = (16, 17, 0)  # torso faces round +X left open for the right arm, over two rows
LEG_SIDES = 10  # half the hips ring, plus the front and back vertices
ARM_SIDES = 10  # the arm hole: three faces across, two rows up
FINGER_SIDES = 8  # twice the four corners of a finger's share of the knuckles


@dataclass
class Shape:
    """The numbers that make one body. Lengths in shaku.

    torso: rings crotch to crown, (z, y, half-width, front, back, extras), extras
        being figure.ring's pinch and bumps. The first is the hips ring the legs
        split from.
    armpit: the torso ring the arm hole starts at; it runs two rings up.
    crown: the point closing the top of the head.
    crotch_drop: how far the hips ring dips towards front and back.
    leg: rings hip to ankle, (x, y, z, half-width, front, back, extras), right leg.
    groin: rings easing the hips' share into the thigh, (part way, swell).
    foot: rings turning forward from the ankle, (centre, axis, half-width, top,
        sole); toe: the point closing it.
    arm_drop: degrees below level the arms hang from shoulder_joint.
    deltoid: the arm's first ring, facing out along +X: (centre, half-width,
        front, back).
    arm: rings along the arm, (distance, half-width across, front, back); the
        last is the wrist.
    palm: rings past the wrist, the same way (front is the back of the hand).
    knuckles: how far past the wrist the knuckle row is; knuckle_row: its
        vertices across the hand in y, thumb side first; knuckle_depth: (back of
        the hand, palm).
    fingers: lengths from the knuckles, index to little finger; finger_radii:
        at the knuckle, the middle joint, the last joint, and rounding the tip.
    thumb: rings out from its base, (distance, radius); thumb_tip: the point.
    shoulder_ease: rings easing the arm hole into the deltoid ring, (part way,
        swell), rounding the shoulder and armpit seam, as groin does for the legs.
    deltoid_tilt: how far the deltoid ring leans from facing straight out towards
        the arm's hang (0 to 1), so it neither caps the shoulder nor notches the
        armpit.
    """

    torso: list
    armpit: int
    crown: tuple
    crotch_drop: float
    leg: list
    groin: list
    foot: list
    toe: tuple
    arm_drop: float
    shoulder_joint: tuple
    deltoid: tuple
    arm: list
    palm: list
    knuckles: float
    knuckle_row: list
    knuckle_depth: tuple
    fingers: list
    finger_radii: list
    thumb: list
    thumb_tip: float
    skin: str = "skin_tan"
    shoulder_ease: list = ()
    deltoid_tilt: float = 0.0


def build(shape, name):
    b = loft.Builder()
    body = _Body(b, shape)
    torso = body.torso()
    for mirror in (1, -1):
        body.leg(torso[0], mirror)
        body.arm(torso, mirror)
    return b.finish(name)


def _mirrored(face):
    """The torso face across from this one, mirrored in x."""
    return (TORSO_SIDES // 2 - 2 - face) % TORSO_SIDES


class _Body:
    def __init__(self, b, shape):
        self.b, self.s = b, shape

    def torso(self):
        b, s = self.b, self.s
        rings = []
        for z, y, half_width, front, back, extras in s.torso:
            rings.append(figure.ring(b, (0, y, z), Z, X, TORSO_SIDES, half_width, front, back, TORSO_OFFSET, **extras))
        for k, vert in enumerate(rings[0]):
            vert.co.z -= s.crotch_drop * math.sin(math.radians(TORSO_OFFSET) + 2 * math.pi * k / TORSO_SIDES) ** 2
        holes = set(ARM_FACES) | {_mirrored(k) for k in ARM_FACES}
        n = TORSO_SIDES
        for i, (lower, upper) in enumerate(zip(rings, rings[1:])):
            for k in range(n):
                if s.armpit <= i < s.armpit + 2 and k in holes:
                    continue
                b.face((lower[k], lower[(k + 1) % n], upper[(k + 1) % n], upper[k]), s.skin)
        figure.cap(b, rings[-1], s.crown, s.skin)
        return rings

    def arm_hole(self, rings, mirror):
        """The loop of vertices round one arm's hole, from the armpit ring up. The
        middle ring's vertices inside the hole belong to no face, so they go."""
        first, last = ARM_FACES[0], ARM_FACES[-1] + 1
        if mirror < 0:
            first, last = _mirrored(ARM_FACES[-1]), _mirrored(ARM_FACES[0]) + 1
        n = TORSO_SIDES
        across = [(first + j) % n for j in range((last - first) % n + 1)]
        low, mid, high = rings[self.s.armpit], rings[self.s.armpit + 1], rings[self.s.armpit + 2]
        for k in across[1:-1]:
            self.b.bm.verts.remove(mid[k])
        return [low[k] for k in across] + [mid[across[-1]]] + [high[k] for k in reversed(across)] + [mid[across[0]]]

    @staticmethod
    def crotch_loop(hips, mirror):
        """One leg's share of the hips ring: its side, plus the front and back vertices."""
        n = TORSO_SIDES
        if mirror > 0:
            return [hips[(FRONT - j) % n] for j in range((FRONT - BACK) % n + 1)]
        return [hips[k] for k in range(FRONT, BACK + 1)]

    def leg(self, hips, mirror):
        b, s = self.b, self.s
        rings = []
        for x, y, z, half_width, front, back, extras in s.leg:
            rings.append(figure.ring(b, (x * mirror, y, z), Z, X, LEG_SIDES, half_width, front, back, **extras))
        groin = self.crotch_loop(hips, mirror)
        eased = [figure.blend(b, groin, rings[0], t, swell) for t, swell in s.groin]
        rings = [groin] + eased + rings
        for (x, y, z), axis, half_width, top, sole in s.foot:
            rings.append(figure.ring(b, (x * mirror, y, z), axis, X, LEG_SIDES, half_width, top, sole))
        figure.tube(b, rings, s.skin)
        figure.cap(b, rings[-1], (s.toe[0] * mirror, s.toe[1], s.toe[2]), s.skin)

    def arm(self, torso, mirror):
        b, s = self.b, self.s
        drop = math.radians(s.arm_drop)
        along = Vector((math.cos(drop) * mirror, 0, -math.sin(drop)))
        joint = Vector((s.shoulder_joint[0] * mirror, s.shoulder_joint[1], s.shoulder_joint[2]))
        (x, y, z), half_width, front, back = s.deltoid
        hole = self.arm_hole(torso, mirror)
        out = (X * mirror).lerp(along, s.deltoid_tilt)
        deltoid = figure.ring(b, (x * mirror, y, z), out, Y * mirror, ARM_SIDES, half_width, front, back)
        rings = [hole] + [figure.blend(b, hole, deltoid, t, swell) for t, swell in s.shoulder_ease] + [deltoid]
        for distance, half_width, front, back in s.arm:
            rings.append(figure.ring(b, joint + along * distance, along, Y * mirror, ARM_SIDES, half_width, front, back))
        wrist = joint + along * s.arm[-1][0]
        for distance, half_width, front, back in s.palm:
            rings.append(figure.ring(b, wrist + along * distance, along, Y * mirror, ARM_SIDES, half_width, front, back))
        _, _, back_of_hand = figure.frame(along, Y * mirror)
        top, bottom = self.knuckle_row(wrist + along * s.knuckles, back_of_hand)
        rings.append(top + bottom[::-1])
        rows = figure.tube(b, rings, s.skin)
        for j, length in enumerate(s.fingers):
            self.finger([top[j], top[j + 1], bottom[j + 1], bottom[j]], along, back_of_hand, length)
        self.thumb(rows[len(s.shoulder_ease) + len(s.arm) + 1], wrist, along, back_of_hand)

    def knuckle_row(self, centre, back_of_hand):
        """The end of the palm: a row of vertices across the back of the hand and
        one across the palm, evenly spaced, so each finger gets an equal share."""
        s = self.s
        top, bottom = [], []
        for y in s.knuckle_row:
            arch = 1 - 0.3 * (y / s.knuckle_row[0]) ** 2
            top.append(self.b.bm.verts.new(centre + Y * y + back_of_hand * s.knuckle_depth[0] * arch))
            bottom.append(self.b.bm.verts.new(centre + Y * y - back_of_hand * s.knuckle_depth[1] * arch))
        return top, bottom

    def finger(self, root, along, back_of_hand, length):
        """A finger grown from its square share of the knuckles: round from the
        first ring, then straight, then curling a little towards the palm."""
        b, s = self.b, self.s
        base = sum((v.co for v in root), Vector()) / 4
        curl = (along - back_of_hand * 0.25).normalized()
        middle = base + along * length * 0.5
        last = middle + curl * length * 0.35
        points = [base + along * 0.025, middle, last, last + curl * length * 0.11]
        rings = [
            figure.ring(b, p, along if k < 2 else curl, Y, FINGER_SIDES, r, r * 1.05, r * 1.05)
            for k, (p, r) in enumerate(zip(points, s.finger_radii))
        ]
        figure.bridge_split(b, root, rings[0], s.skin)
        figure.tube(b, rings, s.skin)
        figure.cap(b, rings[-1], last + curl * length * 0.15, s.skin)

    def thumb(self, row, wrist, along, back_of_hand):
        """The thumb, grown out of a hole opened in the two faces at the base of the
        palm on the thumb side (+y), below the palm's edge."""
        b, s = self.b, self.s
        towards = (Y - back_of_hand * 0.7).normalized()
        score = {f: (f.calc_center_median() - wrist).dot(towards) for f in row}
        first = max(row, key=score.get)
        second = max((g for e in first.edges for g in e.link_faces if g in score and g is not first), key=score.get)
        root = figure.hole(b, [first, second])
        base = sum((v.co for v in root), Vector()) / len(root)
        out = (along * 0.8 + Y * 0.5 - back_of_hand * 0.45).normalized()
        rings = [figure.ring(b, base + out * d, out, along, len(root), r, r, r) for d, r in s.thumb]
        figure.bridge(b, root, rings[0], s.skin)
        figure.tube(b, rings, s.skin)
        figure.cap(b, rings[-1], base + out * s.thumb_tip, s.skin)

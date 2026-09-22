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

It comes with a skeleton (rig.py), exported as a glTF skin. The bones carry
the standard humanoid names avatar tools use (VRM's: hips, spine, chest,
upperChest, neck, head, and per side shoulder, upperArm, lowerArm, hand, three
bones a finger, upperLeg, lowerLeg, foot, toes; the model's right is +X), so
every body built here can share animations. Every ring sits at a joint on
purpose: a ring between joints rides on one bone, and a joint's ring is shared
half and half by the bones either side of it.
"""

import math
from dataclasses import dataclass

from mathutils import Vector

import figure
import loft
import rig
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

TORSO_BONES = ("hips", "spine", "chest", "upperChest", "neck", "head")
FINGER_NAMES = ("Index", "Middle", "Ring", "Little")


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
    spine_joints: the torso ring each of TORSO_BONES after hips starts at (the
        ring the bone before shares with it).
    knee, elbow: which leg and arm ring is the joint.
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
    spine_joints: tuple = (3, 5, 7, 10, 12)
    knee: int = 2
    elbow: int = 1


def build(shape, name):
    """The body's mesh, bound to its skeleton."""
    b = loft.Builder()
    body = _Body(b, shape, rig.Rig(f"{name}-rig"))
    torso = body.torso()
    for mirror in (1, -1):
        body.leg(torso[0], mirror)
        body.arm(torso, mirror)
    b.bm.verts.index_update()
    for verts, pairs in body.weights:
        body.rig.weigh_blend([v.index for v in verts if v.is_valid], pairs)
    obj = b.finish(name)
    body.rig.bind(obj, default="hips")
    return obj


def _mirrored(face):
    """The torso face across from this one, mirrored in x."""
    return (TORSO_SIDES // 2 - 2 - face) % TORSO_SIDES


def _side(mirror):
    return "right" if mirror > 0 else "left"


def _centre(verts):
    return sum((v.co for v in verts), Vector()) / len(verts)


class _Body:
    def __init__(self, b, shape, skeleton):
        self.b, self.s, self.rig = b, shape, skeleton
        self.weights = []  # (vertices, [(bone, weight)]), turned into indices once built

    def weigh(self, verts, *pairs):
        self.weights.append((list(verts), list(pairs)))

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
        crown = figure.cap(b, rings[-1], s.crown, s.skin)
        self.spine_bones(rings)
        self.weigh(crown[0].verts[2:], ("head", 1))
        return rings

    def spine_bones(self, rings):
        """hips to head up the middle, each ring riding on the bone it's in, and
        each joint's ring shared by the bones either side."""
        s = self.s
        starts = (1,) + tuple(s.spine_joints)
        points = [Vector((0, s.torso[i][1], s.torso[i][0])) for i in starts] + [Vector(s.crown)]
        for k, name in enumerate(TORSO_BONES):
            self.rig.bone(name, points[k], points[k + 1], TORSO_BONES[k - 1] if k else None, k > 1, roll_to=Y)
        for i, ring in enumerate(rings):
            k = max([0] + [j for j, start in enumerate(s.spine_joints, 1) if start <= i])
            if k and i == s.spine_joints[k - 1]:
                self.weigh(ring, (TORSO_BONES[k - 1], 0.5), (TORSO_BONES[k], 0.5))
            else:
                self.weigh(ring, (TORSO_BONES[k], 1))

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
        side = _side(mirror)
        rings = []
        for x, y, z, half_width, front, back, extras in s.leg:
            rings.append(figure.ring(b, (x * mirror, y, z), Z, X, LEG_SIDES, half_width, front, back, **extras))
        groin = self.crotch_loop(hips, mirror)
        eased = [figure.blend(b, groin, rings[0], t, swell) for t, swell in s.groin]
        feet = []
        for (x, y, z), axis, half_width, top, sole in s.foot:
            feet.append(figure.ring(b, (x * mirror, y, z), axis, X, LEG_SIDES, half_width, top, sole))
        figure.tube(b, [groin] + eased + rings + feet, s.skin)
        toe = Vector((s.toe[0] * mirror, s.toe[1], s.toe[2]))
        tip = figure.cap(b, feet[-1], toe, s.skin)[0].verts[2]

        top = s.leg[0]
        hip = Vector((top[0] * mirror, 0, (s.torso[0][0] + top[2]) / 2))
        knee, ankle, ball = _centre(rings[s.knee]), _centre(rings[-1]), _centre(feet[-1])
        upper, lower, foot, toes = (f"{side}{n}" for n in ("UpperLeg", "LowerLeg", "Foot", "Toes"))
        self.rig.bone(upper, hip, knee, "hips", roll_to=Y)
        self.rig.bone(lower, knee, ankle, upper, True, roll_to=Y)
        self.rig.bone(foot, ankle, ball, lower, True, roll_to=Z)
        self.rig.bone(toes, ball, toe, foot, True, roll_to=Z)

        for (t, _), ring in zip(s.groin, eased):
            self.weigh(ring, ("hips", 1 - t), (upper, t))
        for i, ring in enumerate(rings):
            if i < s.knee:
                self.weigh(ring, (upper, 1))
            elif i == s.knee:
                self.weigh(ring, (upper, 0.5), (lower, 0.5))
            elif i < len(rings) - 1:
                self.weigh(ring, (lower, 1))
            else:
                self.weigh(ring, (lower, 0.5), (foot, 0.5))
        for ring in feet[:-1]:
            self.weigh(ring, (foot, 1))
        self.weigh(feet[-1], (foot, 0.5), (toes, 0.5))
        self.weigh([tip], (toes, 1))

    def arm(self, torso, mirror):
        b, s = self.b, self.s
        side = _side(mirror)
        drop = math.radians(s.arm_drop)
        along = Vector((math.cos(drop) * mirror, 0, -math.sin(drop)))
        joint = Vector((s.shoulder_joint[0] * mirror, s.shoulder_joint[1], s.shoulder_joint[2]))
        (x, y, z), half_width, front, back = s.deltoid
        hole = self.arm_hole(torso, mirror)
        out = (X * mirror).lerp(along, s.deltoid_tilt)
        deltoid = figure.ring(b, (x * mirror, y, z), out, Y * mirror, ARM_SIDES, half_width, front, back)
        eased = [figure.blend(b, hole, deltoid, t, swell) for t, swell in s.shoulder_ease]
        arm = [figure.ring(b, joint + along * d, along, Y * mirror, ARM_SIDES, w, f, k) for d, w, f, k in s.arm]
        wrist = joint + along * s.arm[-1][0]
        palm = [figure.ring(b, wrist + along * d, along, Y * mirror, ARM_SIDES, w, f, k) for d, w, f, k in s.palm]
        _, _, back_of_hand = figure.frame(along, Y * mirror)
        top, bottom = self.knuckle_row(wrist + along * s.knuckles, back_of_hand)
        rows = figure.tube(b, [hole] + eased + [deltoid] + arm + palm + [top + bottom[::-1]], s.skin)

        shoulder, upper, lower, hand = (f"{side}{n}" for n in ("Shoulder", "UpperArm", "LowerArm", "Hand"))
        elbow = joint + along * s.arm[s.elbow][0]
        knuckles = wrist + along * s.knuckles
        neck_base = Vector((0.1 * mirror, 0, s.torso[s.armpit + 3][0]))
        self.rig.bone(shoulder, neck_base, joint, "upperChest", roll_to=Y)
        self.rig.bone(upper, joint, elbow, shoulder, True, roll_to=Y)
        self.rig.bone(lower, elbow, wrist, upper, True, roll_to=Y)
        self.rig.bone(hand, wrist, knuckles, lower, True, roll_to=back_of_hand)

        self.weigh(hole, (shoulder, 1))  # shared with the torso's own weights
        for (t, _), ring in zip(s.shoulder_ease, eased):
            self.weigh(ring, (shoulder, 1 - t), (upper, t))
        self.weigh(deltoid, (shoulder, 0.3), (upper, 0.7))
        for i, ring in enumerate(arm):
            if i < s.elbow:
                self.weigh(ring, (upper, 1))
            elif i == s.elbow:
                self.weigh(ring, (upper, 0.5), (lower, 0.5))
            elif i < len(arm) - 1:
                self.weigh(ring, (lower, 1))
            else:
                self.weigh(ring, (lower, 0.5), (hand, 0.5))
        for ring in palm:
            self.weigh(ring, (hand, 1))
        self.weigh(top + bottom, (hand, 1))

        for j, length in enumerate(s.fingers):
            root = [top[j], top[j + 1], bottom[j + 1], bottom[j]]
            self.finger(root, along, back_of_hand, length, f"{side}{FINGER_NAMES[j]}", hand)
        self.thumb(rows[len(s.shoulder_ease) + len(s.arm) + 1], wrist, along, back_of_hand, f"{side}Thumb", hand)

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

    def finger(self, root, along, back_of_hand, length, name, hand):
        """A finger grown from its square share of the knuckles: round from the
        first ring, then straight, then curling a little towards the palm. Three
        bones: knuckle to middle joint, middle to last joint, last joint to tip."""
        b, s = self.b, self.s
        base = _centre(root)
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
        end = last + curl * length * 0.15
        tip = figure.cap(b, rings[-1], end, s.skin)[0].verts[2]

        bones = [f"{name}{part}" for part in ("Proximal", "Intermediate", "Distal")]
        self.bones_along(bones, [base, middle, last, end], hand, back_of_hand)
        self.weigh(root, (bones[0], 0.5))  # with the hand's own weight on the knuckles
        self.weigh(rings[0], (bones[0], 1))
        self.weigh(rings[1], (bones[0], 0.5), (bones[1], 0.5))
        self.weigh(rings[2], (bones[1], 0.5), (bones[2], 0.5))
        self.weigh(rings[3] + [tip], (bones[2], 1))

    def thumb(self, row, wrist, along, back_of_hand, name, hand):
        """The thumb, grown out of a hole opened in the two faces at the base of the
        palm on the thumb side (+y), below the palm's edge. Three bones: its root
        in the palm, then its two knuckles."""
        b, s = self.b, self.s
        towards = (Y - back_of_hand * 0.7).normalized()
        score = {f: (f.calc_center_median() - wrist).dot(towards) for f in row}
        first = max(row, key=score.get)
        second = max((g for e in first.edges for g in e.link_faces if g in score and g is not first), key=score.get)
        root = figure.hole(b, [first, second])
        base = _centre(root)
        out = (along * 0.8 + Y * 0.5 - back_of_hand * 0.45).normalized()
        rings = [figure.ring(b, base + out * d, out, along, len(root), r, r, r) for d, r in s.thumb]
        figure.bridge(b, root, rings[0], s.skin)
        figure.tube(b, rings, s.skin)
        end = base + out * s.thumb_tip
        tip = figure.cap(b, rings[-1], end, s.skin)[0].verts[2]

        bones = [f"{name}{part}" for part in ("Metacarpal", "Proximal", "Distal")]
        joints = [base - out * 0.04, base + out * s.thumb[1][0], base + out * s.thumb[2][0], end]
        self.bones_along(bones, joints, hand, back_of_hand)
        self.weigh(root, (bones[0], 1))  # with the hand's own weight round the hole
        self.weigh(rings[0], (bones[0], 1))
        self.weigh(rings[1], (bones[0], 0.5), (bones[1], 0.5))
        self.weigh(rings[2], (bones[1], 0.5), (bones[2], 0.5))
        self.weigh(rings[3] + [tip], (bones[2], 1))

    def bones_along(self, names, points, parent, roll_to):
        for k, name in enumerate(names):
            self.rig.bone(name, points[k], points[k + 1], names[k - 1] if k else parent, k > 0, roll_to=roll_to)

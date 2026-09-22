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

import bmesh

from mathutils import Vector
from mathutils.bvhtree import BVHTree

import figure
import loft
import rig
from figure import X, Y, Z

# The topology every body shares. Torso rings have TORSO_SIDES vertices,
# starting TORSO_OFFSET degrees round from +X, so there is a vertex dead ahead
# (FRONT, 90) and dead behind (BACK, 270) for the legs to share.
TORSO_SIDES, TORSO_OFFSET = 30, 6
_STEP = 360 / TORSO_SIDES
FRONT, BACK = round((90 - TORSO_OFFSET) / _STEP), round((270 - TORSO_OFFSET) / _STEP)
ARM_FACES = (27, 28, 29, 0, 1)  # torso faces round +X left open for the right arm
ARM_ROWS = 4  # torso rows the arm hole spans, armpit to shoulder
LEG_SIDES = TORSO_SIDES // 2 + 1  # half the hips ring, front and back vertices included
ARM_SIDES = 2 * (len(ARM_FACES) + ARM_ROWS)  # round the arm hole
FINGER_SIDES = 6  # a finger's share of the knuckles: two gaps across the back, two across the palm
STEPS = 2  # each gap between key rings is cut in two, on a smooth curve through them
JOINT_ZONE = 1.0  # a joint's bend fades across this many key gaps each way

# An eye's outline, round from the inner corner, in half-widths across (out from
# the nose) and up: a flat top, a sharp outer corner lifted a little, a slanting
# lower edge, the '90s anime eye.
EYE_OUTLINE = [
    (-1.0, -0.05), (-0.75, 0.5), (-0.3, 0.8), (0.25, 0.85), (0.7, 0.62), (1.05, 0.2),
    (0.85, -0.2), (0.45, -0.55), (-0.1, -0.7), (-0.6, -0.5),
]
EYE_CORNER = 5  # the outer corner, where the upper and lower lash lines meet

TORSO_BONES = ("hips", "spine", "chest", "upperChest", "neck", "head")
FINGER_NAMES = ("Index", "Middle", "Ring", "Little")


@dataclass
class Face:
    """Where a face's features sit on the head and how big they are. Positions
    are (x, z) as seen from straight ahead, x out from the middle on the right
    side (the left is its mirror); each feature is placed where a ray from in
    front meets the head there.

    eye: an eye's centre; sclera: (half-width, height over width), the white
        drawn round outline; iris: (half-width, height over width, lift), lift
        moving it up the eye.
    lash: the width of the dark line along the top of each eye, thickest at
        the outer corner.
    outline: the eye's shape (see EYE_OUTLINE).
    brow: points inner to outer, (x, z, width).
    nose: (top, tip, bottom, out): the nose is shaped out of the head itself,
        the middle of each ring pulled forward, from nothing at top to out at
        tip and back to nothing at bottom, its neighbours a little.
    mouth: (z, half-length, width): a short crease.
    ear: rings bottom to top, (z, y, half-depth, out): out is how far it stands
        from the side of the head.
    """

    eye: tuple
    sclera: tuple
    iris: tuple
    lash: float
    brow: list
    nose: tuple
    mouth: tuple
    ear: list
    outline: list = None


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
    face: Face = None


def build(shape, name):
    """The body's mesh, bound to its skeleton."""
    b = loft.Builder()
    body = _Body(b, shape, rig.Rig(f"{name}-rig"))
    torso = body.torso()
    if shape.face:
        body.face()
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
    return (round((180 - 2 * TORSO_OFFSET) / _STEP) - 1 - face) % TORSO_SIDES


def _chain_weights(where, joints, bones):
    """Which bones a ring rides on, from its position along a chain counted in
    key rings: a ring at a joint is shared half and half, and the share fades
    to nothing JOINT_ZONE either way, so a bend spreads over several rings."""
    for j, at in enumerate(joints):
        if abs(where - at) < JOINT_ZONE:
            t = 0.5 + (where - at) / (2 * JOINT_ZONE)
            return [(bones[j], 1 - t), (bones[j + 1], t)]
    return [(bones[sum(at <= where for at in joints)], 1)]


def _bump_keys(extras):
    """Every (angle, spread) bump any key ring has, so each can fade in and out."""
    return sorted({(angle, spread) for e in extras for angle, spread, _ in e.get("bumps", ())})


def _key(values, extras, bumps):
    """A key ring as a dict densify can curve through: its numbers, its pinch and
    the amount of each bump (0 where it has none)."""
    amounts = {(angle, spread): amount for angle, spread, amount in extras.get("bumps", ())}
    key = dict(values, pinch=extras.get("pinch", 0.0))
    key.update({bump: amounts.get(bump, 0.0) for bump in bumps})
    return key


def _shape_of(ring, bumps):
    """figure.ring's pinch and bumps for a densified ring."""
    return {
        "pinch": max(ring["pinch"], 0.0),
        "bumps": [(angle, spread, ring[(angle, spread)]) for angle, spread in bumps if ring[(angle, spread)] > 1e-4],
    }


def _side(mirror):
    return "right" if mirror > 0 else "left"


def _centre(verts):
    return sum((v.co for v in verts), Vector()) / len(verts)


class _Body:
    def __init__(self, b, shape, skeleton):
        self.b, self.s, self.rig = b, shape, skeleton
        self.weights = []  # (vertices, [(bone, weight)]), turned into indices once built
        self.torso_faces = {}  # (row, k): the torso's face k between ring row and the ring above

    def weigh(self, verts, *pairs):
        self.weights.append((list(verts), list(pairs)))

    def torso(self):
        b, s = self.b, self.s
        bumps = _bump_keys(e for *_, e in s.torso)
        keys = [_key(dict(z=z, y=y, hw=w, front=f, back=k), e, bumps) for z, y, w, f, k, e in s.torso]
        dense, where = figure.densify(keys, [STEPS] * (len(keys) - 1))
        rings = [
            figure.ring(b, (0, r["y"], r["z"]), Z, X, TORSO_SIDES, r["hw"], r["front"], r["back"], TORSO_OFFSET,
                        **_shape_of(r, bumps))
            for r in dense
        ]
        for k, vert in enumerate(rings[0]):
            vert.co.z -= s.crotch_drop * math.sin(math.radians(TORSO_OFFSET) + 2 * math.pi * k / TORSO_SIDES) ** 2
        self.hole_rows = (where.index(float(s.armpit)), where.index(float(s.armpit + 2)))
        assert self.hole_rows[1] - self.hole_rows[0] == ARM_ROWS
        holes = set(ARM_FACES) | {_mirrored(k) for k in ARM_FACES}
        n = TORSO_SIDES
        for i, (lower, upper) in enumerate(zip(rings, rings[1:])):
            for k in range(n):
                if self.hole_rows[0] <= i < self.hole_rows[1] and k in holes:
                    continue
                self.torso_faces[i, k] = b.face((lower[k], lower[(k + 1) % n], upper[(k + 1) % n], upper[k]), s.skin)
        self.rings = rings
        crown = figure.cap(b, rings[-1], s.crown, s.skin)
        self.spine_bones(rings, where)
        self.weigh(crown[0].verts[2:], ("head", 1))
        return rings

    def nose(self, rings):
        """A small nose out of the face: the two faces either side of the middle,
        over the rows from under the eyes to the tip, get a ring of vertices inset
        just inside their edge, and the patch within is pushed forward (most at
        the tip, least at the bridge) and squeezed narrow at the top, so the band
        between the ring and the patch makes the nose's sides and underside."""
        top, tip, bottom, out = self.s.face.nose
        rows = [i for i, ring in enumerate(rings[:-1]) if bottom - 0.01 <= ring[FRONT].co.z and rings[i + 1][FRONT].co.z <= top + 0.01]
        patch = [self.torso_faces[i, k] for i in rows for k in (FRONT - 1, FRONT)]
        base = {v for f in patch for v in f.verts}  # stays behind as the ring round the nose's foot
        bmesh.ops.inset_region(self.b.bm, faces=patch, thickness=0.006, depth=0.0, use_even_offset=True)
        for vert in base:
            vert.co.x *= 0.8  # a smaller foot
        rise = (tip - bottom) / (top - bottom)  # where the tip sits, 0 at the bottom, 1 at the top
        for vert in {v for f in patch for v in f.verts}:
            t = min(max((vert.co.z - bottom) / (top - bottom), 0.0), 1.0)
            if t >= rise:
                pull = (1 - t) / (1 - rise)  # the bridge, flush at the top, rising to the tip
            else:
                pull = 0.35 + 0.65 * t / rise  # the underside, tucked back
                vert.co.z += 0.012 * (1 - t / rise)
            vert.co.y += out * pull
            fullness = max(1 - abs(t - rise) / 0.5, 0.0)  # widest round the tip
            vert.co.x *= 0.35 - 0.15 * t + 0.15 * fullness  # drawn in to the middle: a narrow bridge, a button tip

    def spine_bones(self, rings, where):
        """hips to head up the middle, each ring riding on the bones round it."""
        s = self.s
        starts = (1,) + tuple(s.spine_joints)
        points = [Vector((0, s.torso[i][1], s.torso[i][0])) for i in starts] + [Vector(s.crown)]
        for k, name in enumerate(TORSO_BONES):
            self.rig.bone(name, points[k], points[k + 1], TORSO_BONES[k - 1] if k else None, k > 1, roll_to=Y)
        for ring, at in zip(rings, where):
            self.weigh(ring, *_chain_weights(at, s.spine_joints, TORSO_BONES))

    def face(self):
        """Eyes, brows, nose, mouth and ears, each its own small closed piece set on
        the head where a ray from in front (or beside, for the ears) meets it,
        all riding on the head bone."""
        b, f = self.b, self.s.face
        before = set(b.bm.verts)
        self.nose(self.rings)
        b.bm.normal_update()  # the rays read the faces' normals
        tree = BVHTree.FromBMesh(b.bm)

        def surface(origin, direction):
            hit, normal, *_ = tree.ray_cast(Vector(origin), Vector(direction))
            return hit, normal if normal.dot(direction) < 0 else -normal

        def front(x, z):
            return surface((x, 3, z), (0, -1, 0))

        for mirror in (1, -1):
            self.eye(front, mirror)
            points, normals = zip(*(front(x * mirror, z) for x, z, _ in f.brow))
            figure.strip(b, list(points), list(normals), [w for *_, w in f.brow], 0.008, "hair_black")
            self.ear(surface, mirror)
        z, half, width = f.mouth
        points, normals = zip(*(front(half * (2 * k / 4 - 1), z) for k in range(5)))
        figure.strip(b, list(points), list(normals), [width * w for w in (0.5, 1, 1, 1, 0.5)], 0.005, "mouth_line")
        self.weigh([v for v in b.bm.verts if v not in before], ("head", 1))

    def eye(self, front, mirror):
        """A white in the eye's outline, a dark iris with a glint on it (the same
        side on both eyes, as from one light) and a lash line along the top, each
        draped over the head so none of it sinks in."""
        f = self.s.face
        x, z = f.eye
        width, tall = f.sclera
        iris, iris_tall, iris_lift = f.iris
        outline = [(x * mirror + u * width * mirror, z + v * width * tall) for u, v in (f.outline or EYE_OUTLINE)]
        self.decal(front, outline, 0.003, 0.002, "sclera")
        middle = (x * mirror, z + iris_lift)
        oval = [(middle[0] + math.cos(t) * iris, middle[1] + math.sin(t) * iris * iris_tall)
                for t in (2 * math.pi * k / 12 for k in range(12))]
        self.decal(front, oval, 0.0055, 0.001, "iris_brown")
        glint = (middle[0] + iris * 0.35, middle[1] + iris * iris_tall * 0.4)
        spot = [(glint[0] + math.cos(t) * iris * 0.3, glint[1] + math.sin(t) * iris * 0.3)
                for t in (2 * math.pi * k / 6 for k in range(6))]
        self.decal(front, spot, 0.0075, 0.001, "sclera")

        # The lash lines meet in a sideways V at a point just past the outer corner:
        # the upper one thick along the top, the lower just a hint, a short taper
        # back under the corner of the white.
        corner = outline[EYE_CORNER]
        tip = (corner[0] + mirror * width * 0.15, corner[1] + width * tall * 0.02)
        upper = outline[:EYE_CORNER] + [tip]
        lower = [tip, outline[EYE_CORNER + 1]]
        self.lash_line(front, upper, [f.lash * (0.45 + 0.75 * k / (len(upper) - 1)) for k in range(len(upper) - 1)]
                       + [f.lash * 0.35], 0.008)
        self.lash_line(front, lower, [f.lash * 0.35, f.lash * 0.2], 0.006)

    def lash_line(self, front, outline, widths, thickness):
        points, normals = [], []
        for u, v in outline:
            point, normal = front(u, v)
            points.append(point + normal * 0.003)
            normals.append(normal)
        figure.strip(self.b, points, normals, widths, thickness, "hair_black")

    def decal(self, front, outline, lift, height, colour):
        """A thin patch draped over the head: each corner of outline (x, z as seen
        from ahead) set lift above the head there, rising to height more at its
        middle, and closed underneath by a point sunk into the head, so it has an
        inside and its faces turn the right way out (a flat patch can come out
        inside out)."""
        b = self.b
        corners = []
        for x, z in outline:
            point, normal = front(x, z)
            corners.append(b.bm.verts.new(point + normal * lift))
        mx = sum(x for x, _ in outline) / len(outline)
        mz = sum(z for _, z in outline) / len(outline)
        point, normal = front(mx, mz)
        apex = b.bm.verts.new(point + normal * (lift + height))
        root = b.bm.verts.new(point - normal * 0.01)
        n = len(corners)
        for k in range(n):
            b.face((apex, corners[k], corners[(k + 1) % n]), colour)
            b.face((root, corners[(k + 1) % n], corners[k]), colour)

    def ear(self, surface, mirror):
        """A shell of rings up the side of the head, standing a little out from it."""
        b, s = self.b, self.s
        rings, centres = [], []
        for z, y, depth, out in s.face.ear:
            hit, _ = surface((3 * mirror, y, z), (-mirror, 0, 0))
            centre = hit + X * mirror * out
            centres.append(centre)
            rings.append(figure.ring(b, centre, Z, X * mirror, 8, 0.03, depth * 0.8, depth))
        figure.tube(b, rings, s.skin)
        figure.cap(b, rings[0], centres[0] - Z * 0.015, s.skin)
        figure.cap(b, rings[-1], centres[-1] + Z * 0.01 - Y * 0.01, s.skin)

    def arm_hole(self, rings, mirror):
        """The loop of vertices round one arm's hole, from the armpit ring up. The
        middle rings' vertices inside the hole belong to no face, so they go."""
        n = TORSO_SIDES
        first = ARM_FACES[0] if mirror > 0 else _mirrored(ARM_FACES[-1])
        across = [(first + j) % n for j in range(len(ARM_FACES) + 1)]
        low, high = rings[self.hole_rows[0]], rings[self.hole_rows[1]]
        mids = rings[self.hole_rows[0] + 1 : self.hole_rows[1]]
        for mid in mids:
            for k in across[1:-1]:
                self.b.bm.verts.remove(mid[k])
        return (
            [low[k] for k in across]
            + [mid[across[-1]] for mid in mids]
            + [high[k] for k in reversed(across)]
            + [mid[across[0]] for mid in reversed(mids)]
        )

    @staticmethod
    def crotch_loop(hips, mirror):
        """One leg's share of the hips ring: its side, plus the front and back vertices."""
        n = TORSO_SIDES
        if mirror > 0:
            return [hips[(FRONT - j) % n] for j in range((FRONT - BACK) % n + 1)]
        return [hips[k] for k in range(FRONT, BACK + 1)]

    def leg(self, hips, mirror):
        """Hip to toe as one chain of key rings (the leg's, then the foot's turning
        forward), curved through and densified, split from the hips ring."""
        b, s = self.b, self.s
        side = _side(mirror)
        bumps = _bump_keys(e for *_, e in s.leg)
        keys = [
            _key(dict(c=Vector((x * mirror, y, z)), axis=Z.copy(), hw=w, front=f, back=k), e, bumps)
            for x, y, z, w, f, k, e in s.leg
        ] + [
            _key(dict(c=Vector((x * mirror, y, z)), axis=Vector(axis).normalized(), hw=w, front=top, back=sole), {}, bumps)
            for (x, y, z), axis, w, top, sole in s.foot
        ]
        dense, where = figure.densify(keys, [STEPS] * (len(keys) - 1))
        rings = [
            figure.ring(b, r["c"], r["axis"], X, LEG_SIDES, r["hw"], r["front"], r["back"], **_shape_of(r, bumps))
            for r in dense
        ]
        groin = self.crotch_loop(hips, mirror)
        eased = [figure.blend(b, groin, rings[0], t, swell) for t, swell in s.groin]
        figure.tube(b, [groin] + eased + rings, s.skin)
        toe = Vector((s.toe[0] * mirror, s.toe[1], s.toe[2]))
        tip = figure.cap(b, rings[-1], toe, s.skin)[0].verts[2]

        top = s.leg[0]
        ankle_key, ball_key = len(s.leg) - 1, len(keys) - 1
        hip = Vector((top[0] * mirror, 0, (s.torso[0][0] + top[2]) / 2))
        knee, ankle, ball = keys[s.knee]["c"], keys[ankle_key]["c"], keys[ball_key]["c"]
        bones = [f"{side}{n}" for n in ("UpperLeg", "LowerLeg", "Foot", "Toes")]
        upper, lower, foot, toes = bones
        self.rig.bone(upper, hip, knee, "hips", roll_to=Y)
        self.rig.bone(lower, knee, ankle, upper, True, roll_to=Y)
        self.rig.bone(foot, ankle, ball, lower, True, roll_to=Z)
        self.rig.bone(toes, ball, toe, foot, True, roll_to=Z)

        for (t, _), ring in zip(s.groin, eased):
            self.weigh(ring, ("hips", 1 - t), (upper, t))
        for ring, at in zip(rings, where):
            self.weigh(ring, *_chain_weights(at, (s.knee, ankle_key, ball_key), bones))
        self.weigh([tip], (toes, 1))

    def arm(self, torso, mirror):
        """Deltoid to palm as one chain of key rings, curved through and densified,
        bridged into the torso's arm hole and ending in a row of knuckles."""
        b, s = self.b, self.s
        side = _side(mirror)
        drop = math.radians(s.arm_drop)
        along = Vector((math.cos(drop) * mirror, 0, -math.sin(drop)))
        joint = Vector((s.shoulder_joint[0] * mirror, s.shoulder_joint[1], s.shoulder_joint[2]))
        wrist = joint + along * s.arm[-1][0]
        (x, y, z), w, f, k = s.deltoid
        out = (X * mirror).lerp(along, s.deltoid_tilt).normalized()
        keys = [dict(c=Vector((x * mirror, y, z)), axis=out, hw=w, front=f, back=k)]
        keys += [dict(c=joint + along * d, axis=along.copy(), hw=w, front=f, back=k) for d, w, f, k in s.arm]
        keys += [dict(c=wrist + along * d, axis=along.copy(), hw=w, front=f, back=k) for d, w, f, k in s.palm]
        steps = [STEPS] * len(s.arm) + [1] * len(s.palm)  # the palm's rings are close enough already
        dense, where = figure.densify(keys, steps)
        chain = [figure.ring(b, r["c"], r["axis"], Y * mirror, ARM_SIDES, r["hw"], r["front"], r["back"]) for r in dense]
        hole = self.arm_hole(torso, mirror)
        eased = [figure.blend(b, hole, chain[0], t, swell) for t, swell in s.shoulder_ease]
        _, _, back_of_hand = figure.frame(along, Y * mirror)
        top, bottom = self.knuckle_row(wrist + along * s.knuckles, back_of_hand)
        loops = [hole] + eased + chain + [top + bottom[::-1]]
        rows = figure.tube(b, loops, s.skin)

        bones = [f"{side}{n}" for n in ("UpperArm", "LowerArm", "Hand")]
        shoulder, (upper, lower, hand) = f"{side}Shoulder", bones
        elbow = joint + along * s.arm[s.elbow][0]
        neck_base = Vector((0.1 * mirror, 0, s.torso[s.armpit + 3][0]))
        self.rig.bone(shoulder, neck_base, joint, "upperChest", roll_to=Y)
        self.rig.bone(upper, joint, elbow, shoulder, True, roll_to=Y)
        self.rig.bone(lower, elbow, wrist, upper, True, roll_to=Y)
        self.rig.bone(hand, wrist, wrist + along * s.knuckles, lower, True, roll_to=back_of_hand)

        self.weigh(hole, (shoulder, 1))  # shared with the torso's own weights
        for (t, _), ring in zip(s.shoulder_ease, eased):
            self.weigh(ring, (shoulder, 1 - t), (upper, t))
        wrist_key = len(s.arm)
        for ring, at in zip(chain, where):
            self.weigh(ring, *_chain_weights(at, (1 + s.elbow, wrist_key), bones))
        self.weigh(chain[0], (shoulder, 0.4))  # the deltoid, lifting with the shoulder
        self.weigh(top + bottom, (hand, 1))

        for j, length in enumerate(s.fingers):
            root = top[2 * j : 2 * j + 3] + bottom[2 * j : 2 * j + 3][::-1]
            self.finger(root, along, back_of_hand, length, f"{side}{FINGER_NAMES[j]}", hand)
        wrist_loop = loops.index(chain[where.index(float(wrist_key))])
        self.thumb(rows[wrist_loop], wrist, along, back_of_hand, f"{side}Thumb", hand)

    def knuckle_row(self, centre, back_of_hand):
        """The end of the palm: a row of vertices across the back of the hand and
        one across the palm, the fingers' edges (knuckle_row) with one more between
        each pair, so each finger gets an equal share two gaps wide."""
        s = self.s
        edges = s.knuckle_row
        across = [edges[j // 2] if j % 2 == 0 else (edges[j // 2] + edges[j // 2 + 1]) / 2 for j in range(2 * len(edges) - 1)]
        top, bottom = [], []
        for y in across:
            arch = 1 - 0.3 * (y / edges[0]) ** 2
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
        figure.bridge(b, root, rings[0], s.skin)
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

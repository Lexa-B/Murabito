"""The fox: a slightly chibi, low-poly red fox in a neutral standing pose.

    blender -b --python art/fox.py -- --out assets/models/fox.glb [--renders <dir>]

One connected mesh, built so a skeleton can bend it later: a single loft runs
from the tail tip to the nose, and the legs and ears are lofted out of faces of
that body, with a ring of vertices at every joint (shoulder, elbow, wrist, hip,
knee, hock, neck, tail).

Coordinates are Blender's, in shaku: +Y forward, +Z up, +X the fox's right,
feet on z = 0. Shoulder height is about 1.35 shaku (~41 cm).
"""

import argparse
import math
import sys
from pathlib import Path

import bmesh
import bpy
from mathutils import Vector

sys.path.insert(0, str(Path(__file__).resolve().parent))
import style  # noqa: E402

ORANGE, CREAM, SOOT = "fox_orange", "cream", "soot"

# Rings are octagons around the spine. Vertex k sits at 22.5 + 45k degrees, so
# face k of a ring segment is centred on 45 + 45k degrees: 0 upper right,
# 1 top, 2 upper left, 3 left, 4 lower left, 5 bottom, 6 lower right, 7 right
# (right being +X).
RING = 8
ANGLES = [math.radians(22.5 + 45 * k) for k in range(RING)]
UPPER_RIGHT, UPPER_LEFT = 0, 2
LOWER_LEFT, BOTTOM, LOWER_RIGHT = 4, 5, 6
UNDERSIDE = (LOWER_LEFT, BOTTOM, LOWER_RIGHT)

# The spine, tail tip to nose: (y, z, half-width, half-height, colour, underside
# colour). A ring's colours paint the segment arriving at it from the tail side.
TAIL_TIP = Vector((0, -2.25, 1.25))
SPINE = [
    # tail: big and bushy, low, curling up at the tip
    (-2.10, 1.10, 0.16, 0.15, CREAM, CREAM),
    (-1.85, 0.95, 0.30, 0.28, CREAM, CREAM),
    (-1.50, 0.92, 0.34, 0.31, ORANGE, ORANGE),
    (-1.18, 1.02, 0.26, 0.24, ORANGE, ORANGE),
    (-0.95, 1.13, 0.13, 0.13, ORANGE, ORANGE),  # tail root
    # body: short
    (-0.78, 1.08, 0.22, 0.25, ORANGE, ORANGE),  # rump
    (-0.55, 1.07, 0.27, 0.29, ORANGE, ORANGE),  # hip
    (-0.20, 1.03, 0.25, 0.26, ORANGE, ORANGE),
    (0.10, 1.07, 0.27, 0.30, ORANGE, CREAM),  # chest
    (0.32, 1.15, 0.26, 0.30, ORANGE, CREAM),  # shoulder
    # neck: short and thick
    (0.50, 1.38, 0.22, 0.24, ORANGE, CREAM),
    # head: about 1.3x natural, big cheeks, short sharp snout
    (0.58, 1.62, 0.32, 0.30, ORANGE, CREAM),  # back of the skull
    (0.80, 1.72, 0.42, 0.34, ORANGE, CREAM),  # cheeks, the widest point
    (1.02, 1.68, 0.30, 0.24, ORANGE, CREAM),
    (1.20, 1.60, 0.14, 0.11, ORANGE, CREAM),  # snout
    (1.32, 1.57, 0.07, 0.06, SOOT, SOOT),
]
NOSE_TIP = Vector((0, 1.39, 1.57))

# Which spine segment (index of its tail-side ring) and face each limb grows from.
HIND_SEGMENT = 5  # rump -> hip
FRONT_SEGMENT = 8  # chest -> shoulder
EAR_SEGMENT = 11  # back of skull -> cheeks

# Limbs, for the right side (mirrored for the left): rings of
# (x, y, z, half-width, half-depth, colour), each colour painting the segment
# arriving at that ring. The last ring is capped flat, or closed to `tip`.
FRONT_LEG = [
    (0.16, 0.22, 0.72, 0.09, 0.10, ORANGE),  # shoulder
    (0.15, 0.24, 0.45, 0.075, 0.08, ORANGE),  # elbow
    (0.15, 0.26, 0.14, 0.06, 0.065, SOOT),  # wrist
    (0.15, 0.30, 0.07, 0.075, 0.105, SOOT),  # paw
    (0.15, 0.30, 0.00, 0.075, 0.105, SOOT),  # sole
]
HIND_LEG = [
    (0.17, -0.66, 0.72, 0.11, 0.15, ORANGE),  # hip
    (0.16, -0.55, 0.45, 0.08, 0.09, ORANGE),  # knee
    (0.15, -0.70, 0.20, 0.055, 0.065, SOOT),  # hock
    (0.15, -0.63, 0.07, 0.075, 0.105, SOOT),  # paw
    (0.15, -0.63, 0.00, 0.075, 0.105, SOOT),  # sole
]
EAR = [
    (0.22, 0.70, 2.12, 0.15, 0.06, ORANGE),
    (0.26, 0.72, 2.32, 0.08, 0.035, SOOT),
]
EAR_TIP = Vector((0.29, 0.73, 2.50))


def spine_rings(bm, colour_layer):
    """Loft the spine. Returns the rings, each a list of RING verts."""
    points = [TAIL_TIP] + [Vector((0, y, z)) for y, z, *_ in SPINE] + [NOSE_TIP]
    side = Vector((1, 0, 0))
    rings = []
    for i, (y, z, hw, hh, *_) in enumerate(SPINE, start=1):
        tangent = (points[i + 1] - points[i - 1]).normalized()
        up = side.cross(tangent).normalized()
        centre = points[i]
        rings.append(
            [bm.verts.new(centre + side * hw * math.cos(a) + up * hh * math.sin(a)) for a in ANGLES]
        )

    def paint(face, colour):
        face[colour_layer] = style.SWATCH[colour]

    tail_tip = bm.verts.new(TAIL_TIP)
    for k in range(RING):
        paint(bm.faces.new((tail_tip, rings[0][(k + 1) % RING], rings[0][k])), SPINE[0][4])

    segments = []  # segments[i][k]: face k between ring i and ring i + 1
    for i in range(len(rings) - 1):
        a, b = rings[i], rings[i + 1]
        _, _, _, _, colour, under = SPINE[i]
        row = []
        for k in range(RING):
            face = bm.faces.new((a[k], a[(k + 1) % RING], b[(k + 1) % RING], b[k]))
            paint(face, under if k in UNDERSIDE else colour)
            row.append(face)
        segments.append(row)

    nose = bm.verts.new(NOSE_TIP)
    for k in range(RING):
        paint(bm.faces.new((nose, rings[-1][k], rings[-1][(k + 1) % RING])), SPINE[-1][4])
    return segments


def grow_limb(bm, colour_layer, face, spec, mirror, tip=None):
    """Replace a quad face with a limb lofted through the rings in spec.

    Each new ring is a quad in the XY plane. Its corners are matched to the base
    face's corners by which side of the face centre they lie on, so the limb
    comes out untwisted whatever the face's orientation.
    """
    base = list(face.verts)
    centre = face.calc_center_median()
    signs = [
        (1 if v.co.x > centre.x else -1, 1 if v.co.y > centre.y else -1) for v in base
    ]
    assert len(set(signs)) == 4, f"limb base face corners are ambiguous: {signs}"
    bm.faces.remove(face)

    prev = base
    for x, y, z, hw, hd, colour in spec:
        ring = [bm.verts.new((x * mirror + sx * hw, y + sy * hd, z)) for sx, sy in signs]
        for j in range(4):
            f = bm.faces.new((prev[j], prev[(j + 1) % 4], ring[(j + 1) % 4], ring[j]))
            f[colour_layer] = style.SWATCH[colour]
        prev = ring
    colour = spec[-1][5]
    if tip is None:
        cap = bm.faces.new(prev)
        cap[colour_layer] = style.SWATCH[colour]
    else:
        point = bm.verts.new((tip.x * mirror, tip.y, tip.z))
        for j in range(4):
            f = bm.faces.new((point, prev[j], prev[(j + 1) % 4]))
            f[colour_layer] = style.SWATCH[colour]


def build_fox():
    bm = bmesh.new()
    colour_layer = bm.faces.layers.int.new("swatch")
    segments = spine_rings(bm, colour_layer)

    # Collect every base face before growing anything: growing removes faces.
    limbs = []
    for mirror, lower, upper in ((1, LOWER_RIGHT, UPPER_RIGHT), (-1, LOWER_LEFT, UPPER_LEFT)):
        limbs.append((segments[FRONT_SEGMENT][lower], FRONT_LEG, mirror, None))
        limbs.append((segments[HIND_SEGMENT][lower], HIND_LEG, mirror, None))
        limbs.append((segments[EAR_SEGMENT][upper], EAR, mirror, EAR_TIP))
    for face, spec, mirror, tip in limbs:
        grow_limb(bm, colour_layer, face, spec, mirror, tip)

    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    swatches = [style.PALETTE[f[colour_layer]][0] for f in bm.faces]
    bm.faces.layers.int.remove(colour_layer)

    mesh = bpy.data.meshes.new("Fox")
    bm.to_mesh(mesh)
    bm.free()
    obj = bpy.data.objects.new("Fox", mesh)
    bpy.context.scene.collection.objects.link(obj)
    style.apply_swatches(obj, swatches)
    return obj


def main():
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    parser = argparse.ArgumentParser(prog="fox.py")
    parser.add_argument("--out", type=Path, required=True, help="where to write the .glb")
    parser.add_argument("--renders", type=Path, help="directory for a preview sheet")
    args = parser.parse_args(argv)

    style.reset_scene()
    fox = build_fox()
    print(f"fox: {len(fox.data.polygons)} faces, {sum(len(p.vertices) - 2 for p in fox.data.polygons)} triangles")
    args.out.parent.mkdir(parents=True, exist_ok=True)
    style.export_glb(fox, args.out)
    if args.renders:
        args.renders.mkdir(parents=True, exist_ok=True)
        style.render_sheet(target=(0, -0.45, 1.15), extent=3.8, out_png=args.renders / "fox.png")


if __name__ == "__main__":
    main()

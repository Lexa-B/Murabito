"""Lofting animals: a body along a spine, with limbs grown out of its faces.

An animal is one connected mesh, built so a skeleton can bend it later: a single
loft runs from the tail tip to the nose, and the legs and ears are lofted out of
faces of that body, with a ring of vertices at every joint.

Coordinates are Blender's, in shaku: +Y forward, +Z up, +X the animal's right.
"""

import math
import random

import bmesh
import bpy
from mathutils import Vector

import style

# Rings are octagons around the spine. Vertex k sits at 22.5 + 45k degrees, so
# face k of a ring segment is centred on 45 + 45k degrees: 0 upper right,
# 1 top, 2 upper left, 3 left, 4 lower left, 5 bottom, 6 lower right, 7 right
# (right being +X).
RING = 8
ANGLES = [math.radians(22.5 + 45 * k) for k in range(RING)]
UPPER_RIGHT, UPPER_LEFT = 0, 2
LOWER_LEFT, BOTTOM, LOWER_RIGHT = 4, 5, 6
UNDERSIDE = (LOWER_LEFT, BOTTOM, LOWER_RIGHT)


class Builder:
    """A bmesh whose faces each remember a palette swatch."""

    def __init__(self):
        self.bm = bmesh.new()
        self._swatch = self.bm.faces.layers.int.new("swatch")

    def face(self, verts, colour):
        f = self.bm.faces.new(verts)
        f[self._swatch] = style.SWATCH[colour]
        return f

    def paint(self, faces, colour):
        """Recolour faces already built, for patches that don't follow rings."""
        for f in faces:
            f[self._swatch] = style.SWATCH[colour]

    def spine(self, start_tip, rings, end_tip, upright=0):
        """Loft a body along a spine in the YZ plane.

        rings run from start_tip to end_tip, each (y, z, half-width, half-height,
        colour, underside colour). A ring's colours paint the segment arriving at
        it from the start_tip side. Each ring stands square to the spine, except
        the upright ones, which stand straight up: where the spine curves sharply
        (a fan of a tail, a rump under a high tail), squared rings swing past each
        other and fold. upright is how many rings from the start, or a collection
        of ring indices. Returns segments[i][k]: face k between ring i and ring
        i + 1, for growing limbs from.
        """
        is_upright = (lambda i: i < upright) if isinstance(upright, int) else (lambda i: i in upright)
        points = [start_tip] + [Vector((0, y, z)) for y, z, *_ in rings] + [end_tip]
        side = Vector((1, 0, 0))
        verts = []
        for i, (y, z, hw, hh, *_) in enumerate(rings, start=1):
            tangent = (points[i + 1] - points[i - 1]).normalized()
            up = Vector((0, 0, 1)) if is_upright(i - 1) else side.cross(tangent).normalized()
            centre = points[i]
            verts.append(
                [
                    self.bm.verts.new(centre + side * hw * math.cos(a) + up * hh * math.sin(a))
                    for a in ANGLES
                ]
            )

        tip = self.bm.verts.new(start_tip)
        for k in range(RING):
            self.face((tip, verts[0][(k + 1) % RING], verts[0][k]), rings[0][4])

        segments = []
        for i in range(len(verts) - 1):
            a, b = verts[i], verts[i + 1]
            _, _, _, _, colour, under = rings[i]
            segments.append(
                [
                    self.face(
                        (a[k], a[(k + 1) % RING], b[(k + 1) % RING], b[k]),
                        under if k in UNDERSIDE else colour,
                    )
                    for k in range(RING)
                ]
            )

        tip = self.bm.verts.new(end_tip)
        for k in range(RING):
            self.face((tip, verts[-1][k], verts[-1][(k + 1) % RING]), rings[-1][4])
        return segments

    def limb(self, face, rings, mirror, tip=None):
        """Replace a quad face with a limb lofted through rings.

        rings are for the right side, each (x, y, z, half-width, half-depth,
        colour), a colour painting the segment arriving at its ring; mirror is 1
        for the right side and -1 for the left. The last ring is capped flat, or
        closed to the point tip. Each ring is a quad in the XY plane, its corners
        matched to the base face's corners by which side of the face centre they
        lie on, so the limb comes out untwisted whatever the face's orientation.

        Returns the limb's faces, one list per ring plus one for the end.
        """
        base = list(face.verts)
        centre = face.calc_center_median()
        signs = [(1 if v.co.x > centre.x else -1, 1 if v.co.y > centre.y else -1) for v in base]
        assert len(set(signs)) == 4, f"limb base face corners are ambiguous: {signs}"
        self.bm.faces.remove(face)

        prev = base
        rows = []
        for x, y, z, hw, hd, colour in rings:
            ring = [self.bm.verts.new((x * mirror + sx * hw, y + sy * hd, z)) for sx, sy in signs]
            rows.append(
                [
                    self.face((prev[j], prev[(j + 1) % 4], ring[(j + 1) % 4], ring[j]), colour)
                    for j in range(4)
                ]
            )
            prev = ring
        colour = rings[-1][5]
        if tip is None:
            rows.append([self.face(prev, colour)])
        else:
            point = self.bm.verts.new((tip.x * mirror, tip.y, tip.z))
            rows.append([self.face((point, prev[j], prev[(j + 1) % 4]), colour) for j in range(4)])
        return rows

    def finish(self, name, shaded=False):
        """Turn the bmesh into a palette-coloured object linked into the scene.

        shaded varies fur and feathers face by face (see style.SHADES), from a
        random stream seeded by name, so a rebuild comes out the same.
        """
        bmesh.ops.recalc_face_normals(self.bm, faces=self.bm.faces)
        swatches = [style.PALETTE[f[self._swatch]][0] for f in self.bm.faces]
        if shaded:
            rng = random.Random(name)
            swatches = [rng.choice(style.SHADES[s]) if s in style.SHADES else s for s in swatches]
        self.bm.faces.layers.int.remove(self._swatch)

        mesh = bpy.data.meshes.new(name)
        self.bm.to_mesh(mesh)
        self.bm.free()
        obj = bpy.data.objects.new(name, mesh)
        bpy.context.scene.collection.objects.link(obj)
        style.apply_swatches(obj, swatches)
        return obj


def run(build, name, target, extent):
    """The command line every model script shares:

        blender -b --python art/<model>.py -- --out <file.glb> [--renders <dir>]

    Arguments it doesn't know are left for the script's own parser.
    """
    import argparse
    import sys
    from pathlib import Path

    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    parser = argparse.ArgumentParser(prog=f"{name}.py")
    parser.add_argument("--out", type=Path, required=True, help="where to write the .glb")
    parser.add_argument("--renders", type=Path, help="directory for a preview sheet")
    args, _ = parser.parse_known_args(argv)

    style.reset_scene()
    obj = build()
    tris = sum(len(p.vertices) - 2 for p in obj.data.polygons)
    print(f"{name}: {len(obj.data.polygons)} faces, {tris} triangles")
    args.out.parent.mkdir(parents=True, exist_ok=True)
    style.export_glb(obj, args.out)
    if args.renders:
        args.renders.mkdir(parents=True, exist_ok=True)
        style.render_sheet(target=target, extent=extent, out_png=args.renders / f"{name}.png")

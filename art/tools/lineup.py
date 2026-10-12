"""Every model side by side on one ground, from an isometric camera: to compare sizes
and whether things look like they belong together.

    blender -b --python art/tools/lineup.py -- <out_dir> [folder ...]

Folders are relative to assets/entity_models/ (`living`, `spiritual/yokai`,
`flora/trees`); with none, everything. Creatures (everything under living/ and
spiritual/: people, animals, yokai) share the front row, smallest to largest, so
a yokai stands beside the people and animals it's sized against; plants follow, one
species a row (all its versions, colours and seasons), shortest species first.
Each model keeps its own facing, so creatures show their front right side.

Writes <out_dir>/lineup.png (8192 px on its long side) and, when there are plants
too, lineup-creatures.png (the front row alone, closer).
"""

import math
import sys
from pathlib import Path

import bpy
from mathutils import Euler, Vector

ROOT = Path(__file__).resolve().parents[2] / "assets" / "entity_models"
CREATURES = ("living", "spiritual")

VIEW = Vector((1, 1, 1)).normalized()  # a true isometric camera, from front right
ACROSS = Vector((-1, 1, 0)).normalized()  # left to right on screen
AWAY = Vector((-1, -1, 0)).normalized()  # into the picture (up the screen)


def flat_material(name, rgb):
    """Workbench's texture colouring needs an image, so a flat colour is a 1x1 one."""
    img = bpy.data.images.new(name, 1, 1)
    img.pixels = (*rgb, 1.0)
    mat = bpy.data.materials.new(name)
    tex = mat.node_tree.nodes.new("ShaderNodeTexImage")
    tex.image = img
    mat.node_tree.links.new(tex.outputs["Color"], mat.node_tree.nodes["Principled BSDF"].inputs["Base Color"])
    return mat


class Model:
    """One imported .glb: its meshes (a rigged model brings an armature and several
    pieces), moved together by an empty they all hang from."""

    def __init__(self, path):
        before = set(bpy.data.objects)
        bpy.ops.import_scene.gltf(filepath=str(path))
        new = [o for o in bpy.data.objects if o not in before]
        self.name = path.stem
        self.folder = path.parent.relative_to(ROOT).parts
        self.meshes = [o for o in new if o.type == "MESH"]
        self.handle = bpy.data.objects.new(f"model-{self.name}", None)
        bpy.context.scene.collection.objects.link(self.handle)
        for o in new:
            if o.parent is None:
                o.parent = self.handle

    @property
    def creature(self):
        return self.folder[0] in CREATURES

    @property
    def species(self):
        return self.name.split("-")[0]

    def corners(self):
        return [o.matrix_world @ Vector(c) for o in self.meshes for c in o.bound_box]

    def span(self, axis):
        d = [c.dot(axis) for c in self.corners()]
        return min(d), max(d)

    def height(self):
        return max(c.z for c in self.corners())


def rows_of(models):
    creatures = sorted((m for m in models if m.creature), key=lambda m: m.height())
    by_species = {}
    for m in models:
        if not m.creature:
            by_species.setdefault(m.species, []).append(m)
    plants = sorted(by_species.values(), key=lambda row: max(m.height() for m in row))
    return [row for row in [creatures] + [sorted(r, key=lambda m: m.name) for r in plants] if row]


def label(text, size, location, mat):
    curve = bpy.data.curves.new(f"label-{text}", "FONT")
    curve.body = text
    curve.size = size
    curve.align_x = "CENTER"
    obj = bpy.data.objects.new(f"label-{text}", curve)
    obj.location = location
    obj.rotation_euler = Euler((0, 0, math.radians(135)))  # reads left to right on screen
    obj.data.materials.append(mat)
    bpy.context.scene.collection.objects.link(obj)
    return obj


def lay_out(rows, label_mat):
    """Place each row straight across the screen, one behind the other; returns the
    objects of each row (models' meshes and their labels), for framing."""
    v = 0.0  # distance into the picture of the current row's front edge
    placed = []
    for row in rows:
        small = row[0].creature
        gap = 1.0 if small else 4.0
        widths = [m.span(ACROSS)[1] - m.span(ACROSS)[0] for m in row]
        depth = max(m.span(AWAY)[1] - m.span(AWAY)[0] for m in row)
        height = max(m.height() for m in row)
        label_size = 0.3 if small else min(2.0, max(0.5, max(widths) * 0.08))
        v_row = v + label_size * 1.6 + depth / 2  # the row's centre line, behind its labels
        u = -(sum(widths) + gap * (len(row) - 1)) / 2
        objects = []
        for m, w in zip(row, widths):
            (a0, a1), (b0, b1) = m.span(ACROSS), m.span(AWAY)
            target = ACROSS * (u + w / 2) + AWAY * v_row
            here = ACROSS * ((a0 + a1) / 2) + AWAY * ((b0 + b1) / 2)
            m.handle.location += Vector((target.x - here.x, target.y - here.y, 0))
            bpy.context.view_layer.update()
            where = ACROSS * (u + w / 2) + AWAY * (v + label_size * 0.5) + Vector((0, 0, 0.01))
            objects += m.meshes + [label(m.name, label_size, where, label_mat)]
            u += w + gap
        placed.append(objects)
        # the next row starts behind this one's tops: in isometric, height climbs the
        # screen as fast as depth does times sqrt(2), so space rows by their height
        v = v_row + depth / 2 + height * 1.2 + gap
    bpy.context.view_layer.update()
    return placed


def corners(objects):
    return [o.matrix_world @ Vector(c) for o in objects for c in o.bound_box]


def bounds(objects):
    pts = corners(objects)
    lo = Vector((min(p.x for p in pts), min(p.y for p in pts), min(p.z for p in pts)))
    hi = Vector((max(p.x for p in pts), max(p.y for p in pts), max(p.z for p in pts)))
    return lo, hi


def ground(objects, mat):
    lo, hi = bounds(objects)
    bpy.ops.mesh.primitive_plane_add(size=1, location=((lo.x + hi.x) / 2, (lo.y + hi.y) / 2, 0))
    plane = bpy.context.active_object
    plane.scale = (hi.x - lo.x + 40, hi.y - lo.y + 40, 1)
    plane.data.materials.append(mat)


def set_up_render(scene):
    scene.render.engine = "BLENDER_WORKBENCH"
    scene.view_settings.view_transform = "Standard"
    shading = scene.display.shading
    shading.light = "STUDIO"
    shading.color_type = "TEXTURE"
    shading.show_shadows = True
    shading.shadow_intensity = 0.3
    shading.show_object_outline = True
    scene.display.render_aa = "16"
    if scene.world is None:
        scene.world = bpy.data.worlds.new("world")
    scene.world.color = (0.80, 0.84, 0.86)
    scene.render.image_settings.file_format = "PNG"


def render(scene, objects, long_side, path):
    lo, hi = bounds(objects)
    lo.z = 0.0
    cam_data = bpy.data.cameras.new("iso")
    cam_data.type = "ORTHO"
    cam = bpy.data.objects.new("iso", cam_data)
    scene.collection.objects.link(cam)
    centre = (lo + hi) / 2
    cam.location = centre + VIEW * 2000
    cam_data.clip_end = 5000
    cam.rotation_euler = (-VIEW).to_track_quat("-Z", "Y").to_euler()
    bpy.context.view_layer.update()
    right, up = cam.matrix_world.to_3x3().col[0], cam.matrix_world.to_3x3().col[1]
    xs = [(p - centre).dot(right) for p in corners(objects)]
    ys = [(p - centre).dot(up) for p in corners(objects)]
    w, h = (max(xs) - min(xs)) * 1.05, (max(ys) - min(ys)) * 1.05
    cam.location += right * (max(xs) + min(xs)) / 2 + up * (max(ys) + min(ys)) / 2
    cam_data.ortho_scale = max(w, h)
    if w >= h:
        scene.render.resolution_x, scene.render.resolution_y = long_side, max(1, int(long_side * h / w))
    else:
        scene.render.resolution_x, scene.render.resolution_y = max(1, int(long_side * w / h)), long_side
    scene.camera = cam
    scene.render.filepath = str(path)
    bpy.ops.render.render(write_still=True)
    print("RENDERED", path, scene.render.resolution_x, "x", scene.render.resolution_y)


def main():
    args = sys.argv[sys.argv.index("--") + 1 :]
    out, folders = Path(args[0]), args[1:] or ["."]
    paths = sorted({p for f in folders for p in (ROOT / f).rglob("*.glb")})
    bpy.ops.wm.read_factory_settings(use_empty=True)
    scene = bpy.context.scene
    models = [Model(p) for p in paths]
    bpy.context.view_layer.update()
    rows = rows_of(models)
    placed = lay_out(rows, flat_material("label", (0.10, 0.09, 0.08)))
    everything = [o for row in placed for o in row]
    ground(everything, flat_material("ground", (0.62, 0.66, 0.52)))
    set_up_render(scene)
    render(scene, everything, 8192, out / "lineup.png")
    if rows[0][0].creature and len(rows) > 1:
        render(scene, placed[0], 6144, out / "lineup-creatures.png")


main()

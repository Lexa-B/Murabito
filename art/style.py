"""What every Murabito model shares: the palette, the material, export and preview renders.

Models are built by Blender scripts run headless, e.g.
    blender -b --python art/fox.py -- --out assets/models/fox.glb --renders <dir>

Units: one Blender unit is one shaku. Models face Blender +Y with +Z up, which
the glTF exporter turns into Bevy's forward (-Z) and up (+Y).
"""

import math

import bpy
import numpy as np
from mathutils import Quaternion, Vector

# The village palette. Each swatch is one pixel of a small texture, and every face
# maps all its UVs onto the centre of one swatch, so retuning a colour here and
# re-running the scripts recolours every model at once. Append new swatches at
# the end: a swatch's position is its identity.
PALETTE = [
    ("fox_orange", "#D9772B"),
    ("cream", "#F3EBDD"),
    ("soot", "#3B2B26"),
    ("hare_brown", "#8E7456"),
    ("rat_grey", "#6B635D"),
    ("flesh_pink", "#C99A90"),
    ("leaf_green", "#6A9A3E"),
    ("leaf_green_shade", "#46702E"),
    ("bark", "#6A5747"),
    ("leaf_green_light", "#7DAA48"),
    ("leaf_green_deep", "#578536"),
    ("leaf_green_yellow", "#8BA84A"),
    ("leaf_green_shade_deep", "#3A5F28"),
]
PALETTE_SIZE = 8  # the texture is PALETTE_SIZE x PALETTE_SIZE pixels

SWATCH = {name: i for i, (name, _) in enumerate(PALETTE)}


def reset_scene():
    bpy.ops.wm.read_factory_settings(use_empty=True)


def _hex_to_rgb(hex_colour):
    h = hex_colour.lstrip("#")
    return tuple(int(h[i : i + 2], 16) / 255 for i in (0, 2, 4))


def swatch_uv(name):
    i = SWATCH[name]
    return ((i % PALETTE_SIZE + 0.5) / PALETTE_SIZE, (i // PALETTE_SIZE + 0.5) / PALETTE_SIZE)


def palette_image():
    img = bpy.data.images.new("palette", PALETTE_SIZE, PALETTE_SIZE, alpha=False)
    px = np.ones((PALETTE_SIZE, PALETTE_SIZE, 4), dtype=np.float32)
    px[:, :, :3] = 0.5  # unused pixels: neutral grey
    for i, (_, hex_colour) in enumerate(PALETTE):
        px[i // PALETTE_SIZE, i % PALETTE_SIZE, :3] = _hex_to_rgb(hex_colour)
    img.pixels.foreach_set(px.ravel())
    img.pack()
    return img


def palette_material():
    mat = bpy.data.materials.new("palette")
    tree = mat.node_tree
    bsdf = tree.nodes["Principled BSDF"]
    bsdf.inputs["Roughness"].default_value = 1.0
    tex = tree.nodes.new("ShaderNodeTexImage")
    tex.image = palette_image()
    tex.interpolation = "Closest"  # exported as a NEAREST sampler: no bleeding between swatches
    tree.links.new(tex.outputs["Color"], bsdf.inputs["Base Color"])
    return mat


def apply_swatches(obj, face_swatches):
    """Give obj the palette material and point each face's UVs at its swatch.

    face_swatches is a swatch name per polygon, in polygon order.
    """
    mesh = obj.data
    mesh.materials.append(palette_material())
    uv = mesh.uv_layers.new(name="palette")
    for poly, name in zip(mesh.polygons, face_swatches, strict=True):
        u, v = swatch_uv(name)
        for li in poly.loop_indices:
            uv.data[li].uv = (u, v)
        poly.use_smooth = False  # faceted


def export_glb(obj, path):
    bpy.ops.object.select_all(action="DESELECT")
    obj.select_set(True)
    bpy.context.view_layer.objects.active = obj
    bpy.ops.export_scene.gltf(
        filepath=str(path),
        export_format="GLB",
        use_selection=True,
        export_yup=True,
        export_apply=True,
    )


def _camera(name, target, direction, ortho_scale=None, lens=50.0, roll=0.0):
    cam_data = bpy.data.cameras.new(name)
    if ortho_scale is not None:
        cam_data.type = "ORTHO"
        cam_data.ortho_scale = ortho_scale
    else:
        cam_data.lens = lens
    cam = bpy.data.objects.new(name, cam_data)
    bpy.context.scene.collection.objects.link(cam)
    direction = Vector(direction)
    cam.location = Vector(target) + direction
    aim = (-direction).to_track_quat("-Z", "Y")
    cam.rotation_euler = (aim @ Quaternion((0, 0, 1), roll)).to_euler()
    return cam


def render_sheet(target, extent, out_png, size=512):
    """Render front, side, top and three-quarter views into one 2x2 PNG.

    target is the point the cameras look at; extent is roughly the model's longest
    dimension in shaku, used to frame it.
    """
    scene = bpy.context.scene
    scene.render.engine = "BLENDER_WORKBENCH"
    scene.render.resolution_x = scene.render.resolution_y = size
    scene.render.image_settings.file_format = "PNG"
    scene.view_settings.view_transform = "Standard"  # palette colours as written
    shading = scene.display.shading
    shading.light = "STUDIO"
    shading.color_type = "TEXTURE"
    shading.show_object_outline = True
    if scene.world is None:
        scene.world = bpy.data.worlds.new("world")
    scene.world.color = (0.55, 0.6, 0.55)

    far = extent * 4
    ortho = extent * 1.15
    tilt = math.radians(30)
    views = [
        _camera("front", target, (0, far, 0), ortho),
        _camera("side", target, (far, 0, 0), ortho),
        _camera("top", target, (0, 0, far), ortho, roll=math.pi),  # head up the image
        _camera(
            "three_quarter",
            target,
            Vector((1, 1, 0)).normalized() * extent * 2.6 * math.cos(tilt)
            + Vector((0, 0, extent * 2.6 * math.sin(tilt))),
        ),
    ]

    tiles = []
    for cam in views:
        scene.camera = cam
        tile_path = str(out_png) + f".{cam.name}.png"
        scene.render.filepath = tile_path
        bpy.ops.render.render(write_still=True)
        img = bpy.data.images.load(tile_path)
        px = np.empty(size * size * 4, dtype=np.float32)
        img.pixels.foreach_get(px)
        tiles.append(px.reshape(size, size, 4))

    # Rows run bottom-up in Blender images: top row of the sheet is front | side.
    sheet = np.concatenate(
        [np.concatenate([tiles[2], tiles[3]], axis=1), np.concatenate([tiles[0], tiles[1]], axis=1)],
        axis=0,
    )
    out = bpy.data.images.new("sheet", size * 2, size * 2, alpha=True)
    out.pixels.foreach_set(sheet.ravel())
    out.filepath_raw = str(out_png)
    out.file_format = "PNG"
    out.save()

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
    ("leaf_orange", "#D98A3A"),
    ("leaf_orange_light", "#E3A24C"),
    ("leaf_orange_deep", "#C46F2C"),
    ("leaf_orange_green", "#B5A044"),
    ("leaf_orange_shade", "#955527"),
    ("leaf_orange_shade_deep", "#7A4420"),
    ("leaf_plum", "#6A3A4E"),
    ("leaf_plum_light", "#7E4A5C"),
    ("leaf_plum_deep", "#4F2E40"),
    ("leaf_plum_bronze", "#735038"),
    ("leaf_plum_shade", "#3F2433"),
    ("leaf_plum_shade_deep", "#2F1B26"),
    ("leaf_red", "#B8332E"),
    ("leaf_red_light", "#CB4A3A"),
    ("leaf_red_deep", "#982A28"),
    ("leaf_red_scarlet", "#C4563A"),
    ("leaf_red_shade", "#6E1E1E"),
    ("leaf_red_shade_deep", "#561818"),
    ("leaf_blackred", "#4A1E24"),
    ("leaf_blackred_dark", "#3A1A1E"),
    ("leaf_blackred_red", "#5E2A2C"),
    ("leaf_blackred_green", "#2F3B27"),
    ("leaf_blackred_shade", "#2A1216"),
    ("leaf_blackred_shade_green", "#1E2A1C"),
    ("needle_green", "#4B7545"),
    ("needle_green_light", "#5A8750"),
    ("needle_green_deep", "#3D663D"),
    ("needle_green_blue", "#47704F"),
    ("needle_green_shade", "#2F4E31"),
    ("needle_green_shade_deep", "#25402A"),
    ("pine_bark_red", "#B0643E"),
    ("pine_bark_grey", "#6B5B4E"),
    ("pine_bark_mid", "#935F45"),
    ("pine_bark_warm", "#7B5D4D"),
    ("sugi_green", "#3F6A36"),
    ("sugi_green_light", "#4E7C40"),
    ("sugi_green_deep", "#335B2F"),
    ("sugi_green_yellow", "#5A7A3A"),
    ("sugi_green_shade", "#284A26"),
    ("sugi_green_shade_deep", "#1F3B1F"),
    ("sugi_bark", "#7A4E36"),
    ("sugi_bark_light", "#8C5C40"),
    ("sugi_bark_dark", "#613D2C"),
    ("hinoki_green", "#4C8A3F"),
    ("hinoki_green_light", "#62A04C"),
    ("hinoki_green_deep", "#3C7336"),
    ("hinoki_green_blue", "#4A8450"),
    ("hinoki_under_pale", "#86A67F"),
    ("hinoki_shade", "#355A31"),
    ("hinoki_bark", "#8A4F36"),
    ("hinoki_bark_light", "#9C6045"),
    ("hinoki_bark_dark", "#6E3E2C"),
    ("bamboo_culm", "#6F9440"),
    ("bamboo_culm_old", "#8C9448"),
    ("bamboo_node", "#A7B374"),
    ("bamboo_leaf", "#7FA34A"),
    ("bamboo_leaf_light", "#95B95A"),
    ("bamboo_leaf_deep", "#678C3C"),
    ("bamboo_leaf_yellow", "#A2B35A"),
    ("bamboo_leaf_shade", "#4F6E33"),
    ("bamboo_leaf_shade_light", "#5E7F3A"),
    ("azalea_green", "#4F7A3A"),
    ("azalea_green_light", "#5F8C45"),
    ("azalea_green_deep", "#3F6630"),
    ("azalea_green_yellow", "#6E8A40"),
    ("azalea_shade", "#2F4C28"),
    ("azalea_shade_light", "#3A5A30"),
    ("azalea_flower_red", "#E0553A"),
    ("azalea_flower_red_light", "#EE6A48"),
    ("azalea_flower_red_deep", "#C9432E"),
    ("sakura_petal", "#F2B8C8"),
    ("sakura_petal_white", "#FBE3EA"),
    ("sakura_petal_pink", "#EA9FB5"),
    ("sakura_leaf_bronze", "#B87A5E"),
    ("sakura_petal_shade", "#D899AD"),
    ("sakura_petal_shade_deep", "#C08398"),
    ("sakura_green", "#4E7A3C"),
    ("sakura_green_light", "#5E8C46"),
    ("sakura_green_deep", "#3E6632"),
    ("sakura_shade", "#2E4C2A"),
    ("sakura_shade_light", "#3A5A32"),
    ("sakura_bark", "#5B403B"),
    ("sakura_bark_band", "#7D5E54"),
    ("sakura_twig", "#6E4A42"),
    # near shades of the animals' colours, for SHADES
    ("fox_orange_light", "#DF8537"),
    ("fox_orange_deep", "#CF6B24"),
    ("cream_light", "#F8F2E7"),
    ("cream_deep", "#E9DFCE"),
    ("soot_light", "#45332D"),
    ("soot_deep", "#33251F"),
    ("hare_brown_light", "#987E5F"),
    ("hare_brown_deep", "#84694D"),
    ("rat_grey_light", "#746C66"),
    ("rat_grey_deep", "#625A55"),
    ("flesh_pink_light", "#D0A398"),
    ("flesh_pink_deep", "#C09085"),
    # the red-crowned crane
    ("feather_white", "#F4F3EE"),
    ("feather_white_light", "#FAF9F5"),
    ("feather_white_deep", "#E9E7E0"),
    ("feather_black", "#2B2A2C"),
    ("feather_black_light", "#353436"),
    ("feather_black_deep", "#232224"),
    ("crown_red", "#C8322C"),
    ("bill_olive", "#8B8A68"),
    ("crane_leg", "#4A4946"),
    # the Japanese wild boar
    ("boar_brown", "#6A5747"),
    ("boar_brown_light", "#735F4E"),
    ("boar_brown_deep", "#605040"),
    ("boar_dark", "#463A31"),
    ("boar_dark_light", "#4F4238"),
    ("boar_dark_deep", "#3E332B"),
    ("boar_snout", "#6E5650"),
    ("boar_cheek", "#A89A88"),
    ("tusk_ivory", "#E8DFC6"),
    # the sika deer
    ("deer_brown", "#A0643A"),
    ("deer_brown_light", "#AA6E43"),
    ("deer_brown_deep", "#955A33"),
    ("antler", "#B9A57F"),
    # the Japanese black bear
    ("bear_black", "#36312E"),
    ("bear_black_light", "#403A36"),
    ("bear_black_deep", "#2E2A27"),
    ("bear_muzzle", "#A08566"),
    # the Kiso horse
    ("horse_bay", "#8A5A34"),
    ("horse_bay_light", "#94643C"),
    ("horse_bay_deep", "#7F512E"),
    ("hoof_grey", "#4B4541"),
    # native draft cattle
    ("cattle_black", "#423A36"),
    ("cattle_black_light", "#4C4440"),
    ("cattle_black_deep", "#39322F"),
    ("cattle_muzzle", "#6E655F"),
]
PALETTE_SIZE = 16  # the texture is PALETTE_SIZE x PALETTE_SIZE pixels, one per swatch

SWATCH = {name: i for i, (name, _) in enumerate(PALETTE)}

# Fur and feathers: in a shaded model (loft.Builder.finish(shaded=True)) each face
# of one of these swatches takes it or a near shade at random, so a coat isn't one
# flat colour. The base is listed twice so it stays the most common.
SHADES = {
    base: [base, base, f"{base}_light", f"{base}_deep"]
    for base in (
        "fox_orange",
        "cream",
        "soot",
        "hare_brown",
        "rat_grey",
        "flesh_pink",
        "feather_white",
        "feather_black",
        "boar_brown",
        "boar_dark",
        "deer_brown",
        "bear_black",
        "horse_bay",
        "cattle_black",
    )
}


def reset_scene():
    bpy.ops.wm.read_factory_settings(use_empty=True)


def _hex_to_rgb(hex_colour):
    h = hex_colour.lstrip("#")
    return tuple(int(h[i : i + 2], 16) / 255 for i in (0, 2, 4))


def swatch_uv(name):
    i = SWATCH[name]
    return ((i % PALETTE_SIZE + 0.5) / PALETTE_SIZE, (i // PALETTE_SIZE + 0.5) / PALETTE_SIZE)


def palette_image():
    assert len(PALETTE) <= PALETTE_SIZE**2, "the palette has outgrown its texture: raise PALETTE_SIZE"
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

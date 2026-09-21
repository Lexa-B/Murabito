"""Fingerprint models face by face: each face's corners and the palette colour its
UVs land on, hashed. Two exports of the same model match even if their UVs or
embedded palette differ, as long as every face has the same shape and colour.

    blender -b --python art/tools/fingerprint.py -- [--above Z] model.glb ...

Prints one line per model: FINGERPRINT <file> <faces> <hash>. With --above, only
faces lying wholly above height Z (shaku) count.
"""

import argparse
import hashlib
import sys

import bpy
import numpy as np

argv = sys.argv[sys.argv.index("--") + 1 :]
parser = argparse.ArgumentParser(prog="fingerprint.py")
parser.add_argument("--above", type=float, help="only faces wholly above this height")
parser.add_argument("models", nargs="+")
args = parser.parse_args(argv)

for path in args.models:
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=path)
    mesh = next(o for o in bpy.context.scene.objects if o.type == "MESH").data  # imported Z-up
    image = next(n.image for n in mesh.materials[0].node_tree.nodes if n.type == "TEX_IMAGE")
    w, h = image.size
    pixels = np.empty(w * h * 4, dtype=np.float32)
    image.pixels.foreach_get(pixels)
    pixels = pixels.reshape(h, w, 4)
    uv = mesh.uv_layers[0].data
    rows = []
    for poly in mesh.polygons:
        if args.above is not None and min(mesh.vertices[i].co.z for i in poly.vertices) <= args.above:
            continue
        u, v = uv[poly.loop_indices[0]].uv
        colour = tuple(int(round(c * 255)) for c in pixels[min(int(v * h), h - 1), min(int(u * w), w - 1), :3])
        corners = tuple(tuple(round(c, 4) for c in mesh.vertices[i].co) for i in poly.vertices)
        rows.append((corners, colour))
    digest = hashlib.sha256(repr(rows).encode()).hexdigest()[:16]
    print("FINGERPRINT", path.rsplit("/", 1)[-1], len(rows), digest)

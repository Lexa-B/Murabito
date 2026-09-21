"""Crop part of a render and enlarge it, to look closely at something subtle (a
bark blend, an ear base, a branch tip).

    blender -b --python art/tools/zoom.py -- <in.png> <out.png> <x> <y> <width> <height> [scale]

x and y are the crop's top-left corner in pixels, counted from the image's top-left
as image viewers do; scale (default 3) enlarges it by repeating pixels.
"""

import sys

import bpy
import numpy as np

argv = sys.argv[sys.argv.index("--") + 1 :]
source, out_path = argv[0], argv[1]
x, y, w, h = map(int, argv[2:6])
scale = int(argv[6]) if len(argv) > 6 else 3

image = bpy.data.images.load(source)
width, height = image.size
pixels = np.empty(width * height * 4, dtype=np.float32)
image.pixels.foreach_get(pixels)
pixels = pixels.reshape(height, width, 4)
crop = pixels[height - y - h : height - y, x : x + w]  # Blender images run bottom-up
crop = np.repeat(np.repeat(crop, scale, axis=0), scale, axis=1)
out = bpy.data.images.new("zoom", crop.shape[1], crop.shape[0], alpha=True)
out.pixels.foreach_set(crop.ravel())
out.filepath_raw = out_path
out.file_format = "PNG"
out.save()
print("ZOOM", out_path)

"""Put several models' preview views side by side, to compare versions, colours or
seasons: a row of front views over a row of three-quarter views.

    blender -b --python art/tools/sheet.py -- --renders <dir> --out <png> <name> ...

<name> is a model name as rendered by --renders (e.g. maple-00-a-summer); its
views are read from <dir>/<name>.png.front.png and .three_quarter.png.
"""

import argparse
import sys

import bpy
import numpy as np

argv = sys.argv[sys.argv.index("--") + 1 :]
parser = argparse.ArgumentParser(prog="sheet.py")
parser.add_argument("--renders", required=True, help="the directory the models were rendered into")
parser.add_argument("--out", required=True, help="where to write the sheet")
parser.add_argument("names", nargs="+")
args = parser.parse_args(argv)


def view(name, which):
    image = bpy.data.images.load(f"{args.renders}/{name}.png.{which}.png")
    w, h = image.size
    pixels = np.empty(w * h * 4, dtype=np.float32)
    image.pixels.foreach_get(pixels)
    return pixels.reshape(h, w, 4)


# Blender images run bottom-up: the first row here is the bottom of the sheet.
rows = [np.concatenate([view(n, which) for n in args.names], axis=1) for which in ("three_quarter", "front")]
sheet = np.concatenate(rows, axis=0)
out = bpy.data.images.new("sheet", sheet.shape[1], sheet.shape[0], alpha=True)
out.pixels.foreach_set(sheet.ravel())
out.filepath_raw = args.out
out.file_format = "PNG"
out.save()
print("SHEET", args.out)

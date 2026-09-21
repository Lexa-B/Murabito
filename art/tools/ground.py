"""Show how each model meets the ground: its few lowest vertex heights, in shaku.

    blender -b --python art/tools/ground.py -- model.glb ...

A tree's trunk should bottom out at -1.5 (flora.ROOT_DEPTH), an animal's soles
at 0, a shrub's skirt a little below 0 (the azalea's is at about -0.3).
"""

import sys

import bpy

for path in sys.argv[sys.argv.index("--") + 1 :]:
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=path)
    obj = next(o for o in bpy.context.scene.objects if o.type == "MESH")
    heights = sorted({round((obj.matrix_world @ v.co).z, 3) for v in obj.data.vertices})
    print("GROUND", path.rsplit("/", 1)[-1], "lowest heights:", heights[:4])

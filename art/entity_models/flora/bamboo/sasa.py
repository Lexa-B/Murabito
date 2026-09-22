"""Sasa (笹): a patch of dwarf bamboo, about 6 by 5 shaku and knee high.

    blender -b --python art/entity_models/flora/bamboo/sasa.py -- --out assets/entity_models/flora/bamboo/sasa-00-a-summer.glb \\
        [--version 00] [--colour a] [--season summer] [--renders <dir>]

The commonest plant of Honshu's forest floors and hillsides: thickets of dwarf
bamboo, knee to waist high, following every slope. This is a small patch, a
clump at a path's edge, to scatter and tile into bigger stands: a low, lumpy
body (flora.blanket) bristling with clusters of the bamboo's pointed leaf
blades splaying up and out.

It is rigged with drape points (rig.drape_skin), as the kudzu is, so it
settles onto uneven ground: a grid of them under the patch
(blanket_<column>_<row>), each skin vertex blended between the four around
it, and each leaf cluster riding rigidly with the ground where it grows. The
game raises every drape point to the terrain height under its resting spot.

Its triangles are scaled to a shrub (FACET). Kumazasa's leaves whiten at the
edges in winter: a season to come.

Named sasa-<version>-<colour>-<season>, as the other plants are.
"""

import argparse
import math
import sys
from pathlib import Path

sys.path.insert(0, str(next(p for p in Path(__file__).resolve().parents if (p / "loft.py").exists())))
import flora  # noqa: E402
import loft  # noqa: E402
import rig  # noqa: E402

FACET = 0.5  # shaku: the triangle edge for the body, which the leaves mostly hide
DRAPE_SPACING = 1.5  # shaku between the drape points

# colour variant: season: (leaf colours, underside colours), picked per face or blade.
COLOURS = {
    "a": {  # fresh bamboo green
        "summer": (
            ["sasa_green", "sasa_green", "sasa_green_light", "sasa_green_deep", "sasa_green_yellow"],
            ["sasa_shade", "sasa_shade", "sasa_shade_deep", "sasa_green_deep"],
        ),
    },
}

# A version is rules for the patch, in shaku and radians, drawn from its seed.
VERSIONS = {
    "00": {
        "seed": 0,
        "size": (3.0, 2.5),  # half the patch's length and width
        "height": 0.8,  # the body, a base for the leaves: they are the mass
        "lumps": 10,
        "lump_radius": (0.8, 1.4),
        "clusters": 110,
        "blades": (3, 5),  # per cluster
        "blade": (1.3, 0.34),  # length and width, bigger than life
        "rise": (-1.0, -0.2),  # how each blade points: up and out, bristling
    },
}


def build_sasa(version="00", colour="a", season="summer"):
    spec = VERSIONS[version]
    leaves, shades = COLOURS[colour][season]
    rng = flora.rng_for(spec["seed"])
    b = loft.Builder()
    skeleton = rig.Rig(f"Sasa-{version}-{colour}-{season}-rig")
    skeleton.bone("root", (0, 0, 0), (0, 0, 1))
    rx, ry = spec["size"]
    skin, skin_verts = flora.blanket(b, rx, ry, spec["height"], spec["lumps"], leaves, shades, rng, FACET,
                                     lump_radius=spec["lump_radius"])

    # clusters of leaf blades bristling from the body's upper side
    upper = [f for f in skin if f.normal.z > 0.2]
    clusters = []  # (first vertex, end vertex, where it grows)
    length, width = spec["blade"]
    for _ in range(spec["clusters"]):
        face = rng.choice(upper)
        spot = face.calc_center_median() + face.normal * 0.05
        first = len(b.bm.verts)
        heading = rng.uniform(0, 2 * math.pi)
        flora.blade_cluster(
            b, spot, (math.cos(heading), math.sin(heading), 0), rng.randint(*spec["blades"]), length, width,
            leaves, shades, rng, spread=1.2, droop=spec["rise"],
        )
        clusters.append((first, len(b.bm.verts), spot))

    rig.drape_skin(skeleton, skin_verts, clusters, DRAPE_SPACING)
    obj = b.finish(f"Sasa-{version}-{colour}-{season}")
    skeleton.bind(obj, default="root")
    return obj


if __name__ == "__main__":
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    parser = argparse.ArgumentParser(prog="sasa.py")
    parser.add_argument("--version", default="00", choices=VERSIONS)
    parser.add_argument("--colour", default="a", choices=COLOURS)
    parser.add_argument("--season", default="summer")
    args, _ = parser.parse_known_args(argv)
    loft.run(
        lambda: build_sasa(args.version, args.colour, args.season),
        f"sasa-{args.version}-{args.colour}-{args.season}",
        target=(0, 0, 0.8),
        extent=8,
    )

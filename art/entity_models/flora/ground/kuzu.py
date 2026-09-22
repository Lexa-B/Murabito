"""Kudzu (葛, kuzu): a patch of the vine smothering the undergrowth, about 12 shaku
across and knee to waist high.

    blender -b --python art/entity_models/flora/ground/kuzu.py -- --out assets/entity_models/flora/ground/kuzu-00-a-summer.glb \\
        [--version 00] [--colour a] [--season summer] [--renders <dir>]

Kudzu climbs over whatever it meets, and villagers cut it for fibre and dug it
for starch. This is a free-standing patch for forest edges and slopes: a low,
lumpy blanket (one shrink-wrapped skin, as if over buried undergrowth) with
kudzu's big three-part leaves lying all over it (flora.trifoliate), and a few
runners trailing out from its edges across the ground.

It is rigged with drape points (rig.py) so the whole patch can settle onto
uneven ground like a soggy pancake: a grid of them about 2 shaku apart under
the blanket (blanket_<column>_<row>), each skin vertex blended between the
four around it, and one at each runner joint (runner_<n>_0 at the blanket's
edge to runner_<n>_6 at the tip). Each leaf rides rigidly with the ground
where it grows. The game raises every drape point to the terrain height under
its resting spot; at rest they sit on level ground.

Its purple flower spikes of late summer are a season to come.

Its triangles are scaled to a shrub (FACET), as the azalea's are.

Named kuzu-<version>-<colour>-<season>, as the other plants are.
"""

import argparse
import math
import sys
from pathlib import Path

sys.path.insert(0, str(next(p for p in Path(__file__).resolve().parents if (p / "loft.py").exists())))
import flora  # noqa: E402
import loft  # noqa: E402
import rig  # noqa: E402
from mathutils import Vector  # noqa: E402

VINE = "kuzu_vine"
FACET = 0.8  # shaku: the triangle edge for the blanket, which the leaves mostly hide

# colour variant: season: (leaf colours, underside colours), picked per triangle.
COLOURS = {
    "a": {  # fresh mid green
        "summer": (
            ["kuzu_green", "kuzu_green", "kuzu_green_light", "kuzu_green_deep", "kuzu_green_yellow"],
            ["kuzu_shade", "kuzu_shade", "kuzu_shade_deep", "kuzu_green_deep"],
        ),
    },
}

# A version is rules for the patch, in shaku, drawn from its seed.
VERSIONS = {
    "00": {
        "seed": 0,
        "size": (6.0, 5.0),  # half the patch's length and width
        "height": 2.4,
        "lumps": 14,  # bulging over the undergrowth beneath
        "leaves": 90,  # lying all over the blanket
        "leaf": 1.4,  # a leaflet's length, much bigger than life, to read
        "runners": 5,
        "runner": (3.0, 5.0),  # how far each trails out past the edge
    },
}


DRAPE_SPACING = 2.0  # shaku between the drape points under the blanket


def _runner(b, skeleton, name, start, heading, length, spec, leaves, shades, rng):
    """A vine trailing across the ground, wandering, with leaves along it and its
    tip turned up; a drape point at each joint, which its ring there and the leaf
    growing there ride on."""
    points, way = [start], heading
    steps = 6
    for i in range(1, steps + 1):
        way += rng.uniform(-0.6, 0.6)
        lift = 0.35 if i == steps else 0.08
        points.append(points[-1] + Vector((math.cos(way), math.sin(way), 0)) * (length / steps)
                      + Vector((0, 0, lift - (points[-1].z - 0.08) * 0.8)))
    bones = [skeleton.drape_point(f"{name}_{k}", p.x, p.y) for k, p in enumerate(points)]
    first = len(b.bm.verts)
    flora.branch(b, points, [0.05] + [0.035] * (len(points) - 2) + [0.02], VINE)
    for i in range(len(points)):  # a ring of flora.BRANCH_RING vertices per point
        skeleton.weigh(range(first + i * flora.BRANCH_RING, first + (i + 1) * flora.BRANCH_RING), bones[i])
    for i in range(1, len(points), 1):
        heading_here = heading + rng.choice((-1, 1)) * rng.uniform(0.9, 1.6)
        first = len(b.bm.verts)
        flora.trifoliate(b, points[i] + Vector((0, 0, 0.1)), heading_here, spec["leaf"] * 0.7, leaves, shades, rng)
        skeleton.weigh(range(first, len(b.bm.verts)), bones[i])


def build_kuzu(version="00", colour="a", season="summer"):
    spec = VERSIONS[version]
    leaves, shades = COLOURS[colour][season]
    rng = flora.rng_for(spec["seed"])
    b = loft.Builder()
    skeleton = rig.Rig(f"Kuzu-{version}-{colour}-{season}-rig")
    skeleton.bone("root", (0, 0, 0), (0, 0, 1))
    rx, ry = spec["size"]
    skin, skin_verts = flora.blanket(b, rx, ry, spec["height"], spec["lumps"], leaves, shades, rng, FACET)

    # big three-part leaves lying all over the blanket's upper side
    upper = [f for f in skin if f.normal.z > 0.25]
    blanket_leaves = []  # (first vertex, last vertex, where it grows)
    for _ in range(spec["leaves"]):
        face = rng.choice(upper)
        spot = face.calc_center_median() + face.normal * 0.08
        first = len(b.bm.verts)
        flora.trifoliate(b, spot, rng.uniform(0, 2 * math.pi), spec["leaf"] * rng.uniform(0.85, 1.15),
                         leaves, shades, rng)
        blanket_leaves.append((first, len(b.bm.verts), spot))

    # drape points under the blanket; each leaf rides rigidly with the ground where it grows
    rig.drape_skin(skeleton, skin_verts, blanket_leaves, DRAPE_SPACING)

    for i in range(spec["runners"]):
        angle = i * flora.GOLDEN_ANGLE + rng.uniform(-0.3, 0.3)
        start = Vector((math.cos(angle) * rx * 0.9, math.sin(angle) * ry * 0.9, 0.1))
        _runner(b, skeleton, f"runner_{i}", start, angle, rng.uniform(*spec["runner"]), spec, leaves, shades, rng)
    obj = b.finish(f"Kuzu-{version}-{colour}-{season}")
    skeleton.bind(obj, default="root")
    return obj


if __name__ == "__main__":
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    parser = argparse.ArgumentParser(prog="kuzu.py")
    parser.add_argument("--version", default="00", choices=VERSIONS)
    parser.add_argument("--colour", default="a", choices=COLOURS)
    parser.add_argument("--season", default="summer")
    args, _ = parser.parse_known_args(argv)
    loft.run(
        lambda: build_kuzu(args.version, args.colour, args.season),
        f"kuzu-{args.version}-{args.colour}-{args.season}",
        target=(0, 0, 0.8),
        extent=20,
    )

"""The mountain cherry (山桜, yamazakura): a broad, spreading tree about 30 shaku tall.

    blender -b --python art/sakura.py -- --out assets/models/sakura-00-a-spring.glb \
        [--version 00] [--colour a] [--season spring] [--renders <dir>]

The wild cherry of village hillsides. Somei-yoshino, the pale cherry of parks
today, only arose in the 1800s. A short, thick, gnarled trunk of dark, glossy bark
banded with pale rings splits low into big limbs that rise briefly and then
run out nearly level, kinking and twisting as they go, as a cherry's do, each
carrying lumpy masses of foliage wrapped as its own skin; together they close
into a broad, full dome, the limbs showing beneath and in a few gaps. In spring its white-to-pale-pink flowers open with reddish-
bronze young leaves among them; in summer it is dark green.

Named sakura-<version>-<colour>-<season>, as the other trees are.
"""

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import flora  # noqa: E402
import loft  # noqa: E402
from mathutils import Vector  # noqa: E402

BARK, BAND = "sakura_bark", "sakura_bark_band"

# colour variant: season: (leaf colours, underside colours), picked per triangle.
# Within a variant every season's lists must be the same lengths: picking
# draws from the same random stream as the lumps' shapes, and a different
# length would change the tree's shape between seasons, not just its colour.
COLOURS = {
    "a": {  # yamazakura: pale blossom flecked with bronze young leaves
        "spring": (
            ["sakura_petal"] * 5 + ["sakura_petal_white"] * 3 + ["sakura_petal_pink"] * 3 + ["sakura_leaf_bronze"],
            ["sakura_petal_shade", "sakura_petal_shade", "sakura_petal_shade_deep", "sakura_petal_pink"],
        ),
        "summer": (
            ["sakura_green"] * 6 + ["sakura_green_light"] * 3 + ["sakura_green_deep"] * 3,
            ["sakura_shade", "sakura_shade", "sakura_shade_light", "sakura_green_deep"],
        ),
    },
}
for _seasons in COLOURS.values():
    assert len({tuple(map(len, lists)) for lists in _seasons.values()}) == 1, "season lists differ in length"

LIMB = [1.3, 0.85, 0.5, 0.25]  # thick where they leave the trunk

# A version is a trunk, limbs and the masses of foliage on them, in shaku.
# Trunk and limbs are polylines with a radius per point. Each limb carries a
# big mass at its end, side masses flanking it and one partway along, all
# wrapped as one skin; "fill" masses close the top of the crown. Masses are
# (radius, up, down); fill masses are (centre, radius, up, down).
VERSIONS = {
    "00": {
        "seed": 0,
        "trunk": ([(0, 0, -0.5), (0.2, 0, 2.5), (0.7, 0.3, 5), (0.3, 0.2, 7.5)], [2.8, 2.05, 1.8, 1.6]),
        "gnarl": (0.3, 0.8),  # how far the trunk and the limbs are pushed off line
        "limbs": [  # rising briefly from the fork, then running out nearly level
            ([(0.3, 0.2, 6.5), (4, 1, 9), (11, 2.5, 10), (18, 2, 10.5)], LIMB),
            ([(0.3, 0.2, 6.5), (-3, 3, 9), (-9, 9, 10.5), (-14, 13, 10.5)], LIMB),
            ([(0.3, 0.2, 7), (-3, -4, 9.5), (-8, -11, 11), (-12, -14, 11)], LIMB),
            ([(0.3, 0.2, 7), (4, -4, 10), (9, -11, 11.5), (13, -14, 12)], LIMB),
            ([(0.3, 0.2, 7.5), (1, 5, 10.5), (3, 12, 12), (4, 16, 12.5)], LIMB),
            ([(0.3, 0.2, 7.5), (1, 1, 12), (2, 1, 16), (1, 2, 19)], LIMB),  # the one that climbs
        ],
        "end": (8.0, 5.0, 2.2),  # full above, shallow beneath, so the level limbs show
        "side": (6.0, 4.0, 1.8),
        "mid": (5.5, 4.0, 1.6),
        "fill": [((4, -4, 18), 8, 5, 3.0), ((-4, 4, 18), 8, 5, 3.0), ((0, 0, 21), 7, 4, 3.0)],
    },
}


def _masses(points, spec, rng):
    """The masses of foliage a limb carries: a big one on its end, two flanking
    it a little lower, and one partway along."""
    end, before = Vector(points[-1]), Vector(points[-2])
    ahead = Vector((end.x - before.x, end.y - before.y, 0)).normalized()
    across = Vector((-ahead.y, ahead.x, 0))

    def jitter():
        return Vector((rng.uniform(-1, 1), rng.uniform(-1, 1), rng.uniform(-0.5, 0.5)))

    masses = [(end + Vector((0, 0, 1)) + jitter(), *spec["end"])]
    for side in (1, -1):
        masses.append((end - ahead * 2.5 + across * side * 4.5 + Vector((0, 0, -0.5)) + jitter(), *spec["side"]))
    masses.append((before + Vector((0, 0, 1.5)) + jitter(), *spec["mid"]))
    return masses


def build_sakura(version="00", colour="a", season="spring"):
    spec = VERSIONS[version]
    leaves, shades = COLOURS[colour][season]
    rng = flora.rng_for(spec["seed"])
    b = loft.Builder()

    # a gnarled trunk in short rings, every third a pale band, as cherry bark is
    trunk_gnarl, limb_gnarl = spec["gnarl"]
    points, radii = flora.gnarl(*spec["trunk"], rng, trunk_gnarl, step=1.5)
    points, radii = flora.densify(points, radii, step=0.6)
    bands = [BAND if i % 3 == 2 else BARK for i in range(len(points) - 1)]
    flora.branch(b, points, radii, bands)

    for points, radii in spec["limbs"]:
        flora.branch(b, *flora.gnarl(points, radii, rng, limb_gnarl, step=2.0), BARK)
        flora.wrap_sprays(b, _masses(points, spec, rng), leaves, shades, rng)
    fill = [(Vector(c), r, up, down) for c, r, up, down in spec["fill"]]
    flora.wrap_sprays(b, fill, leaves, shades, rng)
    return b.finish(f"Sakura-{version}-{colour}-{season}")


if __name__ == "__main__":
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    parser = argparse.ArgumentParser(prog="sakura.py")
    parser.add_argument("--version", default="00", choices=VERSIONS)
    parser.add_argument("--colour", default="a", choices=COLOURS)
    parser.add_argument("--season", default="spring")
    args, _ = parser.parse_known_args(argv)
    loft.run(
        lambda: build_sakura(args.version, args.colour, args.season),
        f"sakura-{args.version}-{args.colour}-{args.season}",
        target=(0, 0, 12),
        extent=48,
    )

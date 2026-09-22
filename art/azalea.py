"""The mountain azalea (ヤマツツジ): a wild, mounded shrub about 3.6 shaku (~1.1 m) tall.

    blender -b --python art/azalea.py -- --out assets/models/azalea-00-a-summer.glb \
        [--version 00] [--colour a] [--season summer] [--renders <dir>]

The azalea of hillsides and pine woods: a handful of thin stems fanning up
from the base into a low, lumpy mound of small leaves, wrapped as one skin,
its skirt almost on the ground. It flowers in spring, in patches of blossom
over the mound; the flowers are colour alone, so spring is a season like any
other.

Its triangles are scaled to the shrub (FACET) rather than the trees' size,
or a bush this small would get only a few dozen chunky facets.

Named azalea-<version>-<colour>-<season>, as the trees are.
"""

import argparse
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import flora  # noqa: E402
import loft  # noqa: E402
from mathutils import Vector  # noqa: E402

STEM = "bark"
FACET = 0.6  # shaku: the triangle edge for a shrub

GREENS = ["azalea_green", "azalea_green", "azalea_green_light", "azalea_green_deep", "azalea_green_yellow"]
SHADES = ["azalea_shade", "azalea_shade", "azalea_shade_light", "azalea_green_deep"]

# colour variant: season: (leaf colours, underside colours, blossom colours),
# leaves and undersides picked per triangle; blossoms, if any, in patches.
COLOURS = {
    "a": {  # the wild red-orange
        "summer": (GREENS, SHADES, None),
        "spring": (
            GREENS, SHADES,
            ["azalea_flower_red", "azalea_flower_red", "azalea_flower_red_light", "azalea_flower_red_deep"],
        ),
    },
}

# A version is rules for the mound and its stems, in shaku, drawn from its seed.
VERSIONS = {
    "00": {
        "seed": 0,
        "height": 4,
        "width": 5.6,  # across the mound
        "lumps": 11,  # swelling over the dome's top and sides
        "stems": 6,
        "blossoms": 40,  # patches of flowers, when in flower
        "blossom_size": (0.35, 0.65),  # patch radius range: small clumps, not stains
    },
}


def _blossom(b, faces, colours, spec, rng):
    """Flowers in patches, as azaleas bloom: faces of the upper skin near one
    of a few scattered spots take a blossom colour."""
    upper = [f for f in faces if f.normal.z > -0.2]
    spots = [(rng.choice(upper).calc_center_median(), rng.uniform(*spec["blossom_size"]))
             for _ in range(spec["blossoms"])]
    for f in upper:
        centre = f.calc_center_median()
        if any((centre - spot).length < size for spot, size in spots):
            b.paint([f], rng.choice(colours))


def build_azalea(version="00", colour="a", season="summer"):
    spec = VERSIONS[version]
    leaves, shades, blossoms = COLOURS[colour][season]
    rng = flora.rng_for(spec["seed"])
    b = loft.Builder()
    height, radius = spec["height"], spec["width"] / 2

    # a low mound, wider than tall, its skirt almost on the ground
    middle = Vector((0, 0, height * 0.4))
    lumps = [(middle, radius * 0.9, height * 0.48, middle.z + 0.1)]  # down to the ground
    for i in range(spec["lumps"]):
        angle = i * flora.GOLDEN_ANGLE + rng.uniform(-0.3, 0.3)
        out = radius * rng.uniform(0.3, 0.7)
        rise = height * rng.uniform(-0.25, 0.25)  # round the lower sides as well as the top
        lumps.append((
            middle + Vector((math.cos(angle) * out, math.sin(angle) * out, rise)),
            radius * rng.uniform(0.28, 0.42), height * rng.uniform(0.2, 0.28), height * 0.25,
        ))

    for i in range(spec["stems"]):
        angle = i * flora.GOLDEN_ANGLE + rng.uniform(-0.4, 0.4)
        way = Vector((math.cos(angle), math.sin(angle), 0))
        base = way * rng.uniform(0, 0.4) + Vector((0, 0, -0.2))
        flora.branch(
            b, [base, base + way * radius * 0.25 + Vector((0, 0, height * 0.3)), middle + way * radius * 0.5],
            [0.12, 0.08, 0.05], STEM,
        )

    shape = (radius, height * 0.6, middle.z + 0.1)
    faces = flora.canopy(
        b, middle, lumps, leaves, shades, rng, lumpiness=0.05, shape=shape,
        triangles=flora.even_triangles(middle, lumps, shape, edge=FACET),
    )
    if blossoms:
        _blossom(b, faces, blossoms, spec, rng)
    return b.finish(f"Azalea-{version}-{colour}-{season}")


if __name__ == "__main__":
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    parser = argparse.ArgumentParser(prog="azalea.py")
    parser.add_argument("--version", default="00", choices=VERSIONS)
    parser.add_argument("--colour", default="a", choices=COLOURS)
    parser.add_argument("--season", default="summer")
    args, _ = parser.parse_known_args(argv)
    spec = VERSIONS[args.version]
    loft.run(
        lambda: build_azalea(args.version, args.colour, args.season),
        f"azalea-{args.version}-{args.colour}-{args.season}",
        target=(0, 0, spec["height"] * 0.45),
        extent=spec["width"] + 1.5,
    )

"""The Japanese maple (イロハモミジ): a mature hillside tree, about 30 shaku (~9 m) tall.

    blender -b --python art/maple.py -- --out assets/models/maple-00-a-summer.glb \
        [--version 00] [--colour a] [--season summer] [--renders <dir>]

A model is named maple-<version>-<colour>-<season>. The version is the shape
(trunk, leaders, crown); the colour variant is which kind of Japanese maple it
is, for they come green, orange, purple, red and a dark black-red; the season
picks that variant's colours for the time of year. So every version comes in
every colour and season.

Version 00 is a wild tree: a short, thick trunk splitting low, and a lumpy,
mushroom-like crown. pruned-00 is its clipped, cloud-pruned cousin in tiers,
kept for a garden tree.
"""

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import flora  # noqa: E402
import loft  # noqa: E402

BARK = "bark"

# colour variant: season: (leaf colours, underside colours), picked per triangle.
# Repeating a colour makes it more likely.
COLOURS = {
    "a": {  # the familiar green
        "summer": (
            ["leaf_green", "leaf_green", "leaf_green_light", "leaf_green_deep", "leaf_green_yellow"],
            ["leaf_green_shade", "leaf_green_shade", "leaf_green_shade_deep", "leaf_green_deep"],
        ),
    },
    "b": {  # orange
        "summer": (
            ["leaf_orange", "leaf_orange", "leaf_orange_light", "leaf_orange_deep", "leaf_orange_green"],
            ["leaf_orange_shade", "leaf_orange_shade", "leaf_orange_shade_deep", "leaf_orange_deep"],
        ),
    },
    "c": {  # purple
        "summer": (
            ["leaf_plum", "leaf_plum", "leaf_plum_light", "leaf_plum_deep", "leaf_plum_bronze"],
            ["leaf_plum_shade", "leaf_plum_shade", "leaf_plum_shade_deep", "leaf_plum_deep"],
        ),
    },
    "d": {  # red
        "summer": (
            ["leaf_red", "leaf_red", "leaf_red_light", "leaf_red_deep", "leaf_red_scarlet"],
            ["leaf_red_shade", "leaf_red_shade", "leaf_red_shade_deep", "leaf_red_deep"],
        ),
    },
    "e": {  # a dark black-red, with dark green through it
        "summer": (
            ["leaf_blackred", "leaf_blackred", "leaf_blackred_dark", "leaf_blackred_red", "leaf_blackred_green"],
            ["leaf_blackred_shade", "leaf_blackred_shade", "leaf_blackred_shade_green", "leaf_blackred_dark"],
        ),
    },
}

LEADER = [0.7, 0.45, 0.25]

# A version is a trunk, leaders and foliage clumps, in shaku. Trunk and leaders
# are polylines with a radius per point; clumps are (centre, radius, up, down).
# With a "canopy" centre the clumps are wrapped in one continuous skin (a wild
# crown); without one each clump is its own blob (clipped pads).
VERSIONS = {
    "00": {
        "seed": 0,
        "lumpiness": 0.15,
        "trunk": (
            [(0, 0, -0.5), (0, 0, 1.5), (0.2, 0.1, 3.5), (0.3, 0.2, 5.5)],
            [1.8, 1.35, 1.15, 1.0],
        ),
        "leaders": [  # splitting low, spreading up into the underside of the crown
            ([(0.3, 0.2, 4.5), (5, 1, 8.5), (9, 2, 12.5)], LEADER),
            ([(0.3, 0.2, 4.5), (-4, 4, 8.5), (-7, 7, 12.5)], LEADER),
            ([(0.3, 0.2, 4.5), (-3, -4, 9), (-5, -8, 13)], LEADER),
            ([(0.3, 0.2, 4.5), (4, -4, 9), (7, -7, 13)], LEADER),
            ([(0.3, 0.2, 4.5), (3, 5, 9), (4, 8, 13)], [0.6, 0.4, 0.22]),
            ([(0.3, 0.2, 5), (0.6, 0.6, 10), (1, 1, 15)], [0.75, 0.55, 0.35]),
        ],
        "canopy": (1, 0, 16),  # the crown is one skin over these lumps
        "clumps": [
            # the cap: a tall dome with a flattish underside
            ((1, 0, 15), 14, 11, 2.5),
            # lumps around the rim, drooping below the cap's underside like the
            # curled edge of a mushroom
            ((12, 2, 14), 6, 4.5, 3),
            ((-9, 9, 13.5), 6.5, 4.5, 3),
            ((-7, -9, 14), 6, 4.5, 3),
            ((9, -8, 14.5), 5.5, 4.5, 3),
            ((5, 11, 14.5), 5.5, 4, 3),
            ((-13, -1, 14), 5.5, 4, 3),
            # lumps on the upper slopes and top
            ((6, -4, 20), 6.5, 4.5, 2),
            ((-6, 4, 20.5), 6.5, 4.5, 2),
            ((1, 1, 24), 6.5, 4.5, 2),
        ],
    },
    "pruned-00": {  # cloud-pruned in flat tiers, for a garden tree
        "seed": 0,
        "lumpiness": 0.12,
        "trunk": (
            [(0, 0, -0.5), (0, 0, 1.5), (0.2, 0.1, 3.5), (0.3, 0.2, 5.5)],
            [1.8, 1.35, 1.15, 1.0],
        ),
        "leaders": [  # splitting low, spreading wide
            ([(0.3, 0.2, 4.5), (5, 1, 8), (10, 2, 11.5)], [0.7, 0.45, 0.25]),
            ([(0.3, 0.2, 4.5), (-4, 4, 8), (-8, 8, 11)], [0.7, 0.45, 0.25]),
            ([(0.3, 0.2, 4.5), (-3, -4, 8.5), (-6, -9, 12)], [0.7, 0.45, 0.25]),
            ([(0.3, 0.2, 4.5), (4, -4, 9), (8, -8, 12.5)], [0.7, 0.45, 0.25]),
            ([(0.3, 0.2, 4.5), (3, 5, 9), (5, 9, 12.5)], [0.6, 0.4, 0.22]),
            ([(0.3, 0.2, 5), (0.6, 0.6, 11), (1, 1, 17), (1, 1.5, 23)], [0.75, 0.55, 0.35, 0.2]),
        ],
        "clumps": [  # flat layers in three tiers
            # low tier, on the ends of the spreading leaders
            ((11, 2, 12.5), 7, 3, 3),
            ((-8, 8, 12), 7, 3, 3),
            ((-6, -9, 13), 7, 3, 3),
            ((8, -8, 13.5), 6.5, 3, 3),
            ((5, 9, 13.5), 6, 3, 3),
            # middle tier
            ((4, -2, 19), 8, 3.5, 3.5),
            ((-4, 3, 19.5), 8, 3.5, 3.5),
            ((2, 6, 20), 6, 3, 3),
            ((-3, -5, 19), 6.5, 3, 3),
            # the crown's top
            ((1, 1.5, 25), 6.5, 4, 4),
        ],
    },
}


def build_maple(version="00", colour="a", season="summer"):
    spec = VERSIONS[version]
    leaves, shades = COLOURS[colour][season]
    rng = flora.rng_for(spec["seed"])
    b = loft.Builder()
    flora.branch(b, *spec["trunk"], BARK)
    for points, radii in spec["leaders"]:
        flora.branch(b, points, radii, BARK)
    if "canopy" in spec:
        flora.canopy(b, spec["canopy"], spec["clumps"], leaves, shades, rng)
    else:
        for centre, radius, up, down in spec["clumps"]:
            flora.clump(b, centre, radius, up, down, leaves, shades, rng, spec["lumpiness"])
    return b.finish(f"Maple-{version}-{colour}-{season}")


if __name__ == "__main__":
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    parser = argparse.ArgumentParser(prog="maple.py")
    parser.add_argument("--version", default="00", choices=VERSIONS)
    parser.add_argument("--colour", default="a", choices=COLOURS)
    parser.add_argument("--season", default="summer")
    args, _ = parser.parse_known_args(argv)
    loft.run(
        lambda: build_maple(args.version, args.colour, args.season),
        f"maple-{args.version}-{args.colour}-{args.season}",
        target=(1, 0, 14),
        extent=40,
    )

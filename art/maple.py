"""The Japanese maple (イロハモミジ): a mature hillside tree, about 30 shaku (~9 m) tall.

    blender -b --python art/maple.py -- --out assets/models/maple-summer-00.glb [--renders <dir>]

A short, thick trunk splits low into spreading leaders, each carrying a faceted
foliage clump, so the crown is wide and layered like an umbrella.

Each numbered version is an entry in VERSIONS; the season only picks the
foliage colours, so every version comes in every season.
"""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import flora  # noqa: E402
import loft  # noqa: E402

BARK = "bark"
SEASONS = {  # season: (foliage, foliage underside)
    "summer": ("maple_green", "maple_green_shade"),
}

# A version is a trunk, leaders and foliage clumps, in shaku. Trunk and leaders
# are polylines with a radius per point; clumps are (centre, radius, height).
VERSIONS = {
    "00": {
        "seed": 0,
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
            ((11, 2, 12.5), 7, 3),
            ((-8, 8, 12), 7, 3),
            ((-6, -9, 13), 7, 3),
            ((8, -8, 13.5), 6.5, 3),
            ((5, 9, 13.5), 6, 3),
            # middle tier
            ((4, -2, 19), 8, 3.5),
            ((-4, 3, 19.5), 8, 3.5),
            ((2, 6, 20), 6, 3),
            ((-3, -5, 19), 6.5, 3),
            # the crown's top
            ((1, 1.5, 25), 6.5, 4),
        ],
    },
}


def build_maple(version="00", season="summer"):
    spec = VERSIONS[version]
    leaf, shade = SEASONS[season]
    rng = flora.rng_for(spec["seed"])
    b = loft.Builder()
    flora.branch(b, *spec["trunk"], BARK)
    for points, radii in spec["leaders"]:
        flora.branch(b, points, radii, BARK)
    for centre, radius, height in spec["clumps"]:
        flora.clump(b, centre, radius, height, leaf, shade, rng)
    return b.finish(f"Maple-{season}-{version}")


if __name__ == "__main__":
    loft.run(build_maple, "maple-summer-00", target=(1, 0, 14), extent=40)

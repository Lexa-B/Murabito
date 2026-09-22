"""The hinoki cypress (ヒノキ): a village tree about 52 shaku (~16 m) tall and 8 m across.

    blender -b --python art/entity_models/flora/trees/hinoki.py -- --out assets/entity_models/flora/trees/hinoki-00-a-summer.glb \
        [--version 00] [--colour a] [--season summer] [--renders <dir>]

The prized building timber of the mountains. Built like the sugi, but set
apart from it three ways: an egg-shaped crown that stays broad well up and
closes in a rounded dome, where the sugi is a narrow spire; wide, flat fans of
foliage hanging in tiers from drooping branches, each tier and third of the
crown wrapped as one skin, where the sugi has separate knobbly tufts; and a
brighter, glossier green, pale grey-green beneath after the white markings
under real hinoki sprays. Its bark is redder, too.

Named hinoki-<version>-<colour>-<season>, as the other trees are.
"""

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(next(p for p in Path(__file__).resolve().parents if (p / "loft.py").exists())))
import flora  # noqa: E402
import loft  # noqa: E402

BARK = ["hinoki_bark", "hinoki_bark", "hinoki_bark_light", "hinoki_bark_dark"]  # picked per face

# colour variant: season: (foliage colours, underside colours), picked per triangle.
COLOURS = {
    "a": {  # glossy deep green, pale beneath
        "summer": (
            ["hinoki_green", "hinoki_green", "hinoki_green_light", "hinoki_green_deep", "hinoki_green_blue"],
            ["hinoki_under_pale", "hinoki_under_pale", "hinoki_shade", "hinoki_green_deep"],
        ),
    },
}

# A version is a trunk and the rules for growing branches and sprays, in shaku,
# all drawn from its seed. See flora.conifer and flora.grow_branches.
VERSIONS = {
    "00": {
        "seed": 0,
        "trunk": (
            [(0, 0, -0.5), (0, 0, 3), (0.1, 0, 12), (0.1, 0.1, 25), (0.2, 0.1, 38), (0.2, 0.1, 48)],
            [2.5, 1.9, 1.5, 1.1, 0.7, 0.35],
        ),
        "branches": {
            "count": 34,
            "from": 7,  # the crown reaches low
            "to": 47,
            "length": (8, 3.5),  # still egg-shaped, but a village tree's width
            "taper": 2.5,  # staying wide, then pulling in quickly: an egg, not a spire
            "pad": (4.5, 3.5),
            "fork_chance": 0.35,
            "rise": (0.0, 0.08),
            "droop": 0.22,  # branches sag, so the fans hang in tiers
            "wander": 0.8,
            "thickness": (0.45, 0.18),
            "lift": (0, 0),  # fans centred on the branch tips, which end inside them
        },
        "tuft": {"up": 1.4, "down": 0.7},  # wide, flat fans
        "tip": {"radius": 5, "up": 3, "down": 1.5},  # a rounded dome on top
        "clusters": {"tier": 7, "sectors": 3},  # each tier and third wrapped as one skin
    },
}


def build_hinoki(version="00", colour="a", season="summer"):
    foliage, shades = COLOURS[colour][season]
    b = loft.Builder()
    flora.conifer(b, VERSIONS[version], foliage, shades, BARK)
    return b.finish(f"Hinoki-{version}-{colour}-{season}")


if __name__ == "__main__":
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    parser = argparse.ArgumentParser(prog="hinoki.py")
    parser.add_argument("--version", default="00", choices=VERSIONS)
    parser.add_argument("--colour", default="a", choices=COLOURS)
    parser.add_argument("--season", default="summer")
    args, _ = parser.parse_known_args(argv)
    height = VERSIONS[args.version]["trunk"][0][-1][2]
    loft.run(
        lambda: build_hinoki(args.version, args.colour, args.season),
        f"hinoki-{args.version}-{args.colour}-{args.season}",
        target=(0, 0, height / 2),
        extent=height + 6,  # frame each version by its own height
    )

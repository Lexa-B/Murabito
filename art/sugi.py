"""The Japanese cedar (スギ): a mature shrine-grove tree, about 70 shaku (~21 m) tall.

    blender -b --python art/sugi.py -- --out assets/models/sugi-00-a-summer.glb \
        [--version 00] [--colour a] [--season summer] [--renders <dir>]

A dead-straight trunk of stringy red-brown bark, and a narrow cone of knobbly
tufts reaching most of the way down it, on short branches that sag a little
and turn up at the ends. The tip is a pointed tuft.

Named sugi-<version>-<colour>-<season>, as the maple and red pine are.
"""

import argparse
import random
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import flora  # noqa: E402
import loft  # noqa: E402

BARK = ["sugi_bark", "sugi_bark", "sugi_bark_light", "sugi_bark_dark"]  # stringy: picked per face

# colour variant: season: (needle colours, underside colours), picked per triangle.
COLOURS = {
    "a": {  # dark, slightly yellow green
        "summer": (
            ["sugi_green", "sugi_green", "sugi_green_light", "sugi_green_deep", "sugi_green_yellow"],
            ["sugi_green_shade", "sugi_green_shade", "sugi_green_shade_deep", "sugi_green_deep"],
        ),
    },
}

# A version is a trunk and the rules for growing branches and tufts, in shaku,
# all drawn from its seed. See flora.grow_branches for the branch rules.
VERSIONS = {
    "00": {
        "seed": 0,
        "trunk": (
            [(0, 0, -0.5), (0, 0, 3), (0, 0, 15), (0.1, 0, 35), (0.2, 0.1, 55), (0.2, 0.1, 68)],
            [2.6, 1.9, 1.6, 1.2, 0.7, 0.2],
        ),
        "branches": {
            "count": 42,
            "from": 17,  # the lower quarter of the trunk is bare
            "to": 67,
            "length": (8, 1.5),  # short: the crown is a narrow cone
            "pad": (4.4, 2.5),  # big enough to merge into one knobbly cone
            "fork_chance": 0.15,
            "rise": (0.05, 0.2),
            "droop": 0.1,  # sagging, then turning up at the end
            "wander": 0.4,
            "thickness": (0.4, 0.15),
        },
        "tuft": {"up": 2.6, "down": 1.8},  # rounder than a pine's flat pads
        "tip": {"radius": 2.2, "up": 5, "down": 1.5},  # the pointed top
    },
}


def _grown(seed, height=70.0, lean=(0.2, 0.1), branches=42, crown=0.25, droop=0.1, pad=(4.4, 2.5)):
    """A version whose trunk comes from its seed too: height in shaku, crown as
    the fraction of the trunk left bare, lean as the (x, y) drift at the top.
    A sugi's trunk stays nearly straight, so it only wobbles a little."""
    rng = random.Random(seed * 7919 + 1)  # its own stream, apart from the build's
    scale = height / 70
    heights = [-0.5, 3] + [height * f for f in (0.21, 0.5, 0.79)] + [height - 2]
    points = []
    for z in heights:
        t = max(z, 0) / height
        wobble = 0 if z <= 3 else 0.15
        points.append((lean[0] * t + rng.uniform(-wobble, wobble), lean[1] * t + rng.uniform(-wobble, wobble), z))
    return {
        "seed": seed,
        "trunk": (points, [r * scale for r in (2.6, 1.9, 1.6, 1.2, 0.7, 0.2)]),
        "branches": {
            "count": branches,
            "from": height * crown,
            "to": height - 1,
            "length": (8 * scale, 1.5),
            "pad": pad,
            "fork_chance": 0.15,
            "rise": (0.05, 0.2),
            "droop": droop,
            "wander": 0.4,
            "thickness": (0.4, 0.15),
        },
        "tuft": {"up": 2.6, "down": 1.8},
        "tip": {"radius": 2.2, "up": 5, "down": 1.5},
    }


VERSIONS.update({
    "01": _grown(1, height=62, crown=0.3, branches=38, lean=(-0.6, 0.3)),
    "02": _grown(2, height=78, crown=0.33, branches=46, droop=0.14),  # a tall old grove tree
    "03": _grown(3, height=66, crown=0.18, branches=44, lean=(0.8, -0.5), pad=(4.8, 2.6)),  # full to low down
    "04": _grown(4, height=72, crown=0.4, branches=36, droop=0.06),  # a long bare trunk, timber-like
})


def build_sugi(version="00", colour="a", season="summer"):
    needles, shades = COLOURS[colour][season]
    b = loft.Builder()
    flora.conifer(b, VERSIONS[version], needles, shades, BARK)
    return b.finish(f"Sugi-{version}-{colour}-{season}")


if __name__ == "__main__":
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    parser = argparse.ArgumentParser(prog="sugi.py")
    parser.add_argument("--version", default="00", choices=VERSIONS)
    parser.add_argument("--colour", default="a", choices=COLOURS)
    parser.add_argument("--season", default="summer")
    args, _ = parser.parse_known_args(argv)
    height = VERSIONS[args.version]["trunk"][0][-1][2]
    loft.run(
        lambda: build_sugi(args.version, args.colour, args.season),
        f"sugi-{args.version}-{args.colour}-{args.season}",
        target=(0, 0, height / 2),
        extent=height + 6,  # frame each version by its own height
    )

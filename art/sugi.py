"""The Japanese cedar (スギ): a mature shrine-grove tree, about 70 shaku (~21 m) tall.

    blender -b --python art/sugi.py -- --out assets/models/sugi-00-a-summer.glb \
        [--version 00] [--colour a] [--season summer] [--renders <dir>]

A dead-straight trunk of stringy red-brown bark, and a narrow cone of knobbly
tufts reaching most of the way down it, on short branches that sag a little
and turn up at the ends. The tip is a pointed tuft.

Named sugi-<version>-<colour>-<season>, as the maple and red pine are.
"""

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import flora  # noqa: E402
import loft  # noqa: E402
from mathutils import Vector  # noqa: E402

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


def build_sugi(version="00", colour="a", season="summer"):
    spec = VERSIONS[version]
    needles, shades = COLOURS[colour][season]
    rng = flora.rng_for(spec["seed"])
    b = loft.Builder()

    points, radii = flora.densify(*spec["trunk"], step=3)
    for segment in flora.branch(b, points, radii, BARK[0]):
        for face in segment:
            b.paint([face], rng.choice(BARK))

    tuft = spec["tuft"]
    for branch_points, branch_radii, pad_centre, pad_radius in flora.grow_branches(
        spec["trunk"][0], spec["branches"], rng
    ):
        flora.branch(b, branch_points, branch_radii, BARK[0])
        flora.pad(
            b, pad_centre, pad_radius, needles, shades, rng, up=tuft["up"], down=tuft["down"],
            subdivisions=3 if pad_radius >= 4 else 2,  # keeps the triangles one size
        )
    tip = spec["tip"]
    top = Vector(spec["trunk"][0][-1]) + Vector((0, 0, 1))
    flora.pad(b, top, tip["radius"], needles, shades, rng, up=tip["up"], down=tip["down"], lumps=2, subdivisions=2)
    return b.finish(f"Sugi-{version}-{colour}-{season}")


if __name__ == "__main__":
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    parser = argparse.ArgumentParser(prog="sugi.py")
    parser.add_argument("--version", default="00", choices=VERSIONS)
    parser.add_argument("--colour", default="a", choices=COLOURS)
    parser.add_argument("--season", default="summer")
    args, _ = parser.parse_known_args(argv)
    loft.run(
        lambda: build_sugi(args.version, args.colour, args.season),
        f"sugi-{args.version}-{args.colour}-{args.season}",
        target=(0, 0, 34),
        extent=72,
    )

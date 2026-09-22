"""A woodland fern (シダ), after the Japanese shield fern (ベニシダ): a clump about
1.5 shaku (~45 cm) tall and 4 shaku across.

    blender -b --python art/entity_models/flora/ground/shida.py -- --out assets/entity_models/flora/ground/shida-00-a-summer.glb \\
        [--version 00] [--colour a] [--season summer] [--renders <dir>]

The commonest woodland fern of lowland Honshu, and filler for any shady forest
floor: a dozen fronds rising from a central crown in a loose shuttlecock,
arching up and out and over. Each frond is a long leaf widest a little below
the middle, its zig-zag edges suggesting the rows of leaflets (flora.frond).
The shield fern's new fronds unfurl coppery red: a spring to come.

Named shida-<version>-<colour>-<season>, as the other plants are.
"""

import argparse
import math
import sys
from pathlib import Path

sys.path.insert(0, str(next(p for p in Path(__file__).resolve().parents if (p / "loft.py").exists())))
import flora  # noqa: E402
import loft  # noqa: E402
from mathutils import Vector  # noqa: E402

# colour variant: season: (frond colours, underside colours), picked per triangle.
COLOURS = {
    "a": {  # mid fern green
        "summer": (
            ["fern_green", "fern_green", "fern_green_light", "fern_green_deep", "fern_green_yellow"],
            ["fern_shade", "fern_shade", "fern_shade_deep", "fern_green_deep"],
        ),
    },
}

# A version is rules for the clump, in shaku and radians, drawn from its seed.
VERSIONS = {
    "00": {
        "seed": 0,
        "fronds": 16,
        "length": (1.8, 2.6),
        "width": (0.5, 0.65),  # at the widest
        "rise": (1.0, 1.3),  # how steeply each sets off: a loose shuttlecock
        "fall": (0.1, 0.6),  # how far past level each tip arches over
        "bend": 1.6,  # the bend gathers towards the tip
    },
}


def build_shida(version="00", colour="a", season="summer"):
    spec = VERSIONS[version]
    leaves, shades = COLOURS[colour][season]
    rng = flora.rng_for(spec["seed"])
    b = loft.Builder()
    for i in range(spec["fronds"]):
        heading = i * flora.GOLDEN_ANGLE + rng.uniform(-0.3, 0.3)
        base = Vector((math.cos(heading), math.sin(heading), 0)) * rng.uniform(0.05, 0.15) + Vector((0, 0, -0.05))
        flora.frond(
            b, base, heading, rng.uniform(*spec["length"]), rng.uniform(*spec["width"]),
            rng.uniform(*spec["rise"]), rng.uniform(*spec["fall"]), leaves, shades, rng, bend=spec["bend"],
        )
    return b.finish(f"Shida-{version}-{colour}-{season}")


if __name__ == "__main__":
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    parser = argparse.ArgumentParser(prog="shida.py")
    parser.add_argument("--version", default="00", choices=VERSIONS)
    parser.add_argument("--colour", default="a", choices=COLOURS)
    parser.add_argument("--season", default="summer")
    args, _ = parser.parse_known_args(argv)
    loft.run(
        lambda: build_shida(args.version, args.colour, args.season),
        f"shida-{args.version}-{args.colour}-{args.season}",
        target=(0, 0, 0.7),
        extent=4.8,
    )

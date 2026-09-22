"""A grass tuft (草, kusa): a small clump of wild grass about 1.2 shaku (~35 cm) tall.

    blender -b --python art/kusa.py -- --out assets/models/kusa-00-a-summer.glb \\
        [--version 00] [--colour a] [--season summer] [--renders <dir>]

Generic wild grass for path edges, clearings and field margins, scattered by
the hundred: a few dozen narrow blades rising from one point, curving up and
out and arching over at their tips. Each blade is a frond without leaflets
(flora.frond, leaflets=False) of only four segments, so a tuft stays light.
A dry, straw-coloured autumn is a season to come.

Named kusa-<version>-<colour>-<season>, as the other plants are.
"""

import argparse
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import flora  # noqa: E402
import loft  # noqa: E402
from mathutils import Vector  # noqa: E402

# colour variant: season: (blade colours, underside colours), picked per face.
COLOURS = {
    "a": {  # fresh, yellowish green
        "summer": (
            ["grass_green", "grass_green", "grass_green_light", "grass_green_deep", "grass_green_yellow"],
            ["grass_shade", "grass_shade", "grass_shade_deep", "grass_green_deep"],
        ),
    },
}

# A version is rules for the tuft, in shaku and radians, drawn from its seed.
VERSIONS = {
    "00": {
        "seed": 0,
        "blades": 36,
        "length": (0.7, 1.7),
        "width": (0.07, 0.11),  # at the base
        "rise": (0.95, 1.45),  # how steeply each sets off
        "fall": (0.0, 0.8),  # how far past level each tip arches over
        "bend": 1.8,
        "segments": 4,
    },
}


def build_kusa(version="00", colour="a", season="summer"):
    spec = VERSIONS[version]
    leaves, shades = COLOURS[colour][season]
    rng = flora.rng_for(spec["seed"])
    b = loft.Builder()
    for i in range(spec["blades"]):
        heading = i * flora.GOLDEN_ANGLE + rng.uniform(-0.3, 0.3)
        base = Vector((math.cos(heading), math.sin(heading), 0)) * rng.uniform(0.0, 0.08) + Vector((0, 0, -0.05))
        flora.frond(
            b, base, heading, rng.uniform(*spec["length"]), rng.uniform(*spec["width"]),
            rng.uniform(*spec["rise"]), rng.uniform(*spec["fall"]), leaves, shades, rng,
            segments=spec["segments"], bend=spec["bend"], notch=1.0, leaflets=False, profile="grass",
        )
    return b.finish(f"Kusa-{version}-{colour}-{season}")


if __name__ == "__main__":
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    parser = argparse.ArgumentParser(prog="kusa.py")
    parser.add_argument("--version", default="00", choices=VERSIONS)
    parser.add_argument("--colour", default="a", choices=COLOURS)
    parser.add_argument("--season", default="summer")
    args, _ = parser.parse_known_args(argv)
    loft.run(
        lambda: build_kusa(args.version, args.colour, args.season),
        f"kusa-{args.version}-{args.colour}-{args.season}",
        target=(0, 0, 0.6),
        extent=2.6,
    )

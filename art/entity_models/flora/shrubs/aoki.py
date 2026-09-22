"""The aucuba (アオキ): an evergreen shade shrub about 2.8 shaku (~85 cm) tall.

    blender -b --python art/entity_models/flora/shrubs/aoki.py -- --out assets/entity_models/flora/shrubs/aoki-00-a-summer.glb \\
        [--version 00] [--colour a] [--season summer] [--renders <dir>]

The classic shrub of Honshu's shady woods. Its name, "green tree", is for its
green stems: a few rise from the base and fork, and fork again, into a bushy,
rounded shrub, each shoot with a pair of big, glossy, leathery leaves along it
and a rosette of them at its tip. The leaves are broad ovals (flora.blade with
rounded=True), ridged down the middle so they catch the light in two facets,
bigger than life so they read from above. It bears red berries in winter: a season to come.

Named aoki-<version>-<colour>-<season>, as the other plants are.
"""

import argparse
import math
import sys
from pathlib import Path

sys.path.insert(0, str(next(p for p in Path(__file__).resolve().parents if (p / "loft.py").exists())))
import flora  # noqa: E402
import loft  # noqa: E402
from mathutils import Vector  # noqa: E402

STEM = "aoki_stem"

# colour variant: season: (leaf colours, underside colours), picked per leaf.
COLOURS = {
    "a": {  # deep glossy green, some leaves catching the light
        "summer": (
            ["aoki_green", "aoki_green", "aoki_green_shine", "aoki_green_deep"],
            ["aoki_shade", "aoki_shade", "aoki_shade_light", "aoki_green_deep"],
        ),
    },
}

# A version is rules for the shrub, in shaku and radians, drawn from its seed.
VERSIONS = {
    "00": {
        "seed": 0,
        "stems": 4,  # from the base, each forking twice
        "stem": (1.0, 1.5),  # first stem length: forking starts low
        "fork_length": (0.6, 1.0),  # each fork's length, as a fraction of its parent's: tips at varied heights, a dome
        "lean": (0.15, 0.4),  # radians off upright, for the stems from the base
        "fork_turn": (0.35, 0.65),  # how far each fork turns away round the vertical
        "leaves": (5, 7),  # in the rosette at each shoot tip
        "leaf": (1.0, 0.5),  # length and width, bigger than life
    },
}


def _rosette(b, tip, spec, leaves, shades, rng):
    """Big leaves radiating from a shoot tip, drooping a little."""
    length, width = spec["leaf"]
    flora.blade_cluster(
        b, tip, Vector((1, 0, 0)), rng.randint(*spec["leaves"]), length, width,
        leaves, shades, rng, spread=math.pi, droop=(0.05, 0.45), rounded=True,
    )


def _pair(b, point, heading, spec, leaves, shades, rng):
    """Two leaves either side of a shoot, as aucuba's grow."""
    length, width = spec["leaf"]
    for side in (1, -1):
        turn = heading + side * math.pi / 2 + rng.uniform(-0.3, 0.3)
        way = Vector((math.cos(turn), math.sin(turn), -rng.uniform(0.1, 0.4)))
        flora.blade(b, point, way, length * 0.85, width * 0.9, rng.choice(leaves), rng.choice(shades),
                    droop=0.15, rounded=True)


def _shoot(b, start, heading, lean, length, radius, forks, spec, leaves, shades, rng):
    """A green shoot leaning heading-ward by lean radians, with a pair of leaves
    along it; it forks in two until forks runs out, then ends in a rosette."""
    way = Vector((math.cos(heading) * math.sin(lean), math.sin(heading) * math.sin(lean), math.cos(lean)))
    end = start + way * length
    flora.branch(b, [start, start.lerp(end, 0.5), end], [radius, radius * 0.85, radius * 0.7], STEM)
    _pair(b, start.lerp(end, 0.7), heading, spec, leaves, shades, rng)
    if forks == 2:  # the stems from the base: leaves low down too, so the dome reaches the ground
        _pair(b, start.lerp(end, 0.35), heading + 0.8, spec, leaves, shades, rng)
    if forks == 0:
        _rosette(b, end, spec, leaves, shades, rng)
        return
    for side in (1, -1):
        turn = heading + side * rng.uniform(*spec["fork_turn"])
        _shoot(b, end, turn, min(lean + rng.uniform(0.0, 0.2), 1.0), length * rng.uniform(*spec["fork_length"]),
               radius * 0.7, forks - 1, spec, leaves, shades, rng)


def build_aoki(version="00", colour="a", season="summer"):
    spec = VERSIONS[version]
    leaves, shades = COLOURS[colour][season]
    rng = flora.rng_for(spec["seed"])
    b = loft.Builder()
    for i in range(spec["stems"]):
        heading = i * flora.GOLDEN_ANGLE + rng.uniform(-0.3, 0.3)
        base = Vector((math.cos(heading), math.sin(heading), 0)) * rng.uniform(0, 0.2) + Vector((0, 0, -0.3))
        _shoot(b, base, heading, rng.uniform(*spec["lean"]), rng.uniform(*spec["stem"]), 0.08, 2,
               spec, leaves, shades, rng)
    return b.finish(f"Aoki-{version}-{colour}-{season}")


if __name__ == "__main__":
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    parser = argparse.ArgumentParser(prog="aoki.py")
    parser.add_argument("--version", default="00", choices=VERSIONS)
    parser.add_argument("--colour", default="a", choices=COLOURS)
    parser.add_argument("--season", default="summer")
    args, _ = parser.parse_known_args(argv)
    loft.run(
        lambda: build_aoki(args.version, args.colour, args.season),
        f"aoki-{args.version}-{args.colour}-{args.season}",
        target=(0, 0, 1.8),
        extent=6.5,
    )

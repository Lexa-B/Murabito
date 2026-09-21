"""The Japanese red pine (アカマツ): a mature hillside tree, about 50 shaku (~15 m) tall.

    blender -b --python art/redpine.py -- --out assets/models/redpine-00-a-summer.glb \
        [--version 00] [--colour a] [--season summer] [--renders <dir>]

The pine of village hillsides and coppice woods. A tall, crooked trunk, bare
for its lower half, grey-brown low down and orange-red above; crooked branches
in the upper third, each ending in a flat, lumpy pad of needles, with plenty
of branch showing between them.

Named redpine-<version>-<colour>-<season>, as the maple is.
"""

import argparse
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import flora  # noqa: E402
import loft  # noqa: E402
from mathutils import Vector  # noqa: E402

GREY, RED = "pine_bark_grey", "pine_bark_red"
BARK_STEPS = [GREY, "pine_bark_warm", "pine_bark_mid", RED]  # grey to red, evenly

# colour variant: season: (needle colours, underside colours), picked per triangle.
COLOURS = {
    "a": {  # the ordinary dark, slightly blue green
        "summer": (
            ["needle_green", "needle_green", "needle_green_light", "needle_green_deep", "needle_green_blue"],
            ["needle_green_shade", "needle_green_shade", "needle_green_shade_deep", "needle_green_deep"],
        ),
    },
}

# A version is a trunk and the rules for growing branches and needle pads, in
# shaku, all drawn from its seed. The trunk is a polyline with a radius per point.
VERSIONS = {
    "00": {
        "seed": 0,
        "trunk": (
            [(0, 0, -0.5), (0, 0, 4), (0.8, 0.3, 12), (0.2, 0.8, 20), (1.5, 0.5, 28),
             (1.0, 1.5, 36), (2.0, 1.2, 43), (2.2, 1.6, 47.5)],
            [2.0, 1.5, 1.3, 1.15, 1.0, 0.8, 0.55, 0.3],
        ),
        "bark_blend": (10, 32),  # grey below, red above, a ragged mix between
        # dead stubs, left over from the lower branches the tree has shed
        "stubs": [(16, 2.0), (19.5, 4.2), (22, 0.9)],  # (height, angle)
        "branches": {
            "count": 16,
            "from": 23,  # height of the lowest and highest branch
            "to": 45,
            "length": (11, 4),  # at the bottom and top of the crown
            "pad": (5.0, 3.2),  # pad radius at the bottom and top
            "fork_chance": 0.5,
        },
        "top_pad": 4.5,
    },
}

GOLDEN_ANGLE = 2.39996  # radians: spreads branches evenly round the trunk


def _lerp(a, b, t):
    return a + (b - a) * t


def _branches(spec, rng):
    """Grow the branches: (points, radii, pad centre, pad radius), forks included."""
    trunk_points = spec["trunk"][0]
    rules = spec["branches"]
    out = []
    for i in range(rules["count"]):
        t = i / (rules["count"] - 1)
        z = _lerp(rules["from"], rules["to"], t) + rng.uniform(-0.8, 0.8)
        base = flora.point_at_height(trunk_points, z)
        angle = i * GOLDEN_ANGLE + rng.uniform(-0.3, 0.3)
        out_dir = Vector((math.cos(angle), math.sin(angle), 0))
        across = Vector((-out_dir.y, out_dir.x, 0))
        length = _lerp(*rules["length"], t) * rng.uniform(0.8, 1.2)
        rise = length * rng.uniform(0.2, 0.45)
        kink = base + out_dir * length * 0.5 + across * rng.uniform(-1, 1) + Vector((0, 0, rise * 0.4))
        end = base + out_dir * length + across * rng.uniform(-1, 1) + Vector((0, 0, rise))
        thick = _lerp(0.55, 0.3, t)
        pad_radius = _lerp(*rules["pad"], t) * rng.uniform(0.85, 1.15)
        out.append(([base, kink, end], [thick, thick * 0.65, 0.15], end + Vector((0, 0, 1)), pad_radius))
        if rng.random() < rules["fork_chance"]:
            turn = rng.choice((-1, 1)) * rng.uniform(0.6, 1.0)
            fork_dir = Vector((math.cos(angle + turn), math.sin(angle + turn), 0))
            fork_end = kink + fork_dir * length * 0.45 + Vector((0, 0, rise * 0.5))
            out.append(([kink, fork_end], [thick * 0.45, 0.12], fork_end + Vector((0, 0, 0.8)), pad_radius * 0.7))
    return out


def build_redpine(version="00", colour="a", season="summer"):
    spec = VERSIONS[version]
    needles, shades = COLOURS[colour][season]
    rng = flora.rng_for(spec["seed"])
    b = loft.Builder()

    points, radii = flora.densify(*spec["trunk"], step=3)
    lo, hi = spec["bark_blend"]
    for segment in flora.branch(b, points, radii, GREY):
        for face in segment:
            # a ragged change of bark: each face picks by height, with noise
            u = (face.calc_center_median().z - lo) / (hi - lo) + rng.uniform(-0.4, 0.4)
            step = min(max(int(u * len(BARK_STEPS)), 0), len(BARK_STEPS) - 1)
            b.paint([face], BARK_STEPS[step])

    for z, angle in spec["stubs"]:
        base = flora.point_at_height(spec["trunk"][0], z)
        out_dir = Vector((math.cos(angle), math.sin(angle), 0.25))
        flora.branch(b, [base, base + out_dir * 2.5, base + out_dir * 3.8], [0.4, 0.25, 0.12], GREY)

    for branch_points, branch_radii, pad_centre, pad_radius in _branches(spec, rng):
        flora.branch(b, branch_points, branch_radii, RED)
        flora.pad(
            b, pad_centre, pad_radius, needles, shades, rng,
            subdivisions=3 if pad_radius >= 4 else 2,  # keeps the triangles one size
        )
    top = Vector(spec["trunk"][0][-1]) + Vector((0, 0, 1))
    flora.pad(b, top, spec["top_pad"], needles, shades, rng)
    return b.finish(f"Redpine-{version}-{colour}-{season}")


if __name__ == "__main__":
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    parser = argparse.ArgumentParser(prog="redpine.py")
    parser.add_argument("--version", default="00", choices=VERSIONS)
    parser.add_argument("--colour", default="a", choices=COLOURS)
    parser.add_argument("--season", default="summer")
    args, _ = parser.parse_known_args(argv)
    loft.run(
        lambda: build_redpine(args.version, args.colour, args.season),
        f"redpine-{args.version}-{args.colour}-{args.season}",
        target=(1, 1, 25),
        extent=52,
    )

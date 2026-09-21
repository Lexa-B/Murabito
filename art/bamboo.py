"""A tuft of madake bamboo (真竹): a clump of culms about 30-40 shaku (~9-12 m) tall.

    blender -b --python art/bamboo.py -- --out assets/models/bamboo-00-a-summer.glb \
        [--version 00] [--colour a] [--season summer] [--renders <dir>]

Madake rather than mōsō: mōsō, the giant grove bamboo, only reached Japan in
the 1700s. Culms rise from a tight clump and arch gently outward; each is
segmented, with a pale band at every node, green above and yellowing near the
ground. Clusters of bright yellow-green leaves hang along each
culm's upper part and off its tip, each a cluster of a few long, pointed
blades fanning out and drooping (flora.blade_cluster); bamboo's narrow leaves
are its defining look, so they are blades rather than lumpy skins.

Named bamboo-<version>-<colour>-<season>, as the trees are.
"""

import argparse
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import flora  # noqa: E402
import loft  # noqa: E402
from mathutils import Vector  # noqa: E402

CULM, CULM_OLD, NODE = "bamboo_culm", "bamboo_culm_old", "bamboo_node"
NODE_BAND = 0.2  # shaku: how tall each pale node band is

# colour variant: season: (leaf colours, underside colours), picked per triangle.
COLOURS = {
    "a": {  # bright yellow-green
        "summer": (
            ["bamboo_leaf", "bamboo_leaf", "bamboo_leaf_light", "bamboo_leaf_deep", "bamboo_leaf_yellow"],
            ["bamboo_leaf_shade", "bamboo_leaf_shade", "bamboo_leaf_shade_light", "bamboo_leaf_deep"],
        ),
    },
}

# A version is rules for growing the clump, in shaku, all drawn from its seed.
VERSIONS = {
    "00": {
        "seed": 0,
        "culms": 11,
        "spread": 2.5,  # how far from the clump's middle culms rise
        "height": (28, 40),
        "lean": (0.18, 0.35),  # how far the tip arches out, as a fraction of height
        "radius": (0.22, 0.3),  # at the base
        "node_step": 1.8,
        "leaves_from": 0.55,  # leaves on the upper part of each culm
        "cluster_step": 1.3,  # shaku between clusters of blades up a culm
        "blades": (4, 6),  # blades per cluster
        "blade": (2.6, 0.5),  # length and width, in shaku: much bigger than life, to read
        "droop": (0.2, 0.6),  # how far each blade dips, in radians
    },
}


def _culm(b, base, height, out, lean, radius, spec):
    """Loft one culm, node by node, arching out along out; returns where along
    it (a function of 0..1) things can hang."""

    def at(t):
        return base + out * (lean * height * t * t) + Vector((0, 0, height * t))

    points, radii, colours = [at(0)], [radius], []
    z = spec["node_step"]
    while z + NODE_BAND < height:
        t = z / height
        r = radius * (1 - 0.6 * t)
        points += [at(t), at((z + NODE_BAND) / height)]
        radii += [r * 1.15, r * 1.15]
        colours += [CULM_OLD if t < 0.25 else CULM, NODE]
        z += spec["node_step"]
    points.append(at(1))
    radii.append(radius * 0.3)
    colours.append(CULM)
    flora.branch(b, points, radii, colours)
    return at


def build_bamboo(version="00", colour="a", season="summer"):
    spec = VERSIONS[version]
    leaves, shades = COLOURS[colour][season]
    rng = flora.rng_for(spec["seed"])
    b = loft.Builder()

    for i in range(spec["culms"]):
        angle = i * flora.GOLDEN_ANGLE + rng.uniform(-0.4, 0.4)
        reach = spec["spread"] * math.sqrt(rng.uniform(0.05, 1))
        base = Vector((math.cos(angle) * reach, math.sin(angle) * reach, -0.3))
        out = Vector((math.cos(angle), math.sin(angle), 0))
        height = rng.uniform(*spec["height"])
        at = _culm(b, base, height, out, rng.uniform(*spec["lean"]), rng.uniform(*spec["radius"]), spec)

        # clusters of drooping blades on alternate sides up the upper culm,
        # and a bigger one hanging off the tip
        length, width = spec["blade"]
        t = spec["leaves_from"]
        side = 1
        while t < 0.97:
            turn = angle + side * rng.uniform(0.9, 2.0)
            flora.blade_cluster(
                b, at(t), Vector((math.cos(turn), math.sin(turn), 0)), rng.randint(*spec["blades"]),
                length, width, leaves, shades, rng, droop=spec["droop"],
            )
            t += spec["cluster_step"] / height
            side = -side
        flora.blade_cluster(
            b, at(1), out, spec["blades"][1] + 2, length * 1.1, width, leaves, shades, rng,
            spread=1.4, droop=(0.5, 1.1),
        )
    return b.finish(f"Bamboo-{version}-{colour}-{season}")


if __name__ == "__main__":
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    parser = argparse.ArgumentParser(prog="bamboo.py")
    parser.add_argument("--version", default="00", choices=VERSIONS)
    parser.add_argument("--colour", default="a", choices=COLOURS)
    parser.add_argument("--season", default="summer")
    args, _ = parser.parse_known_args(argv)
    top = VERSIONS[args.version]["height"][1]
    loft.run(
        lambda: build_bamboo(args.version, args.colour, args.season),
        f"bamboo-{args.version}-{args.colour}-{args.season}",
        target=(0, 0, top / 2),
        extent=top + 6,
    )

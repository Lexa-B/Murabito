"""The mountain cherry (山桜, yamazakura): a village tree about 25 shaku (~7.5 m) tall and 8-9 m across.

    blender -b --python art/entity_models/flora/trees/sakura.py -- --out assets/entity_models/flora/trees/sakura-00-a-spring.glb \
        [--version 00] [--colour a] [--season spring] [--renders <dir>]

The wild cherry of village hillsides. Somei-yoshino, the pale cherry of parks
today, only arose in the 1800s. A short, thick, gnarled trunk of dark, glossy bark
banded with pale rings splits low into big limbs that rise briefly and then
run out nearly level, kinking and twisting as they go, as a cherry's do, each
carrying lumpy masses of foliage wrapped as its own skin; together they close
into a broad, full dome, the limbs showing beneath and in a few gaps. In spring its white-to-pale-pink flowers open with reddish-
bronze young leaves among them; in summer it is dark green; in winter it is
bare, its limbs branching and forking into reddish twigs.

Named sakura-<version>-<colour>-<season>, as the other trees are.
"""

import argparse
import math
import random
import sys
from pathlib import Path

sys.path.insert(0, str(next(p for p in Path(__file__).resolve().parents if (p / "loft.py").exists())))
import flora  # noqa: E402
import loft  # noqa: E402
from mathutils import Vector  # noqa: E402

BARK, BAND, TWIG = "sakura_bark", "sakura_bark_band", "sakura_twig"

# colour variant: season: (leaf colours, underside colours), picked per triangle.
# A season of None is bare, as a cherry is in winter.
# Within a variant every leafy season's lists must be the same lengths: picking
# draws from the same random stream as the lumps' shapes, and a different
# length would change the tree's shape between seasons, not just its colour.
COLOURS = {
    "a": {  # yamazakura: pale blossom flecked with bronze young leaves
        "spring": (
            ["sakura_petal"] * 5 + ["sakura_petal_white"] * 3 + ["sakura_petal_pink"] * 3 + ["sakura_leaf_bronze"],
            ["sakura_petal_shade", "sakura_petal_shade", "sakura_petal_shade_deep", "sakura_petal_pink"],
        ),
        "summer": (
            ["sakura_green"] * 6 + ["sakura_green_light"] * 3 + ["sakura_green_deep"] * 3,
            ["sakura_shade", "sakura_shade", "sakura_shade_light", "sakura_green_deep"],
        ),
        "winter": None,  # bare: twigs instead of foliage
    },
}
for _seasons in COLOURS.values():
    _leafy = [lists for lists in _seasons.values() if lists]
    assert len({tuple(map(len, lists)) for lists in _leafy}) == 1, "season lists differ in length"

LIMB = [1.3, 0.85, 0.5, 0.25]  # thick where they leave the trunk

# A version is a trunk, limbs and the masses of foliage on them, in shaku.
# Trunk and limbs are polylines with a radius per point. Each limb carries a
# big mass at its end, side masses flanking it and one partway along, all
# wrapped as one skin; "fill" masses close the top of the crown. Masses are
# (radius, up, down); fill masses are (centre, radius, up, down).
VERSIONS = {
    "00": {
        "seed": 0,
        "trunk": ([(0, 0, -0.5), (0.2, 0, 2.5), (0.7, 0.3, 5), (0.3, 0.2, 7.5)], [2.8, 2.05, 1.8, 1.6]),
        "gnarl": (0.3, 0.8),  # how far the trunk and the limbs are pushed off line
        "limbs": [  # rising briefly from the fork, then running out nearly level
            ([(0.15, 0.1, 6.5), (2, 0.5, 9), (5.5, 1.25, 10), (9, 1, 10.5)], LIMB),
            ([(0.15, 0.1, 6.5), (-1.5, 1.5, 9), (-4.5, 4.5, 10.5), (-7, 6.5, 10.5)], LIMB),
            ([(0.15, 0.1, 7), (-1.5, -2, 9.5), (-4, -5.5, 11), (-6, -7, 11)], LIMB),
            ([(0.15, 0.1, 7), (2, -2, 10), (4.5, -5.5, 11.5), (6.5, -7, 12)], LIMB),
            ([(0.15, 0.1, 7.5), (0.5, 2.5, 10.5), (1.5, 6, 12), (2, 8, 12.5)], LIMB),
            ([(0.15, 0.1, 7.5), (0.5, 0.5, 12), (1, 0.5, 16), (0.5, 1, 19)], LIMB),  # the one that climbs
        ],
        "end": (5.5, 4.2, 1.9),  # full above, shallow beneath, so the level limbs show
        "side": (4.2, 3.4, 1.5),
        "mid": (4.0, 3.4, 1.4),
        "fill": [((2.5, -2.5, 18), 6, 5, 3.0), ((-2.5, 2.5, 18), 6, 5, 3.0), ((0, 0, 21), 6, 4, 3.0)],
    },
}


def _grown(seed, fork=7.0, limbs=6, reach=(13, 19), rise=1.0, girth=1.0, gnarl=(0.3, 0.8)):
    """A version whose trunk and limbs come from its seed: fork is the trunk's
    height in shaku, reach the range of limb lengths, rise how much the limbs
    climb (1 is version 00's; less is more level), girth scales the trunk."""
    rng = random.Random(seed * 7919 + 1)  # its own stream, apart from the build's
    top = Vector((0.3, 0.2, fork))
    trunk = (
        [(0, 0, -0.5), (0.2, 0, fork * 0.33), (0.6, 0.3, fork * 0.66), tuple(top)],
        [r * girth for r in (2.8, 2.05, 1.8, 1.6)],
    )
    out = []
    for i in range(limbs):
        angle = i * flora.GOLDEN_ANGLE + rng.uniform(-0.3, 0.3)
        way = Vector((math.cos(angle), math.sin(angle), 0))
        length = rng.uniform(*reach)
        lift = rng.uniform(3.5, 5.5) * rise
        out.append((
            [tuple(top - Vector((0, 0, 0.5))),
             tuple(top + way * length * 0.22 + Vector((0, 0, lift * 0.6))),
             tuple(top + way * length * 0.6 + Vector((0, 0, lift * 0.85))),
             tuple(top + way * length + Vector((0, 0, lift)))],
            LIMB,
        ))
    out.append((  # the one that climbs, holding up the top
        [tuple(top), tuple(top + Vector((0.7, 0.8, 4.5))), tuple(top + Vector((1.7, 0.8, 8.5))),
         tuple(top + Vector((0.7, 1.8, 11.5)))],
        LIMB,
    ))
    return {
        **VERSIONS["00"],
        "seed": seed,
        "trunk": trunk,
        "gnarl": gnarl,
        "limbs": out,
        "fill": [
            (tuple(top + Vector((2.5, -2.5, 10.5))), 6, 5, 3.0),
            (tuple(top + Vector((-2.5, 2.5, 10.5))), 6, 5, 3.0),
            (tuple(top + Vector((0, 0, 13.5))), 6, 4, 3.0),
        ],
    }


VERSIONS.update({
    "01": _grown(1, fork=6, limbs=5, reach=(5.5, 7.5), girth=0.85),  # a younger, compact tree
    "02": _grown(2, fork=7, limbs=7, reach=(8, 11), girth=1.25, gnarl=(0.45, 1.1)),  # the broadest, and gnarled
    "03": _grown(3, fork=5.5, limbs=6, reach=(7, 9.5), rise=0.55),  # low and sprawling, limbs almost flat
    "04": _grown(4, fork=8.5, limbs=6, reach=(6, 8.5), rise=1.4, girth=0.95),  # a taller trunk, limbs lifting
})


def _masses(points, spec, rng):
    """The masses of foliage a limb carries: a big one on its end, two flanking
    it a little lower, and one partway along."""
    end, before = Vector(points[-1]), Vector(points[-2])
    ahead = Vector((end.x - before.x, end.y - before.y, 0)).normalized()
    across = Vector((-ahead.y, ahead.x, 0))

    def jitter():
        return Vector((rng.uniform(-1, 1), rng.uniform(-1, 1), rng.uniform(-0.5, 0.5)))

    masses = [(end + Vector((0, 0, 1)) + jitter(), *spec["end"])]
    for side in (1, -1):
        masses.append((end - ahead * 1.8 + across * side * 3.0 + Vector((0, 0, -0.5)) + jitter(), *spec["side"]))
    masses.append((before + Vector((0, 0, 1.5)) + jitter(), *spec["mid"]))
    return masses


def _twigs(b, start, direction, length, radius, depth, rng):
    """A bare branch that forks, and forks again, into twigs."""
    end = start + direction.normalized() * length
    middle = start.lerp(end, 0.5) + Vector([rng.uniform(-0.3, 0.3) * length * 0.3 for _ in range(3)])
    flora.branch(b, [start, middle, end], [radius, radius * 0.75, radius * 0.5], TWIG)
    if depth == 0:
        return
    for _ in range(2):  # forks spread mostly sideways
        spread = Vector((rng.uniform(-0.8, 0.8), rng.uniform(-0.8, 0.8), rng.uniform(-0.15, 0.3)))
        _twigs(b, end, direction.normalized() + spread, length * 0.7, radius * 0.5, depth - 1, rng)


def _bare_branches(b, points, radii, rng):
    """Branches rising off a limb along its length, where its foliage would be."""
    climbing = Vector((points[-1].x - points[0].x, points[-1].y - points[0].y, 0)).length < 4
    for fraction in (0.5, 1.0) if climbing else (0.45, 0.75, 1.0):
        i = min(int(fraction * (len(points) - 1)), len(points) - 1)
        outward = Vector((points[i].x - points[0].x, points[i].y - points[0].y, 0))
        if outward.length < 0.5 or climbing:  # the climbing limb: out to a random side
            angle = rng.uniform(0, 2 * math.pi)
            outward = Vector((math.cos(angle), math.sin(angle), 0))
        # mostly outward, only a little up, as a cherry's branches spread
        way = outward.normalized() + Vector((0, 0, rng.uniform(0.2, 0.45)))
        way += Vector((rng.uniform(-0.3, 0.3), rng.uniform(-0.3, 0.3), 0))
        _twigs(b, points[i], way, rng.uniform(3, 4.2), max(radii[i] * 0.6, 0.15), 2, rng)  # sized to the limbs' reach


def build_sakura(version="00", colour="a", season="spring"):
    spec = VERSIONS[version]
    bare = COLOURS[colour][season] is None
    # a bare season still draws its foliage, into a builder thrown away after,
    # so the random stream, and so the trunk and limbs, match the leafy seasons
    leaves, shades = next(lists for lists in COLOURS[colour].values() if lists) if bare else COLOURS[colour][season]
    rng = flora.rng_for(spec["seed"])
    twig_rng = random.Random(spec["seed"] * 7919 + 2)
    b = loft.Builder()
    foliage = loft.Builder() if bare else b

    # a gnarled trunk in short rings, every third a pale band, as cherry bark is
    trunk_gnarl, limb_gnarl = spec["gnarl"]
    points, radii = flora.gnarl(*spec["trunk"], rng, trunk_gnarl, step=1.5)
    points, radii = flora.densify(points, radii, step=0.6)
    # sunk deep and level at the base, so it sits in sloping ground with no gap
    points, radii = flora.sink(points, radii)
    bands = [BAND if i % 3 == 2 else BARK for i in range(len(points) - 1)]
    flora.branch(b, points, radii, bands, upright=2)

    for points, radii in spec["limbs"]:
        limb_points, limb_radii = flora.gnarl(points, radii, rng, limb_gnarl, step=2.0)
        flora.branch(b, limb_points, limb_radii, BARK)
        flora.wrap_sprays(foliage, _masses(points, spec, rng), leaves, shades, rng)
        if bare:
            _bare_branches(b, limb_points, limb_radii, twig_rng)
    fill = [(Vector(c), r, up, down) for c, r, up, down in spec["fill"]]
    flora.wrap_sprays(foliage, fill, leaves, shades, rng)
    if bare:
        foliage.bm.free()
    return b.finish(f"Sakura-{version}-{colour}-{season}")


if __name__ == "__main__":
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    parser = argparse.ArgumentParser(prog="sakura.py")
    parser.add_argument("--version", default="00", choices=VERSIONS)
    parser.add_argument("--colour", default="a", choices=COLOURS)
    parser.add_argument("--season", default="spring")
    args, _ = parser.parse_known_args(argv)
    loft.run(
        lambda: build_sakura(args.version, args.colour, args.season),
        f"sakura-{args.version}-{args.colour}-{args.season}",
        target=(0, 0, 12),
        extent=48,
    )

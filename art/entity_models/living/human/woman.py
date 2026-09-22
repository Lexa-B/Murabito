"""The baseline village woman: a blockout of the body, for proportions.

    blender -b --python art/entity_models/living/human/woman.py -- \\
        --out assets/entity_models/living/human/woman.glb [--renders <dir>]

A lean, toned village woman of about 1550, 4.8 shaku (145 cm, the period's
average) and 6.5 heads tall, standing in an A-pose facing +Y: an hourglass
from waist to thighs, a round seat, a small bust. Built by body.py from the
numbers here. Faces, hair and undergarments come once the proportions are right.
"""

import sys
from pathlib import Path

sys.path.insert(0, str(next(p for p in Path(__file__).resolve().parents if (p / "loft.py").exists())))
import body  # noqa: E402
import hair  # noqa: E402
import loft  # noqa: E402

WOMAN = body.Shape(
    torso=[
        (2.38, 0.00, 0.50, 0.235, 0.35, {"bumps": [(240, 45, 0.05), (300, 45, 0.05)]}),  # hips: the legs split from here
        (2.49, 0.00, 0.50, 0.25, 0.37, {"bumps": [(240, 45, 0.07), (300, 45, 0.07)]}),  # seat, low
        (2.64, 0.00, 0.47, 0.245, 0.345, {"bumps": [(240, 45, 0.04), (300, 45, 0.04)]}),  # seat, high
        (2.78, 0.00, 0.44, 0.24, 0.30, {}),  # low belly
        (2.96, 0.00, 0.35, 0.24, 0.25, {}),  # waist
        (3.15, 0.00, 0.39, 0.28, 0.27, {}),  # ribs
        (3.34, 0.00, 0.42, 0.31, 0.29, {"bumps": [(65, 42, 0.06), (115, 42, 0.06)]}),  # bust
        (3.53, 0.00, 0.45, 0.29, 0.29, {}),  # armpits
        (3.70, 0.00, 0.44, 0.23, 0.25, {}),  # shoulders
        (3.80, 0.00, 0.32, 0.18, 0.20, {}),  # trapezius
        (3.90, -0.01, 0.17, 0.14, 0.15, {}),  # neck base
        (4.00, -0.02, 0.15, 0.13, 0.145, {}),  # neck
        (4.10, 0.02, 0.20, 0.22, 0.17, {"pinch": 0.55}),  # under the jaw, to the chin
        (4.22, 0.00, 0.27, 0.27, 0.26, {"pinch": 0.3}),  # jaw
        (4.36, 0.00, 0.305, 0.29, 0.30, {"pinch": 0.15}),  # cheeks
        (4.48, 0.00, 0.31, 0.28, 0.325, {"pinch": 0.05}),  # eyes
        (4.58, 0.00, 0.31, 0.27, 0.33, {}),  # brow
        (4.67, 0.00, 0.28, 0.245, 0.305, {}),  # temples
        (4.75, 0.00, 0.215, 0.18, 0.24, {}),  # crown
        (4.79, -0.01, 0.115, 0.095, 0.125, {}),  # top of the head
    ],
    armpit=6,
    crown=(0, -0.02, 4.80),
    crotch_drop=0.12,
    leg=[
        (0.26, -0.01, 2.09, 0.24, 0.245, 0.31, {}),  # upper thigh
        (0.25, 0.00, 1.78, 0.205, 0.235, 0.25, {}),
        (0.235, 0.00, 1.387, 0.135, 0.15, 0.13, {}),  # knee
        (0.23, 0.00, 1.18, 0.135, 0.13, 0.15, {}),
        (0.225, 0.00, 0.952, 0.145, 0.12, 0.15, {"bumps": [(270, 60, 0.03)]}),  # calf
        (0.22, 0.00, 0.60, 0.10, 0.09, 0.10, {}),
        (0.215, 0.00, 0.29, 0.08, 0.075, 0.075, {}),  # ankle
    ],
    groin=[(0.35, 0.03), (0.7, 0.015)],
    foot=[
        ((0.215, 0.00, 0.12), (0, -1, 1), 0.085, 0.11, 0.10),  # heel, turning forward
        ((0.215, 0.18, 0.085), (0, -1, 0), 0.10, 0.06, 0.085),  # instep
        ((0.215, 0.40, 0.05), (0, -1, 0), 0.10, 0.04, 0.05),  # ball of the foot
    ],
    toe=(0.215, 0.52, 0.04),
    arm_drop=45,
    shoulder_joint=(0.42, 0.0, 3.56),
    deltoid=((0.52, 0.0, 3.50), 0.13, 0.14, 0.14),
    deltoid_tilt=0.5,
    shoulder_ease=[(0.5, 0.03)],
    arm=[
        (0.40, 0.12, 0.125, 0.125),
        (0.85, 0.09, 0.09, 0.09),  # elbow
        (1.05, 0.10, 0.095, 0.095),
        (1.55, 0.07, 0.045, 0.045),  # wrist
    ],
    palm=[(0.06, 0.085, 0.037, 0.033), (0.12, 0.09, 0.031, 0.027)],
    knuckles=0.16,  # in line with the thumb's middle knuckle
    knuckle_row=[0.09, 0.045, 0.0, -0.045, -0.09],
    knuckle_depth=(0.026, 0.024),
    fingers=[0.21, 0.24, 0.22, 0.17],
    finger_radii=[0.022, 0.02, 0.018, 0.013],
    thumb=[(0.03, 0.03), (0.09, 0.027), (0.16, 0.023), (0.195, 0.017)],
    thumb_tip=0.21,
    face=body.Face(
        eye=(0.127, 4.457),
        sclera=(0.068, 0.8),
        iris=(0.034, 1.18, 0.004),
        lash=0.022,
        brow=[(0.066, 4.567, 0.013), (0.127, 4.583, 0.018), (0.187, 4.572, 0.011)],
        nose=(4.43, 4.345, 4.285, 0.027),
        mouth=(4.22, 0.038, 0.013),
        ear=[  # lobe, round to the top, leaning back
            (4.335, -0.025, 0.025, 0.008),
            (4.36, -0.03, 0.04, 0.014),
            (4.40, -0.035, 0.05, 0.02),
            (4.44, -0.042, 0.05, 0.022),
            (4.47, -0.05, 0.038, 0.018),
            (4.485, -0.056, 0.02, 0.012),
        ],
    ),
    hair=hair.Hair(
        centre=(0, -0.02, 4.45),
        hairline=[(0, 4.675), (30, 4.66), (60, 4.58), (90, 4.53), (110, 4.42), (135, 4.28), (160, 4.2), (180, 4.18)],
        thickness=(0.065, 0.01),
        volume=0.07,
        bangs=[(-45, 4.56, 0.055), (-30, 4.535, 0.06), (-15, 4.52, 0.06), (0, 4.515, 0.06), (15, 4.525, 0.06),
               (30, 4.54, 0.06), (45, 4.565, 0.055)],
        locks=[(62, 4.22, 0.08)],
        fall=[(4.45, 0.2, 0.05, -0.02), (4.3, 0.17, 0.05, 0.01), (4.2, 0.13, 0.05, 0.03), (4.1, 0.08, 0.05, 0.05)],
        tail=[(3.95, 0.095, 0.052, 0.065), (3.7, 0.115, 0.052, 0.06), (3.45, 0.105, 0.046, 0.055), (3.25, 0.075, 0.04, 0.05)],
        cord=(4.1, 0.04),
    ),
)


def build_woman():
    return body.build(WOMAN, "woman")


if __name__ == "__main__":
    loft.run(build_woman, "woman", target=(0.00, 0.00, 2.40), extent=5.2)

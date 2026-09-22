"""The baseline village man: a blockout of the body, for proportions.

    blender -b --python art/entity_models/living/human/man.py -- \\
        --out assets/entity_models/living/human/man.glb [--renders <dir>]

A lean, toned village man of about 1550, 5.18 shaku (157 cm, the period's
average) and 6.5 heads tall, standing in an A-pose facing +Y: broad shoulders
and chest over a straighter waist and narrower hips, a thicker neck, a squarer
jaw, bigger hands and feet than the woman's. Built by body.py from the numbers
here. Faces, hair and the fundoshi come once the proportions are right.
"""

import sys
from pathlib import Path

sys.path.insert(0, str(next(p for p in Path(__file__).resolve().parents if (p / "loft.py").exists())))
import body  # noqa: E402
import garment  # noqa: E402
import hair  # noqa: E402
import loft  # noqa: E402

MAN = body.Shape(
    torso=[
        (2.57, 0.00, 0.46, 0.25, 0.30, {}),  # hips: the legs split from here
        (2.69, 0.00, 0.46, 0.26, 0.32, {"bumps": [(240, 45, 0.04), (300, 45, 0.04)]}),  # seat, low
        (2.85, 0.00, 0.44, 0.26, 0.31, {"bumps": [(240, 45, 0.03), (300, 45, 0.03)]}),  # seat, high
        (3.00, 0.00, 0.42, 0.25, 0.28, {}),  # low belly
        (3.19, 0.00, 0.41, 0.26, 0.27, {}),  # waist
        (3.40, 0.00, 0.50, 0.30, 0.31, {}),  # ribs
        (3.60, 0.00, 0.56, 0.35, 0.32, {"bumps": [(60, 35, 0.03), (120, 35, 0.03)]}),  # chest
        (3.81, 0.00, 0.60, 0.32, 0.32, {}),  # armpits
        (3.99, 0.00, 0.58, 0.26, 0.28, {}),  # shoulders
        (4.10, 0.00, 0.44, 0.21, 0.24, {}),  # trapezius
        (4.21, -0.01, 0.21, 0.17, 0.18, {}),  # neck base
        (4.32, -0.02, 0.19, 0.16, 0.17, {}),  # neck
        (4.42, 0.02, 0.24, 0.24, 0.19, {"pinch": 0.4}),  # under the jaw, to the chin
        (4.55, 0.00, 0.31, 0.29, 0.28, {"pinch": 0.15}),  # jaw
        (4.71, 0.00, 0.33, 0.31, 0.32, {"pinch": 0.08}),  # cheeks
        (4.83, 0.00, 0.335, 0.30, 0.35, {"pinch": 0.03}),  # eyes
        (4.94, 0.00, 0.335, 0.30, 0.355, {}),  # brow
        (5.04, 0.00, 0.30, 0.265, 0.33, {}),  # temples
        (5.13, 0.00, 0.23, 0.195, 0.26, {}),  # crown
        (5.17, -0.01, 0.125, 0.10, 0.135, {}),  # top of the head
    ],
    armpit=6,
    crown=(0, -0.02, 5.18),
    crotch_drop=0.13,
    leg=[
        (0.235, -0.01, 2.26, 0.23, 0.25, 0.28, {}),  # upper thigh
        (0.23, 0.00, 1.92, 0.20, 0.23, 0.24, {}),
        (0.22, 0.00, 1.50, 0.14, 0.16, 0.14, {}),  # knee
        (0.215, 0.00, 1.27, 0.14, 0.14, 0.16, {}),
        (0.21, 0.00, 1.03, 0.155, 0.13, 0.17, {"bumps": [(270, 60, 0.035)]}),  # calf
        (0.205, 0.00, 0.65, 0.11, 0.10, 0.11, {}),
        (0.20, 0.00, 0.31, 0.088, 0.082, 0.082, {}),  # ankle
    ],
    groin=[(0.35, 0.0), (0.7, 0.01)],
    foot=[
        ((0.20, 0.00, 0.13), (0, -1, 1), 0.095, 0.12, 0.11),  # heel, turning forward
        ((0.20, 0.20, 0.092), (0, -1, 0), 0.11, 0.065, 0.092),  # instep
        ((0.20, 0.44, 0.055), (0, -1, 0), 0.11, 0.045, 0.055),  # ball of the foot
    ],
    toe=(0.20, 0.57, 0.045),
    arm_drop=45,
    shoulder_joint=(0.56, 0.0, 3.84),
    deltoid=((0.68, 0.0, 3.78), 0.155, 0.165, 0.165),
    deltoid_tilt=0.5,
    shoulder_ease=[(0.5, 0.03)],
    arm=[
        (0.43, 0.152, 0.152, 0.152),
        (0.92, 0.108, 0.108, 0.108),  # elbow
        (1.13, 0.125, 0.12, 0.12),
        (1.67, 0.085, 0.054, 0.054),  # wrist
    ],
    palm=[(0.065, 0.095, 0.041, 0.037), (0.13, 0.10, 0.035, 0.03)],
    knuckles=0.175,  # in line with the thumb's middle knuckle
    knuckle_row=[0.10, 0.05, 0.0, -0.05, -0.10],
    knuckle_depth=(0.029, 0.027),
    fingers=[0.23, 0.26, 0.24, 0.185],
    finger_radii=[0.025, 0.023, 0.02, 0.015],
    thumb=[(0.033, 0.034), (0.10, 0.03), (0.175, 0.026), (0.215, 0.019)],
    thumb_tip=0.23,
    face=body.Face(
        eye=(0.137, 4.81),
        sclera=(0.07, 0.65),
        iris=(0.034, 0.95, 0.003),
        lash=0.012,
        outline=[  # narrower than hers, rising to a high outer corner, the lower edge falling steeply from it
            (-1.0, -0.05), (-0.75, 0.45), (-0.3, 0.7), (0.25, 0.8), (0.7, 0.85), (1.05, 0.55),
            (0.8, -0.15), (0.4, -0.55), (-0.1, -0.65), (-0.6, -0.45),
        ],
        brow=[(0.055, 4.903, 0.013), (0.105, 4.915, 0.016), (0.165, 4.918, 0.015), (0.225, 4.908, 0.011)],
        nose=(4.78, 4.688, 4.623, 0.032),
        mouth=(4.553, 0.045, 0.013),
        ear=[  # lobe, round to the top, leaning back
            (4.677, -0.0275, 0.0275, 0.008),
            (4.705, -0.033, 0.044, 0.014),
            (4.748, -0.0385, 0.055, 0.02),
            (4.792, -0.046, 0.055, 0.022),
            (4.824, -0.055, 0.042, 0.018),
            (4.84, -0.062, 0.022, 0.012),
        ],
    ),
    hair=hair.Hair(
        centre=(0, -0.02, 4.80),
        hairline=[(0, 5.04), (20, 5.02), (40, 4.99), (60, 4.94), (90, 4.9), (110, 4.78), (135, 4.62), (160, 4.5),
                  (180, 4.46)],
        thickness=(0.07, 0.012),
        volume=0.03,
        sweeps=[(-50, 0.09), (-30, 0.1), (-10, 0.1), (10, 0.1), (30, 0.1), (50, 0.09)],
        swoops=[(8, 58, 4.74, 0.075), (18, 72, 4.7, 0.07)],
        ahoge=[(-20, 0.1, 0.1), (25, 0.3, 0.08), (-60, 0.45, 0.07)],
        wisps=[(70, 0.07), (95, 0.06), (150, 0.08), (170, 0.09), (-160, 0.08), (-140, 0.07), (-95, 0.06), (-72, 0.07)],
        knot=(0.24, 0.09, 0.09),
        tuft=[(0, 28, 0.18, 0.1), (50, 32, 0.16, 0.095), (-50, 32, 0.16, 0.095), (100, 40, 0.14, 0.09),
              (-100, 40, 0.14, 0.09), (0, 5, 0.17, 0.09), (150, 30, 0.13, 0.08), (-150, 30, 0.13, 0.08)],
    ),
    garments=[garment.Fundoshi(belt=2.76, crotch=2.44, apron=2.1, back=0.5, under=0.2, front=0.28, flap=0.36)],
)


def build_man():
    return body.build(MAN, "man")


if __name__ == "__main__":
    loft.run(build_man, "man", target=(0.00, 0.00, 2.60), extent=5.6)

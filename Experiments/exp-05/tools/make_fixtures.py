"""Write reference cases from exp-03's tested hexaddr.py, for hexworld's Rust port to
check itself against. exp-03 is read only; nothing there is changed.

Run:  uv run --no-project python tools/make_fixtures.py
"""

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
EXP03_SRC = HERE.parent.parent / "exp-03" / "src"
OUT = HERE.parent / "crates" / "hexworld" / "tests" / "fixtures" / "exp03_cases.txt"

sys.path.insert(0, str(EXP03_SRC))

import hexaddr  # noqa: E402

LEVEL_NAMES = {hexaddr.SHAKU: "shaku", hexaddr.KEN: "ken", hexaddr.CHO: "cho", hexaddr.RI: "ri"}

# A spread of cells: around the origin, across ken/cho/ri borders, and negative, where
# floor and rounding behaviour differs between languages.
CELLS = [(q, r) for q in range(-14, 15) for r in range(-14, 15)]
CELLS += [(q, r) for q in (-12_960, -361, -359, -7, 359, 361, 12_960) for r in (-181, -6, 0, 6, 181)]


def main() -> None:
    lines = []
    for n in (6, 36, 60):
        for q, r in CELLS:
            a, b = hexaddr.owner((q, r), n)
            lines.append(f"owner {n} {q} {r} -> {a} {b}")
    for level in (hexaddr.SHAKU, hexaddr.KEN, hexaddr.CHO):
        for q, r in CELLS:
            for to in range(level + 1, hexaddr.RI + 1):
                a, b = hexaddr.up((q, r), level, to)
                lines.append(f"up {LEVEL_NAMES[level]} {q} {r} {LEVEL_NAMES[to]} -> {a} {b}")
            a, b = hexaddr.local((q, r), level)
            lines.append(f"local {LEVEL_NAMES[level]} {q} {r} -> {a} {b}")
    for level in (hexaddr.KEN, hexaddr.CHO, hexaddr.RI):
        lines.append(f"owned_count {LEVEL_NAMES[level]} -> {len(hexaddr.child_offsets(level))}")

    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text("\n".join(lines) + "\n")
    print(f"{len(lines)} cases -> {OUT}")


if __name__ == "__main__":
    main()

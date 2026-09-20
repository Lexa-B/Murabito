# /// script
# requires-python = ">=3.12"
# dependencies = ["numpy"]
# ///
"""Generate a 16-bit heightmap PNG for a one-ri hexagonal Landscape.

The hexagon is pointy-top and 1 ri (129600/33 m, about 3927 m) flat to flat, centred on the
world origin: its flat sides face +X and -X, its corners point along +Y and -Y. The heightmap is
a square that covers the whole hexagon; the landscape material hides everything outside it.

Height is layered noise with an erosion filter on top: branching gullies and sharp ridges,
made without simulating water. For each octave, stripes run downhill (along the slope at that
point), each grid cell has its own pivot for the stripes, and neighbouring cells are blended.
Each octave follows the slopes left by the octaves before it, so smaller gullies branch off
larger ones. Where the ground is flat the stripes fade toward a target that is a crease low
down and a ridge high up, which keeps peaks and valleys crisp.

Run from the repo root:  uv run Tools/make_heightmap.py [--seed N] [--out DIR]
It prints the numbers to type into the editor's Landscape import.
"""

import argparse
import math
import struct
import zlib
from pathlib import Path

import numpy as np

# Landscape grid: 2017 vertices per side (a UE-recommended size), 2.26 m apart.
VERTS = 2017
SPACING_M = 2.26
RI_M = 129600 / 33                      # one ri, flat to flat
HALF_FLAT_M = RI_M / 2
Z_SCALE = 100                           # Landscape Z scale; 16-bit range is then +/- 256 m


def gradient_noise(x: np.ndarray, y: np.ndarray, rng: np.random.Generator) -> np.ndarray:
    """2D gradient noise in about [-1, 1], one lattice cell per unit of x and y."""
    table = 256
    perm = rng.permutation(table)
    angles = rng.uniform(0, 2 * math.pi, table)
    gx_t, gy_t = np.cos(angles), np.sin(angles)

    x0, y0 = np.floor(x).astype(np.int64), np.floor(y).astype(np.int64)
    fx, fy = x - x0, y - y0

    def corner(ix, iy, dx, dy):
        h = perm[(perm[ix % table] + iy) % table]
        return gx_t[h] * dx + gy_t[h] * dy

    def fade(t):
        return t * t * t * (t * (t * 6 - 15) + 10)

    n00 = corner(x0, y0, fx, fy)
    n10 = corner(x0 + 1, y0, fx - 1, fy)
    n01 = corner(x0, y0 + 1, fx, fy - 1)
    n11 = corner(x0 + 1, y0 + 1, fx - 1, fy - 1)
    u, v = fade(fx), fade(fy)
    return ((n00 * (1 - u) + n10 * u) * (1 - v) + (n01 * (1 - u) + n11 * u) * v) * math.sqrt(2)


def base_height(x: np.ndarray, y: np.ndarray, rng: np.random.Generator, relief_m: float) -> np.ndarray:
    """Rolling country with a few bigger hills: smooth fBm from 3 km down to about 375 m wavelength.
    Kept smooth on purpose: the erosion filter adds the fine detail, following these slopes."""
    h = np.zeros_like(x)
    amplitude, total = 1.0, 0.0
    wavelength = 3000.0
    for _ in range(4):
        ox, oy = rng.uniform(-1e4, 1e4, 2)
        h += amplitude * gradient_noise(x / wavelength + ox, y / wavelength + oy, rng)
        total += amplitude
        amplitude *= 0.5
        wavelength *= 0.5
    h /= total
    # Push the upper half up a little so there are a few distinct hills, not just waves.
    h = np.where(h > 0, h * (1 + 0.8 * h), h)
    return h * relief_m


def erode(h: np.ndarray, x: np.ndarray, y: np.ndarray, rng: np.random.Generator,
          octaves: int, wavelength_m: float, strength: float, slope_ref: float,
          gain: float = 0.5) -> np.ndarray:
    """Apply the erosion filter to height field h (metres) sampled at x, y (metres).

    A gully octave's sides are about strength * 2 pi steep whatever its wavelength, so each
    octave's depth is also scaled by gain**i: smaller gullies are gentler, and the slopes
    don't pile up into crumpled paper."""
    lo, hi = np.percentile(h, 2), np.percentile(h, 98)
    # Fade target: -1 (crease) in the lowlands, +1 (ridge) up high.
    target = np.clip(2 * (h - lo) / (hi - lo) - 1, -1, 1)

    depth_scale = 1.0
    for _ in range(octaves):
        gy, gx = np.gradient(h, SPACING_M)
        slope = np.hypot(gx, gy)
        # Stripes run along the downhill direction; u measures across them.
        safe = np.maximum(slope, 1e-9)
        across_x, across_y = -gy / safe, gx / safe

        cell = wavelength_m
        ci, cj = np.floor(x / cell).astype(np.int64), np.floor(y / cell).astype(np.int64)
        i0, j0 = ci.min() - 1, cj.min() - 1
        # Pivots stay near their cell centres (0.25-0.75), and a pivot's reach is 1.25 cells, so
        # every pivot that can reach a point is in the 3x3 cells around it: no seams at cell edges.
        jitter = rng.uniform(0.25, 0.75, (ci.max() - i0 + 2, cj.max() - j0 + 2, 2))

        acc_c = np.zeros_like(h)
        acc_s = np.zeros_like(h)
        acc_w = np.zeros_like(h)
        for di in (-1, 0, 1):
            for dj in (-1, 0, 1):
                ni, nj = ci + di, cj + dj
                piv = jitter[ni - i0, nj - j0]
                px = (ni + piv[..., 0]) * cell
                py = (nj + piv[..., 1]) * cell
                dx, dy = x - px, y - py
                dist = np.hypot(dx, dy) / cell
                w = np.clip(1 - dist / 1.25, 0, None) ** 3
                phase = (dx * across_x + dy * across_y) * (2 * math.pi / wavelength_m)
                acc_c += w * np.cos(phase)
                acc_s += w * np.sin(phase)
                acc_w += w

        c = acc_c / np.maximum(acc_w, 1e-9)
        s = acc_s / np.maximum(acc_w, 1e-9)
        # Blending unaligned stripes shrinks them; restore full strength where the blend is
        # strong enough, leaving weak blends alone to avoid spikes where ridges loop.
        length = np.hypot(c, s)
        threshold = 0.5
        c = c / np.maximum(length, threshold)

        # Steep ground gets the stripes; flat ground fades toward the target.
        # The ease-in (smoothstep) keeps near-flat ground free of stripes: there the downhill
        # direction swings all the way round a point, which would draw a starburst. The
        # 1 - (1 - x)^2 shaping then spreads the erosion evenly over the steeper ground.
        steep = np.clip(slope / slope_ref, 0, 1)
        steep = steep * steep * (3 - 2 * steep)
        mask = 1 - (1 - steep) ** 2
        faded = target + (c - target) * mask

        h = h + faded * strength * wavelength_m * depth_scale
        # Stacked fading: the next octave also avoids this octave's ridges and creases.
        target = target + (faded - target) * mask

        wavelength_m *= 0.5
        depth_scale *= gain
    return h


def write_png(path: Path, pixels: np.ndarray, bit_depth: int, color_type: int) -> None:
    """Minimal PNG writer: 8-bit RGB (color type 2) or 16-bit grayscale (color type 0)."""
    height, width = pixels.shape[:2]
    if bit_depth == 16:
        rows = pixels.astype(">u2").reshape(height, -1).view(np.uint8)
    else:
        rows = pixels.astype(np.uint8).reshape(height, -1)
    raw = np.hstack([np.zeros((height, 1), np.uint8), rows]).tobytes()

    def chunk(tag: bytes, data: bytes) -> bytes:
        return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", zlib.crc32(tag + data))

    ihdr = struct.pack(">IIBBBBB", width, height, bit_depth, color_type, 0, 0, 0)
    path.write_bytes(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr)
                     + chunk(b"IDAT", zlib.compress(raw, 6)) + chunk(b"IEND", b""))


def inside_hex(x: np.ndarray, y: np.ndarray) -> np.ndarray:
    ax, ay = np.abs(x), np.abs(y)
    return (ax <= HALF_FLAT_M) & (0.5 * ax + (math.sqrt(3) / 2) * ay <= HALF_FLAT_M)


def preview(h: np.ndarray, x: np.ndarray, y: np.ndarray, path: Path) -> None:
    """Shaded-relief picture for checking the look; outside the hexagon is dimmed."""
    gy, gx = np.gradient(h, SPACING_M)
    normal = np.dstack([-gx, -gy, np.ones_like(h)])
    normal /= np.linalg.norm(normal, axis=2, keepdims=True)
    light = np.array([-0.5, 0.6, 0.62])
    light /= np.linalg.norm(light)
    shade = np.clip(normal @ light, 0, 1)
    t = (h - h.min()) / (h.max() - h.min())
    low, high = np.array([0.30, 0.42, 0.20]), np.array([0.62, 0.58, 0.52])
    rgb = (low + (high - low) * t[..., None]) * (0.25 + 0.75 * shade[..., None])
    rgb[~inside_hex(x, y)] *= 0.3
    write_png(path, np.clip(rgb * 255, 0, 255)[::-1], 8, 2)  # flip so +Y is up in the picture


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--seed", type=int, default=1)
    parser.add_argument("--out", type=Path, default=Path("Saved/Heightmap"))
    parser.add_argument("--relief", type=float, default=220.0, help="base terrain height range, m")
    parser.add_argument("--octaves", type=int, default=5, help="erosion octaves")
    parser.add_argument("--gully-wavelength", type=float, default=320.0, help="largest gully spacing, m")
    parser.add_argument("--gully-strength", type=float, default=0.05, help="gully depth per wavelength (largest octave)")
    parser.add_argument("--slope-ref", type=float, default=0.35, help="slope at which gullies reach full strength")
    args = parser.parse_args()

    rng = np.random.default_rng(args.seed)
    half = (VERTS - 1) * SPACING_M / 2
    coords = np.linspace(-half, half, VERTS)
    x, y = np.meshgrid(coords, coords)          # rows run along Y, columns along X

    h = base_height(x, y, rng, args.relief)
    h = erode(h, x, y, rng, args.octaves, args.gully_wavelength, args.gully_strength, args.slope_ref)
    h -= h[inside_hex(x, y)].mean()             # sea level of the tile: its average height

    z_range_m = Z_SCALE * 65536 / 128 / 100     # metres covered by 16 bits at this Z scale
    if np.abs(h).max() >= z_range_m / 2:
        raise SystemExit(f"terrain spans {h.min():.1f}..{h.max():.1f} m, beyond +/-{z_range_m / 2:.0f} m; lower --relief")
    values = np.round(32768 + h * 100 * 128 / Z_SCALE).astype(np.uint16)

    args.out.mkdir(parents=True, exist_ok=True)
    heightmap = args.out / f"heightmap-seed{args.seed}.png"
    write_png(heightmap, values, 16, 0)
    preview(h, x, y, args.out / f"preview-seed{args.seed}.png")

    inside = h[inside_hex(x, y)]
    corner_cm = -half * 100
    print(f"heightmap: {heightmap.resolve()}")
    print(f"preview:   {(args.out / f'preview-seed{args.seed}.png').resolve()}")
    print(f"height inside the hexagon: {inside.min():.1f} to {inside.max():.1f} m")
    print()
    print("Landscape import (Landscape mode > Manage > New > Import from File):")
    print(f"  Heightmap File:        {heightmap.resolve()}")
    print(f"  Location:              X {corner_cm:.0f}  Y {corner_cm:.0f}  Z 0")
    print(f"  Scale:                 X {SPACING_M * 100:.0f}  Y {SPACING_M * 100:.0f}  Z {Z_SCALE}")
    print("  Section Size:          63x63 Quads")
    print("  Sections Per Component: 2x2 Sections")
    print(f"  Number of Components:  {(VERTS - 1) // 126} x {(VERTS - 1) // 126}")
    print(f"  Overall Resolution:    {VERTS} x {VERTS}")
    print()
    print(f"Hexagon for the material (cm): half flat-to-flat = {HALF_FLAT_M * 100:.1f}")


if __name__ == "__main__":
    main()

# exp-06: what I want out of this

Written 2026-09-29, before any code. These are my goals as I stated them; the spec and plan come later and have to fit inside these.

## What it is

A new experiment, not mainline dev. A simple render of voxels in 3D space in Bevy, and I want it GPU-rendered.

## The pieces

- **Ray traversal, no meshing.** Cast a ray per pixel and walk it through the hex-prism grid, the way the "billions of voxels" video does for cubes, adapted to hex prisms: per cell, the ray leaves through one of the three faces it's moving toward; y is its own slab.
- **Columns, run-length encoded.** Build the voxel columns the way the "500 million voxels/sec" video does: each column is a list of runs of one material along y, and that's the runtime data, not just the save format.
- **Nearest-to-farthest, both senses.** The walk visits cells strictly front to back, and the data is laid out so cells that are neighbours in space are neighbours in memory (a Z-order-style layout, or the hierarchical hex addresses, which already are one).
- **Skip empty regions.** Contiguous empty space is jumped in one step rather than visited voxel by voxel.

## Constraints

- **Everything in `qrs`.** Cube coordinates throughout, with the same directionality as the main branch (`murabito_hexcoords`: `+q` is east, anticlockwise from there, pointy-top).
- **Free-fly camera,** like the videos. Rays skim along the ground, so horizontal empty-space skipping has to earn its keep.
- **Test world:** a seeded-noise heightfield with caves and overhangs, so columns have several runs and the vertical skip is really exercised.
- **No performance bar.** Getting the traversal and skipping working and visibly correct is the goal; numbers get reported, not passed or failed.

## How I want to work on it

- Think through every question and mock it up in graphs **before** writing any code.
- One chihuahua-sized bite at a time. Don't run ahead into the far future.
- Each mockup is kept in `viz/` as a standalone HTML page for later reference (`viz/hex-ray-walk.html` so far: the exit-face rule and the incremental walk).


## Links

- [This Tiny Algorithm Can Render BILLIONS of Voxels in Real Time
](https://youtu.be/ztkh1r1ioZo)
- [500 million voxels/sec - Voxel engine optimizations](https://youtu.be/ru_oz09Zo-s)
- [Hexagonal grids](https://www.redblobgames.com/grids/hexagons/)

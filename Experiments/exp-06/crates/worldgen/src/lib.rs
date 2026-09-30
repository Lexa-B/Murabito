#![forbid(unsafe_code)]
//! exp-06 worldgen: the Murabito world as pure functions of seed and position, engine-free.
//!
//! Cube coordinates `(q, r, s)` everywhere. The numeric modules are one source compiled twice
//! (Task 2): `worldgen::f64` is the reference the page's readings are checked against, and
//! `worldgen::f32` mirrors the shader. See `Experiments/exp-06/README.md`.

pub mod hex;

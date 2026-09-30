#![forbid(unsafe_code)]
// The numeric modules are one source compiled twice, once per float twin (see the `f64` and `f32`
// modules below); that is the point, not a mistake.
#![allow(clippy::duplicate_mod)]
//! exp-06 worldgen: the Murabito world as pure functions of seed and position, engine-free.
//!
//! Cube coordinates `(q, r, s)` everywhere (`hex`). The numeric modules are one source compiled twice:
//! `worldgen::f64` is the reference the page's readings are checked against, and `worldgen::f32`
//! mirrors the shader, same source, same order of operations. Each twin fixes `F`, the constants
//! and `rad`, and includes the numeric files by `#[path]`. See `Experiments/exp-06/README.md`.

pub mod hex;

/// The reference path: every reading in `docs/reference-readings.md` is checked against this twin.
pub mod f64 {
    pub type F = core::primitive::f64;
    pub const PI: F = core::f64::consts::PI;
    pub const SQRT_3: F = 1.7320508075688772;
    /// Relative tolerance the twin's own tests allow against the page's numbers.
    pub const TOL: F = 1e-9;
    /// Degrees to radians the way the page does it, `d · π / 180`, so the last bit agrees.
    pub fn rad(deg: F) -> F {
        deg * PI / 180.0
    }
    #[path = "../banana.rs"]
    pub mod banana;
    #[path = "../plane.rs"]
    pub mod plane;
}

/// The shader's twin: the same source in f32, in the order of operations the WGSL will use.
pub mod f32 {
    pub type F = core::primitive::f32;
    pub const PI: F = core::f32::consts::PI;
    #[allow(clippy::excessive_precision)]
    pub const SQRT_3: F = 1.7320508075688772;
    pub const TOL: F = 1e-3;
    pub fn rad(deg: F) -> F {
        deg * PI / 180.0
    }
    #[path = "../banana.rs"]
    pub mod banana;
    #[path = "../plane.rs"]
    pub mod plane;
}

//! Murabito, as a library.
//!
//! # The base unit is the shaku
//!
//! One world unit is one 尺 — 10/33 m, about 30.3 cm — and every length in this crate is
//! in shaku unless it says otherwise. Nothing converts to metres; metres appear only in
//! comments, to say how big something is in terms a reader may find familiar.
//!
//! The units above it are the traditional ones, and round numbers tend to land on them:
//! 1 間 = 6 shaku (~1.8 m), 1 町 = 60 ken = 360 shaku (~109 m), 1 里 = 36 cho (~3.9 km).
//! The sense grid is one shaku flat-to-flat, so a sense range in shaku is also a count of
//! cells — which is why those ranges are integers.
//!
//! Every module lives here rather than in `main.rs` so that tests can reach them:
//! Rust's integration tests (the `tests/` directory) can depend on a library target,
//! but never on a binary. `main.rs` is the binary that builds the app out of these
//! plugins, and is the only thing that opens a window.

// ECS query types are tuples of tuples by nature, and naming each one costs more than it
// explains. Bevy's own examples allow this lint for the same reason.
#![allow(clippy::type_complexity)]

pub mod being;
pub mod camera;
pub mod hex;
pub mod i18n;
pub mod menu;
pub mod scene;
pub mod screenshot;
pub mod senses;
pub mod settings;
pub mod settings_page;
pub mod state;
pub mod ui;
pub mod user_data;
// rustc will not infer a filename from a non-ASCII module name (E0754), so it is
// spelled out. The file really is `src/諸法.rs`.
#[path = "諸法.rs"]
pub mod 諸法;

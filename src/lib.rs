//! Murabito, as a library.
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

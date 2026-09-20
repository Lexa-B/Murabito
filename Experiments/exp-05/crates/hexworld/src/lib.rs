//! Murabito's hex world: ri / cho / ken / shaku addresses with a vertical layer,
//! voxel columns, and chunks that load at nested scales.
//!
//! This crate is engine-free on purpose: no rendering, no ECS, no dependencies.
//! Positions are `(east, north, height)` in metres; mapping to an engine's axes is
//! the caller's job.

pub mod hex;
pub mod level;
pub mod owner;

pub use hex::{d2, distance, neighbours, range, Hex, DIRECTIONS};
pub use level::{Level, CELL_LEVELS, SHAKU_M, SUN_M};
pub use owner::{centre_child, centre_shaku, children, local, owned_offsets, owner, parent_of, up};

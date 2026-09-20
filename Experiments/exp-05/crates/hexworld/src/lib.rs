//! Murabito's hex world: ri / cho / ken / shaku addresses with a vertical layer,
//! voxel columns, and chunks that load at nested scales.
//!
//! This crate is engine-free on purpose: no rendering, no ECS, no dependencies.
//! Positions are `(east, north, height)` in metres; mapping to an engine's axes is
//! the caller's job.

pub mod address;
pub mod column;
pub mod config;
pub mod hex;
pub mod level;
pub mod noise;
pub mod owner;
pub mod placeholder_terrain;
pub mod plane;
pub mod world;

pub use address::{address_of, shaku_of, Address};
pub use column::{Column, Material, Run};
pub use config::WorldConfig;
pub use hex::{d2, distance, neighbours, range, Hex, DIRECTIONS};
pub use level::{Level, CELL_LEVELS, SHAKU_M, SUN_M};
pub use owner::{centre_child, centre_shaku, children, local, owned_offsets, owner, parent_of, up};
pub use placeholder_terrain::{column_at, height_m};
pub use plane::{cell_centre_m, corners_m, hex_round, metres_to_axial, round_at};
pub use world::{cell_in_world, in_world, world_ri};

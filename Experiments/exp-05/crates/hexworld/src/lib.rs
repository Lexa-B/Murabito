//! Murabito's hex world: ri / cho / ken / shaku addresses with a vertical layer,
//! voxel columns, and chunks that load at nested scales.
//!
//! This crate holds no rendering, ECS or asset code. Its only dependencies are
//! `murabito_hexcoords`, the main project's coordinate crate (vendored), and Bevy's math
//! types. Positions are Bevy's `Vec3` in shaku, with east +X, north −Z and up +Y, as in
//! `murabito_hexcoords`.

pub mod address;
pub mod chunk;
pub mod column;
pub mod config;
pub mod hex;
pub mod layer;
pub mod level;
pub mod mesh;
pub mod noise;
pub mod owner;
pub mod placeholder_terrain;
pub mod plane;
pub mod store;
pub mod world;

pub use address::{address_of, voxel_of, Address};
pub use chunk::{generate, Chunk, ChunkKey};
pub use column::{Column, Material, Run};
pub use config::WorldConfig;
pub use hex::{d2, distance, neighbours, range, step, Hex, DIRECTIONS, EDGES};
pub use layer::{layer_bottom, layer_of_height, layer_top, LAYER_HEIGHT};
pub use level::{Level, CELL_LEVELS, SHAKU_M};
pub use mesh::{drawn_cells, mesh_chunk, MeshData};
pub use owner::{
    centre_child, centre_shaku, children, drawn_offsets, local, owned_offsets, owner, parent_of, up,
};
pub use placeholder_terrain::{column_at, height};
pub use plane::{cell_centre, corners, round_at};
pub use store::{
    coarseness_rank, load_order, requests, ChunkStore, LevelStats, Loader, Rings, Stats,
    StoreSettings, StoreUpdate,
};
pub use world::{cell_in_world, in_world, world_ri};

pub use murabito_hexcoords::{Direction, NotOnHexPlane, VoxelCoord, VoxelspacePos};

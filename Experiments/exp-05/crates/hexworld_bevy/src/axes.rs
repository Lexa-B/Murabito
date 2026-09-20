//! The only place where the core's (east, north, height) meets Bevy's axes.
//!
//! Bevy is right-handed with +Y up, and its forward is -Z. Mapping north to +Z instead
//! would flip the ground plane's handedness and mirror the whole world, so: north is -Z.

use bevy::prelude::Vec3;

pub fn to_bevy(east: f64, north: f64, height: f64) -> Vec3 {
    Vec3::new(east as f32, height as f32, -north as f32)
}

pub fn from_bevy(v: Vec3) -> (f64, f64, f64) {
    (v.x as f64, -v.z as f64, v.y as f64)
}

/// The same mapping for a mesh vertex, which the core gives as (east, north, height).
pub fn mesh_position(p: [f32; 3]) -> [f32; 3] {
    [p[0], p[2], -p[1]]
}

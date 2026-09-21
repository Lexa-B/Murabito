//! The ground material: Bevy's standard material plus hex lines and a detail-level tint.
//!
//! No hex maths happens in the shader — the mesh already carries what it needs in its UVs
//! (see `entities::to_bevy_mesh`), so this is just the material plumbing and the two
//! resources (`LineMode`, `TintByLevel`) a viewer can change at runtime.

use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;

pub const SHADER_PATH: &str = "shaders/ground.wgsl";

#[derive(Asset, AsBindGroup, Reflect, Clone, Default, Debug)]
pub struct GroundExtension {
    /// 0 off, 1 shaku only, 2 nested.
    #[uniform(100)]
    pub mode: u32,
    /// 1 tints the ground by the detail level drawn there.
    #[uniform(101)]
    pub tint: u32,
}

impl MaterialExtension for GroundExtension {
    fn fragment_shader() -> ShaderRef {
        SHADER_PATH.into()
    }
}

pub type GroundMaterial = ExtendedMaterial<StandardMaterial, GroundExtension>;

#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum LineMode {
    Off,
    #[default]
    ShakuOnly,
    Nested,
}

impl LineMode {
    pub(crate) fn as_u32(self) -> u32 {
        match self {
            LineMode::Off => 0,
            LineMode::ShakuOnly => 1,
            LineMode::Nested => 2,
        }
    }
}

#[derive(Resource, Clone, Copy, Default)]
pub struct TintByLevel(pub bool);

/// Write `LineMode` and `TintByLevel` into the ground material asset whenever either
/// changes, so a viewer can flip them at runtime without re-spawning anything.
pub fn sync_ground_material(
    line_mode: Res<LineMode>,
    tint_by_level: Res<TintByLevel>,
    handle: Option<Res<crate::GroundMaterialHandle>>,
    mut materials: ResMut<Assets<GroundMaterial>>,
) {
    if !line_mode.is_changed() && !tint_by_level.is_changed() {
        return;
    }
    let Some(handle) = handle else { return };
    let Some(mut material) = materials.get_mut(&handle.0) else {
        return;
    };
    material.extension.mode = line_mode.as_u32();
    material.extension.tint = u32::from(tint_by_level.0);
}

//! Opens a window, loads one loader's worth of chunks, and exits on its own after a
//! handful of frames. This is the only place a WGSL compile error in `ground.wgsl` would
//! actually surface, since Bevy only compiles shaders when something asks to render them.
//!
//! Run from the workspace root, so Bevy's asset folder (`assets/`) resolves correctly:
//! `cargo run -p hexworld_bevy --example shader_check`

use bevy::asset::AssetPlugin;
use bevy::prelude::*;
use hexworld_bevy::{HexWorldPlugin, Loader};

const EXIT_AFTER_FRAMES: u32 = 120;

fn main() {
    App::new()
        // Bevy's asset root is `BEVY_ASSET_ROOT` when set, falling back to
        // `CARGO_MANIFEST_DIR` (set by `cargo run`) otherwise, not the process's cwd.
        // `Experiments/exp-05/.cargo/config.toml` sets `BEVY_ASSET_ROOT` to the workspace
        // root, so `file_path` here is just `assets`, where the shader lives (see
        // `assets/shaders/ground.wgsl` at the workspace root).
        .add_plugins(DefaultPlugins.set(AssetPlugin {
            file_path: "assets".to_string(),
            ..default()
        }))
        .add_plugins(HexWorldPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(Update, exit_after_a_few_frames)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn((Loader::default(), Transform::default()));

    commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: 45.0_f32.to_radians(),
            ..default()
        }),
        Transform::from_xyz(0.0, 60.0, 0.01).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        DirectionalLight {
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_xyz(10.0, 30.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn exit_after_a_few_frames(mut frames: Local<u32>, mut exit: MessageWriter<AppExit>) {
    *frames += 1;
    if *frames >= EXIT_AFTER_FRAMES {
        exit.write(AppExit::Success);
    }
}

//! Murabito hexworld viewer: an overhead camera flying over the procedurally generated
//! hex world.
//!
//! Bevy's asset root, under `cargo run`, resolves against `CARGO_MANIFEST_DIR` (this
//! crate's own directory), not the process's working directory. This crate sits two
//! levels below the workspace root (`crates/viewer`), and the shared ground shader lives
//! in the workspace-root `assets/` directory (see `assets/shaders/ground.wgsl`), so the
//! asset path is overridden the same way `hexworld_bevy`'s `shader_check` example does it.

mod camera;

use bevy::asset::AssetPlugin;
use bevy::prelude::*;
use bevy::window::{WindowPlugin, WindowResolution};
use hexworld_bevy::{HexWorldPlugin, Loader};

use camera::CameraRig;

/// Runs forever unless `--frames N` is given on the command line, in which case the app
/// exits on its own after N frames. Parsed by hand (no `clap`): every later verification
/// of this crate has to be non-interactive, and a windowed app with no exit condition
/// cannot be run from an agent's shell.
fn frames_from_args() -> Option<u32> {
    let args: Vec<String> = std::env::args().collect();
    for (i, arg) in args.iter().enumerate() {
        if let Some(value) = arg.strip_prefix("--frames=") {
            return value.parse().ok();
        }
        if arg == "--frames" {
            return args.get(i + 1).and_then(|v| v.parse().ok());
        }
    }
    None
}

fn main() {
    let frame_limit = frames_from_args();

    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(AssetPlugin {
                file_path: "../../assets".to_string(),
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Murabito exp-05 — hex world".into(),
                    resolution: WindowResolution::new(1600, 900),
                    ..default()
                }),
                ..default()
            }),
    )
    .add_plugins(HexWorldPlugin::default())
    .insert_resource(ClearColor(Color::srgb(0.53, 0.81, 0.92)))
    .add_systems(Startup, setup)
    .add_systems(
        Update,
        (camera::handle_input, camera::follow_ground).chain(),
    );

    if let Some(frames) = frame_limit {
        app.insert_resource(FrameLimit(frames))
            .add_systems(Update, exit_after_frame_limit);
    }

    app.run();
}

/// How many frames to run before exiting on its own. Only present with `--frames N`.
#[derive(Resource)]
struct FrameLimit(u32);

fn exit_after_frame_limit(
    limit: Res<FrameLimit>,
    mut frames: Local<u32>,
    mut exit: MessageWriter<AppExit>,
) {
    *frames += 1;
    if *frames >= limit.0 {
        exit.write(AppExit::Success);
    }
}

fn setup(mut commands: Commands) {
    let rig = CameraRig {
        focus_east: 0.0,
        focus_north: 0.0,
        zoom: 120.0,
    };
    let (focus_pos, camera_pos) = camera::rig_transforms(&rig);

    commands
        .spawn((
            rig,
            Loader::default(),
            Transform::from_translation(focus_pos),
        ))
        .with_children(|parent| {
            parent.spawn((
                Camera3d::default(),
                Projection::Perspective(PerspectiveProjection {
                    fov: 45.0_f32.to_radians(),
                    ..default()
                }),
                AmbientLight {
                    brightness: 240.0,
                    ..default()
                },
                Transform::from_translation(camera_pos - focus_pos).looking_at(Vec3::ZERO, Vec3::Y),
            ));
        });

    commands.spawn((
        DirectionalLight {
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(300.0, 600.0, 200.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

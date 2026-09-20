//! Murabito hexworld viewer: an overhead camera flying over the procedurally generated
//! hex world.
//!
//! Bevy's asset root, under `cargo run`, resolves against `CARGO_MANIFEST_DIR` (this
//! crate's own directory), not the process's working directory. This crate sits two
//! levels below the workspace root (`crates/viewer`), and the shared ground shader lives
//! in the workspace-root `assets/` directory (see `assets/shaders/ground.wgsl`), so the
//! asset path is overridden the same way `hexworld_bevy`'s `shader_check` example does it.

mod camera;

use std::time::{Duration, Instant};

use bevy::asset::AssetPlugin;
use bevy::prelude::*;
use bevy::window::{WindowPlugin, WindowResolution};
use hexworld::WorldConfig;
use hexworld_bevy::{HexWorld, HexWorldPlugin, Loader};

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

/// `--auto-pan`: drives the camera rig's focus in a steady sweep instead of waiting for
/// real input, so a non-interactive `--frames N` run still exercises sustained panning —
/// the scenario that produced the parent-remesh frame spikes this binary is used to
/// measure. Gated behind the flag and off by default; see `docs/plans/...task-12b`.
fn auto_pan_from_args() -> bool {
    std::env::args().any(|arg| arg == "--auto-pan")
}

fn main() {
    let frame_limit = frames_from_args();
    let auto_pan = auto_pan_from_args();

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

    if auto_pan {
        app.add_systems(Update, auto_pan_system.before(camera::follow_ground));
    }

    if let Some(frames) = frame_limit {
        // Only wired up for a `--frames N` run: `FrameTimes` collects one sample every
        // frame for as long as the app runs, so registering it unconditionally would grow
        // an unbounded `Vec<Duration>` for the length of an ordinary interactive session
        // that nothing ever reads.
        app.init_resource::<FrameTimes>()
            // Records the wall-clock gap between consecutive frame starts, in `First` so
            // it brackets the *previous* frame's full cost (every system, main-thread
            // remeshing included) rather than just this frame's own work.
            .add_systems(First, record_frame_time)
            .insert_resource(FrameLimit(frames))
            .add_systems(Update, exit_after_frame_limit);
    }

    app.run();
}

/// How many frames to run before exiting on its own. Only present with `--frames N`.
#[derive(Resource)]
struct FrameLimit(u32);

/// Wall-clock time between consecutive frame starts, collected whenever `--frames` runs
/// the app non-interactively, so a `--frames N` run always ends with a frame-time report.
#[derive(Resource, Default)]
struct FrameTimes(Vec<Duration>);

fn record_frame_time(mut times: ResMut<FrameTimes>, mut last: Local<Option<Instant>>) {
    let now = Instant::now();
    if let Some(prev) = *last {
        times.0.push(now.duration_since(prev));
    }
    *last = Some(now);
}

/// Sweeps the focus east at a steady pace (scaled by zoom, like real panning), crossing
/// cho/ken chunk boundaries again and again for the length of the run. Real elapsed time
/// drives the distance (not a frame count), so a run stays "sustained panning" even while
/// individual frames stall.
fn auto_pan_system(time: Res<Time>, world: Res<HexWorld>, mut rigs: Query<&mut CameraRig>) {
    let cfg: WorldConfig = *world.store().config();
    // Clamped so a slow startup frame (asset/shader load) doesn't register as one huge
    // simulated jump; it only affects how far the synthetic pan moves, never the recorded
    // frame time itself.
    let dt = time.delta_secs().min(0.1);
    for mut rig in &mut rigs {
        let speed = camera::pan_speed_for_zoom(rig.zoom);
        rig.pan((speed * dt) as f64, 0.0, &cfg);
    }
}

fn exit_after_frame_limit(
    limit: Res<FrameLimit>,
    times: Res<FrameTimes>,
    mut frames: Local<u32>,
    mut exit: MessageWriter<AppExit>,
) {
    *frames += 1;
    if *frames >= limit.0 {
        report_frame_times(&times.0);
        exit.write(AppExit::Success);
    }
}

/// Prints worst frame, a 95th-percentile, and counts over common frame-budget thresholds —
/// enough to compare a before/after run without pulling in a stats crate.
fn report_frame_times(samples: &[Duration]) {
    if samples.is_empty() {
        println!("frame-times: no samples recorded");
        return;
    }
    let mut sorted: Vec<Duration> = samples.to_vec();
    sorted.sort();
    let worst = *sorted.last().unwrap();
    let p95_index = ((sorted.len() as f64) * 0.95) as usize;
    let p95 = sorted[p95_index.min(sorted.len() - 1)];
    let mean: Duration = sorted.iter().sum::<Duration>() / sorted.len() as u32;
    let over_33ms = sorted.iter().filter(|d| d.as_secs_f64() > 0.033).count();
    let over_16ms = sorted.iter().filter(|d| d.as_secs_f64() > 0.0167).count();
    println!(
        "frame-times: n={} mean={:.2}ms p95={:.2}ms worst={:.2}ms over_16.7ms={} over_33ms={}",
        sorted.len(),
        mean.as_secs_f64() * 1000.0,
        p95.as_secs_f64() * 1000.0,
        worst.as_secs_f64() * 1000.0,
        over_16ms,
        over_33ms,
    );
}

fn setup(mut commands: Commands) {
    let rig = CameraRig {
        focus_east: 0.0,
        focus_north: 0.0,
        zoom: 120.0,
        last_height: 0.0,
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

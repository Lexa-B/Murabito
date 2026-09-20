//! Murabito hexworld viewer: an overhead camera flying over the procedurally generated
//! hex world.
//!
//! Bevy's asset root, under `cargo run`, resolves against `CARGO_MANIFEST_DIR` (this
//! crate's own directory), not the process's working directory. This crate sits two
//! levels below the workspace root (`crates/viewer`), and the shared ground shader lives
//! in the workspace-root `assets/` directory (see `assets/shaders/ground.wgsl`), so the
//! asset path is overridden the same way `hexworld_bevy`'s `shader_check` example does it.

mod camera;
mod headless;
mod panel;

use std::time::{Duration, Instant};

use bevy::asset::AssetPlugin;
use bevy::diagnostic::{FrameCount, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::window::{WindowPlugin, WindowResolution};
use hexworld::WorldConfig;
use hexworld_bevy::{HexWorld, HexWorldPlugin, LineMode, Loader, TintByLevel};

use camera::CameraRig;
use headless::Headless;

fn main() {
    let headless = Headless::from_args();

    let mut world_config = WorldConfig::default();
    if let Some(seed) = headless.seed {
        world_config.seed = seed;
    }

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
    .add_plugins(HexWorldPlugin {
        config: world_config,
        ..default()
    })
    .add_plugins(FrameTimeDiagnosticsPlugin::default())
    .add_plugins(bevy_egui::EguiPlugin::default())
    .init_resource::<panel::HoveredCell>()
    .init_resource::<panel::ExtraLoaders>()
    .init_resource::<panel::TriangleCounts>()
    .insert_resource(ClearColor(Color::srgb(0.53, 0.81, 0.92)))
    .add_systems(Startup, setup)
    .add_systems(
        Update,
        (
            camera::handle_input,
            camera::follow_ground,
            panel::update_hover,
        )
            .chain(),
    )
    // After `HexWorldSet` (where chunk entities get their `Mesh3d`), before the render
    // app's extraction step strips the mesh's CPU-side data — see `TriangleCounts`.
    .add_systems(
        Update,
        panel::track_triangle_counts.after(hexworld_bevy::HexWorldSet),
    )
    .add_systems(bevy_egui::EguiPrimaryContextPass, panel::draw)
    // Both are no-ops without their flag (`--screenshot-dir`, `--frames`): `shoot` returns
    // immediately with no `screenshot_dir`, and `quit_after` never writes `AppExit`
    // without a `frames` limit, so registering them unconditionally costs nothing in an
    // ordinary interactive session.
    .add_systems(Update, (headless::shoot, headless::quit_after));

    if headless.auto_pan {
        app.add_systems(Update, auto_pan_system.before(camera::follow_ground));
    }

    if headless.frames.is_some() {
        // Only wired up for a `--frames N` run: `FrameTimes` collects one sample every
        // frame for as long as the app runs, so registering it unconditionally would grow
        // an unbounded `Vec<Duration>` for the length of an ordinary interactive session
        // that nothing ever reads.
        app.init_resource::<FrameTimes>()
            // Records the wall-clock gap between consecutive frame starts, in `First` so
            // it brackets the *previous* frame's full cost (every system, main-thread
            // remeshing included) rather than just this frame's own work.
            .add_systems(First, record_frame_time)
            .add_systems(Update, report_frame_times_once_done);
    }

    // `--line-mode`/`--tint`: override the resources `HexWorldPlugin` just initialised to
    // their defaults, above. Neither has a keyboard shortcut (only the egui panel's mouse
    // controls reach them), so a headless run has no other way to start in these modes.
    if let Some(mode) = headless.line_mode.as_deref() {
        let mode = match mode {
            "off" => LineMode::Off,
            "nested" => LineMode::Nested,
            _ => LineMode::ShakuOnly,
        };
        app.insert_resource(mode);
    }
    if headless.tint {
        app.insert_resource(TintByLevel(true));
    }

    app.insert_resource(headless);

    app.run();
}

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

/// Prints the frame-time report exactly once, the first frame `FrameCount` reaches the
/// `--frames` limit. `headless::quit_after` is the system that actually exits — this one
/// only reports, guarded by `reported` so it cannot print twice if `AppExit` takes an
/// extra frame to stop the app.
fn report_frame_times_once_done(
    frame_count: Res<FrameCount>,
    headless: Res<Headless>,
    times: Res<FrameTimes>,
    mut reported: Local<bool>,
) {
    let Some(limit) = headless.frames else {
        return;
    };
    if *reported || frame_count.0 < limit {
        return;
    }
    *reported = true;
    report_frame_times(&times.0);
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

fn setup(mut commands: Commands, headless: Res<Headless>) {
    let (focus_east, focus_north) = headless.start.unwrap_or((0.0, 0.0));
    let zoom = headless
        .zoom
        .unwrap_or(120.0)
        .clamp(camera::MIN_ZOOM, camera::MAX_ZOOM);
    let rig = CameraRig {
        focus_east,
        focus_north,
        zoom,
        last_height: 0.0,
    };
    let (focus_pos, camera_pos) = camera::rig_transforms(&rig);

    let mut loader = Loader::default();
    if let Some(rings_shaku) = headless.rings_shaku {
        loader.rings.shaku = rings_shaku;
    }

    commands
        .spawn((rig, loader, Transform::from_translation(focus_pos)))
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

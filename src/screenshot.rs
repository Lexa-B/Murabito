//! Screenshots: F12 while playing, or a one-shot capture from the command line.
//!
//! The command-line form exists so that anything on screen can be checked without a
//! person at the keyboard — which is how UI work gets verified here. It drives the app
//! to a screen, waits for it to settle, saves a PNG and exits:
//!
//! ```text
//! cargo run -- --shot shots/settings.png --screen settings
//! ```
//!
//! The wait matters. Render pipelines are compiled on first use, so the earliest frames
//! can be an empty clear colour with no meshes and no UI in them.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use bevy::app::AppExit;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};

use crate::camera::CameraRig;

use crate::state::AppState;

/// Where F12 puts its captures.
const DIRECTORY: &str = "screenshots";

/// Frames to wait before capturing, unless `--settle` says otherwise. About 2.5 seconds
/// at 60fps: enough for pipelines, the font, and any state transition to be done.
const DEFAULT_SETTLE: u32 = 150;

/// Frames between the capture request and quitting. The pixels come back from the render
/// world a frame or two later, and the file is written from an observer after that.
const FRAMES_AFTER_CAPTURE: u32 = 60;

pub struct ScreenshotPlugin;

impl Plugin for ScreenshotPlugin {
    fn build(&self, app: &mut App) {
        // Not gated on `AppState`: the point is to be able to capture the menus too.
        app.add_systems(Update, capture_on_key);

        if let Some(request) = ShotRequest::from_args() {
            app.insert_resource(request).add_systems(
                Update,
                run_one_shot.before(crate::debug_screen::toggle_screen),
            );
        }
    }
}

fn capture_on_key(keys: Res<ButtonInput<KeyCode>>, mut commands: Commands) {
    if !keys.just_pressed(KeyCode::F12) {
        return;
    }

    // Seconds since the epoch keeps successive shots in order and never overwrites one.
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or_default();
    capture(
        &mut commands,
        PathBuf::from(format!("{DIRECTORY}/{stamp}.png")),
    );
}

/// A capture asked for on the command line.
#[derive(Resource)]
struct ShotRequest {
    path: PathBuf,
    screen: AppState,
    settle: u32,
    /// Shaku back from the focus. `None` leaves the camera wherever it starts.
    zoom: Option<f32>,
    /// Whether to open the F3 screen before capturing.
    debug: bool,
}

impl ShotRequest {
    /// `--shot <path>` turns this on. `--screen menu|settings|playing` picks what to
    /// capture (default `playing`), `--settle <frames>` how long to wait first, and
    /// `--zoom <shaku>` how far back the camera sits — which is how anything drawn in
    /// the world gets framed, rather than running off the edge at the starting zoom — and
    /// `--debug` opens the F3 screen first.
    ///
    /// Parsed by hand rather than with `clap`: four flags, used by whoever is verifying
    /// a change, and not worth a dependency.
    fn from_args() -> Option<Self> {
        let args: Vec<String> = std::env::args().collect();
        let value = |flag: &str| {
            args.iter()
                .position(|arg| arg == flag)
                .and_then(|i| args.get(i + 1))
                .cloned()
        };

        let path = PathBuf::from(value("--shot")?);
        let screen = match value("--screen").as_deref() {
            None | Some("playing") => AppState::Playing,
            Some("menu") => AppState::Menu,
            Some("settings") => AppState::Settings,
            Some(other) => {
                warn!("unknown --screen {other:?}; capturing the game instead");
                AppState::Playing
            }
        };
        let settle = value("--settle")
            .and_then(|frames| frames.parse().ok())
            .unwrap_or(DEFAULT_SETTLE);

        let zoom = value("--zoom").and_then(|shaku| shaku.parse().ok());
        let debug = args.iter().any(|arg| arg == "--debug");

        Some(Self {
            path,
            screen,
            settle,
            zoom,
            debug,
        })
    }
}

/// Drives the app to the requested screen, captures it, then exits.
fn run_one_shot(
    request: Res<ShotRequest>,
    mut frames: Local<u32>,
    mut commands: Commands,
    mut next: ResMut<NextState<AppState>>,
    mut rigs: Query<&mut CameraRig>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut exit: MessageWriter<AppExit>,
) {
    *frames += 1;

    // Before the state change below, so the camera systems — which only run while
    // playing — still get a frame to turn the new zoom into a transform.
    if *frames == 1
        && let Some(zoom) = request.zoom
    {
        for mut rig in &mut rigs {
            rig.jump_to_zoom(zoom);
        }
    }

    // Early, but not on the first frame: a screen's `OnEnter` needs the resources that
    // `PreStartup` and `Startup` put in place, the UI font among them.
    if *frames == 2 {
        next.set(request.screen);
    }
    // Pressed rather than spawned directly, so what is captured is the screen the real
    // key opens. Ordered ahead of the system that reads it, because `just_pressed` lasts
    // exactly one frame.
    if *frames == 3 && request.debug {
        keys.press(crate::debug_screen::TOGGLE_KEY);
    }
    if *frames == request.settle {
        if let Some(parent) = request.path.parent()
            && !parent.as_os_str().is_empty()
            && let Err(error) = std::fs::create_dir_all(parent)
        {
            warn!("could not create {} ({error})", parent.display());
            exit.write(AppExit::Success);
            return;
        }
        info!(
            "capturing {} -> {}",
            request.screen_name(),
            request.path.display()
        );
        capture(&mut commands, request.path.clone());
    }
    if *frames > request.settle + FRAMES_AFTER_CAPTURE {
        exit.write(AppExit::Success);
    }
}

impl ShotRequest {
    fn screen_name(&self) -> &'static str {
        match self.screen {
            AppState::Playing => "playing",
            AppState::Menu => "menu",
            AppState::Settings => "settings",
        }
    }
}

/// Queues a capture of the window. The pixels arrive from the render world a frame or
/// two later, and `save_to_disk` writes the file from an observer when they do.
fn capture(commands: &mut Commands, path: PathBuf) {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
        && let Err(error) = std::fs::create_dir_all(parent)
    {
        warn!(
            "could not create {} ({error}); no screenshot taken",
            parent.display()
        );
        return;
    }
    let shown = path.display().to_string();
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(path));
    info!("screenshot -> {shown}");
}

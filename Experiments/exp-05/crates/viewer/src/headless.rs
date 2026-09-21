//! Non-interactive CLI flags, parsed by hand from `std::env::args()` (no `clap`, to keep
//! the dependency list short) — for an agent's shell taking screenshots or measuring frame
//! times, never for a person to type. The window still opens; nothing here removes it.
//!
//! Every flag is optional and a missing or malformed value falls back to a default rather
//! than panicking: a headless run that mistypes a flag should behave as if the flag were
//! absent, not crash before the window opens.

use std::path::PathBuf;

use bevy::app::AppExit;
use bevy::diagnostic::FrameCount;
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};

/// Parsed once at startup from `std::env::args()` and inserted as a resource.
#[derive(Resource, Clone, Debug, Default)]
pub struct Headless {
    /// `--frames N`: exit on our own after N frames. `None` (the default) runs forever.
    pub frames: Option<u32>,
    /// `--screenshot-dir DIR`: save a PNG under this directory every `screenshot_every`
    /// frames. `None` (the default) takes no screenshots.
    pub screenshot_dir: Option<PathBuf>,
    /// `--screenshot-every M`: how often to save, in frames. Never 0 — that would be a
    /// modulo by zero — so a missing or non-positive value falls back to 30.
    pub screenshot_every: u32,
    /// `--seed N`: overrides `WorldConfig`'s default seed.
    pub seed: Option<u64>,
    /// `--start-east M --start-north M`: the focus's starting position, in metres. Only
    /// takes effect when both coordinates parse; one without the other is `None` (no
    /// partial start).
    pub start: Option<(f64, f64)>,
    /// `--rings-shaku N`: overrides the focus loader's shaku ring count.
    pub rings_shaku: Option<i32>,
    /// `--auto-pan`: sweep the focus east at a steady pace instead of waiting for input,
    /// so a `--frames N` run still exercises sustained panning.
    pub auto_pan: bool,
    /// `--zoom N`: overrides the camera rig's starting distance from the focus, in
    /// metres. `None` keeps the interactive default. Not part of the brief's listed
    /// flags — added because the spec's close-up "shaku borders" screenshot is
    /// otherwise unreachable headlessly (no keyboard input drives the wheel-zoom).
    pub zoom: Option<f32>,
    /// `--line-mode off|shaku|nested`: overrides `LineMode`'s starting value. `None`
    /// keeps the interactive default (`ShakuOnly`). Not part of the brief's listed
    /// flags — added because there is no keyboard shortcut to cycle line modes (the
    /// only control is a mouse click on the egui panel's radio buttons), so a headless
    /// run has no other way to reach the spec's nested-line screenshot.
    pub line_mode: Option<String>,
    /// `--tint`: overrides `TintByLevel`'s starting value to on. Off by default, for the
    /// same reason as `line_mode`: the only control is a mouse click on the panel's
    /// checkbox.
    pub tint: bool,
}

impl Headless {
    pub fn from_args() -> Self {
        Self::from_args_iter(std::env::args())
    }

    fn from_args_iter(args: impl Iterator<Item = String>) -> Self {
        let args: Vec<String> = args.collect();
        let start_east = flag(&args, "--start-east").and_then(|v| v.parse::<f64>().ok());
        let start_north = flag(&args, "--start-north").and_then(|v| v.parse::<f64>().ok());
        Headless {
            frames: flag(&args, "--frames").and_then(|v| v.parse().ok()),
            screenshot_dir: flag(&args, "--screenshot-dir").map(PathBuf::from),
            screenshot_every: flag(&args, "--screenshot-every")
                .and_then(|v| v.parse::<u32>().ok())
                .filter(|v| *v > 0)
                .unwrap_or(30),
            seed: flag(&args, "--seed").and_then(|v| v.parse().ok()),
            start: start_east.zip(start_north),
            rings_shaku: flag(&args, "--rings-shaku").and_then(|v| v.parse().ok()),
            auto_pan: args.iter().any(|a| a == "--auto-pan"),
            zoom: flag(&args, "--zoom").and_then(|v| v.parse::<f32>().ok()),
            line_mode: flag(&args, "--line-mode"),
            tint: args.iter().any(|a| a == "--tint"),
        }
    }
}

/// The value following `--name value`, or the part after `=` in `--name=value`. The first
/// occurrence wins if a flag repeats.
fn flag(args: &[String], name: &str) -> Option<String> {
    let eq_prefix = format!("{name}=");
    for (i, arg) in args.iter().enumerate() {
        if let Some(v) = arg.strip_prefix(&eq_prefix) {
            return Some(v.to_string());
        }
        if arg == name {
            return args.get(i + 1).cloned();
        }
    }
    None
}

/// Saves a PNG under `--screenshot-dir` every `--screenshot-every` frames. Skips frame 0
/// (nothing has presented yet). A no-op when `--screenshot-dir` was not given.
pub fn shoot(mut commands: Commands, frames: Res<FrameCount>, options: Res<Headless>) {
    let Some(dir) = options.screenshot_dir.as_ref() else {
        return;
    };
    if frames.0 > 0 && frames.0.is_multiple_of(options.screenshot_every) {
        let path = dir.join(format!("frame-{:05}.png", frames.0));
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
    }
}

/// Exits once `--frames N` frames have run. A no-op (never exits) without that flag.
pub fn quit_after(
    frames: Res<FrameCount>,
    options: Res<Headless>,
    mut exit: MessageWriter<AppExit>,
) {
    if let Some(limit) = options.frames {
        if frames.0 >= limit {
            exit.write(AppExit::Success);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Headless {
        Headless::from_args_iter(args.iter().map(|s| s.to_string()))
    }

    #[test]
    fn defaults_are_all_absent() {
        let h = parse(&[]);
        assert_eq!(h.frames, None);
        assert!(h.screenshot_dir.is_none());
        assert_eq!(h.screenshot_every, 30);
        assert_eq!(h.seed, None);
        assert_eq!(h.start, None);
        assert_eq!(h.rings_shaku, None);
        assert!(!h.auto_pan);
        assert_eq!(h.zoom, None);
        assert_eq!(h.line_mode, None);
        assert!(!h.tint);
    }

    #[test]
    fn parses_the_space_form() {
        let h = parse(&[
            "murabito-viewer",
            "--frames",
            "240",
            "--screenshot-dir",
            "out",
            "--screenshot-every",
            "60",
            "--seed",
            "7",
            "--start-east",
            "12.5",
            "--start-north",
            "-3.0",
            "--rings-shaku",
            "6",
            "--auto-pan",
            "--zoom",
            "15.5",
            "--line-mode",
            "nested",
            "--tint",
        ]);
        assert_eq!(h.frames, Some(240));
        assert_eq!(h.screenshot_dir, Some(PathBuf::from("out")));
        assert_eq!(h.screenshot_every, 60);
        assert_eq!(h.seed, Some(7));
        assert_eq!(h.start, Some((12.5, -3.0)));
        assert_eq!(h.rings_shaku, Some(6));
        assert!(h.auto_pan);
        assert_eq!(h.zoom, Some(15.5));
        assert_eq!(h.line_mode.as_deref(), Some("nested"));
        assert!(h.tint);
    }

    #[test]
    fn parses_the_equals_form() {
        let h = parse(&["--frames=120", "--screenshot-dir=screenshots"]);
        assert_eq!(h.frames, Some(120));
        assert_eq!(h.screenshot_dir, Some(PathBuf::from("screenshots")));
    }

    #[test]
    fn malformed_values_fall_back_instead_of_panicking() {
        let h = parse(&[
            "--frames",
            "not-a-number",
            "--screenshot-every",
            "0",
            "--seed",
            "-1",
            "--rings-shaku",
            "banana",
            "--zoom",
            "not-a-float",
        ]);
        assert_eq!(h.frames, None);
        assert_eq!(
            h.screenshot_every, 30,
            "0 must not survive to become a modulo by zero"
        );
        assert_eq!(h.seed, None, "-1 does not fit a u64");
        assert_eq!(h.rings_shaku, None);
        assert_eq!(h.zoom, None);
    }

    #[test]
    fn a_trailing_flag_with_no_value_does_not_panic() {
        let h = parse(&["--frames"]);
        assert_eq!(h.frames, None);
        let h = parse(&["--screenshot-dir"]);
        assert_eq!(h.screenshot_dir, None);
    }

    #[test]
    fn start_needs_both_coordinates() {
        assert_eq!(parse(&["--start-east", "5.0"]).start, None);
        assert_eq!(parse(&["--start-north", "5.0"]).start, None);
    }

    #[test]
    fn absent_screenshot_dir_defaults_to_none() {
        assert_eq!(parse(&["--frames", "10"]).screenshot_dir, None);
    }
}

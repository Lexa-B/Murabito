//! F12 saves a PNG of the window.
//!
//! Here so that anything on screen can be inspected after the fact — a layout that looks
//! wrong, a render that looks off — without having to describe it. Files land in
//! `screenshots/`, which is not tracked by git.

use std::time::{SystemTime, UNIX_EPOCH};

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};

const DIRECTORY: &str = "screenshots";

pub struct ScreenshotPlugin;

impl Plugin for ScreenshotPlugin {
    fn build(&self, app: &mut App) {
        // Not gated on `AppState`: the point is to be able to capture the menus too.
        app.add_systems(Update, capture_on_key);
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
    let path = format!("{DIRECTORY}/{stamp}.png");

    if let Err(error) = std::fs::create_dir_all(DIRECTORY) {
        warn!("could not create {DIRECTORY} ({error}); no screenshot taken");
        return;
    }

    // The capture happens in the render world a frame or two later; `save_to_disk` is an
    // observer that writes the file once the pixels come back.
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(path.clone()));
    info!("screenshot -> {path}");
}

//! What the settings plugin does to the player's file on the way up, and as settings
//! change while the game runs.
//!
//! Every test points `UserDataPlugin` at a temporary directory, so none of this can
//! reach the real `~/.config/murabito`.

use std::fs;
use std::path::PathBuf;

use bevy::prelude::*;
use murabito::i18n::Language;
use murabito::settings::{CameraSettings, SettingsPlugin};
use murabito::user_data::UserDataPlugin;
use tempfile::TempDir;

const SETTINGS_FILE: &str = "settings.yaml";

/// A scratch player directory and an app pointed at it. `MinimalPlugins` is enough:
/// nothing here needs a window, and the plugin reads its file while the app is built.
fn app_in(dir: &TempDir) -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        UserDataPlugin::at(dir.path()),
        SettingsPlugin,
    ));
    app
}

fn settings_file(dir: &TempDir) -> PathBuf {
    dir.path().join(SETTINGS_FILE)
}

fn scratch() -> TempDir {
    tempfile::tempdir().expect("a temporary directory")
}

#[test]
fn a_first_run_writes_a_file_the_player_can_find_and_edit() {
    let dir = scratch();
    let _app = app_in(&dir);
    let written = fs::read_to_string(settings_file(&dir)).expect("a settings file");
    assert!(
        written.contains("pan_speed_scale"),
        "first run wrote {written:?}"
    );
}

#[test]
fn an_existing_file_becomes_the_resources() {
    let dir = scratch();
    fs::write(
        settings_file(&dir),
        "camera:\n  pan_speed_scale: 3.0\nui:\n  language: ja\n",
    )
    .expect("writing the settings file");

    let app = app_in(&dir);
    assert_eq!(
        app.world().resource::<CameraSettings>().pan_speed_scale,
        3.0
    );
    assert_eq!(*app.world().resource::<Language>(), Language::Japanese);
}

/// The regression this whole file exists for. An early version overwrote a file it could
/// not parse with defaults, because inserting a resource counts as changing it on the
/// first frame, and the player lost every setting to one typo. A broken file has to
/// survive both startup and a few frames of the game running.
#[test]
fn a_broken_file_is_never_overwritten() {
    let dir = scratch();
    let broken = "camera: [this is not a mapping\n";
    fs::write(settings_file(&dir), broken).expect("writing the settings file");

    let mut app = app_in(&dir);
    for _ in 0..3 {
        app.update();
    }

    assert_eq!(
        fs::read_to_string(settings_file(&dir)).expect("the settings file"),
        broken,
        "the player's broken file was rewritten"
    );
    // ...and the game still runs, on the defaults.
    assert_eq!(
        *app.world().resource::<CameraSettings>(),
        CameraSettings::default()
    );
}

#[test]
fn changing_a_setting_writes_it_to_the_file() {
    let dir = scratch();
    let mut app = app_in(&dir);
    // One frame first: the resources count as changed on the frame they are inserted,
    // and that frame must not be mistaken for the player changing something.
    app.update();

    app.world_mut()
        .resource_mut::<CameraSettings>()
        .pan_speed_scale = 1.25;
    app.update();

    let written = fs::read_to_string(settings_file(&dir)).expect("the settings file");
    assert!(written.contains("1.25"), "file holds {written:?}");
}

/// Language and camera settings are separate resources but one file, so saving one must
/// not drop the other.
#[test]
fn saving_one_setting_keeps_the_others() {
    let dir = scratch();
    let mut app = app_in(&dir);
    app.update();

    *app.world_mut().resource_mut::<Language>() = Language::Japanese;
    app.update();
    app.world_mut()
        .resource_mut::<CameraSettings>()
        .pan_speed_scale = 0.75;
    app.update();

    let written = fs::read_to_string(settings_file(&dir)).expect("the settings file");
    assert!(written.contains("ja"), "language lost: {written:?}");
    assert!(written.contains("0.75"), "pan speed lost: {written:?}");
}

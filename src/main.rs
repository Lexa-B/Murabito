//! Murabito: a starter scene with an overhead camera.
//!
//! Controls: WASD or the arrow keys pan, the mouse wheel zooms, Escape opens the menu,
//! F12 saves a screenshot.
//!
//! Everything this assembles lives in the library crate (`src/lib.rs`), so that the
//! tests can get at it. This file is the window and the plugin list, and nothing else.

use bevy::prelude::*;
use murabito::{camera, i18n, menu, scene, screenshot, settings, settings_page, state, ui};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Murabito".into(),
                ..default()
            }),
            ..default()
        }))
        // Ours, in dependency order: the camera reads what settings registers.
        // Headless widget behaviour (drag, ranges, stepping) arrives with DefaultPlugins;
        // it draws nothing, so the look of every widget is ours, in `ui.rs`.
        .add_plugins((
            state::StatePlugin,
            i18n::I18nPlugin,
            settings::SettingsPlugin,
            scene::ScenePlugin,
            camera::CameraPlugin,
            ui::UiPlugin,
            menu::MenuPlugin,
            settings_page::SettingsPagePlugin,
            screenshot::ScreenshotPlugin,
        ))
        .run();
}

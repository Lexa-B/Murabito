//! Murabito: a starter scene with an overhead camera.
//!
//! Controls: WASD or the arrow keys pan, the mouse wheel zooms, Escape opens the menu,
//! F12 saves a screenshot.

// ECS query types are tuples of tuples by nature, and naming each one costs more than it
// explains. Bevy's own examples allow this lint for the same reason.
#![allow(clippy::type_complexity)]

mod camera;
mod menu;
mod scene;
mod screenshot;
mod settings;
mod settings_page;
mod state;
mod ui;

use bevy::prelude::*;

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

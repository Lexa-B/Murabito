//! Murabito: a starter scene with an overhead camera.
//!
//! Controls: WASD or the arrow keys pan, the mouse wheel zooms, Escape opens the menu,
//! F12 saves a screenshot.
//!
//! Everything this assembles lives in the library crate (`src/lib.rs`), so that the
//! tests can get at it. This file is the window and the plugin list, and nothing else.

use bevy::prelude::*;
use murabito::{
    being, camera, debug_screen, hex, i18n, menu, scene, screenshot, senses, settings,
    settings_page, state, ui, user_data, 諸法,
};

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
            // First: it says where the player's files live, and the plugins below read
            // that as they are built, not once the app is running.
            user_data::UserDataPlugin::from_config_dir(),
            state::StatePlugin,
            i18n::I18nPlugin,
            諸法::TaxonomyPlugin,
            settings::SettingsPlugin,
            scene::ScenePlugin,
            hex::HexGridPlugin,
            senses::SensesPlugin,
            debug_screen::DebugScreenPlugin,
            being::BeingPlugin,
            camera::CameraPlugin,
            ui::UiPlugin,
            menu::MenuPlugin,
            settings_page::SettingsPagePlugin,
            screenshot::ScreenshotPlugin,
        ))
        .run();
}

//! exp-06: a minimal Bevy app — a starter scene with an overhead camera.
//!
//! Controls: WASD or the arrow keys pan, the mouse wheel zooms.

mod camera;
mod scene;
mod settings;

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Murabito exp-06".into(),
                ..default()
            }),
            ..default()
        }))
        // Ours, in dependency order: the camera reads what settings registers.
        .add_plugins((
            settings::SettingsPlugin,
            scene::ScenePlugin,
            camera::CameraPlugin,
        ))
        .run();
}

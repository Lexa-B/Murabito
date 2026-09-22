use bevy::prelude::*;
use murabito_actions::ActionsPlugin;
use murabito_camera::CameraPlugin;
use murabito_movement::MovementPlugin;
use murabito_progress::ProgressPlugin;
use murabito_scene::ScenePlugin;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            ScenePlugin,
            CameraPlugin,
            MovementPlugin,
            ProgressPlugin,
            ActionsPlugin,
        ))
        .run();
}

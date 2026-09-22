use bevy::prelude::*;
use murabito_actions::ActionsPlugin;
use murabito_camera::CameraPlugin;
use murabito_movement::MovementPlugin;
use murabito_progress::ProgressPlugin;
use murabito_scene::ScenePlugin;
use murabito_user_data::UserDataPlugin;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            // First: it says where the player's files live, and plugins that persist
            // read that as they are built, not once the app runs.
            UserDataPlugin::from_config_dir(),
            ScenePlugin,
            CameraPlugin,
            MovementPlugin,
            ProgressPlugin,
            ActionsPlugin,
        ))
        .run();
}

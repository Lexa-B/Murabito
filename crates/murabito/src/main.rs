use bevy::prelude::*;
use murabito_actions::ActionsPlugin;
use murabito_app_state::{AppStatePlugin, AppStateSettings};
use murabito_camera::{CameraPlugin, CameraSettings};
use murabito_i18n::{I18nPlugin, Language};
use murabito_movement::MovementPlugin;
use murabito_progress::ProgressPlugin;
use murabito_scene::ScenePlugin;
use murabito_settings::{Persist, SettingsPlugin};
use murabito_user_data::UserDataPlugin;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            // First: it says where the player's files live, and plugins that persist
            // read that as they are built, not once the app runs.
            UserDataPlugin::from_config_dir(),
            // Second: it reads the settings file as it is built, and everything
            // persisted below is looked up in it.
            SettingsPlugin,
            I18nPlugin,
            AppStatePlugin,
            ScenePlugin,
            CameraPlugin,
            MovementPlugin,
            ProgressPlugin,
            ActionsPlugin,
        ))
        // Each module owns its settings; the app says which are kept between runs, and
        // under which key in settings.yaml.
        .persist::<CameraSettings>("camera")
        .persist::<Language>("language")
        .persist::<AppStateSettings>("app_state")
        .run();
}

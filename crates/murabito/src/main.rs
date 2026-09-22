use bevy::prelude::*;
use murabito_actions::ActionsPlugin;
use murabito_app_state::{AppStatePlugin, AppStateSettings};
use murabito_camera::{CameraPlugin, CameraSettings};
use murabito_i18n::{I18nPlugin, Language};
use murabito_kinds::KindsPlugin;
use murabito_menu::MenuPlugin;
use murabito_movement::MovementPlugin;
use murabito_navigation::{NavigationPlugin, NavigationSettings};
use murabito_perception::PerceptionPlugin;
use murabito_placement::PlacementPlugin;
use murabito_progress::ProgressPlugin;
use murabito_scene::ScenePlugin;
use murabito_settings::{Persist, SettingsPlugin};
use murabito_settings_page::SettingsPagePlugin;
use murabito_ui::UiPlugin;
use murabito_user_data::UserDataPlugin;
use murabito_vision::VisionPlugin;

fn main() {
    App::new()
        // Grouped, since one tuple holds at most sixteen plugins and a nested tuple is
        // itself a plugin list. The order within and between groups is the order they
        // are built in.
        .add_plugins((
            DefaultPlugins,
            // First: it says where the player's files live, and plugins that persist
            // read that as they are built, not once the app runs. Second: it reads the
            // settings file as it is built, and everything persisted below is looked up
            // in it.
            (UserDataPlugin::from_config_dir(), SettingsPlugin),
            (I18nPlugin, AppStatePlugin),
            (UiPlugin, NavigationPlugin, MenuPlugin, SettingsPagePlugin),
            (KindsPlugin, ScenePlugin, CameraPlugin),
            (
                PlacementPlugin,
                MovementPlugin,
                ProgressPlugin,
                ActionsPlugin,
            ),
            (PerceptionPlugin, VisionPlugin),
        ))
        // Each module owns its settings; the app says which are kept between runs, and
        // under which key in settings.yaml.
        .persist::<CameraSettings>("camera")
        .persist::<Language>("language")
        .persist::<AppStateSettings>("app_state")
        .persist::<NavigationSettings>("navigation")
        .run();
}

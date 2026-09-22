use bevy::prelude::*;
use murabito_camera::CameraPlugin;
use murabito_movement::MovementPlugin;
use murabito_scene::ScenePlugin;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, ScenePlugin, CameraPlugin, MovementPlugin))
        .run();
}

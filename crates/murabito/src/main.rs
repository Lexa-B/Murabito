use bevy::prelude::*;
use murabito_scene::ScenePlugin;

fn main() {
    App::new().add_plugins((DefaultPlugins, ScenePlugin)).run();
}

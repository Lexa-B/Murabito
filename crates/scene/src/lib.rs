use bevy::prelude::*;

pub struct ScenePlugin;

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, announce);
    }
}

fn announce() {
    info!("scene: plugin loaded");
}

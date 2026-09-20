//! What the app is currently doing. Its own module because several unrelated modules
//! gate on it: the menu drives it, the camera reads it, and neither should have to know
//! about the other.

use bevy::prelude::*;

/// Playing, or sat in the escape menu.
#[derive(States, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum AppState {
    #[default]
    Playing,
    Menu,
}

pub struct StatePlugin;

impl Plugin for StatePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<AppState>();
    }
}

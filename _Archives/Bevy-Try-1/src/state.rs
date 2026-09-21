//! What the app is currently doing. Its own module because several unrelated modules
//! gate on it: the screens drive it, the camera reads it, and neither should have to
//! know about the other.

use bevy::prelude::*;

/// Playing, or sat in one of the overlay screens.
#[derive(States, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum AppState {
    #[default]
    Playing,
    Menu,
    Settings,
}

pub struct StatePlugin;

impl Plugin for StatePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<AppState>()
            .add_systems(Update, sync_pause.run_if(state_changed::<AppState>));
    }
}

/// Stops the clock that gameplay reads whenever an overlay screen is up.
///
/// `Time<Virtual>` is what a bare `Res<Time>` resolves to, so pausing it freezes
/// everything driven by elapsed time at once — no system needs to know a menu exists.
/// `Time<Real>` keeps running underneath, which is what the UI and renderer use, so the
/// screens stay responsive.
///
/// Driven by the state rather than by one screen's `OnEnter`/`OnExit`: moving between
/// two paused screens must not unpause for a frame on the way through.
fn sync_pause(state: Res<State<AppState>>, mut time: ResMut<Time<Virtual>>) {
    let should_pause = *state.get() != AppState::Playing;
    if should_pause && !time.is_paused() {
        time.pause();
    } else if !should_pause && time.is_paused() {
        time.unpause();
    }
}

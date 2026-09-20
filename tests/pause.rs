//! The escape screens pause the game clock, and stay paused on the way between them.
//!
//! Headless: `MinimalPlugins` brings the schedules and `Time`, and `StatesPlugin` the
//! state transitions (it rides in with `DefaultPlugins` in the real app, but not here).
//! No window, no GPU, so this runs anywhere `cargo test` does.

use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use murabito::state::{AppState, StatePlugin};

fn test_app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin, StatePlugin));
    // One update to settle into the initial state before anything is asserted.
    app.update();
    app
}

fn go_to(app: &mut App, state: AppState) {
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(state);
    app.update();
}

fn is_paused(app: &App) -> bool {
    app.world().resource::<Time<Virtual>>().is_paused()
}

#[test]
fn the_clock_runs_while_playing() {
    let app = test_app();
    assert!(!is_paused(&app));
}

#[test]
fn opening_the_menu_pauses_the_clock() {
    let mut app = test_app();
    go_to(&mut app, AppState::Menu);
    assert!(is_paused(&app));
}

#[test]
fn resuming_unpauses_the_clock() {
    let mut app = test_app();
    go_to(&mut app, AppState::Menu);
    go_to(&mut app, AppState::Playing);
    assert!(!is_paused(&app));
}

/// The reason pausing lives on the state rather than on one screen's `OnEnter`/`OnExit`:
/// walking from the menu into the settings page must not unpause on the way through.
#[test]
fn moving_between_two_paused_screens_never_unpauses() {
    let mut app = test_app();
    go_to(&mut app, AppState::Menu);
    go_to(&mut app, AppState::Settings);
    assert!(is_paused(&app), "settings page unpaused the game");
    go_to(&mut app, AppState::Menu);
    assert!(is_paused(&app), "going back to the menu unpaused the game");
}

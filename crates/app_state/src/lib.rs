//! What the app is doing: running the world, or holding it still.
//!
//! Its own crate because unrelated modules meet here without meeting each other: the
//! screens will drive the state, the camera reads it, and neither knows the other. It
//! says nothing about screens. Which screen is showing is the UI's own state, so a new
//! screen never touches this crate, and moving between two screens never changes the
//! app state, so it cannot unpause on the way through.
//!
//! Pausing is one call on the world's clock. `Time<Virtual>` is what a bare `Res<Time>`
//! resolves to and what `FixedUpdate` accumulates from, so pausing it freezes the whole
//! simulation at once and no mechanism needs to know a menu exists. `Time<Real>` keeps
//! running underneath, for whatever must stay alive while the world is still.

use bevy::prelude::*;

/// Running the world, or holding it still. Not which screen is up: that is the UI's.
#[derive(States, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum AppState {
    #[default]
    Playing,
    Paused,
}

impl AppState {
    /// Whether the world's clock runs in this state. Only `Playing` lets it: any state
    /// added later holds the world still unless it says otherwise here.
    fn world_runs(self) -> bool {
        self == AppState::Playing
    }
}

pub struct AppStatePlugin;

impl Plugin for AppStatePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<AppState>()
            .add_systems(Update, sync_pause.run_if(state_changed::<AppState>));
    }
}

/// Runs on the frame a transition lands, and reads the state rather than the edge: it
/// asks "should the world run now?", so a transition between two still states, or into
/// one added later, never unpauses by accident.
fn sync_pause(state: Res<State<AppState>>, mut clock: ResMut<Time<Virtual>>) {
    let should_run = state.get().world_runs();
    if should_run && clock.is_paused() {
        clock.unpause();
    } else if !should_run && !clock.is_paused() {
        clock.pause();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::state::app::StatesPlugin;
    use bevy::time::TimeUpdateStrategy;

    /// Headless: `MinimalPlugins` brings the schedules and the clocks, `StatesPlugin`
    /// the transitions (it rides in with `DefaultPlugins` in the app). Every update
    /// is exactly one fixed tick, and the first update, which only starts the clock,
    /// is spent here.
    fn app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin, AppStatePlugin));
        app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
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
    fn the_app_starts_playing_with_the_clock_running() {
        let app = app();

        assert_eq!(
            *app.world().resource::<State<AppState>>().get(),
            AppState::Playing
        );
        assert!(!is_paused(&app));
    }

    #[test]
    fn pausing_stops_the_clock() {
        let mut app = app();

        go_to(&mut app, AppState::Paused);

        assert!(is_paused(&app));
    }

    #[test]
    fn resuming_starts_it_again() {
        let mut app = app();
        go_to(&mut app, AppState::Paused);

        go_to(&mut app, AppState::Playing);

        assert!(!is_paused(&app));
    }

    #[derive(Resource, Default)]
    struct Ticks(u32);

    /// The same app, counting every `FixedUpdate` tick: the simulation's heartbeat.
    fn app_counting_ticks() -> App {
        let mut app = app();
        app.init_resource::<Ticks>();
        app.add_systems(FixedUpdate, |mut ticks: ResMut<Ticks>| ticks.0 += 1);
        app
    }

    fn ticks(app: &App) -> u32 {
        app.world().resource::<Ticks>().0
    }

    #[test]
    fn while_playing_every_update_is_a_tick() {
        let mut app = app_counting_ticks();

        (0..5).for_each(|_| app.update());

        assert_eq!(ticks(&app), 5);
    }

    #[test]
    fn a_paused_app_runs_no_fixed_ticks() {
        let mut app = app_counting_ticks();
        go_to(&mut app, AppState::Paused);
        let at_pause = ticks(&app);

        (0..5).for_each(|_| app.update());

        assert_eq!(ticks(&app), at_pause, "the simulation ticked while paused");
    }

    #[test]
    fn resuming_runs_fixed_ticks_again() {
        let mut app = app_counting_ticks();
        go_to(&mut app, AppState::Paused);
        (0..5).for_each(|_| app.update());
        go_to(&mut app, AppState::Playing);
        let at_resume = ticks(&app);

        (0..5).for_each(|_| app.update());

        assert_eq!(ticks(&app), at_resume + 5);
    }
}

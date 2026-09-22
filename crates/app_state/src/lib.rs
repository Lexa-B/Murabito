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
//!
//! A key toggles between the two, bound in [`AppStateSettings`] by the keybinds rule.
//! Escape for now, until a menu takes Escape and this moves to a key of its own.

use bevy::prelude::*;
use murabito_keybinds::{Binds, Inputs};
use serde::{Deserialize, Serialize};

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

    /// The other one: paused if playing, playing if paused.
    fn toggled(self) -> Self {
        match self {
            AppState::Playing => AppState::Paused,
            AppState::Paused => AppState::Playing,
        }
    }
}

/// How the player has asked this to behave. Owned here, edited by whoever depends on
/// this crate. `#[serde(default)]` so a field a file lacks takes its default.
#[derive(Resource, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppStateSettings {
    /// Holds the world still, or lets it run again.
    pub toggle_pause: Binds,
}

impl Default for AppStateSettings {
    fn default() -> Self {
        Self {
            toggle_pause: Binds::new([KeyCode::Escape]),
        }
    }
}

pub struct AppStatePlugin;

impl Plugin for AppStatePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<AppState>()
            .init_resource::<AppStateSettings>()
            .add_systems(
                Update,
                (toggle_pause, sync_pause.run_if(state_changed::<AppState>)),
            );
    }
}

/// On the frame the bind goes down, asks for the other state. The transition is applied
/// by Bevy before the next frame's `Update`, which is when `sync_pause` sees it.
fn toggle_pause(
    inputs: Inputs,
    settings: Res<AppStateSettings>,
    state: Res<State<AppState>>,
    mut next: ResMut<NextState<AppState>>,
) {
    if settings.toggle_pause.just_pressed(&inputs.held()) {
        next.set(state.get().toggled());
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
    use bevy::input::keyboard::{Key, KeyboardInput};
    use bevy::input::mouse::MouseButtonInput;
    use bevy::input::{ButtonState, InputPlugin};
    use bevy::state::app::StatesPlugin;
    use bevy::time::TimeUpdateStrategy;

    /// Headless: `MinimalPlugins` brings the schedules and the clocks, `StatesPlugin`
    /// the transitions and `InputPlugin` the keyboard and mouse (all three ride in with
    /// `DefaultPlugins` in the app). Every update is exactly one fixed tick, and the
    /// first update, which only starts the clock, is spent here.
    fn app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin, InputPlugin, AppStatePlugin));
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

    fn state(app: &App) -> AppState {
        *app.world().resource::<State<AppState>>().get()
    }

    /// A key going down, the way the window reports it. The input plugin clears
    /// `just_pressed` at the start of every frame, so pressing the resource directly
    /// would be wiped before any system saw it; the message is what survives.
    fn key_down(app: &mut App, key: KeyCode) {
        app.world_mut().write_message(KeyboardInput {
            key_code: key,
            logical_key: Key::Unidentified(bevy::input::keyboard::NativeKey::Unidentified),
            state: ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
    }

    fn key_up(app: &mut App, key: KeyCode) {
        app.world_mut().write_message(KeyboardInput {
            key_code: key,
            logical_key: Key::Unidentified(bevy::input::keyboard::NativeKey::Unidentified),
            state: ButtonState::Released,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
    }

    /// A tap: down on one frame, up on the next, then a frame for the transition to land.
    fn tap(app: &mut App, key: KeyCode) {
        key_down(app, key);
        app.update();
        key_up(app, key);
        app.update();
    }

    #[test]
    fn escape_pauses_a_playing_world() {
        let mut app = app();

        tap(&mut app, KeyCode::Escape);

        assert_eq!(state(&app), AppState::Paused);
        assert!(is_paused(&app));
    }

    #[test]
    fn escape_again_resumes_it() {
        let mut app = app();
        tap(&mut app, KeyCode::Escape);

        tap(&mut app, KeyCode::Escape);

        assert_eq!(state(&app), AppState::Playing);
        assert!(!is_paused(&app));
    }

    #[test]
    fn holding_the_key_toggles_once_not_every_frame() {
        let mut app = app();

        key_down(&mut app, KeyCode::Escape);
        (0..5).for_each(|_| app.update());

        assert_eq!(state(&app), AppState::Paused);
    }

    #[test]
    fn a_key_that_is_not_bound_does_nothing() {
        let mut app = app();

        tap(&mut app, KeyCode::KeyP);

        assert_eq!(state(&app), AppState::Playing);
    }

    #[test]
    fn the_toggle_can_be_rebound_to_a_mouse_button() {
        let side_button = MouseButton::Other(7);
        let mut app = app();
        app.world_mut()
            .resource_mut::<AppStateSettings>()
            .toggle_pause = Binds::new([side_button]);

        app.world_mut().write_message(MouseButtonInput {
            button: side_button,
            state: ButtonState::Pressed,
            window: Entity::PLACEHOLDER,
        });
        app.update();
        app.update();

        assert_eq!(state(&app), AppState::Paused);
    }

    #[test]
    fn the_settings_write_as_one_word_per_bind() {
        let yaml = serde_yaml_ng::to_string(&AppStateSettings::default()).expect("serialises");

        assert_eq!(yaml.trim(), "toggle_pause:\n- Escape");
    }
}

//! Which overlay is up over the paused world, and the key that steps back out of it.
//!
//! The overlays meet here without meeting each other: the menu's Settings button and
//! the settings page's Back button both set [`Overlay`], and neither crate imports the
//! other. Adding an overlay is a variant here and a crate of its own; the menu doesn't
//! change.
//!
//! `Overlay` is a sub-state of `AppState::Paused`: it exists only while the world is
//! held still, because there is no menu over a running world. Leaving `Paused` removes
//! it, and Bevy runs the departing overlay's `OnExit` on the way out, so an overlay
//! that closes because the world resumed tears itself down the same as one closed by
//! its own button. Nothing here keeps two states in step by hand.

use bevy::prelude::*;
use murabito_app_state::AppState;
use murabito_keybinds::{Binds, Inputs};
use serde::{Deserialize, Serialize};

/// What is laid over the paused world. `None` is a pause with nothing over it.
#[derive(SubStates, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[source(AppState = AppState::Paused)]
pub enum Overlay {
    #[default]
    None,
    Menu,
    Settings,
}

/// How the player has asked this to behave. Owned here, edited by whoever depends on
/// this crate. `#[serde(default)]` so a field a file lacks takes its default.
#[derive(Resource, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct NavigationSettings {
    /// Back one overlay: opens the menu over a world with nothing over it, and closes
    /// the menu, or steps back to it from a deeper overlay.
    pub back: Binds,
}

impl Default for NavigationSettings {
    fn default() -> Self {
        Self {
            back: Binds::new([KeyCode::Escape]),
        }
    }
}

pub struct NavigationPlugin;

impl Plugin for NavigationPlugin {
    fn build(&self, app: &mut App) {
        app.add_sub_state::<Overlay>()
            .init_resource::<NavigationSettings>()
            .add_systems(Update, go_back);
    }
}

/// Where "back" leads from here. Written as one table so that every case is in one
/// place and the tests can walk it. The overlay is `None` while the world runs, since
/// the sub-state does not exist then.
fn back_from(state: AppState, overlay: Overlay) -> (AppState, Overlay) {
    match (state, overlay) {
        (AppState::Playing, _) | (AppState::Paused, Overlay::None) => {
            (AppState::Paused, Overlay::Menu)
        }
        (AppState::Paused, Overlay::Menu) => (AppState::Playing, Overlay::None),
        (AppState::Paused, Overlay::Settings) => (AppState::Paused, Overlay::Menu),
    }
}

/// On the frame the back bind goes down, asks for wherever back leads. Both states are
/// requested together and land together: a sub-state set on the frame its parent comes
/// into being starts at the value asked for, not its default.
fn go_back(
    inputs: Inputs,
    settings: Res<NavigationSettings>,
    state: Res<State<AppState>>,
    overlay: Option<Res<State<Overlay>>>,
    mut next_state: ResMut<NextState<AppState>>,
    mut next_overlay: ResMut<NextState<Overlay>>,
) {
    if !settings.back.just_pressed(&inputs.held()) {
        return;
    }
    let here = *state.get();
    let over = overlay.map_or(Overlay::None, |o| *o.get());
    let (to_state, to_overlay) = back_from(here, over);
    // `set` re-runs OnEnter/OnExit even for the state already in force, so only what
    // changes is asked for.
    if to_state != here {
        next_state.set(to_state);
    }
    if to_state == AppState::Paused && to_overlay != over {
        next_overlay.set(to_overlay);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
    use bevy::input::mouse::MouseButtonInput;
    use bevy::input::{ButtonState, InputPlugin};
    use bevy::state::app::StatesPlugin;
    use murabito_app_state::AppStatePlugin;

    /// Headless, with the transitions, the keyboard and mouse, and the app state this
    /// crate builds on. Run once to settle into the starting state.
    fn app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            StatesPlugin,
            InputPlugin,
            AppStatePlugin,
            NavigationPlugin,
        ));
        app.update();
        app
    }

    fn state(app: &App) -> AppState {
        *app.world().resource::<State<AppState>>().get()
    }

    /// `None` when the sub-state does not exist, which is what playing looks like.
    fn overlay(app: &App) -> Option<Overlay> {
        app.world()
            .get_resource::<State<Overlay>>()
            .map(|o| *o.get())
    }

    fn key(app: &mut App, key: KeyCode, state: ButtonState) {
        app.world_mut().write_message(KeyboardInput {
            key_code: key,
            logical_key: Key::Unidentified(NativeKey::Unidentified),
            state,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
    }

    /// Down on one frame, up on the next, then a frame for the transition to land.
    fn tap(app: &mut App, k: KeyCode) {
        key(app, k, ButtonState::Pressed);
        app.update();
        key(app, k, ButtonState::Released);
        app.update();
    }

    fn go_to(app: &mut App, to: AppState) {
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(to);
        app.update();
    }

    fn open(app: &mut App, to: Overlay) {
        app.world_mut().resource_mut::<NextState<Overlay>>().set(to);
        app.update();
    }

    #[test]
    fn while_playing_there_is_no_overlay_at_all() {
        let app = app();

        assert_eq!(state(&app), AppState::Playing);
        assert_eq!(overlay(&app), None);
    }

    #[test]
    fn back_from_playing_pauses_the_world_and_opens_the_menu_in_one_tap() {
        let mut app = app();

        tap(&mut app, KeyCode::Escape);

        assert_eq!(state(&app), AppState::Paused);
        assert_eq!(overlay(&app), Some(Overlay::Menu));
    }

    #[test]
    fn back_from_the_menu_resumes_the_world_and_the_overlay_is_gone() {
        let mut app = app();
        tap(&mut app, KeyCode::Escape);

        tap(&mut app, KeyCode::Escape);

        assert_eq!(state(&app), AppState::Playing);
        assert_eq!(overlay(&app), None);
    }

    #[test]
    fn back_from_settings_steps_back_to_the_menu() {
        let mut app = app();
        tap(&mut app, KeyCode::Escape);
        open(&mut app, Overlay::Settings);

        tap(&mut app, KeyCode::Escape);

        assert_eq!(state(&app), AppState::Paused);
        assert_eq!(overlay(&app), Some(Overlay::Menu));
    }

    #[test]
    fn a_pause_with_nothing_over_it_is_an_overlay_of_none() {
        let mut app = app();

        go_to(&mut app, AppState::Paused);

        assert_eq!(overlay(&app), Some(Overlay::None));
    }

    #[test]
    fn back_over_a_bare_pause_opens_the_menu() {
        let mut app = app();
        go_to(&mut app, AppState::Paused);

        tap(&mut app, KeyCode::Escape);

        assert_eq!(overlay(&app), Some(Overlay::Menu));
    }

    #[test]
    fn resuming_the_world_removes_whatever_overlay_was_up() {
        let mut app = app();
        tap(&mut app, KeyCode::Escape);
        open(&mut app, Overlay::Settings);

        go_to(&mut app, AppState::Playing);

        assert_eq!(overlay(&app), None);
    }

    #[test]
    fn the_back_table_covers_every_case() {
        use AppState::{Paused, Playing};
        use Overlay::{Menu, None, Settings};

        assert_eq!(back_from(Playing, None), (Paused, Menu));
        assert_eq!(back_from(Paused, None), (Paused, Menu));
        assert_eq!(back_from(Paused, Menu), (Playing, None));
        assert_eq!(back_from(Paused, Settings), (Paused, Menu));
    }

    #[test]
    fn a_key_that_is_not_bound_does_nothing() {
        let mut app = app();

        tap(&mut app, KeyCode::KeyP);

        assert_eq!(overlay(&app), None);
    }

    #[test]
    fn back_can_be_rebound_to_a_mouse_button() {
        let side_button = MouseButton::Other(8);
        let mut app = app();
        app.world_mut().resource_mut::<NavigationSettings>().back = Binds::new([side_button]);

        app.world_mut().write_message(MouseButtonInput {
            button: side_button,
            state: ButtonState::Pressed,
            window: Entity::PLACEHOLDER,
        });
        app.update();
        app.update();

        assert_eq!(overlay(&app), Some(Overlay::Menu));
    }

    #[test]
    fn the_settings_write_as_one_word_per_bind() {
        let yaml = serde_yaml_ng::to_string(&NavigationSettings::default()).expect("serialises");

        assert_eq!(yaml.trim(), "back:\n- Escape");
    }
}

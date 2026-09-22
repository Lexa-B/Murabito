//! The menu: a dimmed overlay with three buttons, Settings, Resume and Quit.
//!
//! Built when `Overlay::Menu` is entered and torn down when it is left, whichever way
//! it is left: by its own buttons, by the back key, or by the world resuming under it.
//! It never sets `AppState::Paused` itself; opening the menu is `murabito_navigation`'s
//! doing. Its Settings button asks for `Overlay::Settings` and knows nothing about the
//! crate that answers.

use bevy::app::AppExit;
use bevy::prelude::*;
use murabito_app_state::AppState;
use murabito_navigation::Overlay;
use murabito_ui::{UiFont, overlay, spawn_button};

const BUTTON_WIDTH: f32 = 220.0;

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(Overlay::Menu), spawn_menu)
            .add_systems(OnExit(Overlay::Menu), despawn_menu)
            .add_systems(Update, press.run_if(in_state(Overlay::Menu)));
    }
}

/// Marks the menu's root node, so closing it despawns the whole tree in one go.
#[derive(Component)]
struct MenuRoot;

/// What a button does. One component with a variant per button, so `press` is one
/// match rather than a system per button.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum MenuButton {
    Settings,
    Resume,
    Quit,
}

impl MenuButton {
    /// The catalogue key of the button's label.
    fn label(self) -> &'static str {
        match self {
            MenuButton::Settings => "menu.settings",
            MenuButton::Resume => "menu.resume",
            MenuButton::Quit => "menu.quit",
        }
    }
}

const BUTTONS: [MenuButton; 3] = [MenuButton::Settings, MenuButton::Resume, MenuButton::Quit];

fn spawn_menu(mut commands: Commands, font: Res<UiFont>) {
    commands.spawn((MenuRoot, overlay())).with_children(|menu| {
        for button in BUTTONS {
            spawn_button(menu, &font, button, button.label(), BUTTON_WIDTH);
        }
    });
}

/// Despawning the root takes its children with it.
fn despawn_menu(mut commands: Commands, roots: Query<Entity, With<MenuRoot>>) {
    for root in &roots {
        commands.entity(root).despawn();
    }
}

fn press(
    buttons: Query<(&Interaction, &MenuButton), Changed<Interaction>>,
    mut next_state: ResMut<NextState<AppState>>,
    mut next_overlay: ResMut<NextState<Overlay>>,
    mut exit: MessageWriter<AppExit>,
) {
    for (interaction, button) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match button {
            MenuButton::Settings => next_overlay.set(Overlay::Settings),
            MenuButton::Resume => next_state.set(AppState::Playing),
            MenuButton::Quit => {
                exit.write(AppExit::Success);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::state::app::StatesPlugin;
    use murabito_app_state::AppStatePlugin;
    use murabito_i18n::{I18nPlugin, Language, Localized};
    use murabito_navigation::NavigationPlugin;
    use murabito_ui::UiPlugin;

    /// Headless, with everything the menu builds on: the states, the kit (which wants
    /// the asset server for its font) and i18n for the labels. `InputPlugin` is what
    /// navigation's back key asks for; no test here presses it. Run once to settle.
    fn app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            StatesPlugin,
            bevy::input::InputPlugin,
            AssetPlugin::default(),
        ))
        .init_asset::<Font>()
        .add_plugins((
            AppStatePlugin,
            NavigationPlugin,
            I18nPlugin,
            UiPlugin,
            MenuPlugin,
        ));
        app.update();
        app
    }

    fn open_menu(app: &mut App) {
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Paused);
        app.world_mut()
            .resource_mut::<NextState<Overlay>>()
            .set(Overlay::Menu);
        app.update();
    }

    fn buttons(app: &mut App) -> Vec<(Entity, MenuButton)> {
        let world = app.world_mut();
        world
            .query::<(Entity, &MenuButton)>()
            .iter(world)
            .map(|(e, b)| (e, *b))
            .collect()
    }

    fn press_button(app: &mut App, wanted: MenuButton) {
        let (entity, _) = buttons(app)
            .into_iter()
            .find(|(_, b)| *b == wanted)
            .expect("that button is on screen");
        app.world_mut()
            .entity_mut(entity)
            .insert(Interaction::Pressed);
        app.update();
        app.update();
    }

    fn state(app: &App) -> AppState {
        *app.world().resource::<State<AppState>>().get()
    }

    fn overlay_up(app: &App) -> Option<Overlay> {
        app.world()
            .get_resource::<State<Overlay>>()
            .map(|o| *o.get())
    }

    #[test]
    fn opening_the_menu_shows_its_three_buttons_in_order() {
        let mut app = app();

        open_menu(&mut app);

        let shown: Vec<MenuButton> = buttons(&mut app).into_iter().map(|(_, b)| b).collect();
        assert_eq!(shown, BUTTONS);
    }

    #[test]
    fn nothing_is_on_screen_while_playing() {
        let mut app = app();

        assert!(buttons(&mut app).is_empty());
    }

    #[test]
    fn the_labels_come_from_the_catalogue_in_the_language_in_force() {
        let mut app = app();
        open_menu(&mut app);
        app.update();

        let world = app.world_mut();
        let labels: Vec<String> = world
            .query::<(&Localized, &Text)>()
            .iter(world)
            .map(|(_, text)| text.0.clone())
            .collect();
        assert_eq!(labels, ["Settings", "Resume", "Quit"]);
    }

    #[test]
    fn the_labels_follow_a_change_of_language_without_a_rebuild() {
        let mut app = app();
        open_menu(&mut app);
        app.update();
        let before: Vec<Entity> = buttons(&mut app).into_iter().map(|(e, _)| e).collect();

        *app.world_mut().resource_mut::<Language>() = Language::Japanese;
        app.update();

        let world = app.world_mut();
        let labels: Vec<String> = world
            .query::<(&Localized, &Text)>()
            .iter(world)
            .map(|(_, text)| text.0.clone())
            .collect();
        assert_eq!(labels, ["設定", "再開", "終了"]);
        let after: Vec<Entity> = buttons(&mut app).into_iter().map(|(e, _)| e).collect();
        assert_eq!(before, after, "the buttons were rebuilt");
    }

    #[test]
    fn resume_runs_the_world_and_the_menu_is_gone() {
        let mut app = app();
        open_menu(&mut app);

        press_button(&mut app, MenuButton::Resume);

        assert_eq!(state(&app), AppState::Playing);
        assert_eq!(overlay_up(&app), None);
        assert!(buttons(&mut app).is_empty());
    }

    #[test]
    fn settings_asks_for_the_settings_overlay_and_the_menu_is_gone() {
        let mut app = app();
        open_menu(&mut app);

        press_button(&mut app, MenuButton::Settings);

        assert_eq!(state(&app), AppState::Paused);
        assert_eq!(overlay_up(&app), Some(Overlay::Settings));
        assert!(buttons(&mut app).is_empty());
    }

    #[test]
    fn quit_asks_the_app_to_exit() {
        let mut app = app();
        open_menu(&mut app);

        press_button(&mut app, MenuButton::Quit);

        assert!(app.should_exit().is_some());
    }

    #[test]
    fn the_menu_is_torn_down_when_the_world_resumes_under_it() {
        let mut app = app();
        open_menu(&mut app);

        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Playing);
        app.update();

        assert!(buttons(&mut app).is_empty());
        let world = app.world_mut();
        assert_eq!(world.query::<&MenuRoot>().iter(world).count(), 0);
    }
}

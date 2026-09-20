//! The escape menu: a dimmed overlay with two buttons.

use bevy::app::AppExit;
use bevy::prelude::*;

use crate::state::AppState;
use crate::ui;

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app
            // Escape works in every state, so it is not gated on any of them.
            .add_systems(Update, toggle_menu)
            // Bevy runs these once on the state transition, not every frame: the menu is
            // built when it opens and torn down when it closes.
            .add_systems(OnEnter(AppState::Menu), spawn_menu)
            .add_systems(OnExit(AppState::Menu), despawn_menu)
            .add_systems(Update, button_actions.run_if(in_state(AppState::Menu)));
    }
}

/// Marks the menu's root node, so closing it despawns the whole tree in one go.
#[derive(Component)]
struct MenuRoot;

/// What a button does. One component, one variant per button, so `button_actions` is a
/// single match rather than a system per button.
#[derive(Component, Clone, Copy)]
enum MenuButton {
    Settings,
    Resume,
    Quit,
}

/// Escape opens the menu while playing and closes it while it's open. From the settings
/// page it steps back to the menu, so Escape always means "back one screen".
fn toggle_menu(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<AppState>>,
    mut next: ResMut<NextState<AppState>>,
) {
    if !keys.just_pressed(KeyCode::Escape) {
        return;
    }
    next.set(match state.get() {
        AppState::Playing => AppState::Menu,
        AppState::Menu => AppState::Playing,
        AppState::Settings => AppState::Menu,
    });
}

fn spawn_menu(mut commands: Commands) {
    commands
        .spawn((MenuRoot, ui::overlay()))
        .with_children(|menu| {
            ui::spawn_button(menu, MenuButton::Settings, "Settings", 220.0);
            ui::spawn_button(menu, MenuButton::Resume, "Resume", 220.0);
            ui::spawn_button(menu, MenuButton::Quit, "Quit", 220.0);
        });
}

fn despawn_menu(mut commands: Commands, menu: Single<Entity, With<MenuRoot>>) {
    // Despawning a parent takes its children with it, so this clears the whole menu.
    commands.entity(*menu).despawn();
}

fn button_actions(
    buttons: Query<(&Interaction, &MenuButton), Changed<Interaction>>,
    mut next: ResMut<NextState<AppState>>,
    mut exit: MessageWriter<AppExit>,
) {
    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action {
            MenuButton::Settings => next.set(AppState::Settings),
            MenuButton::Resume => next.set(AppState::Playing),
            MenuButton::Quit => {
                exit.write(AppExit::Success);
            }
        }
    }
}

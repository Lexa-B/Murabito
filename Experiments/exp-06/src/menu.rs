//! The escape menu: a dimmed overlay with two buttons.

use bevy::app::AppExit;
use bevy::prelude::*;

use crate::state::AppState;

/// Dim behind the menu, rather than an opaque panel, so the world stays visible.
const SCRIM: Color = Color::srgba(0.05, 0.06, 0.09, 0.75);

/// Button fills: resting, hovered, and held down.
const BUTTON: Color = Color::srgb(0.22, 0.25, 0.31);
const BUTTON_HOVERED: Color = Color::srgb(0.30, 0.34, 0.42);
const BUTTON_PRESSED: Color = Color::srgb(0.16, 0.18, 0.23);

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app
            // Escape works in both states, so it is not gated on either.
            .add_systems(Update, toggle_menu)
            // Bevy runs these once on the state transition, not every frame: the menu is
            // built when it opens and torn down when it closes.
            .add_systems(OnEnter(AppState::Menu), (spawn_menu, pause_time))
            .add_systems(OnExit(AppState::Menu), (despawn_menu, resume_time))
            .add_systems(
                Update,
                (button_visuals, button_actions).run_if(in_state(AppState::Menu)),
            );
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
    Quit,
}

/// Escape opens the menu while playing, and closes it while it's open.
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
    });
}

/// Stops the clock that gameplay reads. `Time<Virtual>` is what a bare `Res<Time>`
/// resolves to, so pausing it freezes everything driven by elapsed time at once — no
/// system needs to know the menu exists. `Time<Real>` keeps running underneath, which
/// is what the UI and the renderer use, so the menu itself stays responsive.
fn pause_time(mut time: ResMut<Time<Virtual>>) {
    time.pause();
}

fn resume_time(mut time: ResMut<Time<Virtual>>) {
    time.unpause();
}

fn spawn_menu(mut commands: Commands) {
    commands
        .spawn((
            MenuRoot,
            // A full-screen flex column, centred both ways: Bevy's UI layout is flexbox,
            // so this is the same vocabulary as CSS.
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(12.0),
                ..default()
            },
            BackgroundColor(SCRIM),
        ))
        .with_children(|menu| {
            spawn_button(menu, MenuButton::Settings, "Settings");
            spawn_button(menu, MenuButton::Quit, "Quit");
        });
}

fn spawn_button(menu: &mut ChildSpawnerCommands, action: MenuButton, label: &str) {
    menu.spawn((
        Button,
        action,
        Node {
            width: Val::Px(220.0),
            height: Val::Px(48.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            // A field of `Node` in Bevy 0.19, not a component of its own.
            border_radius: BorderRadius::all(Val::Px(6.0)),
            ..default()
        },
        BackgroundColor(BUTTON),
    ))
    .with_child((
        Text::new(label),
        TextFont {
            font_size: FontSize::Px(20.0),
            ..default()
        },
        TextColor(Color::srgb(0.92, 0.93, 0.95)),
    ));
}

fn despawn_menu(mut commands: Commands, menu: Single<Entity, With<MenuRoot>>) {
    // Despawning a parent takes its children with it, so this clears the whole menu.
    commands.entity(*menu).despawn();
}

/// Tints a button as the pointer enters, leaves, or presses it. `Changed<Interaction>`
/// means this runs only for buttons whose state actually changed this frame.
fn button_visuals(
    mut buttons: Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<Button>)>,
) {
    for (interaction, mut colour) in &mut buttons {
        *colour = BackgroundColor(match interaction {
            Interaction::Pressed => BUTTON_PRESSED,
            Interaction::Hovered => BUTTON_HOVERED,
            Interaction::None => BUTTON,
        });
    }
}

fn button_actions(
    buttons: Query<(&Interaction, &MenuButton), Changed<Interaction>>,
    mut exit: MessageWriter<AppExit>,
) {
    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action {
            // Deliberately inert for now: the settings page comes later, and
            // `CameraSettings` is already waiting for it.
            MenuButton::Settings => info!("settings: not wired up yet"),
            MenuButton::Quit => {
                exit.write(AppExit::Success);
            }
        }
    }
}

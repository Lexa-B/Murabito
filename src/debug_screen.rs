//! The F3 screen: terse lines drawn over the living world, some of them clickable.
//!
//! Deliberately **not** an `AppState`. Every state other than `Playing` pauses
//! `Time<Virtual>`, and the whole point of this screen is to watch the world carry on
//! while you change what is drawn on top of it. So it spawns and despawns a UI root
//! instead, the clock never stops, and the camera still pans underneath.
//!
//! It also gets its own hover tinting rather than `ui::button_visuals`, which runs only
//! while a menu is up and would paint these lines with menu chrome. That is a happy
//! accident: a debug line should read as text that happens to respond, not as a button.

use std::collections::BTreeSet;

use bevy::prelude::*;
use bevy::ui::widget::TextShadow;

use crate::being::Being;
use crate::i18n::Localized;
use crate::senses::HideSenses;
use crate::state::AppState;
use crate::ui::{UiFont, text_font};
use crate::諸法::種;

/// Public so the one-shot capture can press it: an F3 screen that cannot be
/// screenshotted is a screen nobody checks.
pub const TOGGLE_KEY: KeyCode = KeyCode::F3;

const LINE_SIZE: f32 = 16.0;
const LINE_TEXT: Color = Color::srgb(0.94, 0.95, 0.97);
/// No panel behind the text, so it is read against grass, sky and gizmos alike. A shadow
/// is what keeps it legible on all three; a background would make this a window.
const LINE_SHADOW: Color = Color::srgba(0.0, 0.0, 0.0, 0.85);
const LINE_IDLE: Color = Color::NONE;
const LINE_HOVERED: Color = Color::srgba(1.0, 1.0, 1.0, 0.16);
const LINE_PRESSED: Color = Color::srgba(1.0, 1.0, 1.0, 0.28);

/// Shown when that kind's overlay is drawn, and when it is not.
const MARK_SHOWN: &str = "[x] ";
const MARK_HIDDEN: &str = "[ ] ";

/// Locale namespace for taxonomy keys. The ontology holds no display names by design —
/// `狐` is the identity, and what a reader sees comes from `assets/locales/`, keyed by
/// the axis it belongs to. A key with no entry falls back to itself, which on a debug
/// screen is a perfectly good answer.
const 分類: &str = "分類";

pub struct DebugScreenPlugin;

impl Plugin for DebugScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (toggle_screen, click_lines, line_visuals, sync_marks)
                .chain()
                .run_if(in_state(AppState::Playing)),
        )
        // Leaving play takes it with us: its systems stop running, so a screen left up
        // would sit there under the menu's scrim looking live and answering nothing.
        .add_systems(OnExit(AppState::Playing), close_screen);
    }
}

/// The screen's root. Its presence is the open/closed flag — no resource needed.
///
/// `pub(crate)` only because it appears in `toggle_screen`'s signature, which the
/// capture harness orders itself against.
#[derive(Component)]
pub(crate) struct DebugScreen;

/// A clickable line that shows or hides one kind's sense overlay.
#[derive(Component)]
struct SenseToggle(String);

/// The `[x]` at the head of a line. Carries the same key as its line so it can be
/// written without walking the hierarchy.
#[derive(Component)]
struct ToggleMark(String);

pub(crate) fn toggle_screen(
    keys: Res<ButtonInput<KeyCode>>,
    open: Query<Entity, With<DebugScreen>>,
    kinds: Query<&種, With<Being>>,
    font: Res<UiFont>,
    mut commands: Commands,
) {
    if !keys.just_pressed(TOGGLE_KEY) {
        return;
    }
    if let Ok(root) = open.single() {
        commands.entity(root).despawn();
        return;
    }

    // Sorted and deduplicated: one line per kind however many beings carry it, and the
    // same order every time the screen is opened.
    let present: BTreeSet<&str> = kinds.iter().map(種::key).collect();

    commands
        .spawn((
            DebugScreen,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(8.0),
                left: Val::Px(8.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(2.0),
                ..default()
            },
        ))
        .with_children(|screen| {
            screen.spawn((
                Text::default(),
                Localized::new("debug.senses"),
                text_font(&font, LINE_SIZE),
                TextColor(LINE_TEXT),
                TextShadow {
                    color: LINE_SHADOW,
                    ..default()
                },
            ));
            for kind in present {
                spawn_toggle(screen, &font, kind);
            }
        });
}

fn spawn_toggle(screen: &mut ChildSpawnerCommands, font: &UiFont, kind: &str) {
    screen
        .spawn((
            Button,
            SenseToggle(kind.to_string()),
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                padding: UiRect::axes(Val::Px(4.0), Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(LINE_IDLE),
        ))
        .with_children(|line| {
            line.spawn((
                Text::new(MARK_SHOWN),
                ToggleMark(kind.to_string()),
                text_font(font, LINE_SIZE),
                TextColor(LINE_TEXT),
                TextShadow {
                    color: LINE_SHADOW,
                    ..default()
                },
            ));
            line.spawn((
                Text::default(),
                Localized::new(format!("{分類}.{kind}")),
                text_font(font, LINE_SIZE),
                TextColor(LINE_TEXT),
                TextShadow {
                    color: LINE_SHADOW,
                    ..default()
                },
            ));
        });
}

fn close_screen(open: Query<Entity, With<DebugScreen>>, mut commands: Commands) {
    for root in &open {
        commands.entity(root).despawn();
    }
}

/// Adds or removes [`HideSenses`] on every being of the clicked kind.
fn click_lines(
    lines: Query<(&Interaction, &SenseToggle), Changed<Interaction>>,
    beings: Query<(Entity, &種, Has<HideSenses>), With<Being>>,
    mut commands: Commands,
) {
    for (interaction, toggle) in &lines {
        if *interaction != Interaction::Pressed {
            continue;
        }
        // Read the kind's current state once, from any being carrying it, so that a kind
        // whose beings have somehow drifted apart is brought back into agreement rather
        // than each being flipping independently.
        let hidden = beings
            .iter()
            .find(|(_, kind, _)| kind.key() == toggle.0)
            .is_some_and(|(_, _, hidden)| hidden);

        for (entity, kind, _) in &beings {
            if kind.key() != toggle.0 {
                continue;
            }
            if hidden {
                commands.entity(entity).remove::<HideSenses>();
            } else {
                commands.entity(entity).insert(HideSenses);
            }
        }
    }
}

fn line_visuals(mut lines: Query<(&Interaction, &mut BackgroundColor), With<SenseToggle>>) {
    for (interaction, mut colour) in &mut lines {
        let next = match interaction {
            Interaction::Pressed => LINE_PRESSED,
            Interaction::Hovered => LINE_HOVERED,
            Interaction::None => LINE_IDLE,
        };
        if colour.0 != next {
            *colour = BackgroundColor(next);
        }
    }
}

/// Keeps every `[x]` honest, including on the frame the screen is first opened.
fn sync_marks(
    beings: Query<(&種, Has<HideSenses>), With<Being>>,
    mut marks: Query<(&ToggleMark, &mut Text)>,
) {
    for (mark, mut text) in &mut marks {
        let hidden = beings
            .iter()
            .find(|(kind, _)| kind.key() == mark.0)
            .is_some_and(|(_, hidden)| hidden);
        let next = if hidden { MARK_HIDDEN } else { MARK_SHOWN };
        if text.as_str() != next {
            **text = next.to_string();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::state::app::StatesPlugin;

    /// Headless: no window, no font file. `UiFont` holds a default handle because
    /// nothing here renders a glyph — the tests are about what the lines *do*.
    fn app_with(kinds: &[&str]) -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin, crate::state::StatePlugin))
            .insert_resource(UiFont(Handle::default()))
            .init_resource::<ButtonInput<KeyCode>>()
            .add_plugins(DebugScreenPlugin);
        for kind in kinds {
            app.world_mut().spawn((Being, 種::new(*kind)));
        }
        app.update();
        app
    }

    /// `reset` and not `clear`: `clear` drops `just_pressed` but leaves the key *held*,
    /// and `press` only records a fresh `just_pressed` for a key that was not already
    /// down — so a second `press` after a `clear` is silently a no-op. In the real app
    /// the input plugin does this on the key actually coming up.
    fn press_toggle(app: &mut App) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(TOGGLE_KEY);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset(TOGGLE_KEY);
    }

    fn screens(app: &mut App) -> usize {
        app.world_mut()
            .query_filtered::<(), With<DebugScreen>>()
            .iter(app.world())
            .count()
    }

    fn lines(app: &mut App) -> Vec<String> {
        let mut found: Vec<String> = app
            .world_mut()
            .query::<&SenseToggle>()
            .iter(app.world())
            .map(|toggle| toggle.0.clone())
            .collect();
        found.sort();
        found
    }

    fn click(app: &mut App, kind: &str) {
        let line = app
            .world_mut()
            .query::<(Entity, &SenseToggle)>()
            .iter(app.world())
            .find(|(_, toggle)| toggle.0 == kind)
            .map(|(entity, _)| entity)
            .expect("a line for that kind");
        app.world_mut()
            .entity_mut(line)
            .insert(Interaction::Pressed);
        app.update();
    }

    fn hidden(app: &mut App, kind: &str) -> bool {
        app.world_mut()
            .query::<(&種, Has<HideSenses>)>()
            .iter(app.world())
            .find(|(seed, _)| seed.key() == kind)
            .map(|(_, hidden)| hidden)
            .expect("a being of that kind")
    }

    #[test]
    fn the_key_opens_the_screen_and_closes_it_again() {
        let mut app = app_with(&["狐"]);
        assert_eq!(screens(&mut app), 0, "it should start closed");
        press_toggle(&mut app);
        assert_eq!(screens(&mut app), 1);
        press_toggle(&mut app);
        assert_eq!(screens(&mut app), 0);
    }

    /// One line per kind, not per being: the toggle is about what a fox is, not about
    /// which fox.
    #[test]
    fn many_beings_of_one_kind_get_one_line() {
        let mut app = app_with(&["狐", "狐", "狐", "兎"]);
        press_toggle(&mut app);
        assert_eq!(lines(&mut app), vec!["兎".to_string(), "狐".to_string()]);
    }

    #[test]
    fn clicking_a_line_hides_that_kind_and_leaves_the_others_alone() {
        let mut app = app_with(&["狐", "兎"]);
        press_toggle(&mut app);
        click(&mut app, "狐");
        assert!(hidden(&mut app, "狐"));
        assert!(!hidden(&mut app, "兎"), "the rabbit should be untouched");
    }

    /// Every fox, not just the first one found.
    #[test]
    fn hiding_a_kind_hides_all_of_them() {
        let mut app = app_with(&["狐", "狐", "狐"]);
        press_toggle(&mut app);
        click(&mut app, "狐");
        let count = app
            .world_mut()
            .query_filtered::<(), With<HideSenses>>()
            .iter(app.world())
            .count();
        assert_eq!(count, 3);
    }

    #[test]
    fn clicking_again_shows_it_once_more() {
        let mut app = app_with(&["狐"]);
        press_toggle(&mut app);
        click(&mut app, "狐");
        assert!(hidden(&mut app, "狐"));
        click(&mut app, "狐");
        assert!(!hidden(&mut app, "狐"));
    }

    #[test]
    fn the_mark_follows_what_is_hidden() {
        let mut app = app_with(&["狐"]);
        press_toggle(&mut app);
        let mark = |app: &mut App| {
            app.world_mut()
                .query::<(&ToggleMark, &Text)>()
                .iter(app.world())
                .next()
                .map(|(_, text)| text.as_str().to_string())
                .expect("a mark")
        };
        assert_eq!(mark(&mut app), MARK_SHOWN);
        click(&mut app, "狐");
        assert_eq!(mark(&mut app), MARK_HIDDEN);
    }

    /// Leaving play takes the screen with it, rather than leaving it under the menu's
    /// scrim looking live while its systems no longer run.
    #[test]
    fn leaving_play_closes_the_screen() {
        let mut app = app_with(&["狐"]);
        press_toggle(&mut app);
        assert_eq!(screens(&mut app), 1);
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Menu);
        app.update();
        assert_eq!(screens(&mut app), 0);
    }
}

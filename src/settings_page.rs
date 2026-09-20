//! The settings page: one row per setting, and a way back.
//!
//! Edits go straight into the settings resources. Saving is not this module's problem —
//! `settings.rs` writes the file whenever a settings resource changes, however it
//! changed, so a value edited here is on disk the same frame.

use bevy::prelude::*;
use bevy::text::LineBreak;
use bevy::ui_widgets::{SliderValue, ValueChange};

use crate::i18n::{Language, Localized};
use crate::settings::CameraSettings;
use crate::state::AppState;
use crate::ui::{self, UiFont};

/// The pan-speed slider runs in octaves: its value is log2 of the multiplier, so -2 is
/// a quarter speed, 0 is the tuned default and +2 is four times. A linear slider over
/// 0.25..4.0 would put the default a sixth of the way along and spend most of its travel
/// on the fast half; in log space the default sits dead centre and each step is the same
/// proportional change wherever you are.
const OCTAVES_MIN: f32 = -2.0;
const OCTAVES_MAX: f32 = 2.0;

/// One step is about 3.5% — fine enough to feel continuous, coarse enough to land on
/// round-ish numbers.
const OCTAVE_STEP: f32 = 0.05;

pub struct SettingsPagePlugin;

impl Plugin for SettingsPagePlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_slider_change)
            .add_systems(OnEnter(AppState::Settings), spawn_page)
            .add_systems(OnExit(AppState::Settings), despawn_page)
            .add_systems(
                Update,
                (
                    button_actions,
                    choose_language,
                    mark_selected_language,
                    // Only redraws the number when the setting actually changed.
                    update_value_text.run_if(resource_changed::<CameraSettings>),
                )
                    .run_if(in_state(AppState::Settings)),
            );
    }
}

#[derive(Component)]
struct PageRoot;

/// Marks the text showing the pan-speed multiplier, so it can be rewritten in place.
#[derive(Component)]
struct PanSpeedValue;

/// Marks the pan-speed slider, so its value changes can be told apart from any other
/// slider added later.
#[derive(Component)]
struct PanSpeedSlider;

#[derive(Component, Clone, Copy)]
enum PageButton {
    Back,
}

/// A button that selects a language. Carries the language it stands for, so one system
/// handles any number of them.
#[derive(Component, Clone, Copy)]
struct LanguageOption(Language);

fn spawn_page(mut commands: Commands, font: Res<UiFont>, settings: Res<CameraSettings>) {
    commands
        .spawn((PageRoot, ui::overlay()))
        .with_children(|page| {
            page.spawn((
                Text::default(),
                Localized("settings.title"),
                ui::text_font(&font, 28.0),
                TextColor(ui::TEXT),
                Node {
                    margin: UiRect::bottom(Val::Px(8.0)),
                    ..default()
                },
            ));

            ui::spawn_row(page, &font, "settings.pan_speed", |controls| {
                ui::spawn_slider(
                    controls,
                    PanSpeedSlider,
                    settings.pan_speed_scale.log2(),
                    OCTAVES_MIN..=OCTAVES_MAX,
                    OCTAVE_STEP,
                );
                controls.spawn((
                    Text::new(format_scale(settings.pan_speed_scale)),
                    ui::label_font(&font),
                    TextLayout::new(Justify::Left, LineBreak::NoWrap),
                    TextColor(ui::TEXT),
                    PanSpeedValue,
                    // Fixed width so the row doesn't twitch as the number changes.
                    Node {
                        width: Val::Px(72.0),
                        ..default()
                    },
                ));
            });

            // Both languages are shown, each named in its own script and never
            // translated, with the one in force highlighted. A single button showing
            // "the other language" reads as a label rather than a choice.
            ui::spawn_row(page, &font, "settings.language", |controls| {
                for (language, name) in [
                    (Language::English, "English"),
                    (Language::Japanese, "日本語"),
                ] {
                    ui::spawn_literal_button(
                        controls,
                        &font,
                        LanguageOption(language),
                        name,
                        138.0,
                    );
                }
            });

            ui::spawn_button(page, &font, PageButton::Back, "settings.back", 220.0);
        });
}

fn despawn_page(mut commands: Commands, page: Single<Entity, With<PageRoot>>) {
    commands.entity(*page).despawn();
}

fn button_actions(
    buttons: Query<(&Interaction, &PageButton), Changed<Interaction>>,
    mut next: ResMut<NextState<AppState>>,
) {
    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action {
            PageButton::Back => next.set(AppState::Menu),
        }
    }
}

/// The slider reports where it was dragged to; storing that is the app's job, not the
/// widget's. `SliderValue` is an immutable component, so it is replaced rather than
/// edited. Writing `CameraSettings` here is all it takes for the camera to pick the new
/// speed up this frame and for `settings.rs` to write it to disk.
fn on_slider_change(
    change: On<ValueChange<f32>>,
    sliders: Query<(), With<PanSpeedSlider>>,
    mut commands: Commands,
    mut settings: ResMut<CameraSettings>,
) {
    if !sliders.contains(change.source) {
        return;
    }
    commands
        .entity(change.source)
        .insert(SliderValue(change.value));
    settings.pan_speed_scale = change.value.exp2();
}

/// Every label on screen is a `Localized` key, so setting the language retranslates the
/// page in place — no rebuild. `settings.rs` notices the change and saves it.
fn choose_language(
    buttons: Query<(&Interaction, &LanguageOption), Changed<Interaction>>,
    mut language: ResMut<Language>,
) {
    for (interaction, option) in &buttons {
        if *interaction == Interaction::Pressed && *language != option.0 {
            *language = option.0;
        }
    }
}

/// Marks whichever language button matches the language in force. Runs when the page
/// appears and whenever the language changes.
fn mark_selected_language(
    mut commands: Commands,
    language: Res<Language>,
    buttons: Query<(Entity, &LanguageOption)>,
) {
    for (entity, option) in &buttons {
        if option.0 == *language {
            commands.entity(entity).insert(ui::ButtonSelected);
        } else {
            commands.entity(entity).remove::<ui::ButtonSelected>();
        }
    }
}

fn update_value_text(
    settings: Res<CameraSettings>,
    mut value: Single<&mut Text, With<PanSpeedValue>>,
) {
    ***value = format_scale(settings.pan_speed_scale);
}

/// Two decimals. The node it goes in has a fixed width, so the row doesn't shuffle as
/// the number changes.
fn format_scale(scale: f32) -> String {
    format!("{scale:.2}x")
}

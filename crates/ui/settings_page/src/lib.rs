//! The settings page: one row per setting, and a way back.
//!
//! Edits go straight into the settings resources their modules own: `Language` and
//! `CameraSettings`. Saving is not this page's concern; `murabito_settings` writes the
//! file whenever a persisted resource changes, however it changed, so a value edited
//! here is on disk the same frame. Built when `Overlay::Settings` is entered and torn
//! down when it is left, however it is left; Back asks for `Overlay::Menu` and knows
//! nothing about the crate that answers.

use bevy::prelude::*;
use bevy::ui_widgets::{SliderValue, ValueChange};
use murabito_camera::CameraSettings;
use murabito_i18n::Language;
use murabito_navigation::Overlay;
use murabito_ui::{
    ButtonSelected, UiFont, overlay, spawn_button, spawn_heading, spawn_literal_button,
    spawn_readout, spawn_row, spawn_slider,
};

/// The pan-speed multiplier the slider covers: a quarter of the designed speed to six
/// times it.
const SCALE_MIN: f32 = 0.25;
const SCALE_MAX: f32 = 6.0;

/// The slider runs in octaves: its value is log2 of the multiplier. A linear slider
/// over the range would spend most of its travel on the fast half; in log space each
/// step is the same proportional change wherever it is. One step is about 3.5%: fine
/// enough to feel continuous, coarse enough to land on round-ish numbers.
const OCTAVE_STEP: f32 = 0.05;

const BUTTON_WIDTH: f32 = 220.0;
const LANGUAGE_BUTTON_WIDTH: f32 = 138.0;
/// Wide enough for "10.00x", so the row doesn't shuffle as the number changes.
const READOUT_WIDTH: f32 = 72.0;

pub struct SettingsPagePlugin;

impl Plugin for SettingsPagePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(Overlay::Settings), spawn_page)
            .add_systems(OnExit(Overlay::Settings), despawn_page)
            .add_systems(
                Update,
                (
                    press,
                    choose_language,
                    mark_selected_language,
                    update_readout.run_if(resource_changed::<CameraSettings>),
                )
                    .run_if(in_state(Overlay::Settings)),
            );
    }
}

#[derive(Component)]
struct PageRoot;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum PageButton {
    Back,
}

/// A button that selects a language. Carries the language it stands for, so one system
/// handles any number of them.
#[derive(Component, Clone, Copy)]
struct LanguageOption(Language);

/// Marks the pan-speed slider, so its value changes are told apart from any other
/// slider added later.
#[derive(Component)]
struct PanSpeedSlider;

/// Marks the text showing the pan-speed multiplier, so it can be rewritten in place.
#[derive(Component)]
struct PanSpeedReadout;

fn spawn_page(mut commands: Commands, font: Res<UiFont>, camera: Res<CameraSettings>) {
    commands.spawn((PageRoot, overlay())).with_children(|page| {
        spawn_heading(page, &font, "settings.title");

        spawn_row(page, &font, "settings.pan_speed", |controls| {
            let slider = spawn_slider(
                controls,
                PanSpeedSlider,
                octaves_of(camera.pan_speed_scale),
                octaves_of(SCALE_MIN)..=octaves_of(SCALE_MAX),
                OCTAVE_STEP,
            );
            // The observer rides on the slider entity, as Bevy's own examples do: it is
            // then scoped to this one slider and needs no marker to check.
            controls.commands().entity(slider).observe(on_slider_change);
            let readout = spawn_readout(
                controls,
                &font,
                format_scale(camera.pan_speed_scale),
                READOUT_WIDTH,
            );
            controls.commands().entity(readout).insert(PanSpeedReadout);
        });

        // Every language is shown, each named in its own script and never
        // translated, with the one in force marked. A single button naming "the
        // other language" reads as a label rather than a choice.
        spawn_row(page, &font, "settings.language", |controls| {
            for language in Language::ALL {
                spawn_literal_button(
                    controls,
                    &font,
                    LanguageOption(language),
                    language.own_name(),
                    LANGUAGE_BUTTON_WIDTH,
                );
            }
        });

        spawn_button(page, &font, PageButton::Back, "settings.back", BUTTON_WIDTH);
    });
}

fn despawn_page(mut commands: Commands, roots: Query<Entity, With<PageRoot>>) {
    for root in &roots {
        commands.entity(root).despawn();
    }
}

fn press(
    buttons: Query<(&Interaction, &PageButton), Changed<Interaction>>,
    mut next_overlay: ResMut<NextState<Overlay>>,
) {
    for (interaction, button) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match button {
            PageButton::Back => next_overlay.set(Overlay::Menu),
        }
    }
}

/// The slider reports where it was dragged to; storing that is the page's job, not the
/// widget's. `SliderValue` is an immutable component, so it is replaced rather than
/// edited. Writing `CameraSettings` is all it takes for the camera to pick the new speed
/// up this frame and for the settings file to be written.
fn on_slider_change(
    change: On<ValueChange<f32>>,
    mut commands: Commands,
    mut camera: ResMut<CameraSettings>,
) {
    commands
        .entity(change.source)
        .insert(SliderValue(change.value));
    camera.pan_speed_scale = scale_of(change.value);
}

/// Every label on screen is a `Localized` key, so setting the language retranslates the
/// page in place. Only written when it differs, so the file isn't rewritten for nothing.
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

/// Marks whichever language button matches the language in force. Runs every frame the
/// page is up: there are two buttons, and this way it needs no first-frame special case.
fn mark_selected_language(
    mut commands: Commands,
    language: Res<Language>,
    buttons: Query<(Entity, &LanguageOption, Has<ButtonSelected>)>,
) {
    for (entity, option, marked) in &buttons {
        let in_force = option.0 == *language;
        if in_force && !marked {
            commands.entity(entity).insert(ButtonSelected);
        } else if !in_force && marked {
            commands.entity(entity).remove::<ButtonSelected>();
        }
    }
}

fn update_readout(
    camera: Res<CameraSettings>,
    mut readouts: Query<&mut Text, With<PanSpeedReadout>>,
) {
    for mut text in &mut readouts {
        text.0 = format_scale(camera.pan_speed_scale);
    }
}

fn octaves_of(scale: f32) -> f32 {
    scale.log2()
}

fn scale_of(octaves: f32) -> f32 {
    octaves.exp2()
}

/// Two decimals, as "1.00x".
fn format_scale(scale: f32) -> String {
    format!("{scale:.2}x")
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::state::app::StatesPlugin;
    use murabito_app_state::{AppState, AppStatePlugin};
    use murabito_i18n::I18nPlugin;
    use murabito_navigation::NavigationPlugin;
    use murabito_ui::UiPlugin;

    /// Headless, with everything the page builds on, opened straight onto the page.
    fn app_on_the_page() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            StatesPlugin,
            bevy::input::InputPlugin,
            AssetPlugin::default(),
        ))
        .init_asset::<Font>()
        .init_resource::<CameraSettings>()
        .add_plugins((
            AppStatePlugin,
            NavigationPlugin,
            I18nPlugin,
            UiPlugin,
            SettingsPagePlugin,
        ));
        app.update();
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Paused);
        app.world_mut()
            .resource_mut::<NextState<Overlay>>()
            .set(Overlay::Settings);
        app.update();
        app.update();
        app
    }

    fn texts(app: &mut App) -> Vec<String> {
        let world = app.world_mut();
        world
            .query::<&Text>()
            .iter(world)
            .map(|t| t.0.clone())
            .collect()
    }

    fn language_button(app: &mut App, language: Language) -> Entity {
        let world = app.world_mut();
        world
            .query::<(Entity, &LanguageOption)>()
            .iter(world)
            .find(|(_, option)| option.0 == language)
            .map(|(e, _)| e)
            .expect("that language's button")
    }

    fn slider(app: &mut App) -> Entity {
        let world = app.world_mut();
        world
            .query_filtered::<Entity, With<PanSpeedSlider>>()
            .single(world)
            .expect("the pan-speed slider")
    }

    fn readout(app: &mut App) -> String {
        let world = app.world_mut();
        world
            .query_filtered::<&Text, With<PanSpeedReadout>>()
            .single(world)
            .expect("the readout")
            .0
            .clone()
    }

    fn press(app: &mut App, entity: Entity) {
        app.world_mut()
            .entity_mut(entity)
            .insert(Interaction::Pressed);
        app.update();
        app.update();
    }

    #[test]
    fn the_page_shows_its_heading_rows_and_back_in_the_language_in_force() {
        let mut app = app_on_the_page();

        let shown = texts(&mut app);

        for expected in [
            "Settings",
            "Pan speed",
            "1.00x",
            "Language",
            "English",
            "日本語",
            "Back",
        ] {
            assert!(
                shown.contains(&expected.to_string()),
                "{expected} missing from {shown:?}"
            );
        }
    }

    #[test]
    fn the_language_in_force_is_marked_and_the_other_is_not() {
        let mut app = app_on_the_page();
        let english = language_button(&mut app, Language::English);
        let japanese = language_button(&mut app, Language::Japanese);

        assert!(app.world().get::<ButtonSelected>(english).is_some());
        assert!(app.world().get::<ButtonSelected>(japanese).is_none());
    }

    #[test]
    fn pressing_a_language_makes_it_the_one_in_force_and_relabels_the_page() {
        let mut app = app_on_the_page();
        let japanese = language_button(&mut app, Language::Japanese);

        press(&mut app, japanese);

        assert_eq!(*app.world().resource::<Language>(), Language::Japanese);
        assert!(app.world().get::<ButtonSelected>(japanese).is_some());
        let english = language_button(&mut app, Language::English);
        assert!(app.world().get::<ButtonSelected>(english).is_none());
        let shown = texts(&mut app);
        assert!(shown.contains(&"設定".to_string()), "{shown:?}");
        assert!(
            shown.contains(&"English".to_string()),
            "own names are never translated"
        );
    }

    #[test]
    fn the_slider_starts_at_the_current_multiplier_in_octaves() {
        let mut app = app_on_the_page();
        let slider = slider(&mut app);

        let value = app.world().get::<SliderValue>(slider).expect("a value").0;

        assert_eq!(value, 0.0, "1.0x is 0 octaves");
    }

    #[test]
    fn dragging_the_slider_sets_the_camera_speed_and_the_readout_follows() {
        let mut app = app_on_the_page();
        let slider = slider(&mut app);

        // `1.0_f32`, or the literal infers as f64 and raises an event nobody observes.
        app.world_mut().trigger(ValueChange {
            source: slider,
            value: 1.0_f32,
            is_final: false,
        });
        app.update();

        assert_eq!(
            app.world().resource::<CameraSettings>().pan_speed_scale,
            2.0
        );
        assert_eq!(
            app.world().get::<SliderValue>(slider).map(|v| v.0),
            Some(1.0)
        );
        assert_eq!(readout(&mut app), "2.00x");
    }

    #[test]
    fn the_slider_runs_from_a_quarter_speed_to_six_times() {
        assert_eq!(scale_of(octaves_of(SCALE_MIN)), SCALE_MIN);
        assert!((scale_of(octaves_of(SCALE_MAX)) - SCALE_MAX).abs() < 1e-5);
        assert_eq!(octaves_of(1.0), 0.0);
    }

    #[test]
    fn back_asks_for_the_menu_and_the_page_is_gone() {
        let mut app = app_on_the_page();
        let back = {
            let world = app.world_mut();
            world
                .query_filtered::<Entity, With<PageButton>>()
                .single(world)
                .expect("the back button")
        };

        press(&mut app, back);

        let overlay = *app.world().resource::<State<Overlay>>().get();
        assert_eq!(overlay, Overlay::Menu);
        let world = app.world_mut();
        assert_eq!(world.query::<&PageRoot>().iter(world).count(), 0);
    }

    #[test]
    fn the_page_is_torn_down_when_the_world_resumes_under_it() {
        let mut app = app_on_the_page();

        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Playing);
        app.update();

        let world = app.world_mut();
        assert_eq!(world.query::<&PageRoot>().iter(world).count(), 0);
        assert_eq!(world.query::<&Text>().iter(world).count(), 0);
    }
}

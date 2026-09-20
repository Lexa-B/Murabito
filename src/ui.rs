//! Shared look and behaviour for the overlay screens.

use bevy::prelude::*;
use bevy::text::LineBreak;
use bevy::ui_widgets::{Slider, SliderRange, SliderStep, SliderThumb, SliderValue, TrackClick};

use crate::i18n::Localized;
use crate::state::AppState;

/// Dim behind a screen, rather than an opaque panel, so the world stays visible.
pub const SCRIM: Color = Color::srgba(0.05, 0.06, 0.09, 0.75);

/// Button fills: resting, hovered, and held down.
const BUTTON: Color = Color::srgb(0.22, 0.25, 0.31);
const BUTTON_HOVERED: Color = Color::srgb(0.30, 0.34, 0.42);
const BUTTON_PRESSED: Color = Color::srgb(0.16, 0.18, 0.23);

/// A button standing for the option currently in force, e.g. the active language.
const BUTTON_SELECTED: Color = Color::srgb(0.30, 0.45, 0.38);
const BUTTON_SELECTED_HOVERED: Color = Color::srgb(0.36, 0.53, 0.45);

pub const TEXT: Color = Color::srgb(0.92, 0.93, 0.95);

/// Slider track and thumb.
const TRACK: Color = Color::srgb(0.14, 0.16, 0.20);
const THUMB: Color = Color::srgb(0.62, 0.68, 0.78);

/// Track size, and the square thumb that runs along it.
const TRACK_WIDTH: f32 = 180.0;
const TRACK_HEIGHT: f32 = 10.0;
const THUMB_SIZE: f32 = 18.0;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        // Before `Startup`, so no screen can be built before the handle exists.
        app.add_systems(PreStartup, load_font);
        // Every overlay screen shares the hover/press tinting, so it runs whenever one
        // is up rather than being repeated per screen.
        app.add_systems(
            Update,
            (button_visuals, position_thumbs).run_if(not(in_state(AppState::Playing))),
        );
    }
}

/// The UI's font. Noto Sans JP covers Latin as well as Japanese, so one font serves
/// both catalogues and the text doesn't change shape when the language does.
///
/// Bevy's built-in font has no CJK glyphs at all: without this, every Japanese string
/// renders as blank boxes.
#[derive(Resource)]
pub struct UiFont(pub Handle<Font>);

fn load_font(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(UiFont(assets.load("fonts/NotoSansJP-Regular.otf")));
}

/// A full-screen dimmed column: the frame every overlay screen is built in.
pub fn overlay() -> impl Bundle {
    (
        // Bevy's UI layout is flexbox, so this is the same vocabulary as CSS.
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
    )
}

/// Marks the button whose option is currently in force, so it is tinted as chosen
/// rather than looking like one of several equal choices.
#[derive(Component)]
pub struct ButtonSelected;

/// A settings row: a label in a fixed-width column, then its controls. The widths are
/// shared by every row, so labels and controls line up down the page whatever language
/// they are in — Japanese labels are wider than English ones at the same font size.
pub const ROW_LABEL_WIDTH: f32 = 230.0;
pub const ROW_CONTROL_WIDTH: f32 = 290.0;

pub fn spawn_row(
    parent: &mut ChildSpawnerCommands,
    font: &UiFont,
    label_key: &'static str,
    controls: impl FnOnce(&mut ChildSpawnerCommands),
) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                Text::default(),
                Localized::new(label_key),
                label_font(font),
                TextLayout::new(Justify::Left, LineBreak::NoWrap),
                TextColor(TEXT),
                Node {
                    width: Val::Px(ROW_LABEL_WIDTH),
                    ..default()
                },
            ));
            row.spawn(Node {
                width: Val::Px(ROW_CONTROL_WIDTH),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(10.0),
                ..default()
            })
            .with_children(controls);
        });
}

/// A button whose label is given outright rather than translated. For text that is the
/// same in every language — a language's own name, a number, a proper noun.
pub fn spawn_literal_button<C: Component>(
    parent: &mut ChildSpawnerCommands,
    font: &UiFont,
    action: C,
    label: &str,
    width: f32,
) -> Entity {
    parent
        .spawn((Button, action, button_node(width), BackgroundColor(BUTTON)))
        .with_child((
            Text::new(label),
            TextLayout::new(Justify::Center, LineBreak::NoWrap),
            label_font(font),
            TextColor(TEXT),
        ))
        .id()
}

fn button_node(width: f32) -> Node {
    Node {
        width: Val::Px(width),
        height: Val::Px(48.0),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        // A field of `Node` in Bevy 0.19, not a component of its own.
        border_radius: BorderRadius::all(Val::Px(6.0)),
        ..default()
    }
}

/// Spawns a button carrying `action`, whatever that component happens to be, labelled
/// with the catalogue string for `key`. Each screen defines its own action enum and this
/// stays agnostic about them.
///
/// The label starts empty and is filled by `i18n` in the same frame, before anything is
/// drawn — which is also what lets the language be switched without rebuilding a screen.
pub fn spawn_button<C: Component>(
    parent: &mut ChildSpawnerCommands,
    font: &UiFont,
    action: C,
    key: &'static str,
    width: f32,
) {
    parent
        .spawn((Button, action, button_node(width), BackgroundColor(BUTTON)))
        .with_child((
            Text::default(),
            Localized::new(key),
            TextLayout::new(Justify::Center, LineBreak::NoWrap),
            label_font(font),
            TextColor(TEXT),
        ));
}

/// Spawns a slider: a track with a thumb on it, carrying `marker` so the page can find
/// it again. The widget crate handles clicking and dragging; everything drawn here and
/// the thumb's position (see `position_thumbs`) are ours.
pub fn spawn_slider<C: Component>(
    parent: &mut ChildSpawnerCommands,
    marker: C,
    value: f32,
    range: std::ops::RangeInclusive<f32>,
    step: f32,
) {
    parent
        .spawn((
            Slider {
                // Clicking the track jumps the value there, rather than stepping toward
                // it: for a settings slider, the click is the intended value.
                track_click: TrackClick::Snap,
                ..default()
            },
            SliderValue(value),
            SliderRange::from_range(range),
            SliderStep(step),
            marker,
            Node {
                width: Val::Px(TRACK_WIDTH),
                height: Val::Px(TRACK_HEIGHT),
                // The thumb is positioned absolutely within the track.
                position_type: PositionType::Relative,
                border_radius: BorderRadius::all(Val::Px(TRACK_HEIGHT / 2.0)),
                ..default()
            },
            BackgroundColor(TRACK),
        ))
        .with_child((
            SliderThumb,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(THUMB_SIZE),
                height: Val::Px(THUMB_SIZE),
                top: Val::Px((TRACK_HEIGHT - THUMB_SIZE) / 2.0),
                left: Val::Px(0.0),
                border_radius: BorderRadius::all(Val::Px(THUMB_SIZE / 2.0)),
                ..default()
            },
            BackgroundColor(THUMB),
        ));
}

/// Places every slider thumb along its track according to the slider's value.
///
/// The widget crate deliberately leaves this to us, and expects the thumb's travel to be
/// the track's width less the thumb's own width — do it any other way and the thumb
/// drifts away from the mouse during a drag.
fn position_thumbs(
    sliders: Query<(&SliderValue, &SliderRange, &ComputedNode, &Children), With<Slider>>,
    mut thumbs: Query<(&mut Node, &ComputedNode), With<SliderThumb>>,
) {
    for (value, range, track, children) in &sliders {
        for child in children {
            let Ok((mut node, thumb)) = thumbs.get_mut(*child) else {
                continue;
            };
            // `ComputedNode` sizes are physical pixels; `Node` positions are logical.
            let track_width = track.size().x * track.inverse_scale_factor();
            let thumb_width = thumb.size().x * thumb.inverse_scale_factor();
            let travel = (track_width - thumb_width).max(0.0);
            node.left = Val::Px(range.thumb_position(value.0) * travel);
        }
    }
}

pub fn label_font(font: &UiFont) -> TextFont {
    text_font(font, 20.0)
}

/// A `TextFont` in the UI's font at the given size.
pub fn text_font(font: &UiFont, size: f32) -> TextFont {
    TextFont {
        font: FontSource::Handle(font.0.clone()),
        font_size: FontSize::Px(size),
        ..default()
    }
}

/// Tints every button from its interaction and whether it is the selected option.
///
/// Runs over all of them rather than only those whose `Interaction` changed: selection
/// changes without any pointer event, and there are a handful of buttons on screen.
fn button_visuals(
    mut buttons: Query<(&Interaction, Option<&ButtonSelected>, &mut BackgroundColor), With<Button>>,
) {
    for (interaction, selected, mut colour) in &mut buttons {
        let next = match (interaction, selected.is_some()) {
            (Interaction::Pressed, _) => BUTTON_PRESSED,
            (Interaction::Hovered, true) => BUTTON_SELECTED_HOVERED,
            (Interaction::Hovered, false) => BUTTON_HOVERED,
            (Interaction::None, true) => BUTTON_SELECTED,
            (Interaction::None, false) => BUTTON,
        };
        if colour.0 != next {
            *colour = BackgroundColor(next);
        }
    }
}

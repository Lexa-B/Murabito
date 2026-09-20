//! Shared look and behaviour for the overlay screens.

use bevy::prelude::*;
use bevy::ui_widgets::{Slider, SliderRange, SliderStep, SliderThumb, SliderValue, TrackClick};

use crate::state::AppState;

/// Dim behind a screen, rather than an opaque panel, so the world stays visible.
pub const SCRIM: Color = Color::srgba(0.05, 0.06, 0.09, 0.75);

/// Button fills: resting, hovered, and held down.
const BUTTON: Color = Color::srgb(0.22, 0.25, 0.31);
const BUTTON_HOVERED: Color = Color::srgb(0.30, 0.34, 0.42);
const BUTTON_PRESSED: Color = Color::srgb(0.16, 0.18, 0.23);

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
        // Every overlay screen shares the hover/press tinting, so it runs whenever one
        // is up rather than being repeated per screen.
        app.add_systems(
            Update,
            (button_visuals, position_thumbs).run_if(not(in_state(AppState::Playing))),
        );
    }
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

/// Spawns a labelled button carrying `action`, whatever that component happens to be.
/// Each screen defines its own action enum and this stays agnostic about them.
pub fn spawn_button<C: Component>(
    parent: &mut ChildSpawnerCommands,
    action: C,
    label: &str,
    width: f32,
) {
    parent
        .spawn((
            Button,
            action,
            Node {
                width: Val::Px(width),
                height: Val::Px(48.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                // A field of `Node` in Bevy 0.19, not a component of its own.
                border_radius: BorderRadius::all(Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(BUTTON),
        ))
        .with_child((Text::new(label), label_font(), TextColor(TEXT)));
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

pub fn label_font() -> TextFont {
    TextFont {
        font_size: FontSize::Px(20.0),
        ..default()
    }
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

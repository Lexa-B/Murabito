//! The look and the parts every overlay is built from: a dimmed frame over the world, a
//! button, a slider, a settings row, headings and readouts, the font, and the tinting
//! that answers the pointer.
//!
//! Widgets are Bevy's headless ones, which handle pressing and (later) dragging and draw
//! nothing; everything visible is here, so every overlay looks like every other. This
//! crate knows no screen and no action: a button carries whatever component the screen
//! that spawned it chose, and the screen reads that back. Its label is a `Localized` key,
//! so no text is ever written into a button as a literal.

use bevy::prelude::*;
use bevy::text::LineBreak;
use bevy::ui_widgets::{Slider, SliderRange, SliderStep, SliderThumb, SliderValue, TrackClick};
use murabito_i18n::Localized;

/// Dim behind an overlay, rather than an opaque panel, so the world stays visible.
pub const SCRIM: Color = Color::srgba(0.05, 0.06, 0.09, 0.75);

/// Button fills: resting, hovered, and held down.
const BUTTON: Color = Color::srgb(0.22, 0.25, 0.31);
const BUTTON_HOVERED: Color = Color::srgb(0.30, 0.34, 0.42);
const BUTTON_PRESSED: Color = Color::srgb(0.16, 0.18, 0.23);

/// A button standing for the option currently in force, such as the active language.
const BUTTON_SELECTED: Color = Color::srgb(0.30, 0.45, 0.38);
const BUTTON_SELECTED_HOVERED: Color = Color::srgb(0.36, 0.53, 0.45);

pub const TEXT: Color = Color::srgb(0.92, 0.93, 0.95);

/// Slider track and thumb.
const TRACK: Color = Color::srgb(0.14, 0.16, 0.20);
const THUMB: Color = Color::srgb(0.62, 0.68, 0.78);

const BUTTON_HEIGHT: f32 = 48.0;
const CORNER_RADIUS: f32 = 6.0;
const LABEL_SIZE: f32 = 20.0;
const HEADING_SIZE: f32 = 28.0;

/// Track size, and the square thumb that runs along it.
const TRACK_WIDTH: f32 = 180.0;
const TRACK_HEIGHT: f32 = 10.0;
const THUMB_SIZE: f32 = 18.0;

/// A settings row: a label in a fixed-width column, then its controls. Shared by every
/// row, so labels and controls line up down the page whatever language they are in;
/// Japanese labels are wider than English ones at the same size.
const ROW_LABEL_WIDTH: f32 = 230.0;
const ROW_CONTROL_WIDTH: f32 = 290.0;

/// Relative to `assets/`.
const FONT: &str = "fonts/NotoSansJP-Regular.otf";

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        // Before Startup, so no overlay can be built before the handle exists.
        app.add_systems(PreStartup, load_font).add_systems(
            Update,
            (
                // Only while there is one to tint or place: while playing there is none.
                button_visuals.run_if(any_with_component::<Button>),
                position_thumbs.run_if(any_with_component::<Slider>),
            ),
        );
    }
}

/// The UI's font. Noto Sans JP covers Latin as well as Japanese, so one font serves both
/// catalogues and text keeps its shape when the language changes. Bevy's built-in font
/// has no Japanese glyphs at all: without this, every Japanese string is a row of boxes.
#[derive(Resource)]
pub struct UiFont(Handle<Font>);

fn load_font(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(UiFont(assets.load(FONT)));
}

/// A full-screen dimmed column: the frame every overlay is built in. Bevy's UI layout is
/// flexbox, so this is the same vocabulary as CSS.
pub fn overlay() -> impl Bundle {
    (
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

/// Marks the button whose option is currently in force, so it is tinted as chosen rather
/// than looking like one of several equal choices.
#[derive(Component)]
pub struct ButtonSelected;

/// Spawns a button carrying `action`, whatever component that is, labelled with the
/// catalogue string for `key`. The label starts empty and `murabito_i18n` fills it the
/// frame it appears, which is also what lets the language change without a rebuild.
pub fn spawn_button<C: Component>(
    parent: &mut ChildSpawnerCommands,
    font: &UiFont,
    action: C,
    key: &'static str,
    width: f32,
) -> Entity {
    parent
        .spawn((Button, action, button_node(width), BackgroundColor(BUTTON)))
        .with_child((Text::default(), Localized::new(key), label(font)))
        .id()
}

/// A button whose label is given outright rather than translated: for text that is the
/// same in every language, such as a language's own name or a number.
pub fn spawn_literal_button<C: Component>(
    parent: &mut ChildSpawnerCommands,
    font: &UiFont,
    action: C,
    text: &str,
    width: f32,
) -> Entity {
    parent
        .spawn((Button, action, button_node(width), BackgroundColor(BUTTON)))
        .with_child((Text::new(text), label(font)))
        .id()
}

fn button_node(width: f32) -> Node {
    Node {
        width: Val::Px(width),
        height: Val::Px(BUTTON_HEIGHT),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        // A field of `Node` in Bevy 0.19, not a component of its own.
        border_radius: BorderRadius::all(Val::Px(CORNER_RADIUS)),
        ..default()
    }
}

/// What every label shares. `NoWrap` because a text node with a fixed width will
/// otherwise line-break onto an invisible whitespace line, doubling its height and
/// pushing the glyphs off centre.
fn label(font: &UiFont) -> impl Bundle {
    (
        text_font(font, LABEL_SIZE),
        TextLayout::new(Justify::Center, LineBreak::NoWrap),
        TextColor(TEXT),
    )
}

/// A heading: the catalogue string for `key`, larger, with room below it.
pub fn spawn_heading(parent: &mut ChildSpawnerCommands, font: &UiFont, key: &'static str) {
    parent.spawn((
        Text::default(),
        Localized::new(key),
        text_font(font, HEADING_SIZE),
        TextColor(TEXT),
        Node {
            margin: UiRect::bottom(Val::Px(8.0)),
            ..default()
        },
    ));
}

/// A number or other literal beside a control, in a column of fixed `width` so the row
/// doesn't shuffle as the text changes. The entity is returned so the screen can find
/// it again and rewrite the `Text`.
pub fn spawn_readout(
    parent: &mut ChildSpawnerCommands,
    font: &UiFont,
    text: String,
    width: f32,
) -> Entity {
    parent
        .spawn((
            Text::new(text),
            text_font(font, LABEL_SIZE),
            TextLayout::new(Justify::Left, LineBreak::NoWrap),
            TextColor(TEXT),
            Node {
                width: Val::Px(width),
                ..default()
            },
        ))
        .id()
}

/// A settings row: the catalogue string for `label_key` in the label column, then
/// whatever `controls` spawns in the control column.
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
                text_font(font, LABEL_SIZE),
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

/// A slider: a track with a thumb on it, carrying `marker` so the screen can tell its
/// value changes from another slider's. The widget handles clicking and dragging and
/// reports a `ValueChange<f32>`; storing that is the screen's job, since the widget
/// never updates its own value. Everything drawn, and the thumb's position, is ours.
pub fn spawn_slider<C: Component>(
    parent: &mut ChildSpawnerCommands,
    marker: C,
    value: f32,
    range: std::ops::RangeInclusive<f32>,
    step: f32,
) -> Entity {
    parent
        .spawn((
            Slider {
                // Clicking the track jumps the value there rather than stepping toward
                // it: for a setting, the click is the intended value.
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
                // The thumb is placed absolutely within the track.
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
        ))
        .id()
}

/// Where a thumb's left edge sits along its track, in logical pixels. The travel is the
/// track's width less the thumb's own: the thumb's centre must be over the pointer at
/// both ends, or it drifts away from the pointer during a drag.
fn thumb_left(track_width: f32, thumb_width: f32, fraction: f32) -> f32 {
    fraction * (track_width - thumb_width).max(0.0)
}

/// Places every slider thumb along its track according to the slider's value.
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
            node.left = Val::Px(thumb_left(
                track_width,
                thumb_width,
                range.thumb_position(value.0),
            ));
        }
    }
}

/// A `TextFont` in the UI's font at the given size, in pixels.
pub fn text_font(font: &UiFont, size: f32) -> TextFont {
    TextFont {
        font: FontSource::Handle(font.0.clone()),
        font_size: FontSize::Px(size),
        ..default()
    }
}

/// The fill a button should show, from what the pointer is doing to it and whether it
/// stands for the option in force.
fn button_colour(interaction: Interaction, selected: bool) -> Color {
    match (interaction, selected) {
        (Interaction::Pressed, _) => BUTTON_PRESSED,
        (Interaction::Hovered, true) => BUTTON_SELECTED_HOVERED,
        (Interaction::Hovered, false) => BUTTON_HOVERED,
        (Interaction::None, true) => BUTTON_SELECTED,
        (Interaction::None, false) => BUTTON,
    }
}

/// Tints every button. Over all of them rather than only those whose `Interaction`
/// changed, because selection changes without any pointer event, and there are only
/// ever a handful of buttons on screen.
fn button_visuals(
    mut buttons: Query<(&Interaction, Has<ButtonSelected>, &mut BackgroundColor), With<Button>>,
) {
    for (interaction, selected, mut fill) in &mut buttons {
        let wanted = button_colour(*interaction, selected);
        if fill.0 != wanted {
            fill.0 = wanted;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use murabito_i18n::I18nPlugin;

    #[derive(Component)]
    struct Press;

    /// Headless: the asset server and a font store are what `load_font` asks for; in the
    /// app they arrive with `DefaultPlugins`. Run once so `PreStartup` has loaded the font.
    fn app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Font>()
            .add_plugins((I18nPlugin, UiPlugin));
        app.update();
        app
    }

    /// One button inside an overlay, the way a screen would spawn it.
    fn spawn_one_button(app: &mut App) -> Entity {
        let world = app.world_mut();
        let font = UiFont(world.resource::<UiFont>().0.clone());
        let mut button = Entity::PLACEHOLDER;
        world.commands().spawn(overlay()).with_children(|overlay| {
            button = spawn_button(overlay, &font, Press, "menu.quit", 200.0);
        });
        world.flush();
        button
    }

    #[test]
    fn the_font_is_loaded_before_startup() {
        let app = app();

        assert!(app.world().contains_resource::<UiFont>());
    }

    #[test]
    fn a_button_carries_its_action_and_a_localised_label_in_the_ui_font() {
        let mut app = app();
        let button = spawn_one_button(&mut app);
        let world = app.world();

        assert!(world.get::<Button>(button).is_some());
        assert!(world.get::<Press>(button).is_some());
        let label = world.get::<Children>(button).expect("a label")[0];
        assert_eq!(
            world.get::<Localized>(label),
            Some(&Localized::new("menu.quit"))
        );
        let font = world.get::<TextFont>(label).expect("a font");
        assert_eq!(
            font.font,
            FontSource::Handle(world.resource::<UiFont>().0.clone())
        );
    }

    #[test]
    fn a_literal_button_carries_its_text_as_given_and_no_key() {
        let mut app = app();
        let world = app.world_mut();
        let font = UiFont(world.resource::<UiFont>().0.clone());
        let mut button = Entity::PLACEHOLDER;
        world.commands().spawn(overlay()).with_children(|overlay| {
            button = spawn_literal_button(overlay, &font, Press, "日本語", 200.0);
        });
        world.flush();

        let label = world.get::<Children>(button).expect("a label")[0];
        assert_eq!(
            world.get::<Text>(label).map(|t| t.0.as_str()),
            Some("日本語")
        );
        assert!(world.get::<Localized>(label).is_none());
    }

    #[test]
    fn a_button_is_tinted_for_the_pointer_and_for_being_the_choice_in_force() {
        assert_eq!(button_colour(Interaction::None, false), BUTTON);
        assert_eq!(button_colour(Interaction::Hovered, false), BUTTON_HOVERED);
        assert_eq!(button_colour(Interaction::Pressed, false), BUTTON_PRESSED);
        assert_eq!(button_colour(Interaction::None, true), BUTTON_SELECTED);
        assert_eq!(
            button_colour(Interaction::Hovered, true),
            BUTTON_SELECTED_HOVERED
        );
        assert_eq!(button_colour(Interaction::Pressed, true), BUTTON_PRESSED);
    }

    #[test]
    fn hovering_a_button_retints_it_on_the_next_frame() {
        let mut app = app();
        let button = spawn_one_button(&mut app);
        app.world_mut()
            .entity_mut(button)
            .insert(Interaction::Hovered);

        app.update();

        let fill = app.world().get::<BackgroundColor>(button).expect("a fill");
        assert_eq!(fill.0, BUTTON_HOVERED);
    }

    #[test]
    fn marking_a_button_as_the_choice_in_force_retints_it_without_any_pointer_event() {
        let mut app = app();
        let button = spawn_one_button(&mut app);
        app.world_mut().entity_mut(button).insert(ButtonSelected);

        app.update();

        let fill = app.world().get::<BackgroundColor>(button).expect("a fill");
        assert_eq!(fill.0, BUTTON_SELECTED);
    }

    #[derive(Component)]
    struct Volume;

    fn spawn_one_slider(app: &mut App) -> Entity {
        let world = app.world_mut();
        let mut slider = Entity::PLACEHOLDER;
        world.commands().spawn(overlay()).with_children(|overlay| {
            slider = spawn_slider(overlay, Volume, 0.5, 0.0..=1.0, 0.1);
        });
        world.flush();
        slider
    }

    #[test]
    fn a_slider_carries_its_marker_its_value_and_range_and_one_thumb() {
        let mut app = app();
        let slider = spawn_one_slider(&mut app);
        let world = app.world();

        assert!(world.get::<Slider>(slider).is_some());
        assert!(world.get::<Volume>(slider).is_some());
        assert_eq!(world.get::<SliderValue>(slider).map(|v| v.0), Some(0.5));
        assert_eq!(world.get::<SliderStep>(slider).map(|s| s.0), Some(0.1));
        let children = world.get::<Children>(slider).expect("a thumb");
        assert_eq!(children.len(), 1);
        assert!(world.get::<SliderThumb>(children[0]).is_some());
    }

    #[test]
    fn the_thumb_travels_the_track_less_its_own_width() {
        assert_eq!(thumb_left(180.0, 18.0, 0.0), 0.0);
        assert_eq!(thumb_left(180.0, 18.0, 1.0), 162.0);
        assert_eq!(thumb_left(180.0, 18.0, 0.5), 81.0);
    }

    #[test]
    fn a_track_narrower_than_its_thumb_pins_the_thumb_at_the_start() {
        assert_eq!(thumb_left(10.0, 18.0, 1.0), 0.0);
    }

    #[test]
    fn placing_thumbs_runs_while_a_slider_is_up() {
        let mut app = app();
        let slider = spawn_one_slider(&mut app);

        app.update();

        // Without layout there is no computed size, so the thumb sits at the start;
        // what is checked is that the system ran over it and left it placed.
        let thumb = app.world().get::<Children>(slider).expect("a thumb")[0];
        let node = app.world().get::<Node>(thumb).expect("a node");
        assert_eq!(node.left, Val::Px(0.0));
    }

    #[test]
    fn a_row_is_a_localised_label_of_fixed_width_and_then_its_controls() {
        let mut app = app();
        let world = app.world_mut();
        let font = UiFont(world.resource::<UiFont>().0.clone());
        let mut control = Entity::PLACEHOLDER;
        world.commands().spawn(overlay()).with_children(|overlay| {
            spawn_row(overlay, &font, "settings.volume", |controls| {
                control = spawn_slider(controls, Volume, 0.5, 0.0..=1.0, 0.1);
            });
        });
        world.flush();

        let world = app.world_mut();
        let (label, node) = world
            .query::<(&Localized, &Node)>()
            .iter(world)
            .next()
            .expect("a label");
        assert_eq!(*label, Localized::new("settings.volume"));
        assert_eq!(node.width, Val::Px(ROW_LABEL_WIDTH));
        let controls = world
            .get::<ChildOf>(control)
            .expect("in the control column")
            .parent();
        assert_eq!(
            world.get::<Node>(controls).map(|n| n.width),
            Some(Val::Px(ROW_CONTROL_WIDTH))
        );
    }

    #[test]
    fn a_readout_holds_its_text_in_a_column_of_the_given_width() {
        let mut app = app();
        let world = app.world_mut();
        let font = UiFont(world.resource::<UiFont>().0.clone());
        let mut readout = Entity::PLACEHOLDER;
        world.commands().spawn(overlay()).with_children(|overlay| {
            readout = spawn_readout(overlay, &font, "1.00x".to_string(), 72.0);
        });
        world.flush();

        assert_eq!(
            world.get::<Text>(readout).map(|t| t.0.as_str()),
            Some("1.00x")
        );
        assert_eq!(
            world.get::<Node>(readout).map(|n| n.width),
            Some(Val::Px(72.0))
        );
        assert!(world.get::<Localized>(readout).is_none());
    }
}

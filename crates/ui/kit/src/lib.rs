//! The look and the parts every overlay is built from: a dimmed frame over the world, a
//! button, the font, and the tinting that answers the pointer.
//!
//! Widgets are Bevy's headless ones, which handle pressing and (later) dragging and draw
//! nothing; everything visible is here, so every overlay looks like every other. This
//! crate knows no screen and no action: a button carries whatever component the screen
//! that spawned it chose, and the screen reads that back. Its label is a `Localized` key,
//! so no text is ever written into a button as a literal.

use bevy::prelude::*;
use bevy::text::LineBreak;
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

const BUTTON_HEIGHT: f32 = 48.0;
const CORNER_RADIUS: f32 = 6.0;
const LABEL_SIZE: f32 = 20.0;

/// Relative to `assets/`.
const FONT: &str = "fonts/NotoSansJP-Regular.otf";

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        // Before Startup, so no overlay can be built before the handle exists.
        app.add_systems(PreStartup, load_font).add_systems(
            Update,
            // Only while there is a button to tint: while playing there is none.
            button_visuals.run_if(any_with_component::<Button>),
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
}

//! One uniform way to bind inputs to an action.
//!
//! An action is bound through a [`Binds`]: up to three keys or mouse buttons, any of
//! which triggers it. This crate owns the type and knows nothing about what the actions
//! are. The module an action belongs to owns its `Binds`, in its own settings.
//!
//! A system asks for [`Inputs`] and hands [`Inputs::held`] to the binds it cares about:
//!
//! ```
//! # use bevy::prelude::*;
//! # use murabito_keybinds::{Binds, Inputs};
//! fn jump(inputs: Inputs) {
//!     let jump = Binds::new([KeyCode::Space]);
//!     if jump.just_pressed(&inputs.held()) {
//!         // ...
//!     }
//! }
//! ```

use std::fmt;
use std::str::FromStr;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use serde::de::IntoDeserializer;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// How many inputs one action can be bound to.
pub const MAX_BINDS: usize = 3;

/// One thing a player can press: a keyboard key or a mouse button.
///
/// In a settings file, and on screen, it is one word: a key by Bevy's name for it
/// (`KeyW`, `ArrowUp`, `Space`), a mouse button as `MouseLeft`, `MouseRight`,
/// `MouseMiddle`, `MouseBack`, `MouseForward`, or `Mouse7` for an extra button.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Bind {
    Key(KeyCode),
    Mouse(MouseButton),
}

const MOUSE_PREFIX: &str = "Mouse";

impl fmt::Display for Bind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // KeyCode's Debug is its variant name, which is also its serde name.
            Self::Key(key) => write!(f, "{key:?}"),
            Self::Mouse(MouseButton::Other(n)) => write!(f, "{MOUSE_PREFIX}{n}"),
            Self::Mouse(button) => write!(f, "{MOUSE_PREFIX}{button:?}"),
        }
    }
}

/// What a bind's word could not be read as.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownBind(pub String);

impl fmt::Display for UnknownBind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "`{}` is not a key or mouse button", self.0)
    }
}

impl std::error::Error for UnknownBind {}

impl FromStr for Bind {
    type Err = UnknownBind;

    fn from_str(word: &str) -> Result<Self, Self::Err> {
        let unknown = || UnknownBind(word.to_string());
        if let Some(button) = word.strip_prefix(MOUSE_PREFIX) {
            let button = match button {
                "Left" => MouseButton::Left,
                "Right" => MouseButton::Right,
                "Middle" => MouseButton::Middle,
                "Back" => MouseButton::Back,
                "Forward" => MouseButton::Forward,
                number => MouseButton::Other(number.parse().map_err(|_| unknown())?),
            };
            return Ok(Self::Mouse(button));
        }
        // Bevy's own serde knows every key's name; hand it the word as if it were a
        // string in a file.
        let key = KeyCode::deserialize(word.into_deserializer())
            .map_err(|_: serde::de::value::Error| unknown())?;
        Ok(Self::Key(key))
    }
}

impl Serialize for Bind {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Bind {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let word = String::deserialize(deserializer)?;
        word.parse().map_err(serde::de::Error::custom)
    }
}

impl From<KeyCode> for Bind {
    fn from(key: KeyCode) -> Self {
        Self::Key(key)
    }
}

impl From<MouseButton> for Bind {
    fn from(button: MouseButton) -> Self {
        Self::Mouse(button)
    }
}

/// Up to three binds, any of which triggers the same action. Each slot holds a bind or
/// is empty, so there is no way to hold a fourth.
///
/// In a settings file it is the list of the set slots, `[KeyW, ArrowUp]`; a fourth
/// entry there is an error, not a silent drop.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Binds([Option<Bind>; MAX_BINDS]);

impl Serialize for Binds {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(self.iter())
    }
}

impl<'de> Deserialize<'de> for Binds {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let listed = Vec::<Bind>::deserialize(deserializer)?;
        if listed.len() > MAX_BINDS {
            return Err(serde::de::Error::custom(format!(
                "an action takes at most {MAX_BINDS} binds, and {} are listed",
                listed.len()
            )));
        }
        let mut slots = [None; MAX_BINDS];
        for (slot, bind) in slots.iter_mut().zip(listed) {
            *slot = Some(bind);
        }
        Ok(Self(slots))
    }
}

impl Binds {
    /// Binds filled from the first slot. More than three fails the build rather than
    /// surprising anyone at runtime. The assertion runs when code is generated, so
    /// `cargo build` and `cargo test` catch it; `cargo check` alone does not.
    pub fn new<B: Into<Bind>, const N: usize>(binds: [B; N]) -> Self {
        const { assert!(N <= MAX_BINDS, "an action takes at most three binds") };
        let mut slots = [None; MAX_BINDS];
        for (slot, bind) in slots.iter_mut().zip(binds) {
            *slot = Some(bind.into());
        }
        Self(slots)
    }

    /// Whether any of these binds is held down.
    pub fn pressed(&self, held: &Held) -> bool {
        self.iter().any(|bind| held.pressed(bind))
    }

    /// Whether any of these binds went down this frame.
    pub fn just_pressed(&self, held: &Held) -> bool {
        self.iter().any(|bind| held.just_pressed(bind))
    }

    /// The binds that are set, skipping empty slots.
    pub fn iter(&self) -> impl Iterator<Item = Bind> {
        self.0.into_iter().flatten()
    }

    /// Every slot in order, empty ones included: what a rebinding screen draws.
    pub fn slots(&self) -> [Option<Bind>; MAX_BINDS] {
        self.0
    }

    /// Rebinds one slot, or clears it with `None`.
    ///
    /// # Panics
    ///
    /// If `slot` is not below [`MAX_BINDS`].
    pub fn set(&mut self, slot: usize, bind: Option<Bind>) {
        self.0[slot] = bind;
    }
}

/// What is held down right now, on the keyboard and the mouse together.
///
/// A plain pair of borrows, so a test can build one from two `ButtonInput`s without an
/// app. In a system, get one from [`Inputs::held`].
#[derive(Clone, Copy)]
pub struct Held<'a> {
    keys: &'a ButtonInput<KeyCode>,
    mouse: &'a ButtonInput<MouseButton>,
}

impl<'a> Held<'a> {
    pub fn new(keys: &'a ButtonInput<KeyCode>, mouse: &'a ButtonInput<MouseButton>) -> Self {
        Self { keys, mouse }
    }

    fn pressed(&self, bind: Bind) -> bool {
        match bind {
            Bind::Key(key) => self.keys.pressed(key),
            Bind::Mouse(button) => self.mouse.pressed(button),
        }
    }

    fn just_pressed(&self, bind: Bind) -> bool {
        match bind {
            Bind::Key(key) => self.keys.just_pressed(key),
            Bind::Mouse(button) => self.mouse.just_pressed(button),
        }
    }
}

/// The system parameter for reading binds: every input device a bind can name, as one
/// parameter. A new kind of device is added here, and no system's signature changes.
#[derive(SystemParam)]
pub struct Inputs<'w> {
    keys: Res<'w, ButtonInput<KeyCode>>,
    mouse: Res<'w, ButtonInput<MouseButton>>,
}

impl Inputs<'_> {
    pub fn held(&self) -> Held<'_> {
        Held::new(&self.keys, &self.mouse)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    use bevy::input::InputPlugin;

    const SIDE_BUTTON: MouseButton = MouseButton::Other(7);

    /// A keyboard and a mouse for a test to press things on.
    #[derive(Default)]
    struct Devices {
        keys: ButtonInput<KeyCode>,
        mouse: ButtonInput<MouseButton>,
    }

    impl Devices {
        fn held(&self) -> Held<'_> {
            Held::new(&self.keys, &self.mouse)
        }
    }

    #[test]
    fn nothing_held_triggers_nothing() {
        let devices = Devices::default();
        let jump = Binds::new([KeyCode::Space]);

        assert!(!jump.pressed(&devices.held()));
    }

    #[test]
    fn any_one_of_the_binds_triggers_the_action() {
        let forward = Binds::new([KeyCode::KeyW, KeyCode::ArrowUp]);
        let mut devices = Devices::default();
        devices.keys.press(KeyCode::ArrowUp);

        assert!(forward.pressed(&devices.held()));
    }

    #[test]
    fn a_key_that_is_not_bound_triggers_nothing() {
        let forward = Binds::new([KeyCode::KeyW]);
        let mut devices = Devices::default();
        devices.keys.press(KeyCode::KeyS);

        assert!(!forward.pressed(&devices.held()));
    }

    #[test]
    fn a_mouse_button_binds_like_a_key_even_an_unusual_one() {
        let ping = Binds::new([SIDE_BUTTON]);
        let mut devices = Devices::default();
        devices.mouse.press(SIDE_BUTTON);

        assert!(ping.pressed(&devices.held()));
    }

    #[test]
    fn keys_and_mouse_buttons_mix_in_one_action() {
        let select = Binds::new([Bind::Key(KeyCode::Enter), Bind::Mouse(MouseButton::Left)]);
        let mut devices = Devices::default();
        devices.mouse.press(MouseButton::Left);

        assert!(select.pressed(&devices.held()));
    }

    #[test]
    fn just_pressed_lasts_one_frame_while_pressed_lasts_as_long_as_the_key_is_down() {
        let jump = Binds::new([KeyCode::Space]);
        let mut devices = Devices::default();
        devices.keys.press(KeyCode::Space);
        assert!(jump.just_pressed(&devices.held()));

        devices.keys.clear();

        assert!(!jump.just_pressed(&devices.held()));
        assert!(jump.pressed(&devices.held()));
    }

    #[test]
    fn binds_fill_from_the_first_slot_and_leave_the_rest_empty() {
        let forward = Binds::new([KeyCode::KeyW, KeyCode::ArrowUp]);

        assert_eq!(
            forward.slots(),
            [
                Some(Bind::Key(KeyCode::KeyW)),
                Some(Bind::Key(KeyCode::ArrowUp)),
                None
            ]
        );
    }

    #[test]
    fn a_slot_can_be_rebound_and_cleared() {
        let mut forward = Binds::new([KeyCode::KeyW, KeyCode::ArrowUp]);

        forward.set(0, Some(SIDE_BUTTON.into()));
        forward.set(1, None);

        assert_eq!(
            forward.iter().collect::<Vec<_>>(),
            [Bind::Mouse(SIDE_BUTTON)]
        );
    }

    #[test]
    fn no_binds_at_all_is_an_action_nothing_triggers() {
        let mut devices = Devices::default();
        devices.keys.press(KeyCode::Space);

        assert!(!Binds::default().pressed(&devices.held()));
    }

    #[test]
    fn a_system_reads_binds_through_the_inputs_parameter() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, InputPlugin));
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(SIDE_BUTTON);

        let triggered = app
            .world_mut()
            .run_system_once(|inputs: Inputs| Binds::new([SIDE_BUTTON]).pressed(&inputs.held()))
            .expect("the system runs");

        assert!(triggered);
    }

    #[test]
    fn a_bind_is_one_word() {
        assert_eq!(Bind::Key(KeyCode::KeyW).to_string(), "KeyW");
        assert_eq!(Bind::Key(KeyCode::ArrowUp).to_string(), "ArrowUp");
        assert_eq!(Bind::Mouse(MouseButton::Left).to_string(), "MouseLeft");
        assert_eq!(Bind::Mouse(SIDE_BUTTON).to_string(), "Mouse7");
    }

    #[test]
    fn every_kind_of_bind_reads_back_from_its_word() {
        for bind in [
            Bind::Key(KeyCode::Space),
            Bind::Mouse(MouseButton::Forward),
            Bind::Mouse(SIDE_BUTTON),
        ] {
            assert_eq!(bind.to_string().parse::<Bind>(), Ok(bind));
        }
    }

    #[test]
    fn a_word_that_names_nothing_is_an_error_that_says_so() {
        assert_eq!(
            "KeyQwerty".parse::<Bind>(),
            Err(UnknownBind("KeyQwerty".to_string()))
        );
        assert!("MouseSeven".parse::<Bind>().is_err());
    }

    #[test]
    fn binds_are_a_list_of_the_set_slots_in_a_file() {
        let forward = Binds::new([Bind::Key(KeyCode::KeyW), Bind::Mouse(SIDE_BUTTON)]);

        let text = serde_yaml_ng::to_string(&forward).expect("serialises");

        assert_eq!(text, "- KeyW\n- Mouse7\n");
        assert_eq!(
            serde_yaml_ng::from_str::<Binds>(&text).expect("parses"),
            forward
        );
    }

    #[test]
    fn an_empty_list_is_an_action_nothing_triggers() {
        assert_eq!(
            serde_yaml_ng::from_str::<Binds>("[]").expect("parses"),
            Binds::default()
        );
    }

    #[test]
    fn a_fourth_bind_in_a_file_is_refused() {
        let error =
            serde_yaml_ng::from_str::<Binds>("[KeyA, KeyB, KeyC, KeyD]").expect_err("four binds");

        assert!(error.to_string().contains("at most 3"), "{error}");
    }
}

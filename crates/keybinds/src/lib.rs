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

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

/// How many inputs one action can be bound to.
pub const MAX_BINDS: usize = 3;

/// One thing a player can press: a keyboard key or a mouse button.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Bind {
    Key(KeyCode),
    Mouse(MouseButton),
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
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Binds([Option<Bind>; MAX_BINDS]);

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
}

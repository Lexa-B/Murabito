//! The overhead camera.
//!
//! Lengths are in shaku: one world unit is one 尺, about 30.3 cm.

use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use murabito_keybinds::{Binds, Held, Inputs};

/// How far the camera starts from its focus: 7 間 (42 shaku, about 12.7 m).
const START_ZOOM: f32 = 42.0;

/// How fast the focus moves across the ground while a pan key is held, at
/// `PAN_REF_ZOOM`: 45 shaku per second, about 7.5 間.
const PAN_SPEED: f32 = 45.0;

/// The zoom distance at which the pan runs at exactly `PAN_SPEED`: the starting zoom.
const PAN_REF_ZOOM: f32 = START_ZOOM;

/// How strongly pan speed follows zoom distance. 0.0 pans at a fixed speed in shaku
/// (fine zoomed out, sluggish up close); 1.0 pans at a fixed speed in screen-widths
/// (fine up close, frantic zoomed out). 0.75 sits three-quarters of the way toward
/// proportional: zooming out 4x speeds the pan up about 2.8x rather than 4x.
const PAN_ZOOM_EXPONENT: f32 = 0.75;

/// Closest and furthest the camera may sit from its focus, in shaku: about 4 m to 60 m.
const ZOOM_MIN: f32 = 13.0;
const ZOOM_MAX: f32 = 198.0;

/// How much one wheel notch changes the distance. A ratio, not a length, so a notch
/// changes the view by the same proportion however far out the camera is.
const ZOOM_STEP: f32 = 1.15;

/// The widest range the player's pan-speed multiplier is allowed to have an effect
/// over. Anything outside it, or not a number at all, is pulled back in where it is
/// used, so a hand-edited file can't reverse the pan or send it to infinity.
const PAN_SPEED_SCALE_MIN: f32 = 0.1;
const PAN_SPEED_SCALE_MAX: f32 = 10.0;

/// How the player has asked the camera to feel. Owned here, and edited by whoever
/// depends on this crate: a settings screen, a saved file. The camera only reads it.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct CameraSettings {
    /// Multiplies pan speed. 1.0 is the designed speed.
    pub pan_speed_scale: f32,
    pub pan_forward: Binds,
    pub pan_back: Binds,
    pub pan_left: Binds,
    pub pan_right: Binds,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            pan_speed_scale: 1.0,
            pan_forward: Binds::new([KeyCode::KeyW, KeyCode::ArrowUp]),
            pan_back: Binds::new([KeyCode::KeyS, KeyCode::ArrowDown]),
            pan_left: Binds::new([KeyCode::KeyA, KeyCode::ArrowLeft]),
            pan_right: Binds::new([KeyCode::KeyD, KeyCode::ArrowRight]),
        }
    }
}

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraSettings>()
            .add_systems(Startup, spawn_camera)
            .add_systems(Update, (pan_camera, zoom_camera, apply_rig).chain());
    }
}

/// Where the camera is, said the way a player thinks of it: what it looks at, from
/// which side, and from how far. `apply_rig` turns this into a `Transform`, and is the
/// only thing that writes one, so nothing else can fight over where the camera is.
#[derive(Component)]
struct CameraRig {
    /// The point on the ground the camera looks at.
    focus: Vec3,
    /// Which way the camera sits from the focus. A unit vector: the distance is `zoom`.
    direction: Vec3,
    /// How far from the focus the camera sits, in shaku.
    zoom: f32,
}

impl Default for CameraRig {
    fn default() -> Self {
        Self {
            focus: Vec3::ZERO,
            // Up and back in equal parts: 45 degrees above the ground.
            direction: Vec3::new(0.0, 1.0, 1.0).normalize(),
            zoom: START_ZOOM,
        }
    }
}

impl CameraRig {
    fn eye(&self) -> Vec3 {
        self.focus + self.direction * self.zoom
    }

    fn transform(&self) -> Transform {
        Transform::from_translation(self.eye()).looking_at(self.focus, Vec3::Y)
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((Camera3d::default(), CameraRig::default()));
}

fn pan_camera(
    inputs: Inputs,
    settings: Res<CameraSettings>,
    time: Res<Time>,
    mut rigs: Query<&mut CameraRig>,
) {
    let direction = pan_direction(&inputs.held(), &settings);
    for mut rig in &mut rigs {
        let speed = pan_speed(rig.zoom, settings.pan_speed_scale);
        rig.focus += direction * speed * time.delta_secs();
    }
}

/// Top pan speed at a zoom distance, in shaku per second, with the player's multiplier.
fn pan_speed(zoom: f32, scale: f32) -> f32 {
    let scale = if scale.is_nan() { 1.0 } else { scale };
    let scale = scale.clamp(PAN_SPEED_SCALE_MIN, PAN_SPEED_SCALE_MAX);
    PAN_SPEED * scale * (zoom / PAN_REF_ZOOM).powf(PAN_ZOOM_EXPONENT)
}

/// Which way the held binds ask to go across the ground, as a unit vector, or zero when
/// none are held or they cancel out. Forward is away from the camera: -Z.
fn pan_direction(held: &Held, settings: &CameraSettings) -> Vec3 {
    let mut direction = Vec3::ZERO;
    if settings.pan_forward.pressed(held) {
        direction -= Vec3::Z;
    }
    if settings.pan_back.pressed(held) {
        direction += Vec3::Z;
    }
    if settings.pan_left.pressed(held) {
        direction -= Vec3::X;
    }
    if settings.pan_right.pressed(held) {
        direction += Vec3::X;
    }
    direction.normalize_or_zero()
}

fn zoom_camera(mut wheel: MessageReader<MouseWheel>, mut rigs: Query<&mut CameraRig>) {
    let notches: f32 = wheel.read().map(notches_of).sum();
    if notches == 0.0 {
        return;
    }
    for mut rig in &mut rigs {
        rig.zoom = zoomed(rig.zoom, notches);
    }
}

/// One wheel message as a count of notches. A mouse reports whole lines, one per notch;
/// a touchpad reports pixels, many per notch, so those are scaled down to match.
fn notches_of(scroll: &MouseWheel) -> f32 {
    match scroll.unit {
        MouseScrollUnit::Line => scroll.y,
        MouseScrollUnit::Pixel => scroll.y / MouseScrollUnit::SCROLL_UNIT_CONVERSION_FACTOR,
    }
}

/// The zoom distance after scrolling by `notches`. Scrolling up is positive and moves
/// the camera closer, hence the negated exponent.
fn zoomed(zoom: f32, notches: f32) -> f32 {
    (zoom * ZOOM_STEP.powf(-notches)).clamp(ZOOM_MIN, ZOOM_MAX)
}

fn apply_rig(mut cameras: Query<(&CameraRig, &mut Transform)>) {
    for (rig, mut transform) in &mut cameras {
        *transform = rig.transform();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::input::InputPlugin;

    /// A headless app: no window, no GPU. Spawning a camera needs neither; drawing
    /// through one does, and no test here draws. `InputPlugin` is the keyboard the pan
    /// system asks for, with no keys held; in the real app it arrives with
    /// `DefaultPlugins`.
    fn headless_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, InputPlugin, CameraPlugin));
        app
    }

    fn camera_after_startup() -> Transform {
        let mut app = headless_app();
        app.update();

        let world = app.world_mut();
        *world
            .query_filtered::<&Transform, With<Camera3d>>()
            .single(world)
            .expect("exactly one camera")
    }

    #[test]
    fn the_camera_looks_at_the_origin() {
        let camera = camera_after_startup();

        let toward_origin = (Vec3::ZERO - camera.translation).normalize();

        assert!(camera.forward().abs_diff_eq(toward_origin, 1e-5));
    }

    #[test]
    fn the_camera_looks_down_at_45_degrees() {
        let camera = camera_after_startup();

        let degrees_below_horizontal = (-camera.forward().y).asin().to_degrees();

        assert!(
            (degrees_below_horizontal - 45.0).abs() < 0.01,
            "the camera looks down at {degrees_below_horizontal} degrees"
        );
    }

    #[test]
    fn the_camera_sits_seven_ken_from_the_origin() {
        let camera = camera_after_startup();

        let shaku = camera.translation.length();

        assert!((shaku - 42.0).abs() < 0.01, "it sits {shaku} shaku away");
    }

    /// A rig somewhere other than the default, so a test can't pass by coincidence
    /// with the origin or the starting zoom.
    fn rig_off_centre() -> CameraRig {
        CameraRig {
            focus: Vec3::new(10.0, 0.0, -5.0),
            zoom: 20.0,
            ..default()
        }
    }

    #[test]
    fn the_eye_is_zoom_shaku_from_the_focus() {
        let rig = rig_off_centre();

        let shaku = rig.eye().distance(rig.focus);

        assert!(
            (shaku - rig.zoom).abs() < 1e-4,
            "it sits {shaku} shaku away"
        );
    }

    #[test]
    fn the_rig_looks_at_its_focus() {
        let rig = rig_off_centre();
        let transform = rig.transform();

        let toward_focus = (rig.focus - transform.translation).normalize();

        assert!(transform.forward().abs_diff_eq(toward_focus, 1e-5));
    }

    #[test]
    fn the_camera_follows_its_rig() {
        let mut app = headless_app();
        app.update();
        let world = app.world_mut();
        let mut rigs = world.query::<&mut CameraRig>();
        *rigs.single_mut(world).expect("exactly one rig") = rig_off_centre();

        app.update();

        let world = app.world_mut();
        let camera = *world
            .query_filtered::<&Transform, With<Camera3d>>()
            .single(world)
            .expect("exactly one camera");
        assert!(camera.translation.abs_diff_eq(rig_off_centre().eye(), 1e-4));
    }

    /// Which way the camera is asked to pan with exactly these held, under `settings`.
    fn direction_with(
        keys: &[KeyCode],
        buttons: &[MouseButton],
        settings: &CameraSettings,
    ) -> Vec3 {
        let mut keyboard = ButtonInput::default();
        let mut mouse = ButtonInput::default();
        keys.iter().for_each(|key| keyboard.press(*key));
        buttons.iter().for_each(|button| mouse.press(*button));
        pan_direction(&Held::new(&keyboard, &mouse), settings)
    }

    /// The same, with the default binds: what a new player gets.
    fn direction_holding(keys: &[KeyCode]) -> Vec3 {
        direction_with(keys, &[], &CameraSettings::default())
    }

    #[test]
    fn no_keys_held_pans_nowhere() {
        assert_eq!(direction_holding(&[]), Vec3::ZERO);
    }

    #[test]
    fn w_pans_away_from_the_camera() {
        assert_eq!(direction_holding(&[KeyCode::KeyW]), -Vec3::Z);
    }

    #[test]
    fn the_arrow_keys_pan_as_wasd_does() {
        let pairs = [
            (KeyCode::ArrowUp, KeyCode::KeyW),
            (KeyCode::ArrowDown, KeyCode::KeyS),
            (KeyCode::ArrowLeft, KeyCode::KeyA),
            (KeyCode::ArrowRight, KeyCode::KeyD),
        ];
        for (arrow, letter) in pairs {
            assert_eq!(
                direction_holding(&[arrow]),
                direction_holding(&[letter]),
                "{arrow:?} and {letter:?} disagree"
            );
        }
    }

    #[test]
    fn opposite_keys_cancel_out() {
        let both = direction_holding(&[KeyCode::KeyW, KeyCode::KeyS]);

        assert_eq!(both, Vec3::ZERO);
    }

    #[test]
    fn a_diagonal_pan_is_no_faster_than_a_straight_one() {
        let diagonal = direction_holding(&[KeyCode::KeyW, KeyCode::KeyD]);

        assert!(
            (diagonal.length() - 1.0).abs() < 1e-6,
            "length {}",
            diagonal.length()
        );
    }

    #[test]
    fn a_held_key_pans_at_pan_speed_in_shaku_per_second() {
        use bevy::time::TimeUpdateStrategy;
        use std::time::Duration;

        let mut app = headless_app();
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            100,
        )));
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyD);

        app.update();

        let world = app.world_mut();
        let focus = world
            .query::<&CameraRig>()
            .single(world)
            .expect("exactly one rig")
            .focus;
        let a_tenth_of_a_second = Vec3::X * PAN_SPEED * 0.1;
        assert!(
            focus.abs_diff_eq(a_tenth_of_a_second, 1e-4),
            "focus is at {focus}"
        );
    }

    fn scroll(unit: MouseScrollUnit, y: f32) -> MouseWheel {
        MouseWheel {
            unit,
            x: 0.0,
            y,
            window: Entity::PLACEHOLDER,
            phase: bevy::input::touch::TouchPhase::Moved,
        }
    }

    #[test]
    fn scrolling_up_moves_the_camera_closer() {
        assert!(zoomed(START_ZOOM, 1.0) < START_ZOOM);
    }

    #[test]
    fn a_notch_changes_the_distance_by_the_same_proportion_anywhere() {
        let near = 20.0 / zoomed(20.0, 1.0);
        let far = 100.0 / zoomed(100.0, 1.0);

        assert!(
            (near - ZOOM_STEP).abs() < 1e-4,
            "near, a notch is a factor of {near}"
        );
        assert!(
            (far - ZOOM_STEP).abs() < 1e-4,
            "far, a notch is a factor of {far}"
        );
    }

    #[test]
    fn scrolling_in_and_back_out_returns_to_the_start() {
        let there_and_back = zoomed(zoomed(START_ZOOM, 3.0), -3.0);

        assert!(
            (there_and_back - START_ZOOM).abs() < 1e-3,
            "ended at {there_and_back}"
        );
    }

    #[test]
    fn the_camera_stops_at_its_closest_and_furthest() {
        assert_eq!(zoomed(START_ZOOM, 1000.0), ZOOM_MIN);
        assert_eq!(zoomed(START_ZOOM, -1000.0), ZOOM_MAX);
    }

    #[test]
    fn a_mouse_line_is_one_notch() {
        assert_eq!(notches_of(&scroll(MouseScrollUnit::Line, 1.0)), 1.0);
    }

    #[test]
    fn touchpad_pixels_are_scaled_down_to_notches() {
        let one_notch_of_pixels = MouseScrollUnit::SCROLL_UNIT_CONVERSION_FACTOR;

        assert_eq!(
            notches_of(&scroll(MouseScrollUnit::Pixel, one_notch_of_pixels)),
            1.0
        );
    }

    #[test]
    fn a_wheel_notch_zooms_the_rig() {
        let mut app = headless_app();
        app.update();
        app.world_mut()
            .write_message(scroll(MouseScrollUnit::Line, 1.0));

        app.update();

        let world = app.world_mut();
        let zoom = world
            .query::<&CameraRig>()
            .single(world)
            .expect("exactly one rig")
            .zoom;
        assert!(
            (zoom - START_ZOOM / ZOOM_STEP).abs() < 1e-4,
            "zoom is {zoom}"
        );
    }

    #[test]
    fn at_the_reference_zoom_the_pan_runs_at_pan_speed() {
        assert_eq!(pan_speed(PAN_REF_ZOOM, 1.0), PAN_SPEED);
    }

    #[test]
    fn zooming_out_four_times_pans_about_2_8_times_faster() {
        let ratio = pan_speed(PAN_REF_ZOOM * 4.0, 1.0) / pan_speed(PAN_REF_ZOOM, 1.0);

        assert!(
            (ratio - 2.828).abs() < 0.01,
            "the pan is {ratio} times faster"
        );
    }

    #[test]
    fn the_further_out_the_faster_the_pan() {
        assert!(pan_speed(ZOOM_MIN, 1.0) < pan_speed(PAN_REF_ZOOM, 1.0));
        assert!(pan_speed(PAN_REF_ZOOM, 1.0) < pan_speed(ZOOM_MAX, 1.0));
    }

    const SIDE_BUTTON: MouseButton = MouseButton::Other(7);

    #[test]
    fn a_pan_speed_scale_of_two_doubles_the_pan() {
        assert_eq!(pan_speed(PAN_REF_ZOOM, 2.0), PAN_SPEED * 2.0);
    }

    #[test]
    fn a_scale_no_player_could_mean_is_pulled_back_into_range() {
        let reversed = pan_speed(PAN_REF_ZOOM, -5.0);
        let runaway = pan_speed(PAN_REF_ZOOM, f32::INFINITY);

        assert_eq!(reversed, PAN_SPEED * PAN_SPEED_SCALE_MIN);
        assert_eq!(runaway, PAN_SPEED * PAN_SPEED_SCALE_MAX);
    }

    #[test]
    fn a_scale_that_is_not_a_number_pans_at_the_designed_speed() {
        assert_eq!(pan_speed(PAN_REF_ZOOM, f32::NAN), PAN_SPEED);
    }

    #[test]
    fn a_pan_can_be_rebound_to_a_mouse_button() {
        let settings = CameraSettings {
            pan_forward: Binds::new([SIDE_BUTTON]),
            ..default()
        };

        assert_eq!(direction_with(&[], &[SIDE_BUTTON], &settings), -Vec3::Z);
    }

    #[test]
    fn a_key_that_was_rebound_away_no_longer_pans() {
        let settings = CameraSettings {
            pan_forward: Binds::new([SIDE_BUTTON]),
            ..default()
        };

        assert_eq!(direction_with(&[KeyCode::KeyW], &[], &settings), Vec3::ZERO);
    }

    #[test]
    fn the_camera_starts_with_the_default_settings() {
        let mut app = headless_app();
        app.update();

        assert_eq!(
            *app.world().resource::<CameraSettings>(),
            CameraSettings::default()
        );
    }

    #[test]
    fn settings_already_there_when_the_plugin_is_added_are_kept() {
        let loaded = CameraSettings {
            pan_speed_scale: 2.0,
            ..default()
        };
        let mut app = App::new();
        app.insert_resource(loaded.clone());
        app.add_plugins((MinimalPlugins, InputPlugin, CameraPlugin));
        app.update();

        assert_eq!(*app.world().resource::<CameraSettings>(), loaded);
    }

    #[test]
    fn editing_the_settings_changes_how_the_camera_pans() {
        use bevy::time::TimeUpdateStrategy;
        use std::time::Duration;

        let mut app = headless_app();
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            100,
        )));
        app.update();
        app.world_mut()
            .resource_mut::<CameraSettings>()
            .pan_speed_scale = 2.0;
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyD);

        app.update();

        let world = app.world_mut();
        let focus = world
            .query::<&CameraRig>()
            .single(world)
            .expect("exactly one rig")
            .focus;
        let twice_a_tenth_of_a_second = Vec3::X * PAN_SPEED * 2.0 * 0.1;
        assert!(
            focus.abs_diff_eq(twice_a_tenth_of_a_second, 1e-4),
            "focus is at {focus}"
        );
    }
}

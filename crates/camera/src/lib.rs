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

/// How sharply the pan velocity chases the velocity the binds ask for, per second.
/// Higher is snappier; at 13.0 a pan reaches about 63% of top speed in 0.077 s and 95%
/// in 0.23 s, and coasts to a stop over about the same time when the binds are let go.
/// Not scaled by zoom: the top speed scales instead, so the ramp takes the same time,
/// and so looks the same on screen, however far out the camera is.
const PAN_RESPONSE: f32 = 13.0;

/// Below this fraction of top speed, a pan that is coasting to a stop is stopped. An
/// exponential ease never quite arrives, and would leave the camera creeping forever.
const PAN_REST_FRACTION: f32 = 1e-3;

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
    /// How fast the focus is moving across the ground, in shaku per second. Eased toward
    /// what the binds ask for rather than set from them, so a pan ramps up and coasts.
    velocity: Vec3,
}

impl Default for CameraRig {
    fn default() -> Self {
        Self {
            focus: Vec3::ZERO,
            // Up and back in equal parts: 45 degrees above the ground.
            direction: Vec3::new(0.0, 1.0, 1.0).normalize(),
            zoom: START_ZOOM,
            velocity: Vec3::ZERO,
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
    let dt = time.delta_secs();
    for mut rig in &mut rigs {
        let top_speed = pan_speed(rig.zoom, settings.pan_speed_scale);
        let velocity = eased_velocity(rig.velocity, direction * top_speed, top_speed, dt);
        if velocity == Vec3::ZERO && rig.velocity == Vec3::ZERO {
            continue;
        }
        rig.velocity = velocity;
        rig.focus += velocity * dt;
    }
}

/// The pan velocity one frame later: eased toward `wanted`, and brought to a dead stop
/// once a pan that is coasting down is too slow to see.
fn eased_velocity(current: Vec3, wanted: Vec3, top_speed: f32, dt: f32) -> Vec3 {
    let eased = current.lerp(wanted, ease_fraction(PAN_RESPONSE, dt));
    let coasting_to_rest = wanted == Vec3::ZERO;
    if coasting_to_rest && eased.length() < top_speed * PAN_REST_FRACTION {
        Vec3::ZERO
    } else {
        eased
    }
}

/// How much of the gap to a target to close in `dt` seconds, from 0 (none) to 1 (all).
///
/// `1 - e^(-response * dt)` rather than a fixed fraction per frame, so the ease covers
/// the same ground in the same time at any frame rate: two 8 ms frames close exactly as
/// much of the gap as one 16 ms frame.
fn ease_fraction(response: f32, dt: f32) -> f32 {
    1.0 - (-response * dt).exp()
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

    /// A headless app whose every frame is a tenth of a second, run once so the camera
    /// exists.
    fn app_in_tenths_of_a_second() -> App {
        use bevy::time::TimeUpdateStrategy;
        use std::time::Duration;

        let mut app = headless_app();
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            100,
        )));
        app.update();
        app
    }

    fn hold(app: &mut App, key: KeyCode, frames: usize) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        (0..frames).for_each(|_| app.update());
    }

    fn let_go(app: &mut App, key: KeyCode, frames: usize) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(key);
        (0..frames).for_each(|_| app.update());
    }

    fn pan_velocity(app: &mut App) -> Vec3 {
        let world = app.world_mut();
        world
            .query::<&CameraRig>()
            .single(world)
            .expect("exactly one rig")
            .velocity
    }

    /// Long enough for the ease to have arrived, to well within any tolerance here.
    const A_GOOD_WHILE: usize = 20;

    #[test]
    fn a_held_key_reaches_pan_speed_in_shaku_per_second() {
        let mut app = app_in_tenths_of_a_second();

        hold(&mut app, KeyCode::KeyD, A_GOOD_WHILE);

        let velocity = pan_velocity(&mut app);
        assert!(
            velocity.abs_diff_eq(Vec3::X * PAN_SPEED, 1e-3),
            "panning at {velocity}"
        );
    }

    #[test]
    fn editing_the_settings_changes_how_fast_the_camera_pans() {
        let mut app = app_in_tenths_of_a_second();
        app.world_mut()
            .resource_mut::<CameraSettings>()
            .pan_speed_scale = 2.0;

        hold(&mut app, KeyCode::KeyD, A_GOOD_WHILE);

        let velocity = pan_velocity(&mut app);
        assert!(
            velocity.abs_diff_eq(Vec3::X * PAN_SPEED * 2.0, 1e-3),
            "panning at {velocity}"
        );
    }

    #[test]
    fn a_pan_ramps_up_rather_than_starting_at_full_speed() {
        let mut app = app_in_tenths_of_a_second();

        hold(&mut app, KeyCode::KeyD, 1);

        let speed = pan_velocity(&mut app).x;
        assert!(
            0.0 < speed && speed < PAN_SPEED,
            "after one frame the pan runs at {speed}"
        );
    }

    #[test]
    fn letting_go_coasts_rather_than_stopping_dead() {
        let mut app = app_in_tenths_of_a_second();
        hold(&mut app, KeyCode::KeyD, A_GOOD_WHILE);

        let_go(&mut app, KeyCode::KeyD, 1);

        let speed = pan_velocity(&mut app).x;
        assert!(
            0.0 < speed && speed < PAN_SPEED,
            "a frame after letting go: {speed}"
        );
    }

    #[test]
    fn a_coasting_pan_comes_to_a_dead_stop() {
        let mut app = app_in_tenths_of_a_second();
        hold(&mut app, KeyCode::KeyD, A_GOOD_WHILE);

        let_go(&mut app, KeyCode::KeyD, A_GOOD_WHILE);

        assert_eq!(pan_velocity(&mut app), Vec3::ZERO);
    }

    #[test]
    fn an_ease_closes_none_of_the_gap_in_no_time_and_nearly_all_of_it_in_a_long_time() {
        assert_eq!(ease_fraction(PAN_RESPONSE, 0.0), 0.0);
        assert!(ease_fraction(PAN_RESPONSE, 10.0) > 0.999_999);
    }

    #[test]
    fn two_short_frames_ease_exactly_as_far_as_one_long_one() {
        let one_long = ease_fraction(PAN_RESPONSE, 0.016);
        let left_after_a_short_one = 1.0 - ease_fraction(PAN_RESPONSE, 0.008);
        let two_short = 1.0 - left_after_a_short_one * left_after_a_short_one;

        assert!(
            (one_long - two_short).abs() < 1e-6,
            "{one_long} against {two_short}"
        );
    }

    #[test]
    fn a_pan_too_slow_to_see_is_stopped_only_when_it_is_coasting_down() {
        let barely_moving = Vec3::X * PAN_SPEED * PAN_REST_FRACTION * 0.5;
        let a_frame = 0.016;

        let coasting = eased_velocity(barely_moving, Vec3::ZERO, PAN_SPEED, a_frame);
        let setting_off = eased_velocity(Vec3::ZERO, Vec3::X * PAN_SPEED, PAN_SPEED, 1e-7);

        assert_eq!(coasting, Vec3::ZERO);
        assert!(
            setting_off.x > 0.0,
            "a pan setting off was stopped at {setting_off}"
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
}

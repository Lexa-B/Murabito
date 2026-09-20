//! An overhead camera that pans across the ground and zooms in and out.
//!
//! The rig holds the state — a ground point, a zoom distance, a pan velocity — and
//! `apply_rig` is the only thing that writes the camera's `Transform`. Input systems
//! change rig state and nothing else, so panning and zooming cannot fight over the
//! transform, and a future follow-the-terrain step has one place to hook into.

use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;

use crate::settings::CameraSettings;
use crate::state::AppState;

/// Default top pan speed in metres per second at `PAN_REF_ZOOM`, before the zoom
/// scaling below and before the player's own multiplier in `CameraSettings`.
const PAN_SPEED: f32 = 13.5;

/// The zoom distance at which the pan runs at exactly `PAN_SPEED`.
const PAN_REF_ZOOM: f32 = 13.0;

/// How strongly pan speed follows zoom distance. 0.0 pans at a fixed speed in metres
/// (fine zoomed out, sluggish up close); 1.0 pans at a fixed speed in screen-widths
/// (fine up close, frantic zoomed out). 0.75 sits three-quarters of the way toward
/// proportional: zooming out 4x speeds the pan up about 2.8x rather than 4x.
const PAN_ZOOM_EXPONENT: f32 = 0.75;

/// How sharply the pan velocity chases the velocity the keys ask for, per second.
/// Higher is snappier; at 13.0 a pan reaches ~63% of top speed in 0.077 s and ~95% in
/// 0.23 s, and coasts to a stop over about the same time when the keys are released.
/// Deliberately not scaled by zoom: the top speed scales instead, so the ramp takes
/// the same time (and therefore looks the same on screen) however far out you are.
const PAN_RESPONSE: f32 = 13.0;

/// Closest and furthest the camera may sit from its focus, in metres.
const ZOOM_MIN: f32 = 4.0;
const ZOOM_MAX: f32 = 60.0;

/// Distance multiplier per wheel notch. Multiplicative, not additive, so one notch
/// covers the same *proportion* of the view whether you are close in or far out.
const ZOOM_STEP: f32 = 1.15;

/// How sharply `zoom` chases `zoom_target`, per second. Snappier than the pan ease:
/// a wheel notch is a discrete request, not a held key.
const ZOOM_RESPONSE: f32 = 16.0;

/// Spawns the camera and runs the rig. Expects `SettingsPlugin` to have been added.
pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_camera).add_systems(
            Update,
            // Chained: both input systems mutate the rig, and `apply_rig` must see the
            // result of both, so the order is part of the design rather than incidental.
            // Frozen while the escape menu is open: a click on a button should not
            // also be a click in the world, and WASD should not pan behind the menu.
            (pan_camera, zoom_camera, apply_rig)
                .chain()
                .run_if(in_state(AppState::Playing)),
        );
    }
}

/// An overhead camera that looks at a point on the ground from a fixed angle.
/// Panning moves the point; the camera follows it, so the viewing angle never changes.
#[derive(Component)]
pub struct CameraRig {
    /// The spot on the ground (y = 0) the camera is aimed at.
    focus: Vec3,
    /// Which way the camera sits from that spot — up and back, a fixed 45 degrees.
    /// A unit vector: distance is `zoom`, not baked in here.
    direction: Vec3,
    /// How far back along `direction` the camera sits, in metres. Eased toward
    /// `zoom_target` so the wheel glides rather than snaps.
    zoom: f32,
    /// Where the wheel has asked `zoom` to end up.
    zoom_target: f32,
    /// Current pan velocity, in metres per second across the ground. Eased toward the
    /// velocity the keys ask for rather than set from them directly.
    velocity: Vec3,
}

impl Default for CameraRig {
    fn default() -> Self {
        Self {
            focus: Vec3::ZERO,
            direction: Vec3::new(0.0, 1.0, 1.0).normalize(),
            zoom: 13.0,
            zoom_target: 13.0,
            velocity: Vec3::ZERO,
        }
    }
}

impl CameraRig {
    /// Top pan speed at the current zoom, in metres per second, including the player's
    /// speed multiplier. The zoom curve's shape is fixed; the multiplier only shifts the
    /// whole curve up or down, which is the knob that turned out to be worth exposing.
    fn pan_speed(&self, settings: &CameraSettings) -> f32 {
        PAN_SPEED * settings.pan_speed_scale * (self.zoom / PAN_REF_ZOOM).powf(PAN_ZOOM_EXPONENT)
    }

    /// Jumps straight to a zoom distance, skipping the ease.
    ///
    /// For the one-shot capture, which has no time to glide: it needs the framing it
    /// asked for on the very next frame, not half a second later.
    pub fn jump_to_zoom(&mut self, zoom: f32) {
        self.zoom = zoom.clamp(ZOOM_MIN, ZOOM_MAX);
        self.zoom_target = self.zoom;
    }

    fn transform(&self) -> Transform {
        let eye = self.focus + self.direction * self.zoom;
        Transform::from_translation(eye).looking_at(self.focus, Vec3::Y)
    }
}

/// The camera spawns itself, so the scene only has to describe the world.
/// `AmbientLight` rides on the camera in Bevy 0.19; without it, everything the sun
/// doesn't reach is pure black.
fn spawn_camera(mut commands: Commands) {
    let rig = CameraRig::default();
    commands.spawn((
        Camera3d::default(),
        AmbientLight {
            brightness: 200.0,
            ..default()
        },
        rig.transform(),
        rig,
    ));
}

/// Runs every frame. `ButtonInput<KeyCode>` is the keyboard's current state (a resource
/// the input plugin refreshes), and `Time` carries the length of the last frame — so the
/// pan covers the same ground per second whether the machine runs at 60 or 300 fps.
fn pan_camera(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    settings: Res<CameraSettings>,
    mut rig: Single<&mut CameraRig>,
) {
    // Screen-space directions on the ground plane: -Z is away from the camera, +X right.
    let mut direction = Vec2::ZERO;
    if keys.any_pressed([KeyCode::KeyW, KeyCode::ArrowUp]) {
        direction.y -= 1.0;
    }
    if keys.any_pressed([KeyCode::KeyS, KeyCode::ArrowDown]) {
        direction.y += 1.0;
    }
    if keys.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]) {
        direction.x -= 1.0;
    }
    if keys.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]) {
        direction.x += 1.0;
    }
    let camera_rig = &mut **rig;

    // What the keys are asking for: full speed in that direction, or a standstill.
    // Normalised, so holding two keys pans at the same speed as one rather than 1.41x.
    let target = if direction == Vec2::ZERO {
        Vec3::ZERO
    } else {
        let d = direction.normalize();
        Vec3::new(d.x, 0.0, d.y) * camera_rig.pan_speed(&settings)
    };

    // Exponential ease toward that target. Written as 1 - e^(-k dt) rather than a plain
    // lerp factor so the feel is identical at any framerate: two 8 ms frames ease exactly
    // as far as one 16 ms frame.
    let dt = time.delta_secs();
    let t = 1.0 - (-PAN_RESPONSE * dt).exp();
    camera_rig.velocity = camera_rig.velocity.lerp(target, t);

    // Below a thousandth of top speed, call it stopped — otherwise the ease leaves the
    // camera creeping forever at ever-smaller speeds. Scaled like the speed itself, so
    // the cut-off is equally invisible at any zoom.
    let stop_below = (camera_rig.pan_speed(&settings) * 1e-3).powi(2);
    if camera_rig.velocity.length_squared() < stop_below {
        camera_rig.velocity = Vec3::ZERO;
        return;
    }

    let step = camera_rig.velocity * dt;
    camera_rig.focus += step;
}

/// Turns wheel notches into a new `zoom_target`, then eases `zoom` toward it.
fn zoom_camera(
    mut wheel: MessageReader<MouseWheel>,
    time: Res<Time>,
    mut rig: Single<&mut CameraRig>,
) {
    // A mouse reports whole lines per notch; a touchpad reports pixels, so scale those
    // down to comparable units rather than zooming a hundred times too far.
    let notches: f32 = wheel
        .read()
        .map(|event| match event.unit {
            MouseScrollUnit::Line => event.y,
            MouseScrollUnit::Pixel => event.y / 50.0,
        })
        .sum();

    if notches != 0.0 {
        // Scrolling up (positive) moves the camera closer, hence the negated exponent.
        rig.zoom_target = (rig.zoom_target * ZOOM_STEP.powf(-notches)).clamp(ZOOM_MIN, ZOOM_MAX);
    }

    let t = 1.0 - (-ZOOM_RESPONSE * time.delta_secs()).exp();
    let eased = rig.zoom.lerp(rig.zoom_target, t);
    rig.zoom = if (eased - rig.zoom_target).abs() < 0.001 {
        rig.zoom_target
    } else {
        eased
    };
}

/// The only system that writes the camera's `Transform`: pan and zoom change rig state,
/// this turns that state into a position and a look direction, once per frame.
fn apply_rig(mut rig: Single<(&CameraRig, &mut Transform)>) {
    let (camera_rig, transform) = &mut *rig;
    **transform = camera_rig.transform();
}

#[cfg(test)]
mod tests {
    use super::*;

    /// These pin down the pan-speed curve, which was arrived at by feel over several
    /// rounds rather than derived. Nothing will catch a change to it at runtime except
    /// a person noticing the camera feels wrong, so it is worth a test.
    fn rig_at(zoom: f32) -> CameraRig {
        CameraRig {
            zoom,
            zoom_target: zoom,
            ..default()
        }
    }

    fn scaled(scale: f32) -> CameraSettings {
        CameraSettings {
            pan_speed_scale: scale,
        }
    }

    #[test]
    fn jumping_to_a_zoom_skips_the_ease_and_respects_the_limits() {
        let mut rig = CameraRig::default();
        rig.jump_to_zoom(30.0);
        // Both, or the ease would drag it straight back toward the old target.
        assert_eq!(rig.zoom, 30.0);
        assert_eq!(rig.zoom_target, 30.0);

        rig.jump_to_zoom(ZOOM_MAX * 10.0);
        assert_eq!(rig.zoom, ZOOM_MAX);
        rig.jump_to_zoom(0.0);
        assert_eq!(rig.zoom, ZOOM_MIN);
    }

    #[test]
    fn pans_at_the_base_speed_at_the_reference_zoom() {
        let speed = rig_at(PAN_REF_ZOOM).pan_speed(&scaled(1.0));
        assert!(
            (speed - PAN_SPEED).abs() < 1e-4,
            "{speed} should be PAN_SPEED"
        );
    }

    #[test]
    fn pan_speed_follows_zoom_by_the_tuned_exponent() {
        let near = rig_at(PAN_REF_ZOOM).pan_speed(&scaled(1.0));
        let far = rig_at(PAN_REF_ZOOM * 4.0).pan_speed(&scaled(1.0));
        // Four times further out pans about 2.83x faster, not 4x (proportional) and
        // not 1x (fixed). This is the whole point of the 0.75 exponent.
        let ratio = far / near;
        assert!(
            (ratio - 4.0_f32.powf(PAN_ZOOM_EXPONENT)).abs() < 1e-4,
            "zooming out 4x changed the pan speed {ratio}x"
        );
        assert!(ratio > 1.0 && ratio < 4.0, "{ratio} left the tuned range");
    }

    #[test]
    fn the_player_multiplier_scales_the_whole_curve() {
        for zoom in [ZOOM_MIN, PAN_REF_ZOOM, ZOOM_MAX] {
            let plain = rig_at(zoom).pan_speed(&scaled(1.0));
            let doubled = rig_at(zoom).pan_speed(&scaled(2.0));
            assert!(
                (doubled - plain * 2.0).abs() < 1e-3,
                "at zoom {zoom}, doubling the setting gave {doubled} not {}",
                plain * 2.0
            );
        }
    }
}

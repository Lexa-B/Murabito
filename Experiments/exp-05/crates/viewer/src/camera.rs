//! The overhead camera rig: pure maths (this module) plus the systems that read input and
//! follow the ground.
//!
//! The rig has two entities: a focus that sits on the ground and carries a `Loader` (so
//! the world loads in around where the player is looking, not around the camera itself,
//! which is up in the air and often outside the loaded area at high zoom), and a camera
//! that is its child, sitting up and behind, looking at it. There is no rotation: the
//! camera always looks north, so panning is always along the east/north axes.

use bevy::input::keyboard::KeyCode;
use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseButton};
use bevy::input::ButtonInput;
use bevy::prelude::*;
use hexworld::{Level, WorldConfig};
use hexworld_bevy::axes::to_bevy;
use hexworld_bevy::HexWorld;

pub const MIN_ZOOM: f32 = 3.0;
pub const MAX_ZOOM: f32 = 4_000.0;

/// 45 degrees close in, 75 far out, easing on the logarithm of the zoom so the change
/// feels even across the whole range.
pub fn tilt_for_zoom(zoom: f32) -> f32 {
    let t = ((zoom / MIN_ZOOM).ln() / (MAX_ZOOM / MIN_ZOOM).ln()).clamp(0.0, 1.0);
    (45.0 + 30.0 * t).to_radians()
}

/// Metres per second of panning: a constant fraction of the zoom.
pub fn pan_speed_for_zoom(zoom: f32) -> f32 {
    zoom * 0.9
}

#[derive(Component, Clone, Copy, Debug)]
pub struct CameraRig {
    pub focus_east: f64,
    pub focus_north: f64,
    pub zoom: f32,
    /// The last ground height we actually resolved from the store. Held (not reset to
    /// zero) whenever nothing is loaded at the focus yet — see `resolve_height`.
    pub last_height: f32,
}

impl CameraRig {
    pub fn zoom_by(&mut self, delta: f32) {
        // Multiplicative, so a wheel click feels the same at every distance.
        self.zoom = (self.zoom * (1.0 + delta * 0.1)).clamp(MIN_ZOOM, MAX_ZOOM);
    }

    pub fn pan(&mut self, east: f64, north: f64, cfg: &WorldConfig) {
        let (e, n) = (self.focus_east + east, self.focus_north + north);
        // Stay in the world: keep the focus inside the outermost ri.
        let ri = hexworld::plane::round_at(e, n, Level::Ri);
        if hexworld::world::in_world(ri, cfg) {
            self.focus_east = e;
            self.focus_north = n;
        }
    }

    /// The ground height to use this frame: the store's real height where the focus has
    /// loaded ground, or the last height we had otherwise. At `MAX_ZOOM`, pan speed is
    /// about 3,600 m/s — fast enough to genuinely outrun the load frontier — so falling
    /// back to 0.0 would make the focus visibly drop and then pop back up once the real
    /// height arrives. Holding the last value avoids that pop; it is not a smoothing
    /// system, just "don't snap to a height that was never true."
    pub fn resolve_height(&mut self, loaded_height_m: Option<f64>) -> f32 {
        if let Some(h) = loaded_height_m {
            self.last_height = h as f32;
        }
        self.last_height
    }
}

/// Focus and camera positions in Bevy space. The camera sits south of the focus,
/// so the view looks north; height comes from the tilt.
pub fn rig_transforms(rig: &CameraRig) -> (Vec3, Vec3) {
    let focus = to_bevy(rig.focus_east, rig.focus_north, 0.0);
    let tilt = tilt_for_zoom(rig.zoom);
    let back = rig.zoom * tilt.cos();
    let up = rig.zoom * tilt.sin();
    // +Z is behind the focus, i.e. south, since north is -Z (see `hexworld_bevy::axes`).
    (focus, focus + Vec3::new(0.0, up, back))
}

/// WASD and the arrow keys pan; middle-drag pans; the wheel zooms. No rotation, no
/// screen-edge panning.
pub fn handle_input(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    mouse_scroll: Res<AccumulatedMouseScroll>,
    world: Res<HexWorld>,
    mut rigs: Query<&mut CameraRig>,
) {
    let cfg = *world.store().config();
    let dt = time.delta_secs();

    for mut rig in &mut rigs {
        if mouse_scroll.delta.y != 0.0 {
            // Scrolling up (delta.y > 0) zooms in, i.e. shrinks the distance.
            rig.zoom_by(-mouse_scroll.delta.y);
        }

        let mut dir = Vec2::ZERO;
        if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
            dir.y += 1.0;
        }
        if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
            dir.y -= 1.0;
        }
        if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
            dir.x += 1.0;
        }
        if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
            dir.x -= 1.0;
        }
        if dir != Vec2::ZERO {
            let dir = dir.normalize();
            let speed = pan_speed_for_zoom(rig.zoom);
            rig.pan(
                (dir.x * speed * dt) as f64,
                (dir.y * speed * dt) as f64,
                &cfg,
            );
        }

        if mouse_buttons.pressed(MouseButton::Middle) {
            let delta = mouse_motion.delta;
            if delta != Vec2::ZERO {
                // Grab-and-drag: the ground under the cursor should follow the mouse, so
                // the focus moves opposite the drag in east (screen right) and with the
                // drag in north (screen down looks south, i.e. toward the camera).
                let per_pixel = rig.zoom * 0.0025;
                rig.pan(
                    (-delta.x * per_pixel) as f64,
                    (delta.y * per_pixel) as f64,
                    &cfg,
                );
            }
        }
    }
}

/// Sets the focus entity's transform (ground height comes from the loaded terrain) and
/// the camera child's transform (up and behind, looking at the focus).
pub fn follow_ground(
    world: Res<HexWorld>,
    mut focus_query: Query<(&mut CameraRig, &mut Transform)>,
    mut camera_query: Query<&mut Transform, (With<Camera3d>, Without<CameraRig>)>,
) {
    for (mut rig, mut focus_transform) in &mut focus_query {
        let (focus_flat, camera_flat) = rig_transforms(&rig);
        let loaded = world
            .store()
            .surface_height_m(rig.focus_east, rig.focus_north);
        let height = rig.resolve_height(loaded);
        focus_transform.translation = Vec3::new(focus_flat.x, height, focus_flat.z);

        if let Ok(mut camera_transform) = camera_query.single_mut() {
            // The camera is a child of the focus, and the focus never rotates, so its
            // local offset is just the flat-world difference: ground height and pan both
            // cancel out, leaving only the tilt/zoom-driven offset.
            let offset = camera_flat - focus_flat;
            *camera_transform = Transform::from_translation(offset).looking_at(Vec3::ZERO, Vec3::Y);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tilt_goes_from_45_close_to_75_far() {
        assert!((tilt_for_zoom(MIN_ZOOM).to_degrees() - 45.0).abs() < 1.0);
        assert!((tilt_for_zoom(MAX_ZOOM).to_degrees() - 75.0).abs() < 1.0);
        // and it is monotonic
        let mut previous = 0.0;
        for i in 0..=20 {
            let zoom = MIN_ZOOM + (MAX_ZOOM - MIN_ZOOM) * i as f32 / 20.0;
            let tilt = tilt_for_zoom(zoom);
            assert!(tilt >= previous, "tilt should not decrease as you zoom out");
            previous = tilt;
        }
    }

    #[test]
    fn pan_speed_scales_with_zoom() {
        assert!(pan_speed_for_zoom(MAX_ZOOM) > pan_speed_for_zoom(MIN_ZOOM) * 10.0);
    }

    #[test]
    fn the_camera_sits_above_and_behind_the_focus() {
        let rig = CameraRig {
            focus_east: 0.0,
            focus_north: 0.0,
            zoom: 50.0,
            last_height: 0.0,
        };
        let (focus, camera) = rig_transforms(&rig);
        assert!(
            camera.y > focus.y + 10.0,
            "the camera should be above the focus"
        );
        assert!(camera.z > focus.z, "and south of it, looking north");
        assert!(
            (camera.distance(focus) - 50.0).abs() < 0.5,
            "zoom is the distance"
        );
    }

    #[test]
    fn zoom_is_clamped() {
        let mut rig = CameraRig {
            focus_east: 0.0,
            focus_north: 0.0,
            zoom: 50.0,
            last_height: 0.0,
        };
        rig.zoom_by(-100.0);
        assert!(rig.zoom >= MIN_ZOOM);
        rig.zoom_by(1e9);
        assert!(rig.zoom <= MAX_ZOOM);
    }

    #[test]
    fn the_focus_stays_in_the_world() {
        let cfg = WorldConfig::default();
        let mut rig = CameraRig {
            focus_east: 0.0,
            focus_north: 0.0,
            zoom: 50.0,
            last_height: 0.0,
        };
        rig.pan(1e6, 0.0, &cfg);
        let cell = hexworld::plane::round_at(rig.focus_east, rig.focus_north, Level::Ri);
        assert!(
            hexworld::world::in_world(cell, &cfg),
            "panned out of the world"
        );
    }

    #[test]
    fn ground_height_holds_when_nothing_is_loaded() {
        // A fast pan at high zoom can outrun the load frontier. When that happens,
        // `surface_height_m` returns `None`, and the focus must keep its last known
        // height rather than snapping to 0.0 and popping back up once the real height
        // arrives.
        let mut rig = CameraRig {
            focus_east: 0.0,
            focus_north: 0.0,
            zoom: 50.0,
            last_height: 12.0,
        };
        let height = rig.resolve_height(None);
        assert_eq!(height, 12.0, "height should hold, not snap to zero");
        assert_eq!(rig.last_height, 12.0);
    }

    #[test]
    fn ground_height_updates_once_the_real_height_arrives() {
        let mut rig = CameraRig {
            focus_east: 0.0,
            focus_north: 0.0,
            zoom: 50.0,
            last_height: 0.0,
        };
        let height = rig.resolve_height(Some(7.5));
        assert_eq!(height, 7.5);
        assert_eq!(rig.last_height, 7.5);

        // And it keeps holding that new value once loading falls behind again.
        let height = rig.resolve_height(None);
        assert_eq!(height, 7.5);
    }
}

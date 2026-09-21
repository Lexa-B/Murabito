//! The overhead camera.
//!
//! Lengths are in shaku: one world unit is one 尺, about 30.3 cm.

use bevy::prelude::*;

/// Where the camera sits: 7 間 (42 shaku, about 12.7 m) from the origin, up and back
/// at 45 degrees.
const EYE: Vec3 = Vec3::new(0.0, 29.7, 29.7);

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_camera);
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(EYE).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A headless app, run for one frame: no window, no GPU. Spawning a camera needs
    /// neither; drawing through one does, and no test here draws.
    fn camera_after_startup() -> Transform {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, CameraPlugin));
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
}

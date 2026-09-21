//! The overhead camera.
//!
//! Lengths are in shaku: one world unit is one 尺, about 30.3 cm.

use bevy::prelude::*;

/// How far the camera starts from its focus: 7 間 (42 shaku, about 12.7 m).
const START_ZOOM: f32 = 42.0;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_camera)
            .add_systems(Update, apply_rig);
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

fn apply_rig(mut cameras: Query<(&CameraRig, &mut Transform)>) {
    for (rig, mut transform) in &mut cameras {
        *transform = rig.transform();
    }
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
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, CameraPlugin));
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
}

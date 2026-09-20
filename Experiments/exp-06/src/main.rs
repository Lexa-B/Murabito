//! exp-06 step 5: a starter scene with an overhead camera you can pan and zoom.

use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Murabito exp-06".into(),
                ..default()
            }),
            ..default()
        }))
        // The colour the framebuffer is cleared to each frame: everything the camera
        // doesn't draw over. Stands in for a sky.
        .insert_resource(ClearColor(Color::srgb(0.53, 0.81, 0.92)))
        .init_resource::<CameraSettings>()
        .add_systems(Startup, setup)
        .add_systems(Update, (pan_camera, zoom_camera, apply_rig).chain())
        .run();
}

/// Runs once, before the first frame. `Commands` queues entity spawns; the two
/// `ResMut<Assets<_>>` are the engine's mesh and material stores — `add` uploads an
/// asset and hands back a `Handle`, which is what an entity actually carries.
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Ground: 20 x 20 m, centred on the origin, facing up (Y is up in Bevy).
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(20.0, 20.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.35, 0.42, 0.30))),
    ));

    // A 1 m cube, lifted half its height so it sits on the ground rather than in it.
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.85, 0.45, 0.20))),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));

    // Sun. A directional light has no position — only a direction, which is why this is
    // aimed with `looking_at` and the translation only serves to set that angle.
    commands.spawn((
        DirectionalLight {
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(-10.0, 14.0, -4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Camera. The rig owns the ground point being looked at; the camera's own
    // transform is recomputed from it every frame by `pan_camera`.
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

/// An overhead camera that looks at a point on the ground from a fixed offset.
/// Panning moves the point; the camera follows it, so the viewing angle never changes.
#[derive(Component)]
struct CameraRig {
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

    fn transform(&self) -> Transform {
        let eye = self.focus + self.direction * self.zoom;
        Transform::from_translation(eye).looking_at(self.focus, Vec3::Y)
    }
}

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

/// Camera preferences a player can change. A `Resource`, not a component: there is one
/// set of them for the whole app, independent of how many camera rigs exist.
///
/// Nothing writes to this yet — a settings page will. Until then every field keeps its
/// `Default`, which is the value tuned by hand while building the rig.
#[derive(Resource)]
struct CameraSettings {
    /// Multiplies the whole pan-speed curve. 1.0 is the tuned default.
    pan_speed_scale: f32,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            pan_speed_scale: 1.0,
        }
    }
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

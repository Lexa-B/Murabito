//! What a body can sense, as geometry. No brain: nothing here decides anything, and
//! nothing reads the world. These are the *fields* — the shape of a being's senses —
//! plus the pure functions that say how strongly a point inside them registers, and a
//! debug overlay so the shapes can be seen.
//!
//! Perception (turning these fields into beliefs) is a later bite. Keeping the geometry
//! separate from the sensing means the shapes can be tuned, drawn and tested on their
//! own, and it is the same split the eye and the visual cortex have.
//!
//! # One file per sense, because they are not the same shape
//!
//! It is tempting to give the senses a common trait and be done. They resist it, because
//! they differ in what *drives* them:
//!
//! | | Driven by | Obstacles | State |
//! |---|---|---|---|
//! | vision  | the perceiver — a query from the head | block outright | none |
//! | hearing | the source — an event, then who heard it | attenuate, bending around | none |
//! | smell   | the world — a field a nose samples | attenuate, and it pools | persists, drifts |
//!
//! Vision is a pull, hearing is a push, and smell is neither: its state does not live in
//! the perceiver at all. So what they share is the ground-plane geometry in this file
//! and (later) a way to ask what lies between two points — not an interface.
//!
//! Everything works in the ground plane: `Vec2` here is world XZ, never XY. Lengths are
//! shaku, the crate's base unit.

pub mod hearing;
pub mod vision;

use bevy::prelude::*;

pub use hearing::Hearing;
pub use vision::{Vision, VisionBand};

/// How high above the ground the debug overlay is drawn, to keep it off the surface.
/// Shaku, like every other length.
const OVERLAY_HEIGHT: f32 = 0.15;
/// Points per arc or polar curve. Enough to look smooth at the zooms we use.
const CURVE_STEPS: usize = 48;

/// Draws the sense fields. Nothing else here needs a plugin: the components are data.
pub struct SensesPlugin;

impl Plugin for SensesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (vision::draw_vision, hearing::draw_hearing));
    }
}

/// Suppresses this being's sense overlay.
///
/// The drawing systems skip anything carrying it, so nothing in `senses` needs to know
/// that a debug screen exists, or what a 種 is. Whoever wants a being quiet adds the
/// marker; the senses themselves are unaffected, and still sense.
#[derive(Component, Clone, Copy, Debug)]
pub struct HideSenses;

/// The colour a being's sense overlay is drawn in. On the being so that each one
/// is identifiable at a glance, rather than hard-coded per species in the drawing code.
#[derive(Component, Clone, Copy, Debug)]
pub struct SenseOverlay {
    pub color: Color,
}

/// Where a being faces, in the ground plane. Bevy's forward is local -Z.
fn facing(transform: &GlobalTransform) -> Vec2 {
    let forward = transform.forward();
    Vec2::new(forward.x, forward.z).normalize_or(Vec2::new(0.0, -1.0))
}

/// Ground-plane point to a world point on the overlay plane.
fn lift(origin: Vec3, offset: Vec2) -> Vec3 {
    Vec3::new(origin.x + offset.x, OVERLAY_HEIGHT, origin.z + offset.y)
}

/// Rotates a ground-plane vector by `angle` radians.
pub(crate) fn rotate(v: Vec2, angle: f32) -> Vec2 {
    let (sin, cos) = angle.sin_cos();
    Vec2::new(v.x * cos - v.y * sin, v.x * sin + v.y * cos)
}

/// Shared by the senses' own tests: every sense is tested against the same facing and
/// the same way of placing something at a bearing, so the numbers stay comparable.
#[cfg(test)]
pub(crate) mod testing {
    use super::*;

    /// Facing -Z, which is Bevy's forward, expressed in the XZ plane we work in.
    pub(crate) const FORWARD: Vec2 = Vec2::new(0.0, -1.0);

    pub(crate) fn at(bearing_degrees: f32, distance: f32) -> Vec2 {
        rotate(FORWARD, bearing_degrees.to_radians()) * distance
    }
}

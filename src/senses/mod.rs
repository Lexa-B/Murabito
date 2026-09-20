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

use bevy::gizmos::AppGizmoBuilder;
use bevy::gizmos::config::{GizmoConfig, GizmoConfigGroup, GizmoLineConfig};
use bevy::prelude::*;

pub use hearing::Hearing;
pub use vision::{Vision, VisionBand};

/// How high above the ground the debug overlay is drawn, to keep it off the surface.
/// Shaku, like every other length.
const OVERLAY_HEIGHT: f32 = 0.15;

/// Draws the sense fields. Nothing else here needs a plugin: the components are data.
pub struct SensesPlugin;

impl Plugin for SensesPlugin {
    fn build(&self, app: &mut App) {
        // Gizmo line width is a property of the config group, not of the call, so a
        // stroke weight means a group. Three of them is what lets density read as
        // heaviness as well as count — at a cell ten to thirty pixels across, stroke
        // count alone does not carry it.
        app.insert_gizmo_config(BoldStroke, stroke_config(2.6))
            .insert_gizmo_config(MidStroke, stroke_config(1.7))
            .insert_gizmo_config(FaintStroke, stroke_config(1.0))
            .add_systems(Update, (vision::draw_vision, hearing::draw_hearing));
    }
}

fn stroke_config(width: f32) -> GizmoConfig {
    GizmoConfig {
        line: GizmoLineConfig { width, ..default() },
        ..default()
    }
}

/// The heaviest stroke: the nearest, sharpest band.
#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct BoldStroke;

#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct MidStroke;

/// The lightest stroke: the far edge of a sense, where it is barely there.
#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct FaintStroke;

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

/// A ground-plane point, in world coordinates, lifted onto the overlay plane.
///
/// Unlike [`lift`], which offsets from a being, this takes an absolute position — which
/// is what a cell centre is.
pub(crate) fn ground_point(point: Vec2) -> Vec3 {
    Vec3::new(point.x, OVERLAY_HEIGHT, point.y)
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

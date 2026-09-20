//! What a body can sense, as geometry. No brain: nothing here decides anything, and
//! nothing reads the world. These are the *fields* — the shape of a being's vision
//! and hearing — plus the pure functions that say how strongly a point inside them
//! registers, and a debug overlay so the shapes can be seen.
//!
//! Perception (turning these fields into beliefs) is a later bite. Keeping the geometry
//! separate from the sensing means the shapes can be tuned, drawn and tested on their
//! own, and it is the same split the eye and the visual cortex have.
//!
//! Everything works in the ground plane: `Vec2` here is world XZ, never XY.

use bevy::prelude::*;

/// How high above the ground the debug overlay is drawn, to keep it off the surface.
const OVERLAY_HEIGHT: f32 = 0.05;
/// Points per arc or polar curve. Enough to look smooth at the zooms we use.
const CURVE_STEPS: usize = 48;

/// Draws the sense fields. Nothing else here needs a plugin: the components are data.
pub struct SensesPlugin;

impl Plugin for SensesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (draw_vision, draw_hearing));
    }
}

/// One band of a vision cone: everything out to `range` that isn't in a nearer band.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VisionBand {
    /// Outer edge, in metres.
    pub range: f32,
    /// How well something in this band registers, 0..1.
    pub sensitivity: f32,
}

/// A cone of vision, centred on where the being faces, with acuity falling off in
/// three steps rather than continuously — near, middle and far.
///
/// Three bands and not a curve because the bands are what perception will actually
/// branch on later, and because a discrete fall-off is easier to reason about when
/// something behaves oddly.
#[derive(Component, Clone, Debug)]
pub struct Vision {
    /// Full angular width, in radians. The cone spans `arc / 2` either side of forward.
    pub arc: f32,
    /// Near to far. Ranges must increase; sensitivities are expected to fall.
    pub bands: [VisionBand; 3],
}

impl Vision {
    /// Half the cone's width: the largest bearing still inside it.
    pub fn half_arc(&self) -> f32 {
        self.arc * 0.5
    }

    /// The outer edge of the furthest band.
    pub fn far_range(&self) -> f32 {
        self.bands[2].range
    }

    /// How strongly something at `offset` registers for a being facing `forward`.
    /// Zero means it cannot be seen at all — outside the cone, or past the last band.
    ///
    /// `forward` must be a unit vector; `offset` is relative to the being. Both are
    /// world XZ.
    pub fn sensitivity_at(&self, forward: Vec2, offset: Vec2) -> f32 {
        let distance = offset.length();
        if distance > self.far_range() {
            return 0.0;
        }
        // Something at the being's own position has no bearing; treat it as seen.
        if distance > f32::EPSILON {
            let cos = forward.dot(offset / distance).clamp(-1.0, 1.0);
            if cos.acos() > self.half_arc() {
                return 0.0;
            }
        }
        for band in &self.bands {
            if distance <= band.range {
                return band.sensitivity;
            }
        }
        0.0
    }
}

/// Hearing, shaped like a microphone's polar pattern.
///
/// `r(θ) = (1 - d) + d·cos θ`, the standard family: `d = 0` is omnidirectional, `d = 0.5`
/// a true cardioid, and above that it narrows toward the front and grows a null behind.
/// One number covers everything from a rabbit's near-spherical awareness to a fox's
/// forward-pointed listening, which is why it is a parameter rather than two shapes.
#[derive(Component, Clone, Debug)]
pub struct Hearing {
    /// How far it reaches at the most sensitive bearing, in metres.
    pub range: f32,
    /// 0 is a circle, 0.5 a cardioid, approaching 1 an ever tighter forward lobe.
    pub directionality: f32,
}

impl Hearing {
    /// Sensitivity at a bearing given by its cosine, 0..1. Never negative: past a
    /// cardioid the pattern would invert, and a null is silence, not anti-hearing.
    pub fn gain(&self, bearing_cos: f32) -> f32 {
        ((1.0 - self.directionality) + self.directionality * bearing_cos).max(0.0)
    }

    /// How far hearing reaches at that bearing.
    pub fn reach(&self, bearing_cos: f32) -> f32 {
        self.range * self.gain(bearing_cos)
    }

    /// How strongly something at `offset` registers for a being facing `forward`.
    /// Zero means out of earshot in that direction.
    pub fn gain_at(&self, forward: Vec2, offset: Vec2) -> f32 {
        let distance = offset.length();
        if distance <= f32::EPSILON {
            return self.gain(1.0);
        }
        let cos = forward.dot(offset / distance).clamp(-1.0, 1.0);
        if distance > self.reach(cos) {
            return 0.0;
        }
        self.gain(cos)
    }
}

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

/// One arc per band, plus the two edges of the cone. Fainter bands are further out.
fn draw_vision(mut gizmos: Gizmos, beings: Query<(&GlobalTransform, &Vision, &SenseOverlay)>) {
    for (transform, vision, overlay) in &beings {
        let origin = transform.translation();
        let forward = facing(transform);
        let half = vision.half_arc();

        for band in &vision.bands {
            let color = overlay.color.with_alpha(band.sensitivity);
            let arc = (0..=CURVE_STEPS).map(|step| {
                let t = -half + vision.arc * step as f32 / CURVE_STEPS as f32;
                lift(origin, rotate(forward, t) * band.range)
            });
            gizmos.linestrip(arc, color);
        }

        // The cone's two straight edges, out to the furthest band.
        let edge = overlay.color.with_alpha(vision.bands[0].sensitivity);
        for side in [-half, half] {
            let end = rotate(forward, side) * vision.far_range();
            gizmos.line(lift(origin, Vec2::ZERO), lift(origin, end), edge);
        }
    }
}

/// The hearing pattern as a closed polar curve around the being.
fn draw_hearing(mut gizmos: Gizmos, beings: Query<(&GlobalTransform, &Hearing, &SenseOverlay)>) {
    for (transform, hearing, overlay) in &beings {
        let origin = transform.translation();
        let forward = facing(transform);
        let color = overlay.color.with_alpha(0.35);

        let curve = (0..=CURVE_STEPS).map(|step| {
            let bearing =
                -std::f32::consts::PI + std::f32::consts::TAU * step as f32 / CURVE_STEPS as f32;
            let reach = hearing.reach(bearing.cos());
            lift(origin, rotate(forward, bearing) * reach)
        });
        gizmos.linestrip(curve, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Facing -Z, which is Bevy's forward, expressed in the XZ plane we work in.
    const FORWARD: Vec2 = Vec2::new(0.0, -1.0);

    fn at(bearing_degrees: f32, distance: f32) -> Vec2 {
        rotate(FORWARD, bearing_degrees.to_radians()) * distance
    }

    fn cone(arc_degrees: f32) -> Vision {
        Vision {
            arc: arc_degrees.to_radians(),
            bands: [
                VisionBand {
                    range: 4.0,
                    sensitivity: 1.0,
                },
                VisionBand {
                    range: 9.0,
                    sensitivity: 0.6,
                },
                VisionBand {
                    range: 15.0,
                    sensitivity: 0.3,
                },
            ],
        }
    }

    #[test]
    fn each_band_reports_its_own_sensitivity() {
        let vision = cone(120.0);
        assert_eq!(vision.sensitivity_at(FORWARD, at(0.0, 2.0)), 1.0);
        assert_eq!(vision.sensitivity_at(FORWARD, at(0.0, 7.0)), 0.6);
        assert_eq!(vision.sensitivity_at(FORWARD, at(0.0, 12.0)), 0.3);
    }

    #[test]
    fn nothing_is_seen_past_the_last_band() {
        assert_eq!(cone(120.0).sensitivity_at(FORWARD, at(0.0, 20.0)), 0.0);
    }

    /// The arc is the *full* width, so a 120 degree cone reaches 60 degrees either side.
    /// Getting that factor of two wrong would silently double every field of view.
    #[test]
    fn the_arc_is_full_width_not_half_width() {
        let vision = cone(120.0);
        assert_eq!(vision.sensitivity_at(FORWARD, at(50.0, 2.0)), 1.0);
        assert_eq!(vision.sensitivity_at(FORWARD, at(-50.0, 2.0)), 1.0);
        assert_eq!(vision.sensitivity_at(FORWARD, at(70.0, 2.0)), 0.0);
        assert_eq!(vision.sensitivity_at(FORWARD, at(-70.0, 2.0)), 0.0);
    }

    #[test]
    fn a_wide_cone_sees_where_a_narrow_one_cannot() {
        let narrow = cone(120.0);
        let wide = cone(240.0);
        let behind_the_shoulder = at(100.0, 2.0);
        assert_eq!(narrow.sensitivity_at(FORWARD, behind_the_shoulder), 0.0);
        assert_eq!(wide.sensitivity_at(FORWARD, behind_the_shoulder), 1.0);
        // Still not all the way round: 240 degrees leaves a 120 degree blind arc behind.
        assert_eq!(wide.sensitivity_at(FORWARD, at(130.0, 2.0)), 0.0);
    }

    /// Something standing on top of you has no bearing to test, and dividing by its
    /// distance would be a division by zero.
    #[test]
    fn a_zero_offset_does_not_divide_by_zero() {
        assert_eq!(cone(120.0).sensitivity_at(FORWARD, Vec2::ZERO), 1.0);
    }

    #[test]
    fn zero_directionality_hears_equally_in_every_direction() {
        let ears = Hearing {
            range: 10.0,
            directionality: 0.0,
        };
        assert_eq!(ears.gain(1.0), 1.0);
        assert_eq!(ears.gain(0.0), 1.0);
        assert_eq!(ears.gain(-1.0), 1.0);
    }

    /// The defining property of a cardioid: exactly one null, directly behind.
    #[test]
    fn half_directionality_is_a_true_cardioid() {
        let ears = Hearing {
            range: 10.0,
            directionality: 0.5,
        };
        assert_eq!(ears.gain(1.0), 1.0);
        assert_eq!(ears.gain(0.0), 0.5);
        assert_eq!(ears.gain(-1.0), 0.0);
    }

    /// Past a cardioid the polar pattern would go negative behind. Silence is zero;
    /// there is no such thing as anti-hearing.
    #[test]
    fn a_tight_pattern_never_reports_negative_gain() {
        let ears = Hearing {
            range: 10.0,
            directionality: 0.8,
        };
        assert_eq!(ears.gain(-1.0), 0.0);
        assert_eq!(ears.reach(-1.0), 0.0);
    }

    /// Bearings that come out of a rotation carry float error - a quarter turn leaves
    /// the cosine at about 4e-8 rather than zero - so anything derived from one is
    /// compared with a tolerance.
    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 1e-5,
            "{actual} is not about {expected}"
        );
    }

    #[test]
    fn hearing_runs_out_at_its_reach_for_that_bearing() {
        let ears = Hearing {
            range: 10.0,
            directionality: 0.5,
        };
        // Sideways the pattern is at half gain, so it reaches half as far.
        assert_eq!(ears.reach(0.0), 5.0);
        assert_close(ears.gain_at(FORWARD, at(90.0, 4.0)), 0.5);
        assert_close(ears.gain_at(FORWARD, at(90.0, 6.0)), 0.0);
    }
}

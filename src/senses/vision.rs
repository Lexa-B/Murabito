//! Sight: a cone from the head, with acuity falling off in steps.
//!
//! A pull sense — the perceiver asks what is in front of it, and the answer is recomputed
//! from scratch each time, holding no state between frames. Obstacles will block it
//! outright rather than dimming it, which is what separates it from the other two.

use bevy::prelude::*;

use super::{CURVE_STEPS, HideSenses, SenseOverlay, facing, lift, rotate};

/// One band of a vision cone: everything out to `range` that isn't in a nearer band.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VisionBand {
    /// Outer edge, in whole shaku. Integer because a range is a count of cells on the
    /// sense grid, not a continuous distance — see the crate docs on the base unit.
    pub range: u32,
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

    /// The outer edge of the furthest band, in shaku.
    pub fn far_range(&self) -> u32 {
        self.bands[2].range
    }

    /// How strongly something at `offset` registers for a being facing `forward`.
    /// Zero means it cannot be seen at all — outside the cone, or past the last band.
    ///
    /// Geometry only: nothing here knows whether something stands in the way. Line of
    /// sight is a later bite, and will gate this rather than change it.
    ///
    /// `forward` must be a unit vector; `offset` is relative to the being. Both are
    /// world XZ.
    pub fn sensitivity_at(&self, forward: Vec2, offset: Vec2) -> f32 {
        let distance = offset.length();
        if distance > self.far_range() as f32 {
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
            if distance <= band.range as f32 {
                return band.sensitivity;
            }
        }
        0.0
    }
}

/// One arc per band, plus the two edges of the cone. Fainter bands are further out.
pub(super) fn draw_vision(
    mut gizmos: Gizmos,
    beings: Query<(&GlobalTransform, &Vision, &SenseOverlay), Without<HideSenses>>,
) {
    for (transform, vision, overlay) in &beings {
        let origin = transform.translation();
        let forward = facing(transform);
        let half = vision.half_arc();

        for band in &vision.bands {
            let color = overlay.color.with_alpha(band.sensitivity);
            let arc = (0..=CURVE_STEPS).map(|step| {
                let t = -half + vision.arc * step as f32 / CURVE_STEPS as f32;
                lift(origin, rotate(forward, t) * band.range as f32)
            });
            gizmos.linestrip(arc, color);
        }

        // The cone's two straight edges, out to the furthest band.
        let edge = overlay.color.with_alpha(vision.bands[0].sensitivity);
        for side in [-half, half] {
            let end = rotate(forward, side) * vision.far_range() as f32;
            gizmos.line(lift(origin, Vec2::ZERO), lift(origin, end), edge);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::{FORWARD, at};
    use super::*;

    fn cone(arc_degrees: f32) -> Vision {
        Vision {
            arc: arc_degrees.to_radians(),
            bands: [
                VisionBand {
                    range: 12,
                    sensitivity: 1.0,
                },
                VisionBand {
                    range: 30,
                    sensitivity: 0.6,
                },
                VisionBand {
                    range: 48,
                    sensitivity: 0.3,
                },
            ],
        }
    }

    #[test]
    fn each_band_reports_its_own_sensitivity() {
        let vision = cone(120.0);
        assert_eq!(vision.sensitivity_at(FORWARD, at(0.0, 6.0)), 1.0);
        assert_eq!(vision.sensitivity_at(FORWARD, at(0.0, 20.0)), 0.6);
        assert_eq!(vision.sensitivity_at(FORWARD, at(0.0, 40.0)), 0.3);
    }

    #[test]
    fn nothing_is_seen_past_the_last_band() {
        assert_eq!(cone(120.0).sensitivity_at(FORWARD, at(0.0, 60.0)), 0.0);
    }

    /// The arc is the *full* width, so a 120 degree cone reaches 60 degrees either side.
    /// Getting that factor of two wrong would silently double every field of view.
    #[test]
    fn the_arc_is_full_width_not_half_width() {
        let vision = cone(120.0);
        assert_eq!(vision.sensitivity_at(FORWARD, at(50.0, 6.0)), 1.0);
        assert_eq!(vision.sensitivity_at(FORWARD, at(-50.0, 6.0)), 1.0);
        assert_eq!(vision.sensitivity_at(FORWARD, at(70.0, 6.0)), 0.0);
        assert_eq!(vision.sensitivity_at(FORWARD, at(-70.0, 6.0)), 0.0);
    }

    #[test]
    fn a_wide_cone_sees_where_a_narrow_one_cannot() {
        let narrow = cone(120.0);
        let wide = cone(240.0);
        let behind_the_shoulder = at(100.0, 6.0);
        assert_eq!(narrow.sensitivity_at(FORWARD, behind_the_shoulder), 0.0);
        assert_eq!(wide.sensitivity_at(FORWARD, behind_the_shoulder), 1.0);
        // Still not all the way round: 240 degrees leaves a 120 degree blind arc behind.
        assert_eq!(wide.sensitivity_at(FORWARD, at(130.0, 6.0)), 0.0);
    }

    /// Something standing on top of you has no bearing to test, and dividing by its
    /// distance would be a division by zero.
    #[test]
    fn a_zero_offset_does_not_divide_by_zero() {
        assert_eq!(cone(120.0).sensitivity_at(FORWARD, Vec2::ZERO), 1.0);
    }
}

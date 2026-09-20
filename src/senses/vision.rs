//! Sight: a cone from the head, with acuity falling off in steps.
//!
//! A pull sense — the perceiver asks what is in front of it, and the answer is recomputed
//! from scratch each time, holding no state between frames. Obstacles will block it
//! outright rather than dimming it, which is what separates it from the other two.

use bevy::prelude::*;

use super::{
    BoldStroke, FaintStroke, HideSenses, MidStroke, SenseOverlay, facing, ground_point, rotate,
};
use crate::hex::{Hex, steps_covering};

/// Strokes per cell, nearest band first. Density and weight together carry acuity.
const STROKES_PER_CELL: [usize; 3] = [3, 2, 1];

/// Fill opacity per band. Not the band's own sensitivity: count and weight already say
/// how sharp a band is, and reusing 0.3 as an alpha leaves the far band invisible.
const FILL_ALPHA: [f32; 3] = [0.55, 0.42, 0.30];

const OUTLINE_ALPHA: f32 = 0.85;

/// Hatch direction, fixed in world space.
const HATCH_ANGLE: f32 = std::f32::consts::FRAC_PI_4;

/// Half a stroke, in shaku. A whole cell long, so strokes meet across cell edges.
const STROKE_HALF_LENGTH: f32 = 0.5;

/// How far apart the outermost strokes in a cell sit.
const HATCH_SPAN: f32 = 0.62;

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
        self.band_at(forward, offset)
            .map_or(0.0, |band| self.bands[band].sensitivity)
    }

    /// Which band something at `offset` falls in, nearest first, or `None` if it is not
    /// seen at all. The overlay draws from this, so what is on screen is exactly what
    /// perception will later read — a wrongly included cell shows up as a notch in the
    /// outline rather than hiding behind a smooth arc.
    pub fn band_at(&self, forward: Vec2, offset: Vec2) -> Option<usize> {
        let distance = offset.length();
        if distance > self.far_range() as f32 {
            return None;
        }
        // Something at the being's own position has no bearing; treat it as seen.
        if distance > f32::EPSILON {
            let cos = forward.dot(offset / distance).clamp(-1.0, 1.0);
            if cos.acos() > self.half_arc() {
                return None;
            }
        }
        self.bands
            .iter()
            .position(|band| distance <= band.range as f32)
    }
}

/// Crosshatch over every covered cell, heavier and denser the sharper the band, with
/// the region outlined along the cells' own edges.
///
/// The outline is boundary-extracted rather than drawn as an arc: an edge is stroked
/// when the cell across it falls in a different band, or in none. So the picture is the
/// coverage, and cannot flatter it.
pub(super) fn draw_vision(
    mut bold: Gizmos<BoldStroke>,
    mut mid: Gizmos<MidStroke>,
    mut faint: Gizmos<FaintStroke>,
    beings: Query<(&GlobalTransform, &Vision, &SenseOverlay), Without<HideSenses>>,
) {
    for (transform, vision, overlay) in &beings {
        let origin = transform.translation();
        let ground = Vec2::new(origin.x, origin.z);
        let forward = facing(transform);
        let here = Hex::from_world(ground);

        // Steps, not shaku: see `steps_covering`. Scanning `far_range` directly leaves
        // notches in the diagonal directions, where cells inside the range sit more than
        // that many steps out.
        for hex in here.within(steps_covering(vision.far_range() as f32)) {
            let Some(band) = vision.band_at(forward, hex.center() - ground) else {
                continue;
            };

            let colour = overlay.color.with_alpha(FILL_ALPHA[band]);
            for (a, b) in hatch(hex.center(), STROKES_PER_CELL[band]) {
                match band {
                    0 => bold.line(a, b, colour),
                    1 => mid.line(a, b, colour),
                    _ => faint.line(a, b, colour),
                }
            }

            let outline = overlay.color.with_alpha(OUTLINE_ALPHA);
            for direction in 0..6 {
                let across = vision.band_at(forward, hex.neighbour(direction).center() - ground);
                if across == Some(band) {
                    continue;
                }
                let (a, b) = hex.edge(direction);
                mid.line(ground_point(a), ground_point(b), outline);
            }
        }
    }
}

/// Parallel strokes across one cell, as a hatch.
///
/// The angle is fixed in world space rather than per cell, so neighbouring cells knit
/// into continuous hatching instead of reading as a grid of separate marks.
fn hatch(centre: Vec2, count: usize) -> impl Iterator<Item = (Vec3, Vec3)> {
    let along = rotate(Vec2::X, HATCH_ANGLE);
    let across = Vec2::new(-along.y, along.x);
    let spacing = if count > 1 {
        HATCH_SPAN / (count - 1) as f32
    } else {
        0.0
    };
    (0..count).map(move |i| {
        let shift = across * ((i as f32) - (count as f32 - 1.0) * 0.5) * spacing;
        let a = centre + shift - along * STROKE_HALF_LENGTH;
        let b = centre + shift + along * STROKE_HALF_LENGTH;
        (ground_point(a), ground_point(b))
    })
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

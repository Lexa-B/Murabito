//! Hearing: the shape of the ear, not the shape of the sound.
//!
//! A push sense. Something makes a noise, the noise travels, and this says how much of
//! what arrives a given listener actually registers. So what lives here is a *receiver*
//! pattern, applied at the ear to whatever intensity and bearing reach it — propagation
//! through the world, and the way obstacles attenuate rather than block, is a later bite
//! and belongs elsewhere.
//!
//! Until that exists, `range` stands in for propagation: it is how far a sound of
//! unremarkable loudness carries, in whole shaku.

use bevy::prelude::*;

use super::{FaintStroke, HideSenses, MidStroke, SenseOverlay, facing, ground_point};
use crate::hex::{Hex, steps_covering};
use crate::senses::vision::SeenCells;

/// Half an arm of the X, in shaku, at the faintest and the loudest a cell can be.
///
/// Short and centred at either end, so the cells stay visibly tiled — which is what
/// tells an X apart from vision's hatching, since that runs edge to edge and knits
/// across cells into a continuous field.
///
/// Size carries the falloff, rather than weight or opacity. The polar pattern is
/// continuous, and so is this; weight could only step, and dimming the marks cost them
/// the crispness that makes them read as tiling in the first place. The floor keeps the
/// quietest cells legible instead of letting them thin away into aliasing.
const ARM_MIN: f32 = 0.09;
const ARM_MAX: f32 = 0.30;

const OUTLINE_ALPHA: f32 = 0.85;

/// Hearing, shaped like a microphone's polar pattern.
///
/// `r(θ) = (1 - d) + d·cos θ`, the standard family: `d = 0` is omnidirectional, `d = 0.5`
/// a true cardioid, and above that it narrows toward the front and grows a null behind.
/// One number covers everything from a rabbit's near-spherical awareness to a fox's
/// forward-pointed listening, which is why it is a parameter rather than two shapes.
#[derive(Component, Clone, Debug)]
pub struct Hearing {
    /// How far it reaches at the most sensitive bearing, in whole shaku. Integer for
    /// the same reason a vision band's range is: it is a count of cells on the sense
    /// grid, not a continuous distance.
    pub range: u32,
    /// 0 is a circle, 0.5 a cardioid, approaching 1 an ever tighter forward lobe.
    pub directionality: f32,
}

impl Hearing {
    /// Sensitivity at a bearing given by its cosine, 0..1. Never negative: past a
    /// cardioid the pattern would invert, and a null is silence, not anti-hearing.
    pub fn gain(&self, bearing_cos: f32) -> f32 {
        ((1.0 - self.directionality) + self.directionality * bearing_cos).max(0.0)
    }

    /// How far hearing reaches at that bearing, in shaku. Fractional even though the
    /// range is not: the pattern scales it continuously with bearing.
    pub fn reach(&self, bearing_cos: f32) -> f32 {
        self.range as f32 * self.gain(bearing_cos)
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

/// An X in every cell within earshot, weighted by how well it is heard — except where
/// this being's own vision is already drawn, so what is left is what hearing *adds* to
/// sight rather than a second pattern over the top of the first.
///
/// The exclusion is per being and not global: a rabbit's hearing has no reason to go
/// quiet because a fox happens to be looking that way.
///
/// The outline traces the whole field, vision overlap included. Where hearing reaches is
/// a fact about the ear; not drawing an X there is a choice about clutter, and the
/// boundary should report the first rather than the second.
pub(super) fn draw_hearing(
    mut mid: Gizmos<MidStroke>,
    mut faint: Gizmos<FaintStroke>,
    beings: Query<
        (
            &GlobalTransform,
            &Hearing,
            Option<&SeenCells>,
            &SenseOverlay,
        ),
        Without<HideSenses>,
    >,
) {
    for (transform, hearing, seen, overlay) in &beings {
        let origin = transform.translation();
        let ground = Vec2::new(origin.x, origin.z);
        let forward = facing(transform);
        let here = Hex::from_world(ground);

        // Steps, not shaku — see `hex::steps_covering`. Scanning the range directly
        // leaves notches in the diagonal directions.
        for hex in here.within(steps_covering(hearing.range as f32)) {
            let offset = hex.center() - ground;
            let gain = hearing.gain_at(forward, offset);
            if gain <= 0.0 {
                continue;
            }

            // Against what is actually seen, not the bare cone: inside a shadow sight
            // reaches nothing, so hearing has something to add there.
            let sighted = seen.is_some_and(|cells| cells.contains(hex));
            if !sighted {
                // Hairline and fully opaque, rather than heavy and translucent. Two
                // strokes crossing in a small mark put a lot of ink in one place, so
                // weight is what made these shout; at one pixel they can be solid and
                // still sit behind the hatching. How well the cell is heard is then
                // free to be carried by the size of the mark.
                let arm = ARM_MIN + (ARM_MAX - ARM_MIN) * gain;
                for (a, b) in cross(hex.center(), arm) {
                    faint.line(a, b, overlay.color);
                }
            }

            let outline = overlay.color.with_alpha(OUTLINE_ALPHA);
            for direction in 0..6 {
                let across = hex.neighbour(direction).center() - ground;
                if hearing.gain_at(forward, across) > 0.0 {
                    continue;
                }
                let (a, b) = hex.edge(direction);
                mid.line(ground_point(a), ground_point(b), outline);
            }
        }
    }
}

/// The two arms of an X, centred on a cell, each `arm` long from the centre.
fn cross(centre: Vec2, arm: f32) -> [(Vec3, Vec3); 2] {
    [
        (
            ground_point(centre + Vec2::new(-arm, -arm)),
            ground_point(centre + Vec2::new(arm, arm)),
        ),
        (
            ground_point(centre + Vec2::new(-arm, arm)),
            ground_point(centre + Vec2::new(arm, -arm)),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::super::testing::{FORWARD, at};
    use super::*;

    fn ears(directionality: f32) -> Hearing {
        Hearing {
            range: 30,
            directionality,
        }
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
    fn zero_directionality_hears_equally_in_every_direction() {
        let ears = ears(0.0);
        assert_eq!(ears.gain(1.0), 1.0);
        assert_eq!(ears.gain(0.0), 1.0);
        assert_eq!(ears.gain(-1.0), 1.0);
    }

    /// The defining property of a cardioid: exactly one null, directly behind.
    #[test]
    fn half_directionality_is_a_true_cardioid() {
        let ears = ears(0.5);
        assert_eq!(ears.gain(1.0), 1.0);
        assert_eq!(ears.gain(0.0), 0.5);
        assert_eq!(ears.gain(-1.0), 0.0);
    }

    /// Past a cardioid the polar pattern would go negative behind. Silence is zero;
    /// there is no such thing as anti-hearing.
    #[test]
    fn a_tight_pattern_never_reports_negative_gain() {
        let ears = ears(0.8);
        assert_eq!(ears.gain(-1.0), 0.0);
        assert_eq!(ears.reach(-1.0), 0.0);
    }

    #[test]
    fn hearing_runs_out_at_its_reach_for_that_bearing() {
        let ears = ears(0.5);
        // Sideways the pattern is at half gain, so it reaches half as far.
        assert_eq!(ears.reach(0.0), 15.0);
        assert_close(ears.gain_at(FORWARD, at(90.0, 12.0)), 0.5);
        assert_close(ears.gain_at(FORWARD, at(90.0, 18.0)), 0.0);
    }
}

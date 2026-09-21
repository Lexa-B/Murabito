//! Sight: a cone from the head, with acuity falling off in steps.
//!
//! A pull sense — the perceiver asks what is in front of it, and the answer is recomputed
//! from scratch each time, holding no state between frames. Obstacles will block it
//! outright rather than dimming it, which is what separates it from the other two.

use bevy::prelude::*;

use std::collections::HashMap;
use std::f32::consts::{PI, TAU};

use super::occlusion::{HEIGHTS, Height, Occluders, Opacity};
use super::{
    BoldStroke, FaintStroke, HideSenses, MidStroke, SenseOverlay, facing, ground_point, rotate,
};
use crate::hex::{Hex, steps_covering};

/// Angular slack, in radians, when comparing against a shadow's edge. A cell sitting
/// exactly on a boundary then resolves the same way every time rather than on the last
/// bit of a float, and touching shadows leave no hairline gap for sight to leak through.
const EPSILON: f32 = 1e-6;

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

/// What a being can see: each visible cell, and the band it is seen at.
///
/// Recomputed whole each frame, because nothing moves yet. When something does, this
/// becomes a system gated on the being or an occluder having changed, and the work stops
/// being repeated for an answer that cannot have altered.
#[derive(Component, Default)]
pub struct SeenCells(HashMap<Hex, [Option<u8>; HEIGHTS]>);

impl SeenCells {
    /// The band something of this height is seen at in that cell, if at all.
    ///
    /// Height is asked for rather than assumed because the answer genuinely differs: a
    /// rabbit in long grass is hidden where a person standing in the same cell is not.
    pub fn band(&self, hex: Hex, height: Height) -> Option<usize> {
        self.0
            .get(&hex)
            .and_then(|bands| bands[height.index()])
            .map(usize::from)
    }

    pub fn contains(&self, hex: Hex, height: Height) -> bool {
        self.band(hex, height).is_some()
    }

    /// Every cell where something of this height would be seen, and at what band.
    pub fn iter(&self, height: Height) -> impl Iterator<Item = (Hex, usize)> + '_ {
        self.0.iter().filter_map(move |(hex, bands)| {
            bands[height.index()].map(|band| (*hex, usize::from(band)))
        })
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// An angular interval, in radians, never crossing ±π.
type Arc = (f32, f32);

fn normalize(angle: f32) -> f32 {
    let wrapped = (angle + PI).rem_euclid(TAU) - PI;
    if wrapped <= -PI { PI } else { wrapped }
}

/// Pushes an interval, splitting it if it crosses ±π so that later comparisons are
/// plain ordering rather than modular arithmetic.
fn push_arc(arcs: &mut Vec<Arc>, lo: f32, hi: f32) {
    if lo < -PI {
        arcs.push((lo + TAU, PI));
        arcs.push((-PI, hi));
    } else if hi > PI {
        arcs.push((lo, PI));
        arcs.push((-PI, hi - TAU));
    } else {
        arcs.push((lo, hi));
    }
}

/// The angular extent a cell subtends from `eye`, measured across its six corners.
///
/// A blocker casts from its *whole* span but a cell is judged occluded on its *centre*.
/// That asymmetry is deliberate and is the usual compromise: cast from centres and light
/// leaks past walls, judge on "any corner visible" and almost nothing is ever hidden.
/// The cost is that a cell half in shadow is wholly hidden.
fn subtends(eye: Vec2, hex: Hex) -> Arc {
    let centre = hex.center() - eye;
    let mid = centre.y.atan2(centre.x);
    let (mut lo, mut hi) = (0.0f32, 0.0f32);
    for corner in hex.corners() {
        let to = corner - eye;
        lo = lo.min(normalize(to.y.atan2(to.x) - mid));
        hi = hi.max(normalize(to.y.atan2(to.x) - mid));
    }
    (mid + lo, mid + hi)
}

fn covered_by(arcs: &[Arc], angle: f32) -> usize {
    arcs.iter()
        .filter(|(lo, hi)| angle > lo + EPSILON && angle < hi - EPSILON)
        .count()
}

/// Merges overlapping and touching intervals, so two blockers side by side leave no gap.
fn merge(mut arcs: Vec<Arc>) -> Vec<Arc> {
    arcs.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut merged: Vec<Arc> = Vec::with_capacity(arcs.len());
    for (lo, hi) in arcs {
        match merged.last_mut() {
            Some(last) if lo <= last.1 + EPSILON => last.1 = last.1.max(hi),
            _ => merged.push((lo, hi)),
        }
    }
    merged
}

/// One list per target height class: `blocking[c]` is what hides something of class
/// `c`. An occluder contributes to every class no taller than itself.
type ByHeight<T> = [T; HEIGHTS];

/// Shadowcasting, ring by ring outward.
///
/// Each cell subtends an angle. A blocker adds its span to a shadow list, and shadows
/// raised at one ring only darken rings beyond it — which is what makes this linear in
/// cells rather than cells times blockers, and why the walk has to be in ring order.
///
/// 不透明 stops sight outright. 半透明 costs a band of acuity instead, and costs it
/// again for each one sight passes through, so two thickets are worse than one and a
/// cell demoted past the last band is not seen at all.
///
/// Height enters as *which list an arc goes in*, and nothing else. An occluder subtends
/// the same angle whatever its height; all that changes is who it counts against. So the
/// geometry is computed once and the classes are three counters over it, rather than
/// three casts recomputing identical arcs.
///
/// A blocker standing in shadow casts nothing, there being no line to it — judged at the
/// blocker's own height, since that is the height at which it would have been seen.
pub fn cast(eye: Vec2, forward: Vec2, vision: &Vision, occluders: &Occluders) -> SeenCells {
    let mut seen: HashMap<Hex, ByHeight<Option<u8>>> = HashMap::new();
    let here = Hex::from_world(eye);

    if let Some(band) = vision.band_at(forward, here.center() - eye) {
        seen.insert(here, [Some(band as u8); HEIGHTS]);
    }

    let mut blocking: ByHeight<Vec<Arc>> = Default::default();
    let mut obscuring: ByHeight<Vec<Arc>> = Default::default();
    let mut pending_blocking: ByHeight<Vec<Arc>> = Default::default();
    let mut pending_obscuring: ByHeight<Vec<Arc>> = Default::default();

    for ring in 1..=steps_covering(vision.far_range() as f32) {
        for class in 0..HEIGHTS {
            let raised = std::mem::take(&mut pending_blocking[class]);
            blocking[class] = merge([std::mem::take(&mut blocking[class]), raised].concat());
            // Not merged: overlapping thickets have to count separately, or two would
            // cost no more than one.
            obscuring[class].append(&mut pending_obscuring[class]);
        }

        for hex in here.ring(ring) {
            let offset = hex.center() - eye;
            let angle = normalize(offset.y.atan2(offset.x));
            let blocked: ByHeight<bool> =
                std::array::from_fn(|class| covered_by(&blocking[class], angle) > 0);

            // Cast before judging what is seen: a thing too faint to make out still
            // stands in the way of whatever is behind it.
            let (opacity, height) = occluders.at(hex);
            if opacity != Opacity::Clear && !blocked[height.index()] {
                let (lo, hi) = subtends(eye, hex);
                let raising = match opacity {
                    Opacity::Blocking => &mut pending_blocking,
                    _ => &mut pending_obscuring,
                };
                // Every class no taller than the occluder: what it stands in the way of.
                for arcs in raising.iter_mut().take(height.index() + 1) {
                    push_arc(arcs, lo, hi);
                }
            }

            if let Some(band) = vision.band_at(forward, offset) {
                let bands: ByHeight<Option<u8>> = std::array::from_fn(|class| {
                    if blocked[class] {
                        return None;
                    }
                    let demoted = band + covered_by(&obscuring[class], angle);
                    (demoted < vision.bands.len()).then_some(demoted as u8)
                });
                if bands.iter().any(Option::is_some) {
                    seen.insert(hex, bands);
                }
            }
        }
    }
    SeenCells(seen)
}

/// Recomputes every being's visible set, once, before anything draws it.
pub(super) fn cast_vision(
    occluders: Res<Occluders>,
    mut beings: Query<(&GlobalTransform, &Vision, &mut SeenCells)>,
) {
    for (transform, vision, mut seen) in &mut beings {
        let origin = transform.translation();
        *seen = cast(
            Vec2::new(origin.x, origin.z),
            facing(transform),
            vision,
            &occluders,
        );
    }
}

/// Crosshatch over every visible cell, heavier and denser the sharper the band, with the
/// region outlined along the cells' own edges.
///
/// The outline is boundary-extracted: an edge is stroked when the cell across it is seen
/// at a different band, or not seen at all. So a shadow's edge is drawn by the same rule
/// as the cone's, and the picture is the coverage rather than a curve drawn near it.
pub(super) fn draw_vision(
    mut bold: Gizmos<BoldStroke>,
    mut mid: Gizmos<MidStroke>,
    mut faint: Gizmos<FaintStroke>,
    beings: Query<(&SeenCells, Option<&Height>, &SenseOverlay), Without<HideSenses>>,
) {
    for (seen, height, overlay) in &beings {
        // A being's overlay is drawn for its *own* height: where it could make out
        // something its own size. A fox's cone therefore stops at grass that a person
        // would see straight over.
        let height = height.copied().unwrap_or_default();
        for (hex, band) in seen.iter(height) {
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
                if seen.band(hex.neighbour(direction), height) == Some(band) {
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
mod cast_tests {
    use super::super::occlusion::{Height, Occluders, Opacity};
    use super::*;

    fn eyes() -> Vision {
        Vision {
            arc: 120.0_f32.to_radians(),
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

    /// Cells along one axis of the grid, which are *exactly* collinear with the origin:
    /// `Hex::new(0, -r).center()` is `(-0.5r, -0.866r)`, a straight ray.
    ///
    /// This matters. Placing test cells with `Hex::from_world(forward * n)` snaps each to
    /// whichever cell contains that point, and the small lateral drift is enough to walk
    /// a distant cell out of a near cell's shadow — so the test fails for a reason that
    /// has nothing to do with the code under test.
    fn along(steps: i32) -> Hex {
        Hex::new(0, -steps)
    }

    /// The bearing those cells lie on, as a unit vector.
    fn axis() -> Vec2 {
        along(1).center().normalize()
    }

    /// The baseline: with nothing in the way, sight is exactly the geometry, cell for
    /// cell. If this fails, nothing below means anything.
    #[test]
    fn with_nothing_in_the_way_sight_is_the_bare_cone() {
        let vision = eyes();
        let eye = Vec2::ZERO;
        let seen = cast(eye, axis(), &vision, &Occluders::default());
        let here = Hex::from_world(eye);
        for hex in here.within(steps_covering(vision.far_range() as f32)) {
            assert_eq!(
                seen.band(hex, Height::Full),
                vision.band_at(axis(), hex.center() - eye),
                "{hex:?}"
            );
        }
    }

    #[test]
    fn something_opaque_hides_what_is_behind_it() {
        let vision = eyes();
        let (near, far) = (along(6), along(14));

        let clear = cast(Vec2::ZERO, axis(), &vision, &Occluders::default());
        assert!(
            clear.contains(near, Height::Full) && clear.contains(far, Height::Full),
            "baseline"
        );

        let blocked = cast(
            Vec2::ZERO,
            axis(),
            &vision,
            &Occluders::from_cells([(near, Opacity::Blocking, Height::Full)]),
        );
        assert!(
            blocked.contains(near, Height::Full),
            "the blocker itself is still seen"
        );
        assert!(
            !blocked.contains(far, Height::Full),
            "the cell behind it should be hidden"
        );
    }

    #[test]
    fn something_half_transparent_costs_a_band_instead() {
        let vision = eyes();
        let (near, far) = (along(6), along(10));

        let clear = cast(Vec2::ZERO, axis(), &vision, &Occluders::default());
        let obscured = cast(
            Vec2::ZERO,
            axis(),
            &vision,
            &Occluders::from_cells([(near, Opacity::Obscuring, Height::Full)]),
        );
        assert_eq!(
            obscured.band(far, Height::Full),
            clear.band(far, Height::Full).map(|band| band + 1),
            "one thicket should cost exactly one band"
        );
    }

    /// Demotion accumulates, which is the whole reason it is a count and not a flag.
    #[test]
    fn two_thickets_cost_two_bands() {
        let vision = eyes();
        let far = along(10);

        let clear = cast(Vec2::ZERO, axis(), &vision, &Occluders::default());
        let through = cast(
            Vec2::ZERO,
            axis(),
            &vision,
            &Occluders::from_cells([
                (along(3), Opacity::Obscuring, Height::Full),
                (along(6), Opacity::Obscuring, Height::Full),
            ]),
        );
        assert_eq!(clear.band(far, Height::Full), Some(0), "baseline band");
        assert_eq!(through.band(far, Height::Full), Some(2));
    }

    /// Demoted past the last band is not seen at all.
    #[test]
    fn enough_thickets_hide_a_thing_outright() {
        let vision = eyes();
        let thickets = [2, 4, 6].map(|steps| (along(steps), Opacity::Obscuring, Height::Full));

        let through = cast(
            Vec2::ZERO,
            axis(),
            &vision,
            &Occluders::from_cells(thickets),
        );
        assert!(!through.contains(along(10), Height::Full));
    }

    /// The point of the height axis: the same grass, the same cell, two different
    /// answers depending on what is standing there.
    #[test]
    fn knee_high_grass_hides_a_rabbit_and_not_a_person() {
        let vision = eyes();
        let far = along(10);
        let grass = Occluders::from_cells([(along(4), Opacity::Obscuring, Height::Knee)]);

        let bare = cast(Vec2::ZERO, axis(), &vision, &Occluders::default());
        let through = cast(Vec2::ZERO, axis(), &vision, &grass);

        assert_eq!(
            through.band(far, Height::Knee),
            bare.band(far, Height::Knee).map(|band| band + 1),
            "a rabbit behind grass is harder to make out"
        );
        assert_eq!(
            through.band(far, Height::Full),
            bare.band(far, Height::Full),
            "a person standing in it is not"
        );
    }

    /// And a waist-high thicket catches the middle: it costs sight of a wolf as well as
    /// of a rabbit, but a standing person is still in plain view.
    #[test]
    fn a_waist_high_thicket_reaches_further_up_than_grass_does() {
        let vision = eyes();
        let far = along(10);
        let bush = Occluders::from_cells([(along(4), Opacity::Obscuring, Height::Waist)]);

        let bare = cast(Vec2::ZERO, axis(), &vision, &Occluders::default());
        let through = cast(Vec2::ZERO, axis(), &vision, &bush);

        for height in [Height::Knee, Height::Waist] {
            assert_eq!(
                through.band(far, height),
                bare.band(far, height).map(|band| band + 1),
                "{height:?} should be obscured"
            );
        }
        assert_eq!(
            through.band(far, Height::Full),
            bare.band(far, Height::Full)
        );
    }

    /// Obscuring is not blocking: sight still reaches the tree beyond the thicket, so the
    /// tree still casts and what is behind *it* is hidden outright rather than demoted.
    #[test]
    fn a_thicket_does_not_stop_a_tree_behind_it_from_casting() {
        let vision = eyes();
        let seen = cast(
            Vec2::ZERO,
            axis(),
            &vision,
            &Occluders::from_cells([
                (along(4), Opacity::Obscuring, Height::Full),
                (along(8), Opacity::Blocking, Height::Full),
            ]),
        );
        assert!(
            seen.contains(along(8), Height::Full),
            "the tree is seen through the thicket"
        );
        assert!(
            !seen.contains(along(14), Height::Full),
            "and hides what is behind it"
        );
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

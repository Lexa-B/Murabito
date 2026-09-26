//! Sight: a cone from the eye, with acuity falling off in three steps, stopped by
//! whatever stands in the way.
//!
//! A pull sense: each tick a sighted body asks what is in front of it, and the answer is
//! computed from scratch, holding nothing between ticks. What it reports is its own
//! list, [`Seen`]: each thing in view, by the engine's handle and by its `ThingId`,
//! where it is relative to the looker, and how well it was made out. The field the cast computes on the way, which cells are in view, is
//! this crate's business and nobody else's.
//!
//! Everything is opaque for now: any thing standing in a voxel stops sight through it, a
//! fox as much as a tree. Obscuring, heights and the blurring of a poorly seen thing into
//! something vaguer are facet debt, taken up in `murabito_perception` when facets return.
//! `docs/perception_readme.md` is the design.

use std::f32::consts::{PI, TAU};

use bevy::prelude::*;
use murabito_hexcoords::{Direction, Offset, VoxelCoord, rings_covering};
use murabito_identity::ThingId;
use murabito_perception::{Occupancy, PerceptionSet};
use murabito_placement::{Facing, VoxelPosition};

pub struct VisionPlugin;

impl Plugin for VisionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, look.in_set(PerceptionSet::Sense));
        #[cfg(feature = "debug")]
        app.register_type::<Vision>().register_type::<Seen>();
    }
}

/// Angular slack, in radians, at a shadow's edge. A cell centre sitting exactly on the
/// edge then resolves the same way every time rather than on the last bit of a float,
/// and touching shadows leave no hairline for sight to leak through.
const EDGE: f32 = 1e-5;

/// How well something was made out: which of the three bands it was seen in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "debug", derive(Reflect))]
pub enum Acuity {
    Near,
    Mid,
    Far,
}

impl Acuity {
    pub const ALL: [Self; 3] = [Self::Near, Self::Mid, Self::Far];
}

/// One band of a cone: everything out to `range` that isn't in a nearer band.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "debug", derive(Reflect))]
pub struct Band {
    /// Outer edge, in whole shaku: a range is a count of cells, not a length.
    pub range: u32,
    /// How well something in this band registers, 0..1. For tuning; the cast reports
    /// the band, not the number.
    pub sensitivity: f32,
}

/// A cone of sight, centred on where the body faces, with acuity falling off in three
/// steps rather than a curve: the steps are what perception branches on, and a discrete
/// fall-off is easier to reason about when something behaves oddly.
///
/// A member of `Sentient`: a species puts its own cone in its own `require`, or
/// [`Vision::BLIND`] if it has none.
#[derive(Component, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "debug", derive(Reflect), reflect(Component))]
#[require(Seen)]
pub struct Vision {
    /// Full width, in degrees. The cone spans half this either side of forward.
    pub arc: f32,
    /// Near to far. Ranges must not decrease; sensitivities are expected to fall.
    pub bands: [Band; 3],
}

impl Vision {
    /// No sight at all: a species that cannot see requires this. The cast returns
    /// nothing, not even the eye's own cell.
    pub const BLIND: Self = Self {
        arc: 0.0,
        bands: [Band {
            range: 0,
            sensitivity: 0.0,
        }; 3],
    };

    pub fn is_blind(&self) -> bool {
        self.far_range() == 0
    }

    /// The outer edge of the furthest band, in shaku.
    pub fn far_range(&self) -> u32 {
        self.bands[2].range
    }

    pub fn sensitivity(&self, acuity: Acuity) -> f32 {
        self.bands[acuity as usize].sensitivity
    }

    /// Half the cone's width, in radians: the widest bearing still inside it.
    fn half_arc(&self) -> f32 {
        self.arc.to_radians() * 0.5
    }

    /// Which band something at `offset` from the eye is seen in, for an eye facing
    /// `forward`, or `None` if it is outside the cone or past the last band. Geometry
    /// only: nothing here knows what stands in the way. A cell on the cone's edge is
    /// inside it, and one on a band's edge is in the nearer band.
    ///
    /// Both vectors are in the ground plane, world XZ, and `forward` is a unit vector.
    fn acuity_at(&self, forward: Vec2, offset: Vec2) -> Option<Acuity> {
        let distance = offset.length();
        if self.is_blind() || distance > self.far_range() as f32 + EDGE {
            return None;
        }
        // The eye's own cell has no bearing, and is in view.
        if distance > EDGE {
            let bearing = forward.dot(offset / distance).clamp(-1.0, 1.0).acos();
            if bearing > self.half_arc() + EDGE {
                return None;
            }
        }
        Acuity::ALL
            .into_iter()
            .find(|band| distance <= self.bands[*band as usize].range as f32 + EDGE)
    }
}

/// What a sighted body sees this tick: each thing in view, where it is relative to the
/// looker, and how well it was made out. Replaced whole every tick, and empty for a body
/// that is blind or has nothing in view.
#[derive(Component, Debug, Default)]
#[cfg_attr(feature = "debug", derive(Reflect), reflect(Component))]
pub struct Seen(Vec<Sighting>);

/// One thing seen, this tick. It is named twice: `entity` is the handle the engine acts
/// through, good for this run only; `id` is the thing's number for life, what anything
/// that remembers it across ticks and saves keys on.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "debug", derive(Reflect))]
pub struct Sighting {
    pub entity: Entity,
    pub id: ThingId,
    /// Where it is relative to the looker: `its voxel - mine`.
    pub offset: Offset,
    pub acuity: Acuity,
}

impl FromIterator<Sighting> for Seen {
    /// A list of sightings as given, for whatever tests what reads one.
    fn from_iter<I: IntoIterator<Item = Sighting>>(sightings: I) -> Self {
        Self(sightings.into_iter().collect())
    }
}

impl Seen {
    pub fn iter(&self) -> impl Iterator<Item = &Sighting> {
        self.0.iter()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// How well this thing was seen, if it was seen at all.
    pub fn sees(&self, entity: Entity) -> Option<Acuity> {
        self.0
            .iter()
            .find(|sighting| sighting.entity == entity)
            .map(|sighting| sighting.acuity)
    }
}

/// Every sighted body looks, once, after the map of what stands where is gathered:
/// its `Seen` is replaced with what is in view this tick. A blind body's list is empty.
///
/// Everything with a place is a thing and has its number; one without is a spawner's
/// mistake (spawn things as kinds), and this says so rather than leaving it unseen.
fn look(
    occupancy: Res<Occupancy>,
    ids: Query<&ThingId>,
    mut lookers: Query<(Entity, &VoxelPosition, &Facing, &Vision, &mut Seen)>,
) {
    let id_of = |thing: Entity| {
        *ids.get(thing)
            .expect("a thing with a place has a ThingId: was it spawned as a kind?")
    };
    for (looker, position, facing, vision, mut seen) in &mut lookers {
        let in_view = cast(position.0, facing.0, vision, &occupancy);
        seen.0 = sightings(looker, position.0, &in_view, &occupancy, id_of);
    }
}

/// What stands in the cells in view, as sightings relative to the looker. Two things in
/// one cell are two sightings; the looker never sees itself.
fn sightings(
    looker: Entity,
    eye: VoxelCoord,
    in_view: &[(VoxelCoord, Acuity)],
    occupancy: &Occupancy,
    id_of: impl Fn(Entity) -> ThingId,
) -> Vec<Sighting> {
    in_view
        .iter()
        .flat_map(|(cell, acuity)| {
            occupancy
                .at(*cell)
                .iter()
                .filter(move |thing| **thing != looker)
                .map(|thing| Sighting {
                    entity: *thing,
                    id: id_of(*thing),
                    offset: *cell - eye,
                    acuity: *acuity,
                })
        })
        .collect()
}

/// A point of world space in the ground plane: XZ, with north −Z.
fn ground(point: Vec3) -> Vec2 {
    Vec2::new(point.x, point.z)
}

/// The angle of a ground-plane vector, in `(-π, π]`.
fn angle_of(v: Vec2) -> f32 {
    normalize(v.y.atan2(v.x))
}

fn normalize(angle: f32) -> f32 {
    let wrapped = (angle + PI).rem_euclid(TAU) - PI;
    if wrapped <= -PI { PI } else { wrapped }
}

/// An angular interval, in radians, never crossing ±π.
type Arc = (f32, f32);

/// Pushes an interval, split if it crosses ±π, so that later comparisons are plain
/// ordering rather than modular arithmetic.
fn push_arc(arcs: &mut Vec<Arc>, (lo, hi): Arc) {
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

/// The angle a cell subtends from `eye`, measured across its six corners: what it hides
/// when it blocks. It casts from its whole span while a cell is judged hidden on its
/// centre; that asymmetry is the usual compromise, and its cost is that a cell half in
/// shadow is wholly hidden.
fn subtends(eye: Vec2, cell: VoxelCoord) -> Arc {
    let mid = angle_of(ground(cell.to_world()) - eye);
    let (mut lo, mut hi) = (0.0f32, 0.0f32);
    for corner in cell.corners() {
        let off = normalize(angle_of(ground(corner) - eye) - mid);
        lo = lo.min(off);
        hi = hi.max(off);
    }
    (mid + lo, mid + hi)
}

fn in_shadow(shadows: &[Arc], angle: f32) -> bool {
    shadows
        .iter()
        .any(|(lo, hi)| angle > lo + EDGE && angle < hi - EDGE)
}

/// Merges overlapping and touching intervals, so two blockers side by side leave no gap.
fn merged(mut arcs: Vec<Arc>) -> Vec<Arc> {
    arcs.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut merged: Vec<Arc> = Vec::with_capacity(arcs.len());
    for (lo, hi) in arcs {
        match merged.last_mut() {
            Some(last) if lo <= last.1 + EDGE => last.1 = last.1.max(hi),
            _ => merged.push((lo, hi)),
        }
    }
    merged
}

/// Which way an eye at `eye` facing `facing` looks, in the ground plane: toward the
/// neighbour that way, so the cone points where the body's heading does with no angle
/// arithmetic to get wrong.
fn forward_of(eye: VoxelCoord, facing: Direction) -> Vec2 {
    (ground(eye.neighbour(facing).to_world()) - ground(eye.to_world())).normalize()
}

/// The cells an eye at `eye` facing `facing` can see, and how well: the field of view,
/// this crate's own intermediate. Shadowcasting, ring by ring outward, on the eye's
/// layer.
///
/// Each occupied cell is a blocker and casts a shadow across the angle it subtends;
/// shadows raised at one ring darken only the rings beyond it, which is what makes the
/// walk linear in cells rather than cells times blockers, and why it must go in ring
/// order. A blocker is itself in view, so a fox sees the tree and not the hare behind
/// it; a blocker standing in shadow casts nothing, there being no line to it. The eye's
/// own cell is always in view and is never a blocker.
fn cast(
    eye: VoxelCoord,
    facing: Direction,
    vision: &Vision,
    occupancy: &Occupancy,
) -> Vec<(VoxelCoord, Acuity)> {
    if vision.is_blind() {
        return Vec::new();
    }
    let here = ground(eye.to_world());
    let forward = forward_of(eye, facing);
    let mut in_view = vec![(eye, Acuity::Near)];

    let mut shadows: Vec<Arc> = Vec::new();
    let mut raised: Vec<Arc> = Vec::new();
    for ring in 1..=rings_covering(vision.far_range() as f32) {
        shadows = merged([shadows, std::mem::take(&mut raised)].concat());
        for cell in eye.ring(ring) {
            let offset = ground(cell.to_world()) - here;
            if in_shadow(&shadows, angle_of(offset)) {
                continue;
            }
            // Cast before judging: a thing too far to make out still stands in the way
            // of whatever is behind it.
            if occupancy.is_occupied(cell) {
                push_arc(&mut raised, subtends(here, cell));
            }
            if let Some(acuity) = vision.acuity_at(forward, offset) {
                in_view.push((cell, acuity));
            }
        }
    }
    in_view
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn voxel(q: i32, r: i32) -> VoxelCoord {
        VoxelCoord::new(q, r, -q - r, 0).expect("on the plane")
    }

    /// A cone like the archive's fox: 120° wide, sharp to 12, then 30, then 48 shaku.
    fn eyes() -> Vision {
        Vision {
            arc: 120.0,
            bands: [
                Band {
                    range: 12,
                    sensitivity: 1.0,
                },
                Band {
                    range: 30,
                    sensitivity: 0.6,
                },
                Band {
                    range: 48,
                    sensitivity: 0.3,
                },
            ],
        }
    }

    /// A short cone, so a test's numbers stay small: 120° wide, bands to 2, 4 and 6.
    fn short_eyes() -> Vision {
        Vision {
            arc: 120.0,
            bands: [
                Band {
                    range: 2,
                    sensitivity: 1.0,
                },
                Band {
                    range: 4,
                    sensitivity: 0.5,
                },
                Band {
                    range: 6,
                    sensitivity: 0.2,
                },
            ],
        }
    }

    fn nothing() -> Occupancy {
        Occupancy::default()
    }

    fn things_at(cells: &[VoxelCoord]) -> Occupancy {
        let mut world = World::new();
        Occupancy::from_cells(cells.iter().map(|cell| (*cell, world.spawn_empty().id())))
    }

    fn view(
        eye: VoxelCoord,
        facing: Direction,
        vision: &Vision,
        occupancy: &Occupancy,
    ) -> HashMap<VoxelCoord, Acuity> {
        cast(eye, facing, vision, occupancy).into_iter().collect()
    }

    #[test]
    fn a_blind_body_sees_nothing_not_even_its_own_cell() {
        assert!(Vision::BLIND.is_blind());
        assert!(cast(voxel(0, 0), Direction::E, &Vision::BLIND, &nothing()).is_empty());
    }

    #[test]
    fn a_sighted_body_is_not_blind_and_each_band_has_its_own_sensitivity() {
        let eyes = eyes();

        assert!(!eyes.is_blind());
        assert_eq!(eyes.far_range(), 48);
        assert_eq!(eyes.sensitivity(Acuity::Near), 1.0);
        assert_eq!(eyes.sensitivity(Acuity::Far), 0.3);
    }

    #[test]
    fn the_eyes_own_cell_is_in_view_near() {
        let seen = view(voxel(3, -1), Direction::N, &short_eyes(), &nothing());

        assert_eq!(seen.get(&voxel(3, -1)), Some(&Acuity::Near));
    }

    #[test]
    fn with_nothing_in_the_way_sight_is_the_bare_cone() {
        let eye = voxel(0, 0);
        let eyes = eyes();
        let forward = forward_of(eye, Direction::NNE);

        let seen = view(eye, Direction::NNE, &eyes, &nothing());

        // Every cell that could be within range, judged by the geometry alone.
        let mut expected = HashMap::new();
        let reach = rings_covering(eyes.far_range() as f32) as i32 + 2;
        for q in -reach..=reach {
            for r in -reach..=reach {
                let cell = voxel(q, r);
                let offset = ground(cell.to_world()) - ground(eye.to_world());
                if let Some(acuity) = eyes.acuity_at(forward, offset) {
                    expected.insert(cell, acuity);
                }
            }
        }
        assert_eq!(seen, expected);
        assert!(
            seen.len() > 1000,
            "only {} cells in a 48-shaku cone",
            seen.len()
        );
    }

    #[test]
    fn the_cone_follows_the_facing() {
        let eye = voxel(0, 0);
        let east = voxel(3, 0);

        let facing_east = view(eye, Direction::E, &short_eyes(), &nothing());
        let facing_west = view(eye, Direction::W, &short_eyes(), &nothing());

        assert!(facing_east.contains_key(&east));
        assert!(!facing_west.contains_key(&east));
        assert!(facing_west.contains_key(&voxel(-3, 0)));
    }

    #[test]
    fn a_cell_on_a_bands_edge_is_in_the_nearer_band() {
        let seen = view(voxel(0, 0), Direction::E, &short_eyes(), &nothing());

        assert_eq!(seen.get(&voxel(2, 0)), Some(&Acuity::Near));
        assert_eq!(seen.get(&voxel(3, 0)), Some(&Acuity::Mid));
        assert_eq!(seen.get(&voxel(6, 0)), Some(&Acuity::Far));
        assert_eq!(seen.get(&voxel(7, 0)), None);
    }

    #[test]
    fn a_cell_on_the_cones_edge_is_inside_it() {
        // Facing east with a 120° cone, the edge runs exactly along NNE and SSE.
        let eye = voxel(0, 0);
        let seen = view(eye, Direction::E, &short_eyes(), &nothing());

        assert!(seen.contains_key(&eye.neighbour(Direction::NNE).neighbour(Direction::NNE)));
        assert!(seen.contains_key(&eye.neighbour(Direction::SSE).neighbour(Direction::SSE)));
        assert!(!seen.contains_key(&eye.neighbour(Direction::N)));
    }

    #[test]
    fn something_opaque_hides_what_is_behind_it_and_not_what_is_beside_it() {
        let eye = voxel(0, 0);
        let tree = voxel(3, 0);

        let seen = view(eye, Direction::E, &short_eyes(), &things_at(&[tree]));

        assert_eq!(
            seen.get(&tree),
            Some(&Acuity::Mid),
            "the tree itself is seen"
        );
        assert_eq!(seen.get(&voxel(4, 0)), None, "straight behind it");
        assert_eq!(seen.get(&voxel(6, 0)), None, "further behind it");
        assert!(seen.contains_key(&voxel(5, -2)), "off to one side, past it");
        assert!(seen.contains_key(&voxel(3, 2)), "off to the other side");
    }

    #[test]
    fn two_blockers_side_by_side_leave_no_gap_between_their_shadows() {
        let eye = voxel(0, 0);
        let wall = [voxel(3, -1), voxel(2, 0), voxel(2, 1)];

        let seen = view(eye, Direction::E, &short_eyes(), &things_at(&wall));

        for behind in [
            voxel(5, -2),
            voxel(4, -1),
            voxel(4, 0),
            voxel(3, 1),
            voxel(3, 2),
        ] {
            assert_eq!(seen.get(&behind), None, "{behind:?} shows through the wall");
        }
    }

    #[test]
    fn a_blocker_standing_in_shadow_casts_nothing() {
        let eye = voxel(0, 0);
        let near = voxel(2, 0);
        // Directly behind `near` on the same line, so it is hidden; its own shadow, cast
        // from a longer baseline, would be narrower than `near`'s and change nothing.
        // So the test uses one off the line: hidden by `near`, yet its span would reach
        // past `near`'s shadow on one side if it were allowed to cast.
        let hidden = voxel(5, -1);
        let here = ground(eye.to_world());
        let (near_lo, _) = subtends(here, near);
        let (hidden_lo, _) = subtends(here, hidden);
        assert!(
            hidden_lo < near_lo,
            "the test's blocker would widen the shadow"
        );

        let with = view(
            eye,
            Direction::E,
            &short_eyes(),
            &things_at(&[near, hidden]),
        );
        let without = view(eye, Direction::E, &short_eyes(), &things_at(&[near]));

        assert_eq!(with.get(&hidden), None);
        let mut without_hidden = without.clone();
        without_hidden.remove(&hidden);
        assert_eq!(with, without_hidden);
    }

    #[test]
    fn a_thing_in_the_eyes_own_cell_does_not_block_the_view() {
        let eye = voxel(0, 0);

        let seen = view(eye, Direction::E, &short_eyes(), &things_at(&[eye]));

        assert_eq!(seen, view(eye, Direction::E, &short_eyes(), &nothing()));
    }

    #[test]
    fn a_thing_out_of_the_cone_still_casts_its_shadow_where_it_is() {
        // A blocker behind the looker shadows only what is behind it, which is out of the
        // cone anyway: the view is exactly the bare cone.
        let eye = voxel(0, 0);

        let seen = view(
            eye,
            Direction::E,
            &short_eyes(),
            &things_at(&[voxel(-2, 0)]),
        );

        assert_eq!(seen, view(eye, Direction::E, &short_eyes(), &nothing()));
    }

    use murabito_identity::{IdentityPlugin, NextThingId};
    use murabito_movement::{Locomotion, MovementPlugin, Step};
    use murabito_perception::PerceptionPlugin;
    use murabito_progress::ProgressPlugin;

    /// A headless app whose every `update` is exactly one 64 Hz tick, with the
    /// mechanisms and perception running. An app's first update only starts its clock,
    /// so it is spent here.
    fn ticking_app() -> App {
        use bevy::time::TimeUpdateStrategy;

        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            IdentityPlugin,
            ProgressPlugin,
            MovementPlugin,
            PerceptionPlugin,
            VisionPlugin,
        ));
        app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
        app.update();
        app
    }

    /// The tests spawn no kinds, so they stamp the numbers themselves.
    fn mint(app: &mut App) -> ThingId {
        app.world_mut().resource_mut::<NextThingId>().mint()
    }

    fn looker_at(app: &mut App, cell: VoxelCoord, facing: Direction, vision: Vision) -> Entity {
        let id = mint(app);
        app.world_mut()
            .spawn((id, VoxelPosition(cell), Facing(facing), vision))
            .id()
    }

    fn thing_at(app: &mut App, cell: VoxelCoord) -> Entity {
        let id = mint(app);
        app.world_mut().spawn((id, VoxelPosition(cell))).id()
    }

    fn id_of(app: &App, thing: Entity) -> ThingId {
        *app.world().get::<ThingId>(thing).expect("stamped at spawn")
    }

    /// Through the serializer the debug server uses: a sighting on the wire is the four
    /// fields as they are, the offset in its axial form, the entity as one number. A
    /// one-field tuple struct (`Seen`, `ThingId`) is flattened to its field.
    #[cfg(feature = "debug")]
    #[test]
    fn on_the_wire_a_sighting_is_its_four_fields_as_they_are() {
        use bevy::reflect::serde::ReflectSerializer;
        use serde_json::json;

        let mut app = ticking_app();
        let fox = looker_at(&mut app, voxel(0, 0), Direction::E, eyes());
        let hare = thing_at(&mut app, voxel(2, 0));
        app.update();

        let registry = app.world().resource::<AppTypeRegistry>().read();
        let wire = serde_json::to_value(ReflectSerializer::new(seen_by(&app, fox), &registry))
            .expect("serialises");
        println!("WIRE {wire}");
        assert_eq!(
            wire,
            json!({"murabito_vision::Seen": [{
                "entity": hare.to_bits(),
                "id": id_of(&app, hare).number(),
                "offset": {"dq": 2, "dr": 0, "dlayer": 0},
                "acuity": "Near",
            }]})
        );
    }

    fn seen_by(app: &App, looker: Entity) -> &Seen {
        app.world()
            .get::<Seen>(looker)
            .expect("a sighted body has a Seen")
    }

    #[test]
    fn a_sighted_body_has_a_list_without_asking_and_it_starts_empty() {
        let mut app = ticking_app();
        let fox = looker_at(&mut app, voxel(0, 0), Direction::E, short_eyes());

        assert!(seen_by(&app, fox).is_empty());
    }

    #[test]
    fn a_hare_in_front_of_a_fox_is_seen_on_the_first_tick_near_and_where_it_is() {
        let mut app = ticking_app();
        let fox = looker_at(&mut app, voxel(0, 0), Direction::E, short_eyes());
        let hare = thing_at(&mut app, voxel(2, 0));

        app.update();

        let seen = seen_by(&app, fox);
        assert_eq!(seen.len(), 1);
        assert_eq!(seen.sees(hare), Some(Acuity::Near));
        let sighting = seen.iter().next().expect("one sighting");
        assert_eq!(sighting.offset, voxel(2, 0) - voxel(0, 0));
        assert_eq!((sighting.offset.dq(), sighting.offset.dr()), (2, 0));
    }

    #[test]
    fn a_sighting_names_the_thing_by_handle_and_by_number() {
        let mut app = ticking_app();
        let fox = looker_at(&mut app, voxel(0, 0), Direction::E, eyes());
        let hare = thing_at(&mut app, voxel(2, 0));
        app.update();

        let sighting = *seen_by(&app, fox).iter().next().expect("the hare");
        assert_eq!(sighting.entity, hare);
        assert_eq!(sighting.id, id_of(&app, hare));
        assert_ne!(sighting.id, id_of(&app, fox));
    }

    #[test]
    fn a_tree_between_them_hides_the_hare_and_the_fox_sees_the_tree() {
        let mut app = ticking_app();
        let fox = looker_at(&mut app, voxel(0, 0), Direction::E, short_eyes());
        let tree = thing_at(&mut app, voxel(2, 0));
        let hare = thing_at(&mut app, voxel(4, 0));

        app.update();

        let seen = seen_by(&app, fox);
        assert_eq!(seen.sees(tree), Some(Acuity::Near));
        assert_eq!(seen.sees(hare), None);
    }

    #[test]
    fn the_looker_never_sees_itself() {
        let mut app = ticking_app();
        let fox = looker_at(&mut app, voxel(0, 0), Direction::E, short_eyes());

        app.update();

        assert_eq!(seen_by(&app, fox).sees(fox), None);
        assert!(seen_by(&app, fox).is_empty());
    }

    #[test]
    fn a_thing_sharing_the_lookers_cell_is_seen_near_at_no_offset() {
        let mut app = ticking_app();
        let fox = looker_at(&mut app, voxel(0, 0), Direction::E, short_eyes());
        let flea = thing_at(&mut app, voxel(0, 0));

        app.update();

        let seen = seen_by(&app, fox);
        assert_eq!(seen.sees(flea), Some(Acuity::Near));
        assert_eq!(seen.iter().next().map(|s| s.offset), Some(Offset::ZERO));
    }

    #[test]
    fn a_blind_body_sees_nothing_however_close() {
        let mut app = ticking_app();
        let mole = looker_at(&mut app, voxel(0, 0), Direction::E, Vision::BLIND);
        thing_at(&mut app, voxel(1, 0));

        app.update();

        assert!(seen_by(&app, mole).is_empty());
    }

    #[test]
    fn two_lookers_each_see_the_other_and_each_has_its_own_list() {
        let mut app = ticking_app();
        let fox = looker_at(&mut app, voxel(0, 0), Direction::E, short_eyes());
        let hare = looker_at(&mut app, voxel(3, 0), Direction::W, short_eyes());

        app.update();

        assert_eq!(seen_by(&app, fox).sees(hare), Some(Acuity::Mid));
        assert_eq!(seen_by(&app, hare).sees(fox), Some(Acuity::Mid));
        assert_eq!(
            seen_by(&app, hare)
                .iter()
                .next()
                .map(|s| (s.offset.dq(), s.offset.dr())),
            Some((-3, 0))
        );
    }

    #[test]
    fn a_hare_stepping_out_of_the_cone_is_gone_on_the_tick_its_step_lands() {
        let mut app = ticking_app();
        let fox = looker_at(&mut app, voxel(0, 0), Direction::E, short_eyes());
        // Two cells east and facing north: its first step north takes it to (3, -2),
        // which is 30° above the fox's eastward line and still inside the 120° cone. So
        // the hare starts on the cone's edge instead, at NNE twice, and steps NNW out.
        let edge = voxel(0, 0)
            .neighbour(Direction::NNE)
            .neighbour(Direction::NNE);
        let id = mint(&mut app);
        let hare = app
            .world_mut()
            .spawn((
                id,
                VoxelPosition(edge),
                Facing(Direction::NNW),
                Locomotion {
                    speed: 4.0,
                    turn_speed: 180.0,
                },
                Step(Direction::NNW),
            ))
            .id();

        // An edge step at 4 shaku/s lands on tick 16.
        (0..15).for_each(|_| app.update());
        assert_eq!(
            seen_by(&app, fox).sees(hare),
            Some(Acuity::Near),
            "still on the edge"
        );

        app.update();

        assert_eq!(
            seen_by(&app, fox).sees(hare),
            None,
            "stepped out on tick 16"
        );
    }
}

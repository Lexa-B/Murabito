//! Reflexes: what a body does without thinking.
//!
//! The catalogue of every reflex a sentient body could have, each with its own code, and
//! the [`Reflexes`] a body carries: which of them it has, with what dials, at what
//! priority. A kind gives its repertoire in its own `require`; an individual may be
//! spawned with the dials turned, "a tad jumpy", since what is given at spawn wins.
//!
//! A reflex is one trigger to one response, named `category_reaction_trigger`, and it
//! answers only a [`Short`]: something over within a round of the slower mind. That is
//! the type's promise, not a rule to remember. Every tick, in the brainstem's
//! `Reflexes` slot, each body's repertoire is checked against what it sees now and what
//! it saw last tick; the highest-priority match preempts the brainstem, cancelling
//! whatever the body was doing, by the reflex's name, so the mind learns what took its
//! body. Ties go to the earlier entry.
//!
//! What a body reveals by its own turning is not an apparition: a tick on which the body
//! faces a new way is remembered but never fires. The rest of that judgement, what is
//! newly *noticed* against what is newly *known*, wants a believed world the body carries
//! with it, which is later work (`TODO.md`). Design: `docs/ai_readme.md`.

use bevy::prelude::*;
use murabito_brainstem::{Brainstem, BrainstemSet, Short};
use murabito_hexcoords::Direction;
use murabito_identity::{Kind, ThingId};
use murabito_placement::Facing;
use murabito_vision::{Acuity, Seen};

pub struct ReflexesPlugin;

impl Plugin for ReflexesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, twitch.in_set(BrainstemSet::Reflexes));
        #[cfg(feature = "debug")]
        app.register_type::<Reflexes>();
    }
}

/// The catalogue. Each is one trigger to one response, with its dials as its fields, and
/// its code is [`Reflex::check`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "debug", derive(Reflect))]
pub enum Reflex {
    /// Startle: something of a kind that matters is in view at the nearest acuity this
    /// tick and was not in view at all last tick. The body turns to face it; the
    /// nearest, if several appeared.
    StartleFaceApparition {
        /// What can startle this body: anything of this kind or under it. A tree can't.
        by: Kind,
    },
}

impl Reflex {
    /// The name the outcome carries when this reflex takes the body.
    pub fn name(self) -> &'static str {
        match self {
            Self::StartleFaceApparition { .. } => "startle_face_apparition",
        }
    }

    /// Whether this is the same reflex as another, whatever their dials.
    pub fn is(self, other: Reflex) -> bool {
        std::mem::discriminant(&self) == std::mem::discriminant(&other)
    }

    /// This reflex, wired at a priority: one entry of a repertoire.
    pub fn at(self, priority: u8) -> Wired {
        Wired {
            reflex: self,
            priority,
        }
    }

    /// Whether this reflex fires, given what the body sees now, the ids it saw last
    /// tick, and a way to ask what kind each seen thing is, and what it does if so. Pure:
    /// the whole of a reflex's behaviour, testable on two lists.
    pub fn check(
        self,
        now: &Seen,
        before: &[ThingId],
        kind_of: &dyn Fn(Entity) -> Option<Kind>,
    ) -> Option<Short> {
        match self {
            Self::StartleFaceApparition { by } => now
                .iter()
                .filter(|sighting| sighting.acuity == Acuity::Near)
                .filter(|sighting| !before.contains(&sighting.id))
                .filter(|sighting| {
                    kind_of(sighting.entity).is_some_and(|kind| kind == by || kind.is_under(by))
                })
                .min_by_key(|sighting| sighting.offset.steps())
                .map(|apparition| Short::FaceThing(apparition.id)),
        }
    }
}

/// One entry of a repertoire: a reflex, its dials, and how it ranks against the body's
/// others.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "debug", derive(Reflect))]
pub struct Wired {
    pub reflex: Reflex,
    /// Higher fires first when several would. A dial: the kind's number by default, an
    /// individual's if given at spawn.
    pub priority: u8,
}

/// A body's repertoire: the reflexes it has, with their dials. Empty by default, so a
/// sentient whose kind names none has none. Requires the body's memory of its last look.
#[derive(Component, Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "debug", derive(Reflect), reflect(Component))]
#[require(LastLook)]
pub struct Reflexes(Vec<Wired>);

impl Reflexes {
    pub fn new(wired: impl IntoIterator<Item = Wired>) -> Self {
        Self(wired.into_iter().collect())
    }

    pub fn none() -> Self {
        Self::default()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Wired> {
        self.0.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The same repertoire with one reflex's priority turned: an individual's dial. The
    /// reflex is matched whatever its own dials say.
    pub fn tuned(mut self, reflex: Reflex, priority: u8) -> Self {
        for wired in &mut self.0 {
            if wired.reflex.is(reflex) {
                wired.priority = priority;
            }
        }
        self
    }

    /// The reflex that takes the body this tick, if any: the highest-priority one that
    /// fires, the earlier entry on a tie.
    fn pick(
        &self,
        now: &Seen,
        before: &[ThingId],
        kind_of: &dyn Fn(Entity) -> Option<Kind>,
    ) -> Option<(Reflex, Short)> {
        let mut winner: Option<(Wired, Short)> = None;
        for &wired in &self.0 {
            let Some(short) = wired.reflex.check(now, before, kind_of) else {
                continue;
            };
            if winner.is_none_or(|(best, _)| wired.priority > best.priority) {
                winner = Some((wired, short));
            }
        }
        winner.map(|(wired, short)| (wired.reflex, short))
    }
}

/// What the body saw last tick, by id, and which way it was facing when it looked.
/// `None` before it has ever looked, so nothing startles at the world's first sight of it.
#[derive(Component, Debug, Default, Clone, PartialEq, Eq)]
pub struct LastLook(Option<Look>);

#[derive(Debug, Clone, PartialEq, Eq)]
struct Look {
    ids: Vec<ThingId>,
    facing: Direction,
}

/// Everything of a body that twitch touches. The id is only for the trace.
type Twitched<'a> = (
    Option<&'a ThingId>,
    &'a Reflexes,
    Ref<'a, Seen>,
    &'a Facing,
    &'a mut LastLook,
    &'a mut Brainstem,
);

/// Runs every body's repertoire against this tick's view and last tick's, preempts the
/// brainstem with the winner, and remembers this view for next tick. Two ticks never
/// fire: a body whose eyes were only just added has not looked yet, so its empty view is
/// not a look and is not remembered as one; and a body that faces a new way since its
/// last look revealed whatever is new by turning, which is no apparition.
fn twitch(mut bodies: Query<Twitched>, kinds: Query<&Kind>) {
    let kind_of = |thing: Entity| kinds.get(thing).ok().copied();
    for (id, reflexes, seen, facing, mut last_look, mut brainstem) in &mut bodies {
        if seen.is_added() {
            continue;
        }
        if let Some(before) = &last_look.0
            && before.facing == facing.0
            && let Some((reflex, short)) = reflexes.pick(&seen, &before.ids, &kind_of)
        {
            let who = id.map_or_else(|| "a body".to_owned(), ThingId::to_string);
            info!("{who}: {} → {short:?}", reflex.name());
            brainstem.preempt(short, reflex.name());
        }
        last_look.0 = Some(Look {
            ids: seen.iter().map(|sighting| sighting.id).collect(),
            facing: facing.0,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use murabito_actions::{ActionQueue, ActionsPlugin};
    use murabito_brainstem::{BrainstemPlugin, Intent, Outcome, Sustained};
    use murabito_hexcoords::{Offset, VoxelCoord};
    use murabito_identity::{IdentityPlugin, NextThingId};
    use murabito_movement::{Locomotion, MovementPlugin};
    use murabito_perception::PerceptionPlugin;
    use murabito_placement::VoxelPosition;
    use murabito_progress::ProgressPlugin;
    use murabito_vision::{Band, Sighting, Vision, VisionPlugin};

    /// A little tree of kinds for the tests: creatures startle, trees don't.
    const CREATURES: Kind = Kind::at("test::creatures");
    const HARE: Kind = Kind::at("test::creatures::hare");
    const TREE: Kind = Kind::at("test::plants::tree");

    const STARTLE: Reflex = Reflex::StartleFaceApparition { by: CREATURES };

    /// Eyes all round, sharp to 8 cells, then 16, then 24.
    const ALL_ROUND: Vision = Vision {
        arc: 360.0,
        bands: [
            Band {
                range: 8,
                sensitivity: 1.0,
            },
            Band {
                range: 16,
                sensitivity: 0.6,
            },
            Band {
                range: 24,
                sensitivity: 0.3,
            },
        ],
    };

    fn voxel(q: i32, r: i32) -> VoxelCoord {
        VoxelCoord::new(q, r, -q - r, 0).expect("on the plane")
    }

    fn id(n: u64) -> ThingId {
        let mut counter = NextThingId::default();
        (1..n).for_each(|_| {
            counter.mint();
        });
        counter.mint()
    }

    /// The one entity the tests call a tree; every other is a creature.
    const A_TREE: u32 = 999;

    /// A creature's sighting: its entity is made from its number.
    fn sighting(id: ThingId, dq: i32, dr: i32, acuity: Acuity) -> Sighting {
        Sighting {
            entity: Entity::from_raw_u32(id.number() as u32).unwrap(),
            id,
            offset: Offset::new(dq, dr, -dq - dr, 0).unwrap(),
            acuity,
        }
    }

    fn tree_sighting(id: ThingId, dq: i32, dr: i32, acuity: Acuity) -> Sighting {
        Sighting {
            entity: Entity::from_raw_u32(A_TREE).unwrap(),
            ..sighting(id, dq, dr, acuity)
        }
    }

    fn kind_of(thing: Entity) -> Option<Kind> {
        Some(if thing == Entity::from_raw_u32(A_TREE).unwrap() {
            TREE
        } else {
            HARE
        })
    }

    fn seen(sightings: impl IntoIterator<Item = Sighting>) -> Seen {
        Seen::from_iter(sightings)
    }

    // -- the reflex alone -----------------------------------------------------------

    #[test]
    fn startle_fires_at_a_creature_near_that_was_not_in_view_before() {
        let newcomer = id(7);
        let now = seen([sighting(newcomer, 2, -4, Acuity::Near)]);
        assert_eq!(
            STARTLE.check(&now, &[], &kind_of),
            Some(Short::FaceThing(newcomer))
        );
    }

    #[test]
    fn startle_ignores_what_was_already_in_view_and_what_is_not_near() {
        let old = id(1);
        let far_newcomer = id(2);
        let now = seen([
            sighting(old, 1, 0, Acuity::Near),
            sighting(far_newcomer, 10, 0, Acuity::Mid),
        ]);
        assert_eq!(STARTLE.check(&now, &[old], &kind_of), None);
    }

    #[test]
    fn startle_ignores_a_tree_however_suddenly_seen_and_a_thing_of_no_kind() {
        let tree = id(4);
        let now = seen([tree_sighting(tree, 1, 0, Acuity::Near)]);
        assert_eq!(STARTLE.check(&now, &[], &kind_of), None);
        let unlabelled = |_: Entity| None;
        let creature = seen([sighting(id(5), 1, 0, Acuity::Near)]);
        assert_eq!(STARTLE.check(&creature, &[], &unlabelled), None);
    }

    #[test]
    fn startle_faces_the_nearest_of_several_apparitions() {
        let near = id(1);
        let nearer = id(2);
        let now = seen([
            sighting(near, 4, 0, Acuity::Near),
            sighting(nearer, 2, 0, Acuity::Near),
        ]);
        assert_eq!(
            STARTLE.check(&now, &[], &kind_of),
            Some(Short::FaceThing(nearer))
        );
    }

    #[test]
    fn a_reflex_is_named_category_reaction_trigger_whatever_its_dials() {
        assert_eq!(STARTLE.name(), "startle_face_apparition");
        let other_dial = Reflex::StartleFaceApparition { by: TREE };
        assert!(STARTLE.is(other_dial));
        assert_ne!(STARTLE, other_dial);
    }

    // -- the repertoire -------------------------------------------------------------

    #[test]
    fn an_empty_repertoire_never_fires() {
        let now = seen([sighting(id(1), 1, 0, Acuity::Near)]);
        assert_eq!(Reflexes::none().pick(&now, &[], &kind_of), None);
        assert!(Reflexes::default().is_empty());
    }

    #[test]
    fn a_repertoire_fires_its_reflex_by_name() {
        let newcomer = id(3);
        let now = seen([sighting(newcomer, 1, 0, Acuity::Near)]);
        let repertoire = Reflexes::new([STARTLE.at(10)]);
        assert_eq!(
            repertoire.pick(&now, &[], &kind_of),
            Some((STARTLE, Short::FaceThing(newcomer)))
        );
    }

    #[test]
    fn tuning_turns_one_reflexs_priority_and_nothing_else() {
        let jumpy =
            Reflexes::new([STARTLE.at(10)]).tuned(Reflex::StartleFaceApparition { by: TREE }, 200);
        let wired = jumpy.iter().next().unwrap();
        assert_eq!(wired.priority, 200, "matched by reflex, not by dial");
        assert_eq!(wired.reflex, STARTLE, "the dial is untouched");
        assert_eq!(jumpy.iter().count(), 1, "the repertoire is the kind's");
    }

    // -- in the tick ----------------------------------------------------------------

    /// A ticking app with everything a body needs to look, twitch, be told and move.
    fn app() -> App {
        use bevy::time::TimeUpdateStrategy;

        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            IdentityPlugin,
            ProgressPlugin,
            MovementPlugin,
            ActionsPlugin,
            PerceptionPlugin,
            VisionPlugin,
            BrainstemPlugin,
            ReflexesPlugin,
        ));
        app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
        app.update();
        app
    }

    fn mint(app: &mut App) -> ThingId {
        app.world_mut().resource_mut::<NextThingId>().mint()
    }

    /// A body at the origin facing east with the startle reflex, walking 4 shaku a
    /// second and turning 180 degrees a second.
    fn jumpy_body(app: &mut App) -> Entity {
        let id = mint(app);
        app.world_mut()
            .spawn((
                id,
                VoxelPosition(voxel(0, 0)),
                Facing(Direction::E),
                Locomotion {
                    speed: 4.0,
                    turn_speed: 180.0,
                },
                ALL_ROUND,
                ActionQueue::default(),
                Brainstem::default(),
                Reflexes::new([STARTLE.at(10)]),
            ))
            .id()
    }

    fn creature_at(app: &mut App, cell: VoxelCoord) -> ThingId {
        let id = mint(app);
        app.world_mut().spawn((id, HARE, VoxelPosition(cell)));
        id
    }

    fn tree_at(app: &mut App, cell: VoxelCoord) -> ThingId {
        let id = mint(app);
        app.world_mut().spawn((id, TREE, VoxelPosition(cell)));
        id
    }

    fn tick(app: &mut App, times: usize) {
        for _ in 0..times {
            app.update();
        }
    }

    fn brainstem(app: &App, body: Entity) -> Brainstem {
        app.world().get::<Brainstem>(body).unwrap().clone()
    }

    fn facing(app: &App, body: Entity) -> Direction {
        app.world().get::<Facing>(body).unwrap().0
    }

    #[test]
    fn a_body_has_its_last_look_beside_its_repertoire() {
        let mut app = app();
        let body = jumpy_body(&mut app);
        assert_eq!(
            app.world().get::<LastLook>(body),
            Some(&LastLook(None)),
            "required, and it has not looked yet"
        );
        tick(&mut app, 2);
        assert_eq!(
            app.world().get::<LastLook>(body),
            Some(&LastLook(Some(Look {
                ids: vec![],
                facing: Direction::E,
            })))
        );
    }

    #[test]
    fn a_creature_appearing_near_cancels_the_walk_and_the_body_faces_it_that_tick() {
        let mut app = app();
        let body = jumpy_body(&mut app);
        app.world_mut()
            .get_mut::<Brainstem>(body)
            .unwrap()
            .order(Intent::Sustained(Sustained::GoTo(voxel(6, 0))));
        tick(&mut app, 3);

        let apparition = creature_at(&mut app, voxel(2, -4)); // two cells north, near
        tick(&mut app, 1); // the body's look at the end of this tick sees it
        assert_eq!(
            brainstem(&app, body).previous(),
            None,
            "the reflex reads last tick's look: not yet"
        );
        tick(&mut app, 1);
        let doing = brainstem(&app, body).doing().expect("taken by the reflex");
        assert_eq!(doing.intent(), Intent::Short(Short::FaceThing(apparition)));
        assert_eq!(doing.since(), 5);
        assert_eq!(
            brainstem(&app, body).previous().map(|p| p.outcome()),
            Some(Outcome::Cancelled("startle_face_apparition"))
        );

        tick(&mut app, 31);
        assert_eq!(
            facing(&app, body),
            Direction::N,
            "a quarter turn, 32 ticks from the twitch"
        );
        tick(&mut app, 1);
        assert_eq!(
            brainstem(&app, body).previous().map(|p| p.outcome()),
            Some(Outcome::Done)
        );
        tick(&mut app, 20);
        assert_eq!(
            brainstem(&app, body).previous().map(|p| p.outcome()),
            Some(Outcome::Done),
            "the creature still standing there startles nobody"
        );
    }

    #[test]
    fn a_tree_appearing_near_startles_nobody() {
        let mut app = app();
        let body = jumpy_body(&mut app);
        tick(&mut app, 2);
        tree_at(&mut app, voxel(2, -4));
        tick(&mut app, 3);
        assert_eq!(brainstem(&app, body).doing(), None);
    }

    #[test]
    fn what_a_body_reveals_by_its_own_turn_startles_it_not() {
        // A body with a narrow cone, facing east, with a creature standing close behind
        // its left shoulder. Told to face north, it sweeps its cone across the creature:
        // the creature comes into view because the body turned, so nothing fires, and
        // once seen it is remembered.
        let mut app = app();
        let body = jumpy_body(&mut app);
        let narrow = Vision {
            arc: 90.0,
            ..ALL_ROUND
        };
        app.world_mut().entity_mut(body).insert(narrow);
        creature_at(&mut app, voxel(1, -3)); // north-north-east-ish, three cells off
        tick(&mut app, 3);
        assert_eq!(
            brainstem(&app, body).doing(),
            None,
            "out of the cone, unseen"
        );

        app.world_mut()
            .get_mut::<Brainstem>(body)
            .unwrap()
            .order(Intent::Short(Short::Face(Direction::N)));
        tick(&mut app, 40);
        assert_eq!(facing(&app, body), Direction::N);
        assert_eq!(
            brainstem(&app, body).previous().map(|p| p.outcome()),
            Some(Outcome::Done),
            "the turn ran to its end, untaken"
        );
        tick(&mut app, 5);
        assert_eq!(
            brainstem(&app, body).doing(),
            None,
            "and the creature is old news"
        );
    }

    #[test]
    fn nothing_startles_a_body_at_the_worlds_first_sight_of_it() {
        let mut app = app();
        let body = jumpy_body(&mut app);
        creature_at(&mut app, voxel(2, -4));
        tick(&mut app, 3);
        assert_eq!(brainstem(&app, body).doing(), None);
        assert_eq!(brainstem(&app, body).previous(), None);
        assert_eq!(facing(&app, body), Direction::E);
    }

    #[test]
    fn something_appearing_at_mid_range_startles_nobody() {
        let mut app = app();
        let body = jumpy_body(&mut app);
        tick(&mut app, 2);
        creature_at(&mut app, voxel(12, 0)); // in the mid band
        tick(&mut app, 3);
        assert_eq!(brainstem(&app, body).doing(), None);
    }

    #[test]
    fn a_body_with_no_repertoire_is_never_taken() {
        let mut app = app();
        let body = jumpy_body(&mut app);
        *app.world_mut().get_mut::<Reflexes>(body).unwrap() = Reflexes::none();
        tick(&mut app, 2);
        creature_at(&mut app, voxel(2, -4));
        tick(&mut app, 3);
        assert_eq!(brainstem(&app, body).doing(), None);
    }
}

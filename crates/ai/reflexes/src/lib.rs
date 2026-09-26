//! Reflexes: what a body does without thinking.
//!
//! The catalogue of every reflex a sentient body could have, each with its own code, and
//! the [`Reflexes`] a body carries: which of them it has, at what priority. A kind gives
//! its repertoire in its own `require`; an individual may be spawned with the dials
//! turned, "a tad jumpy", since what is given at spawn wins.
//!
//! A reflex is one trigger to one response, named `category_reaction_trigger`, and it
//! answers only a [`Short`]: something over within a round of the slower mind. That is
//! the type's promise, not a rule to remember. Every tick, in the brainstem's
//! `Reflexes` slot, each body's repertoire is checked against what it sees now and what
//! it saw last tick; the highest-priority match preempts the brainstem, cancelling
//! whatever the body was doing, by the reflex's name, so the mind learns what took its
//! body. Ties go to the earlier entry. Design: `docs/ai_readme.md`.

use bevy::prelude::*;
use murabito_brainstem::{Brainstem, BrainstemSet, Short};
use murabito_identity::ThingId;
use murabito_vision::{Acuity, Seen, Sighting};

pub struct ReflexesPlugin;

impl Plugin for ReflexesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, twitch.in_set(BrainstemSet::Reflexes));
        #[cfg(feature = "debug")]
        app.register_type::<Reflexes>();
    }
}

/// The catalogue. Each is one trigger to one response, and its code is [`Reflex::check`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "debug", derive(Reflect))]
pub enum Reflex {
    /// Startle: something is in view at the nearest acuity this tick and was not in view
    /// at all last tick. The body turns to face it; the nearest, if several appeared.
    StartleFaceApparition,
}

impl Reflex {
    /// The name the outcome carries when this reflex takes the body.
    pub fn name(self) -> &'static str {
        match self {
            Self::StartleFaceApparition => "startle_face_apparition",
        }
    }

    /// This reflex, wired at a priority: one entry of a repertoire.
    pub fn at(self, priority: u8) -> Wired {
        Wired {
            reflex: self,
            priority,
        }
    }

    /// Whether this reflex fires, given what the body sees now and the ids it saw last
    /// tick, and what it does if so. Pure: the whole of a reflex's behaviour, testable on
    /// two lists.
    pub fn check(self, now: &Seen, before: &[ThingId]) -> Option<Short> {
        match self {
            Self::StartleFaceApparition => now
                .iter()
                .filter(|sighting| sighting.acuity == Acuity::Near)
                .filter(|sighting| !before.contains(&sighting.id))
                .min_by_key(|sighting| sighting.offset.steps())
                .map(|apparition: &Sighting| Short::FaceThing(apparition.id)),
        }
    }
}

/// One entry of a repertoire: a reflex and how it ranks against the body's others.
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

    /// The same repertoire with one reflex's priority turned: an individual's dial.
    pub fn tuned(mut self, reflex: Reflex, priority: u8) -> Self {
        for wired in &mut self.0 {
            if wired.reflex == reflex {
                wired.priority = priority;
            }
        }
        self
    }

    /// The reflex that takes the body this tick, if any: the highest-priority one that
    /// fires, the earlier entry on a tie.
    fn pick(&self, now: &Seen, before: &[ThingId]) -> Option<(Reflex, Short)> {
        let mut winner: Option<(Wired, Short)> = None;
        for &wired in &self.0 {
            let Some(short) = wired.reflex.check(now, before) else {
                continue;
            };
            if winner.is_none_or(|(best, _)| wired.priority > best.priority) {
                winner = Some((wired, short));
            }
        }
        winner.map(|(wired, short)| (wired.reflex, short))
    }
}

/// What the body saw last tick, by id: `None` before it has ever looked, so nothing
/// startles at the world's first sight of it.
#[derive(Component, Debug, Default, Clone, PartialEq, Eq)]
pub struct LastLook(Option<Vec<ThingId>>);

/// Runs every body's repertoire against this tick's view and last tick's, preempts the
/// brainstem with the winner, and remembers this view for next tick. A body whose eyes
/// were only just added has not looked yet, so its empty view is not a look and is not
/// remembered as one: nothing startles at the world's first sight of it.
fn twitch(mut bodies: Query<(&Reflexes, Ref<Seen>, &mut LastLook, &mut Brainstem)>) {
    for (reflexes, seen, mut last_look, mut brainstem) in &mut bodies {
        if seen.is_added() {
            continue;
        }
        if let Some(before) = &last_look.0
            && let Some((reflex, short)) = reflexes.pick(&seen, before)
        {
            brainstem.preempt(short, reflex.name());
        }
        last_look.0 = Some(seen.iter().map(|sighting| sighting.id).collect());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use murabito_actions::{ActionQueue, ActionsPlugin};
    use murabito_brainstem::{BrainstemPlugin, Intent, Outcome, Sustained};
    use murabito_hexcoords::{Direction, Offset, VoxelCoord};
    use murabito_identity::{IdentityPlugin, NextThingId};
    use murabito_movement::{Locomotion, MovementPlugin};
    use murabito_perception::PerceptionPlugin;
    use murabito_placement::{Facing, VoxelPosition};
    use murabito_progress::ProgressPlugin;
    use murabito_vision::{Band, Vision, VisionPlugin};

    const STARTLE: Reflex = Reflex::StartleFaceApparition;

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

    fn sighting(id: ThingId, dq: i32, dr: i32, acuity: Acuity) -> Sighting {
        Sighting {
            entity: Entity::PLACEHOLDER,
            id,
            offset: Offset::new(dq, dr, -dq - dr, 0).unwrap(),
            acuity,
        }
    }

    fn seen(sightings: impl IntoIterator<Item = Sighting>) -> Seen {
        Seen::from_iter(sightings)
    }

    // -- the reflex alone -----------------------------------------------------------

    #[test]
    fn startle_fires_at_something_near_that_was_not_in_view_before() {
        let newcomer = id(7);
        let now = seen([sighting(newcomer, 2, -4, Acuity::Near)]);
        assert_eq!(STARTLE.check(&now, &[]), Some(Short::FaceThing(newcomer)));
    }

    #[test]
    fn startle_ignores_what_was_already_in_view_and_what_is_not_near() {
        let old = id(1);
        let far_newcomer = id(2);
        let now = seen([
            sighting(old, 1, 0, Acuity::Near),
            sighting(far_newcomer, 10, 0, Acuity::Mid),
        ]);
        assert_eq!(STARTLE.check(&now, &[old]), None);
    }

    #[test]
    fn startle_faces_the_nearest_of_several_apparitions() {
        let near = id(1);
        let nearer = id(2);
        let now = seen([
            sighting(near, 4, 0, Acuity::Near),
            sighting(nearer, 2, 0, Acuity::Near),
        ]);
        assert_eq!(STARTLE.check(&now, &[]), Some(Short::FaceThing(nearer)));
    }

    #[test]
    fn a_reflex_is_named_category_reaction_trigger() {
        assert_eq!(STARTLE.name(), "startle_face_apparition");
    }

    // -- the repertoire -------------------------------------------------------------

    #[test]
    fn an_empty_repertoire_never_fires() {
        let now = seen([sighting(id(1), 1, 0, Acuity::Near)]);
        assert_eq!(Reflexes::none().pick(&now, &[]), None);
        assert!(Reflexes::default().is_empty());
    }

    #[test]
    fn a_repertoire_fires_its_reflex_by_name() {
        let newcomer = id(3);
        let now = seen([sighting(newcomer, 1, 0, Acuity::Near)]);
        let repertoire = Reflexes::new([STARTLE.at(10)]);
        assert_eq!(
            repertoire.pick(&now, &[]),
            Some((STARTLE, Short::FaceThing(newcomer)))
        );
    }

    #[test]
    fn tuning_turns_one_reflexs_priority_and_nothing_else() {
        let jumpy = Reflexes::new([STARTLE.at(10)]).tuned(STARTLE, 200);
        assert_eq!(jumpy.iter().next().unwrap().priority, 200);
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

    fn thing_at(app: &mut App, cell: VoxelCoord) -> ThingId {
        let id = mint(app);
        app.world_mut().spawn((id, VoxelPosition(cell)));
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
            Some(&LastLook(Some(vec![])))
        );
    }

    #[test]
    fn something_appearing_near_cancels_the_walk_and_the_body_faces_it_that_tick() {
        let mut app = app();
        let body = jumpy_body(&mut app);
        app.world_mut()
            .get_mut::<Brainstem>(body)
            .unwrap()
            .order(Intent::Sustained(Sustained::GoTo(voxel(6, 0))));
        tick(&mut app, 3);

        let apparition = thing_at(&mut app, voxel(2, -4)); // two cells north, in the near band
        tick(&mut app, 1); // the body's look at the end of this tick sees it
        assert_eq!(
            brainstem(&app, body).previous_outcome(),
            Outcome::Idle,
            "the reflex reads last tick's look: not yet"
        );
        tick(&mut app, 1);
        let doing = brainstem(&app, body).doing().expect("taken by the reflex");
        assert_eq!(doing.intent(), Intent::Short(Short::FaceThing(apparition)));
        assert_eq!(doing.since(), 5);
        assert_eq!(
            brainstem(&app, body).previous_outcome(),
            Outcome::Cancelled("startle_face_apparition")
        );

        tick(&mut app, 31);
        assert_eq!(
            facing(&app, body),
            Direction::N,
            "a quarter turn, 32 ticks from the twitch"
        );
        tick(&mut app, 1);
        assert_eq!(brainstem(&app, body).previous_outcome(), Outcome::Done);
        tick(&mut app, 20);
        assert_eq!(
            brainstem(&app, body).previous_outcome(),
            Outcome::Done,
            "the thing still standing there startles nobody"
        );
    }

    #[test]
    fn nothing_startles_a_body_at_the_worlds_first_sight_of_it() {
        let mut app = app();
        let body = jumpy_body(&mut app);
        thing_at(&mut app, voxel(2, -4));
        tick(&mut app, 3);
        assert_eq!(brainstem(&app, body).doing(), None);
        assert_eq!(brainstem(&app, body).previous_outcome(), Outcome::Idle);
        assert_eq!(facing(&app, body), Direction::E);
    }

    #[test]
    fn something_appearing_at_mid_range_startles_nobody() {
        let mut app = app();
        let body = jumpy_body(&mut app);
        tick(&mut app, 2);
        thing_at(&mut app, voxel(12, 0)); // in the mid band
        tick(&mut app, 3);
        assert_eq!(brainstem(&app, body).doing(), None);
    }

    #[test]
    fn a_body_with_no_repertoire_is_never_taken() {
        let mut app = app();
        let body = jumpy_body(&mut app);
        *app.world_mut().get_mut::<Reflexes>(body).unwrap() = Reflexes::none();
        tick(&mut app, 2);
        thing_at(&mut app, voxel(2, -4));
        tick(&mut app, 3);
        assert_eq!(brainstem(&app, body).doing(), None);
    }
}

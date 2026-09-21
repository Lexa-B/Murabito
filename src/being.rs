//! Two bodies, no brains: a fox and a rabbit, each with the sense fields its species
//! would plausibly have. Nothing decides anything yet — they stand where they are put
//! and face where they are pointed.
//!
//! The pairing is deliberate. A predator and its prey want opposite things from the
//! same two senses, so the numbers below are a sanity check on whether the sense
//! geometry can express a real difference at all: the fox sees a long way down a narrow
//! cone and listens forward, the rabbit sees most of the way around itself but not far,
//! and hears in every direction at once.
//!
//! Every number here is placeholder tuning, in shaku, scaled to the 66 shaku (11 ken)
//! starter ground. Sense ranges are whole shaku because they are cell counts.

use bevy::prelude::*;

#[cfg(test)]
use crate::senses::rotate;
use crate::senses::{Hearing, SeenCells, SenseOverlay, Vision, VisionBand};
use crate::諸法::種;

/// Marks a body that senses and (later) acts. What perception will query for.
///
/// "Being" rather than "creature" because this will also cover people and things that
/// were never alive; and not "entity", which is Bevy's own ID type.
#[derive(Component, Clone, Copy, Debug)]
pub struct Being;

/// Where each stands, and where it looks. Constants rather than locals so the tableau
/// can be asserted: "a predator watching prey that has not noticed" is a claim, and a
/// claim belongs in a test rather than in a screenshot read through a projection that
/// makes ground angles hard to judge.
///
/// Both stand at half their body height, so they sit on the ground rather than in it,
/// and both look at a point at their own height, so `looking_at` turns them about Y
/// only and leaves them level.
pub(crate) const FOX_AT: Vec3 = Vec3::new(-16.0, 0.8, 10.0);
pub(crate) const RABBIT_AT: Vec3 = Vec3::new(13.0, 0.6, -10.0);

/// Off toward the far corner: the fox is behind the rabbit, in the wedge its 240 degree
/// cone does not cover.
const RABBIT_LOOKS_AT: Vec3 = Vec3::new(30.0, RABBIT_AT.y, -26.0);

/// Watching the rabbit.
pub(crate) fn fox_transform() -> Transform {
    Transform::from_translation(FOX_AT).looking_at(RABBIT_AT.with_y(FOX_AT.y), Vec3::Y)
}

pub(crate) fn rabbit_transform() -> Transform {
    Transform::from_translation(RABBIT_AT).looking_at(RABBIT_LOOKS_AT, Vec3::Y)
}

const FOX_COLOR: Color = Color::srgb(0.78, 0.36, 0.12);
const RABBIT_COLOR: Color = Color::srgb(0.74, 0.70, 0.64);
/// Sense overlays are deliberately different hues rather than species colours, so the
/// two sets of fields stay tellable apart where they overlap.
const FOX_OVERLAY: Color = Color::srgb(1.0, 0.45, 0.1);
const RABBIT_OVERLAY: Color = Color::srgb(0.35, 0.8, 1.0);

pub struct BeingPlugin;

impl Plugin for BeingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_beings);
    }
}

/// A hunter's senses: a narrow cone reaching a long way, and hearing pulled forward into
/// a cardioid, which is what lets a fox place a sound precisely enough to pounce on it.
fn fox_vision() -> Vision {
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

fn fox_hearing() -> Hearing {
    Hearing {
        range: 48,
        directionality: 0.55,
    }
}

/// Prey senses: eyes on the sides of the head, so most of the way around but short and
/// low-acuity, and hearing that is nearly a circle — it matters far more to a rabbit
/// that *something* is there than exactly where.
fn rabbit_vision() -> Vision {
    Vision {
        arc: 240.0_f32.to_radians(),
        bands: [
            VisionBand {
                range: 8,
                sensitivity: 1.0,
            },
            VisionBand {
                range: 18,
                sensitivity: 0.55,
            },
            VisionBand {
                range: 26,
                sensitivity: 0.25,
            },
        ],
    }
}

fn rabbit_hearing() -> Hearing {
    Hearing {
        range: 33,
        directionality: 0.12,
    }
}

fn spawn_beings(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        // Which kind it is, as a taxonomy key rather than a Rust type: the same reason
        // the tree is data. A `Fox` component would put the ontology back in the type
        // system, where it cannot be walked.
        種::new("狐"),
        Being,
        Mesh3d(meshes.add(Cuboid::new(1.6, 1.6, 3.6))),
        MeshMaterial3d(materials.add(FOX_COLOR)),
        fox_transform(),
        fox_vision(),
        SeenCells::default(),
        fox_hearing(),
        SenseOverlay { color: FOX_OVERLAY },
    ));

    commands.spawn((
        種::new("兎"),
        Being,
        Mesh3d(meshes.add(Cuboid::new(1.2, 1.2, 1.8))),
        MeshMaterial3d(materials.add(RABBIT_COLOR)),
        rabbit_transform(),
        rabbit_vision(),
        SeenCells::default(),
        rabbit_hearing(),
        SenseOverlay {
            color: RABBIT_OVERLAY,
        },
    ));
}

#[cfg(test)]
mod tableau {
    //! What the starting arrangement claims: a predator watching prey that has not
    //! noticed, with something in the way so that looking is not enough.
    //!
    //! Built from the same constants the app spawns from, through the same
    //! `gather_occluders` and the same taxonomy, so the test cannot drift from the scene
    //! by agreeing with a copy of it.

    use bevy::prelude::*;

    use super::*;
    use crate::hex::Hex;
    use crate::scene::{BUSHES, GRASS, TREES};
    use crate::senses::facing;
    use crate::senses::occlusion::{Height, Occluders, gather_occluders};
    use crate::senses::vision::cast;
    use crate::諸法::実相;

    fn ground(at: Vec3) -> Vec2 {
        Vec2::new(at.x, at.z)
    }

    /// The real plants, through the real gather: what blocks is read off 木 and 草
    /// carrying 不透明 and 半透明, not asserted here.
    fn plants() -> Occluders {
        let mut app = App::new();
        app.insert_resource(実相::load())
            .init_resource::<Occluders>()
            .add_systems(Update, gather_occluders);
        for (kind, places) in [
            ("木", &TREES[..]),
            ("茂み", &BUSHES[..]),
            ("草", &GRASS[..]),
        ] {
            for (x, z) in places {
                app.world_mut().spawn((
                    種::new(kind),
                    Transform::from_xyz(*x, 0.0, *z),
                    GlobalTransform::from_xyz(*x, 0.0, *z),
                ));
            }
        }
        app.update();
        app.world_mut().remove_resource::<Occluders>().unwrap()
    }

    fn fox_sees(occluders: &Occluders) -> crate::senses::SeenCells {
        let eye = GlobalTransform::from(fox_transform());
        cast(ground(FOX_AT), facing(&eye), &fox_vision(), occluders)
    }

    /// The half of the claim that is about geometry: the rabbit is in range and in the
    /// cone. Without this, "the fox cannot see the rabbit" would pass for the boring
    /// reason that it never could.
    #[test]
    fn with_the_ground_bare_the_fox_would_see_the_rabbit() {
        let seen = fox_sees(&Occluders::default());
        // At the rabbit's height, which is the question actually being asked: could the
        // fox make out something rabbit-sized standing there.
        assert!(seen.contains(Hex::from_world(ground(RABBIT_AT)), Height::Knee));
    }

    /// The half that is about the world: something stands in the way.
    #[test]
    fn the_trees_hide_the_rabbit_from_the_fox() {
        let seen = fox_sees(&plants());
        assert!(
            !seen.contains(Hex::from_world(ground(RABBIT_AT)), Height::Knee),
            "the rabbit should be behind a tree"
        );
    }

    /// And the prey has not noticed — not because it is blind, but because the fox is in
    /// the wedge even a 240 degree cone leaves behind.
    #[test]
    fn the_rabbit_does_not_see_the_fox() {
        let eye = GlobalTransform::from(rabbit_transform());
        let seen = cast(ground(RABBIT_AT), facing(&eye), &rabbit_vision(), &plants());
        assert!(!seen.contains(Hex::from_world(ground(FOX_AT)), Height::Knee));
    }

    /// Grass is knee-high, so it is the rabbit's problem and not a person's. The same
    /// cells, the same fox, two answers.
    #[test]
    fn the_grass_troubles_the_rabbit_and_not_a_person() {
        let bare = fox_sees(&Occluders::default());
        let grown = fox_sees(&plants());

        let dimmed_for_a_rabbit = bare
            .iter(Height::Knee)
            .filter(|(hex, band)| {
                grown
                    .band(*hex, Height::Knee)
                    .is_some_and(|now| now > *band)
            })
            .count();
        let dimmed_for_a_person = bare
            .iter(Height::Full)
            .filter(|(hex, band)| {
                grown
                    .band(*hex, Height::Full)
                    .is_some_and(|now| now > *band)
            })
            .count();

        assert!(
            dimmed_for_a_rabbit > 0,
            "grass should cost sight of a rabbit"
        );
        assert_eq!(
            dimmed_for_a_person, 0,
            "and none at all of someone standing"
        );
    }

    /// The two kinds of occluder do different things, and both do something.
    ///
    /// Asserted over the whole visible set rather than at a chosen cell: picking one
    /// means guessing how much grass happens to lie along that bearing, and the answer
    /// moves whenever a tuft does. The first version of this test picked a cell behind
    /// four stacked tufts — demoted past the last band, so hidden outright, which is the
    /// model working rather than failing. A deep enough thicket *is* a wall.
    #[test]
    fn the_grass_costs_sight_where_the_trees_take_it() {
        let bare = fox_sees(&Occluders::default());
        let grown = fox_sees(&plants());

        let dimmed = bare
            .iter(Height::Knee)
            .filter(|(hex, band)| {
                grown
                    .band(*hex, Height::Knee)
                    .is_some_and(|now| now > *band)
            })
            .count();
        assert!(dimmed > 0, "the grass should cost a band somewhere");

        let lost = bare
            .iter(Height::Knee)
            .filter(|(hex, _)| !grown.contains(*hex, Height::Knee))
            .count();
        assert!(lost > 0, "the trees should take sight outright somewhere");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FORWARD: Vec2 = Vec2::new(0.0, -1.0);

    /// These assert the *species claim* rather than the numbers: that a hunter and its
    /// prey are shaped oppositely by the same two senses. The numbers are placeholder
    /// tuning and may move; if one of these fails, the tuning stopped saying what it
    /// was supposed to say.
    #[test]
    fn the_fox_sees_further_but_through_a_narrower_cone() {
        let fox = fox_vision();
        let rabbit = rabbit_vision();
        assert!(fox.far_range() > rabbit.far_range());
        assert!(fox.arc < rabbit.arc);
    }

    #[test]
    fn the_rabbit_sees_behind_its_own_shoulder_and_the_fox_cannot() {
        let over_the_shoulder = rotate(FORWARD, 100.0_f32.to_radians()) * 6.0;
        assert_eq!(fox_vision().sensitivity_at(FORWARD, over_the_shoulder), 0.0);
        assert!(rabbit_vision().sensitivity_at(FORWARD, over_the_shoulder) > 0.0);
    }

    #[test]
    fn the_fox_listens_forward_and_the_rabbit_listens_everywhere() {
        let fox = fox_hearing();
        let rabbit = rabbit_hearing();
        // Focused forward: further down its best bearing than the rabbit manages.
        assert!(fox.reach(1.0) > rabbit.reach(1.0));
        // Wide: the rabbit still hears well behind itself, where the fox hears nothing.
        assert_eq!(fox.gain(-1.0), 0.0);
        assert!(rabbit.gain(-1.0) > 0.7);
    }
}

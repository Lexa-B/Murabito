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
//! Every number here is placeholder tuning, scaled to the 20x20 m starter ground.

use bevy::prelude::*;

#[cfg(test)]
use crate::senses::rotate;
use crate::senses::{Hearing, SenseOverlay, Vision, VisionBand};
use crate::諸法::種;

/// Marks a body that senses and (later) acts. What perception will query for.
///
/// "Being" rather than "creature" because this will also cover people and things that
/// were never alive; and not "entity", which is Bevy's own ID type.
#[derive(Component, Clone, Copy, Debug)]
pub struct Being;

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

fn fox_hearing() -> Hearing {
    Hearing {
        range: 14.0,
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
                range: 2.5,
                sensitivity: 1.0,
            },
            VisionBand {
                range: 5.0,
                sensitivity: 0.55,
            },
            VisionBand {
                range: 8.0,
                sensitivity: 0.25,
            },
        ],
    }
}

fn rabbit_hearing() -> Hearing {
    Hearing {
        range: 10.0,
        directionality: 0.12,
    }
}

fn spawn_beings(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Both stand at the height of half their body, so they sit on the ground rather
    // than in it, and both look at a point at their own height, so `looking_at` turns
    // them about Y only and leaves them level.
    let fox_at = Vec3::new(-5.0, 0.25, 3.0);
    let rabbit_at = Vec3::new(4.0, 0.18, -3.0);

    commands.spawn((
        // Which kind it is, as a taxonomy key rather than a Rust type: the same reason
        // the tree is data. A `Fox` component would put the ontology back in the type
        // system, where it cannot be walked.
        種::new("狐"),
        Being,
        Mesh3d(meshes.add(Cuboid::new(0.5, 0.5, 1.1))),
        MeshMaterial3d(materials.add(FOX_COLOR)),
        // Watching the rabbit.
        Transform::from_translation(fox_at).looking_at(rabbit_at.with_y(fox_at.y), Vec3::Y),
        fox_vision(),
        fox_hearing(),
        SenseOverlay { color: FOX_OVERLAY },
    ));

    commands.spawn((
        種::new("兎"),
        Being,
        Mesh3d(meshes.add(Cuboid::new(0.36, 0.36, 0.55))),
        MeshMaterial3d(materials.add(RABBIT_COLOR)),
        // Facing away, off toward the far corner: the fox is behind it, in the part of
        // its vision the 240 degree cone does not cover.
        Transform::from_translation(rabbit_at)
            .looking_at(Vec3::new(9.0, rabbit_at.y, -8.0), Vec3::Y),
        rabbit_vision(),
        rabbit_hearing(),
        SenseOverlay {
            color: RABBIT_OVERLAY,
        },
    ));
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
        let over_the_shoulder = rotate(FORWARD, 100.0_f32.to_radians()) * 2.0;
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

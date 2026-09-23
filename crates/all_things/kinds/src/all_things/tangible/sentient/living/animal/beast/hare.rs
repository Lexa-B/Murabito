//! 兎 Hare

use bevy::prelude::*;
use murabito_movement::Locomotion;
use murabito_vision::{Band, Vision};

use crate::{Beast, Model};

/// 兎: the Japanese hare (野兎). Its pace is a guess at a walk, 5 shaku (about
/// 1.5 m) a second, until someone who knows says otherwise.
/// Prey's eyes: on the sides of the head, so most of the way round but short and
/// low-acuity, sharp to 8 shaku, then 18, then 26. Placeholder tuning from the first
/// attempt.
const HARE_VISION: Vision = Vision {
    arc: 240.0,
    bands: [
        Band {
            range: 8,
            sensitivity: 1.0,
        },
        Band {
            range: 18,
            sensitivity: 0.55,
        },
        Band {
            range: 26,
            sensitivity: 0.25,
        },
    ],
};

#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "debug", derive(Reflect), reflect(Component))]
#[require(
    Beast,
    Locomotion = Locomotion { speed: 5.0, turn_speed: 240.0 },
    Vision = HARE_VISION,
    Model = Model("entity_models/living/animals/hare.glb"),
)]
pub struct Hare;

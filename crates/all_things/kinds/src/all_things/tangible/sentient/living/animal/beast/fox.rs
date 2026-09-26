//! 狐 Fox

use bevy::prelude::*;
use murabito_identity::Kind;
use murabito_movement::Locomotion;
use murabito_vision::{Band, Vision};

use crate::{Beast, Model};

/// A hunter's eyes: a narrow cone reaching a long way, sharp to 12 shaku, then 30,
/// then 48. Placeholder tuning from the first attempt.
const FOX_VISION: Vision = Vision {
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
};

/// 狐: the red fox. A brisk walk of 4 shaku (about 1.2 m) a second, and half a turn a
/// second: the numbers the first attempt settled on by eye.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "debug", derive(Reflect), reflect(Component))]
#[require(
    Beast,
    Locomotion = Locomotion { speed: 4.0, turn_speed: 180.0 },
    Vision = FOX_VISION,
    Model = Model("entity_models/living/animals/fox.glb"),
    Kind = Kind::at(module_path!()),
)]
pub struct Fox;

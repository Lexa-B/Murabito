//! 有情 Sentient
//!
//! Its children are the files in `sentient/`, beside this one.

use bevy::prelude::*;
use murabito_actions::ActionQueue;
use murabito_hexcoords::Direction;
use murabito_movement::Locomotion;
use murabito_placement::Facing;
use murabito_vision::{Band, Vision};

use crate::Tangible;

pub mod living;
pub mod spiritual;

pub use living::*;
pub use spiritual::*;

/// A pace for a sentient thing whose kind hasn't said better: 2 shaku (about 60 cm) a
/// second, and a quarter turn a second. A kind puts its own numbers in its own
/// `require`, which wins over these.
const SENTIENT_LOCOMOTION: Locomotion = Locomotion {
    speed: 2.0,
    turn_speed: 90.0,
};

/// Eyes for a sentient thing whose kind hasn't said better: a half circle, sharp to 8
/// shaku, then 16, then 24. A guess, like the pace. A kind puts its own cone in its own
/// `require`, or `Vision::BLIND` if it has none.
const SENTIENT_VISION: Vision = Vision {
    arc: 180.0,
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

/// 有情: whatever takes いる. It faces somewhere, it moves, it looks where it faces, and
/// it can be asked to do things. Yokai move and see as much as beasts do.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[require(
    Tangible,
    Facing = Facing(Direction::E),
    Locomotion = SENTIENT_LOCOMOTION,
    Vision = SENTIENT_VISION,
    ActionQueue,
)]
pub struct Sentient;

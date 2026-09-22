//! 有情 Sentient
//!
//! Its children are the files in `sentient/`, beside this one.

use bevy::prelude::*;
use murabito_actions::ActionQueue;
use murabito_hexcoords::Direction;
use murabito_movement::{Facing, Locomotion};

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

/// 有情: whatever takes いる. It faces somewhere, it moves, and it can be asked to do
/// things. Yokai move as much as beasts do.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[require(
    Tangible,
    Facing = Facing(Direction::E),
    Locomotion = SENTIENT_LOCOMOTION,
    ActionQueue,
)]
pub struct Sentient;

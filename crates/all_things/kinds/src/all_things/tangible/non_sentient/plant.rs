//! 植物 Plant
//!
//! Its children are the files in `plant/`, beside this one.

use bevy::prelude::*;

use crate::NonSentient;

pub mod bamboo;
pub mod shrub;
pub mod tree;
pub mod undergrowth;

pub use bamboo::*;
pub use shrub::*;
pub use tree::*;
pub use undergrowth::*;

/// 植物: the tiers below match `assets/entity_models/flora/`.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[require(NonSentient)]
pub struct Plant;

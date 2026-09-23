//! 物 Tangible
//!
//! Its children are the files in `tangible/`, beside this one.

use bevy::prelude::*;

use crate::AllThings;

pub mod non_sentient;
pub mod sentient;

pub use non_sentient::*;
pub use sentient::*;

/// 物: a thing with a place in the world, facing some way. Both are the instance's,
/// given at spawn beside the kind and checked there; the tree names no place.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[require(AllThings)]
pub struct Tangible;

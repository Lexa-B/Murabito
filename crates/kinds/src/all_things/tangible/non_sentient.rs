//! 非情 NonSentient
//!
//! Its children are the files in `non_sentient/`, beside this one.

use bevy::prelude::*;

use crate::Tangible;

pub mod object;
pub mod plant;

pub use object::*;
pub use plant::*;

/// 非情: whatever takes ある.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[require(Tangible)]
pub struct NonSentient;

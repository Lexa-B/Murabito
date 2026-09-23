//! 諸法 AllThings
//!
//! Its children are the files in `all_things/`, beside this one.

use bevy::prelude::*;

pub mod intangible;
pub mod tangible;

pub use intangible::*;
pub use tangible::*;

/// 諸法: every phenomenon. The root; nothing is not this.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AllThings;

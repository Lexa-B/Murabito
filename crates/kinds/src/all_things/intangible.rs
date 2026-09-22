//! 事 Intangible
//!
//! A leaf tier: the kinds under it arrive as they are needed.

use bevy::prelude::*;

use crate::AllThings;

/// 事: an event or a happening, with no place to point at. A tier nothing requires yet.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[require(AllThings)]
pub struct Intangible;

//! 草 Undergrowth
//!
//! A leaf tier: the kinds under it arrive as they are needed.

use bevy::prelude::*;

use crate::Plant;

/// 草: what grows at ground level: kusa, kuzu, shida. 草 is wider than *grass*.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[require(Plant)]
pub struct Undergrowth;

//! 茂み Shrub
//!
//! A leaf tier: the kinds under it arrive as they are needed.

use bevy::prelude::*;

use crate::Plant;

/// 茂み: azalea, aoki.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[require(Plant)]
pub struct Shrub;

//! 竹 Bamboo
//!
//! A leaf tier: the kinds under it arrive as they are needed.

use bevy::prelude::*;

use crate::Plant;

/// 竹: bamboo and sasa. Neither tree nor grass, as the saying goes.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[require(Plant)]
pub struct Bamboo;

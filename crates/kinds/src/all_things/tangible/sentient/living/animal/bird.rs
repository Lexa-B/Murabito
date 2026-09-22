//! 鳥 Bird
//!
//! A leaf tier: the kinds under it arrive as they are needed.

use bevy::prelude::*;

use crate::Animal;

/// 鳥: crane, heron, chicken, pheasant.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[require(Animal)]
pub struct Bird;

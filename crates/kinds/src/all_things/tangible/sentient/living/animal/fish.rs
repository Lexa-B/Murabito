//! 魚 Fish
//!
//! A leaf tier: the kinds under it arrive as they are needed.

use bevy::prelude::*;

use crate::Animal;

/// 魚
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[require(Animal)]
pub struct Fish;

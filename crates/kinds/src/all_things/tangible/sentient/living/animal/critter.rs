//! 虫 Critter
//!
//! A leaf tier: the kinds under it arrive as they are needed.

use bevy::prelude::*;

use crate::Animal;

/// 虫: the old sense, wider than *insect*.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[require(Animal)]
pub struct Critter;

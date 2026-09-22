//! 獣 Beast
//!
//! A leaf tier: the kinds under it arrive as they are needed.

use bevy::prelude::*;

use crate::Animal;

/// 獣: fox, wolf, hare, boar, deer, bear, macaque, cat, rat, dog, tanuki, horse, ox.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[require(Animal)]
pub struct Beast;

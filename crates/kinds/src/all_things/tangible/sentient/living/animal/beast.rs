//! 獣 Beast
//!
//! Its children are the files in `beast/`, beside this one.

use bevy::prelude::*;

use crate::Animal;

pub mod fox;

pub use fox::*;

/// 獣: fox, wolf, hare, boar, deer, bear, macaque, cat, rat, dog, tanuki, horse, ox.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[require(Animal)]
pub struct Beast;

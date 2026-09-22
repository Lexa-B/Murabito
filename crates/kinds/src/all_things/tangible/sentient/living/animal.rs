//! 動物 Animal
//!
//! Its children are the files in `animal/`, beside this one.

use bevy::prelude::*;

use crate::Living;

pub mod beast;
pub mod bird;
pub mod critter;
pub mod fish;

pub use beast::*;
pub use bird::*;
pub use critter::*;
pub use fish::*;

/// 動物
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[require(Living)]
pub struct Animal;

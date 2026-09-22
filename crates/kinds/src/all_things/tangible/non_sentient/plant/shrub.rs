//! 茂み Shrub
//!
//! Its children are the files in `shrub/`, beside this one.

use bevy::prelude::*;

use crate::Plant;

pub mod aoki;
pub mod azalea;

pub use aoki::*;
pub use azalea::*;

/// 茂み: azalea, aoki.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[require(Plant)]
pub struct Shrub;

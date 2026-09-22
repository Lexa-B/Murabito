//! 竹 Bamboo
//!
//! Its children are the files in `bamboo/`, beside this one.

use bevy::prelude::*;

use crate::Plant;

pub mod madake;
pub mod sasa;

pub use madake::*;
pub use sasa::*;

/// 竹: bamboo and sasa. Neither tree nor grass, as the saying goes.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[require(Plant)]
pub struct Bamboo;

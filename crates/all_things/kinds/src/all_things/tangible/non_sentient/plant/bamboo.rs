//! 竹 Bamboo
//!
//! Its children are the files in `bamboo/`, beside this one.

use bevy::prelude::*;
use murabito_identity::Kind;

use crate::Plant;

pub mod madake;
pub mod sasa;

pub use madake::*;
pub use sasa::*;

/// 竹: bamboo and sasa. Neither tree nor grass, as the saying goes.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "debug", derive(Reflect), reflect(Component))]
#[require(Plant, Kind = Kind::at(module_path!()))]
pub struct Bamboo;

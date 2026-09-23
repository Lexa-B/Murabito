//! 鳥 Bird
//!
//! Its children are the files in `bird/`, beside this one.

use bevy::prelude::*;

use crate::Animal;

pub mod chicken;
pub mod crane;
pub mod heron;
pub mod pheasant;

pub use chicken::*;
pub use crane::*;
pub use heron::*;
pub use pheasant::*;

/// 鳥: crane, heron, chicken, pheasant.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "debug", derive(Reflect), reflect(Component))]
#[require(Animal)]
pub struct Bird;

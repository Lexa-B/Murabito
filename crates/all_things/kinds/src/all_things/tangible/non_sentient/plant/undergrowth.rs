//! 草 Undergrowth
//!
//! Its children are the files in `undergrowth/`, beside this one.

use bevy::prelude::*;

use crate::Plant;

pub mod kusa;
pub mod kuzu;
pub mod shida;

pub use kusa::*;
pub use kuzu::*;
pub use shida::*;

/// 草: what grows at ground level: kusa, kuzu, shida. 草 is wider than *grass*.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "debug", derive(Reflect), reflect(Component))]
#[require(Plant)]
pub struct Undergrowth;

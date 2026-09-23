//! 生き物 Living
//!
//! Its children are the files in `living/`, beside this one.

use bevy::prelude::*;

use crate::Sentient;

pub mod animal;
pub mod human;

pub use animal::*;
pub use human::*;

/// 生き物: a living thing.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "debug", derive(Reflect), reflect(Component))]
#[require(Sentient)]
pub struct Living;

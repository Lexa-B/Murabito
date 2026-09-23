//! 幽 Spiritual
//!
//! Its children are the files in `spiritual/`, beside this one.

use bevy::prelude::*;

use crate::Sentient;

pub mod akuma;
pub mod kami;
pub mod rei;
pub mod yokai;

pub use akuma::*;
pub use kami::*;
pub use rei::*;
pub use yokai::*;

/// 幽: the unseen. Sentient, and not living.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "debug", derive(Reflect), reflect(Component))]
#[require(Sentient)]
pub struct Spiritual;

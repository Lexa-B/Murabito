//! 神 Kami
//!
//! A leaf tier: the kinds under it arrive as they are needed.

use bevy::prelude::*;

use crate::Spiritual;

/// 神
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[require(Spiritual)]
pub struct Kami;

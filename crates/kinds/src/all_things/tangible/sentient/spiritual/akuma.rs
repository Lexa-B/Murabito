//! 悪魔 Akuma
//!
//! A leaf tier: the kinds under it arrive as they are needed.

use bevy::prelude::*;

use crate::Spiritual;

/// 悪魔
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[require(Spiritual)]
pub struct Akuma;

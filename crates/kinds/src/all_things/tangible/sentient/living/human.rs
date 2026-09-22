//! 人間 Human
//!
//! A leaf tier: the kinds under it arrive as they are needed.

use bevy::prelude::*;

use crate::Living;

/// 人間
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[require(Living)]
pub struct Human;

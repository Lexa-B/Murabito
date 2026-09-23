//! 岩 Rock
//!
//! A leaf tier: the kinds under it arrive as they are needed.

use bevy::prelude::*;

use crate::Object;

/// 岩
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[require(Object)]
pub struct Rock;

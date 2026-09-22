//! 木 Tree
//!
//! A leaf tier: the kinds under it arrive as they are needed.

use bevy::prelude::*;

use crate::Plant;

/// 木: maple, sakura, hinoki, redpine, sugi.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[require(Plant)]
pub struct Tree;

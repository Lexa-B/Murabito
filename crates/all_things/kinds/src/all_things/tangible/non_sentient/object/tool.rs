//! 道具 Tool
//!
//! A leaf tier: the kinds under it arrive as they are needed.

use bevy::prelude::*;
use murabito_identity::Kind;

use crate::Object;

/// 道具
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "debug", derive(Reflect), reflect(Component))]
#[require(Object, Kind = Kind::at(module_path!()))]
pub struct Tool;

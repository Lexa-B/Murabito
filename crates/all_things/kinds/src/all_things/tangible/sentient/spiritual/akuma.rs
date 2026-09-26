//! 悪魔 Akuma
//!
//! A leaf tier: the kinds under it arrive as they are needed.

use bevy::prelude::*;
use murabito_identity::Kind;

use crate::Spiritual;

/// 悪魔
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "debug", derive(Reflect), reflect(Component))]
#[require(Spiritual, Kind = Kind::at(module_path!()))]
pub struct Akuma;

//! 虫 Critter
//!
//! A leaf tier: the kinds under it arrive as they are needed.

use bevy::prelude::*;
use murabito_identity::Kind;

use crate::Animal;

/// 虫: the old sense, wider than *insect*.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "debug", derive(Reflect), reflect(Component))]
#[require(Animal, Kind = Kind::at(module_path!()))]
pub struct Critter;

//! 事 Intangible
//!
//! A leaf tier: the kinds under it arrive as they are needed.

use bevy::prelude::*;
use murabito_identity::Kind;

use crate::AllThings;

/// 事: an event or a happening, with no place to point at. A tier nothing requires yet.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "debug", derive(Reflect), reflect(Component))]
#[require(AllThings, Kind = Kind::at(module_path!()))]
pub struct Intangible;

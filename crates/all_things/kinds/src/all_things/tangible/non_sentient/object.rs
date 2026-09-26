//! 物体 Object
//!
//! Its children are the files in `object/`, beside this one.

use bevy::prelude::*;
use murabito_identity::Kind;

use crate::NonSentient;

pub mod furniture;
pub mod rock;
pub mod tool;

pub use furniture::*;
pub use rock::*;
pub use tool::*;

/// 物体: things that are neither plant nor animal.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "debug", derive(Reflect), reflect(Component))]
#[require(NonSentient, Kind = Kind::at(module_path!()))]
pub struct Object;

//! 木 Tree
//!
//! Its children are the files in `tree/`, beside this one.

use bevy::prelude::*;
use murabito_identity::Kind;

use crate::Plant;

pub mod hinoki;
pub mod maple;
pub mod redpine;
pub mod sakura;
pub mod sugi;

pub use hinoki::*;
pub use maple::*;
pub use redpine::*;
pub use sakura::*;
pub use sugi::*;

/// 木: maple, sakura, hinoki, redpine, sugi.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "debug", derive(Reflect), reflect(Component))]
#[require(Plant, Kind = Kind::at(module_path!()))]
pub struct Tree;

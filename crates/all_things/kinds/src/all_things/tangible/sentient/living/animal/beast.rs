//! 獣 Beast
//!
//! Its children are the files in `beast/`, beside this one.

use bevy::prelude::*;
use murabito_identity::Kind;

use crate::Animal;

pub mod bear;
pub mod boar;
pub mod cat;
pub mod deer;
pub mod dog;
pub mod fox;
pub mod hare;
pub mod horse;
pub mod macaque;
pub mod ox;
pub mod rat;
pub mod tanuki;
pub mod wolf;

pub use bear::*;
pub use boar::*;
pub use cat::*;
pub use deer::*;
pub use dog::*;
pub use fox::*;
pub use hare::*;
pub use horse::*;
pub use macaque::*;
pub use ox::*;
pub use rat::*;
pub use tanuki::*;
pub use wolf::*;

/// 獣: fox, wolf, hare, boar, deer, bear, macaque, cat, rat, dog, tanuki, horse, ox.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "debug", derive(Reflect), reflect(Component))]
#[require(Animal, Kind = Kind::at(module_path!()))]
pub struct Beast;

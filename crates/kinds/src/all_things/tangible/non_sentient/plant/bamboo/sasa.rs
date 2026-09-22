//! 笹 Sasa

use bevy::prelude::*;

use crate::{Bamboo, Model};

/// 笹: dwarf bamboo, knee high. Drawn from its first version in summer unless whatever spawns it
/// says which.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[require(Bamboo, Model = Model("entity_models/flora/bamboo/sasa-00-a-summer.glb"))]
pub struct Sasa;

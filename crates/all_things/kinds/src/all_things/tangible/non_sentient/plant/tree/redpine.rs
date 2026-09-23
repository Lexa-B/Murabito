//! 赤松 Redpine

use bevy::prelude::*;

use crate::{Model, Tree};

/// 赤松: the Japanese red pine (アカマツ). Drawn from its first version in summer unless whatever spawns it
/// says which.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "debug", derive(Reflect), reflect(Component))]
#[require(Tree, Model = Model("entity_models/flora/trees/redpine-00-a-summer.glb"))]
pub struct Redpine;

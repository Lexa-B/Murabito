//! 桜 Sakura

use bevy::prelude::*;

use crate::{Model, Tree};

/// 桜: the mountain cherry (山桜). Drawn from its first version in summer unless whatever spawns it
/// says which.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "debug", derive(Reflect), reflect(Component))]
#[require(Tree, Model = Model("entity_models/flora/trees/sakura-00-a-summer.glb"))]
pub struct Sakura;

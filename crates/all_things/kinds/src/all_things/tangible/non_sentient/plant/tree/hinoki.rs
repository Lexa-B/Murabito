//! 檜 Hinoki

use bevy::prelude::*;

use crate::{Model, Tree};

/// 檜: the hinoki cypress. Drawn from its first version in summer unless whatever spawns it
/// says which.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[require(Tree, Model = Model("entity_models/flora/trees/hinoki-00-a-summer.glb"))]
pub struct Hinoki;

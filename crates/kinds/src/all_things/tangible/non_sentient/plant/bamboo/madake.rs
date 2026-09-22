//! 真竹 Madake

use bevy::prelude::*;

use crate::{Bamboo, Model};

/// 真竹: the giant timber bamboo, the bamboo of groves. Drawn from its first version in summer unless whatever spawns it
/// says which.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[require(Bamboo, Model = Model("entity_models/flora/bamboo/bamboo-00-a-summer.glb"))]
pub struct Madake;

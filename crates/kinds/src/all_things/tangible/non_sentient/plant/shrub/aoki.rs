//! 青木 Aoki

use bevy::prelude::*;

use crate::{Model, Shrub};

/// 青木: the aucuba. Drawn from its first version in summer unless whatever spawns it
/// says which.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[require(Shrub, Model = Model("entity_models/flora/shrubs/aoki-00-a-summer.glb"))]
pub struct Aoki;

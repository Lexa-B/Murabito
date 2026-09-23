//! 躑躅 Azalea

use bevy::prelude::*;

use crate::{Model, Shrub};

/// 躑躅: the mountain azalea (ヤマツツジ). Drawn from its first version in summer unless whatever spawns it
/// says which.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[require(Shrub, Model = Model("entity_models/flora/shrubs/azalea-00-a-summer.glb"))]
pub struct Azalea;

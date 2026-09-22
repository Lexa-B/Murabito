//! 草 Kusa

use bevy::prelude::*;

use crate::{Model, Undergrowth};

/// 草: a tuft of wild grass. Drawn from its first version in summer unless whatever spawns it
/// says which.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[require(Undergrowth, Model = Model("entity_models/flora/ground/kusa-00-a-summer.glb"))]
pub struct Kusa;

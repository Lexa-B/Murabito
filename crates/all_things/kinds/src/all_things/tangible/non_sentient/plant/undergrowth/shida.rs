//! 羊歯 Shida

use bevy::prelude::*;

use crate::{Model, Undergrowth};

/// 羊歯: a woodland fern (ベニシダ). Drawn from its first version in summer unless whatever spawns it
/// says which.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[require(Undergrowth, Model = Model("entity_models/flora/ground/shida-00-a-summer.glb"))]
pub struct Shida;

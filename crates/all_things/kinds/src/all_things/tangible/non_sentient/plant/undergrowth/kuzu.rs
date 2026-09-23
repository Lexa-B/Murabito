//! 葛 Kuzu

use bevy::prelude::*;

use crate::{Model, Undergrowth};

/// 葛: a kudzu patch. Drawn from its first version in summer unless whatever spawns it
/// says which.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "debug", derive(Reflect), reflect(Component))]
#[require(Undergrowth, Model = Model("entity_models/flora/ground/kuzu-00-a-summer.glb"))]
pub struct Kuzu;

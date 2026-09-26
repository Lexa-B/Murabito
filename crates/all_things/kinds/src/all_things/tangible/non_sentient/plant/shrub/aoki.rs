//! 青木 Aoki

use bevy::prelude::*;
use murabito_identity::Kind;

use crate::{Model, Shrub};

/// 青木: the aucuba. Drawn from its first version in summer unless whatever spawns it
/// says which.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "debug", derive(Reflect), reflect(Component))]
#[require(Shrub, Model = Model("entity_models/flora/shrubs/aoki-00-a-summer.glb"), Kind = Kind::at(module_path!()))]
pub struct Aoki;

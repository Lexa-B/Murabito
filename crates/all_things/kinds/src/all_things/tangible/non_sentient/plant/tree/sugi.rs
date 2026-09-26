//! 杉 Sugi

use bevy::prelude::*;
use murabito_identity::Kind;

use crate::{Model, Tree};

/// 杉: the Japanese cedar (スギ). Drawn from its first version in summer unless whatever spawns it
/// says which.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "debug", derive(Reflect), reflect(Component))]
#[require(Tree, Model = Model("entity_models/flora/trees/sugi-00-a-summer.glb"), Kind = Kind::at(module_path!()))]
pub struct Sugi;

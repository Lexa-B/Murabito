//! 杉 Sugi

use bevy::prelude::*;

use crate::{Model, Tree};

/// 杉: the Japanese cedar (スギ). Drawn from its first version in summer unless whatever spawns it
/// says which.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[require(Tree, Model = Model("entity_models/flora/trees/sugi-00-a-summer.glb"))]
pub struct Sugi;

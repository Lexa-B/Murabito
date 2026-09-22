//! 鹿 Deer

use bevy::prelude::*;
use murabito_movement::Locomotion;

use crate::{Beast, Model};

/// 鹿: the sika deer (ニホンジカ). Its pace is a guess at a walk, 5 shaku (about
/// 1.5 m) a second, until someone who knows says otherwise.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[require(
    Beast,
    Locomotion = Locomotion { speed: 5.0, turn_speed: 180.0 },
    Model = Model("entity_models/living/animals/deer.glb"),
)]
pub struct Deer;

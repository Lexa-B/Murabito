//! 馬 Horse

use bevy::prelude::*;
use murabito_movement::Locomotion;

use crate::{Beast, Model};

/// 馬: the Kiso horse (木曽馬). Its pace is a guess at a walk, 5 shaku (about
/// 1.5 m) a second, until someone who knows says otherwise.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[require(
    Beast,
    Locomotion = Locomotion { speed: 5.0, turn_speed: 90.0 },
    Model = Model("entity_models/living/animals/horse.glb"),
)]
pub struct Horse;

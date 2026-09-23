//! 犬 Dog

use bevy::prelude::*;
use murabito_movement::Locomotion;

use crate::{Beast, Model};

/// 犬: the native dog (柴犬). Its pace is a guess at a walk, 4 shaku (about
/// 1.2 m) a second, until someone who knows says otherwise.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "debug", derive(Reflect), reflect(Component))]
#[require(
    Beast,
    Locomotion = Locomotion { speed: 4.0, turn_speed: 180.0 },
    Model = Model("entity_models/living/animals/dog.glb"),
)]
pub struct Dog;

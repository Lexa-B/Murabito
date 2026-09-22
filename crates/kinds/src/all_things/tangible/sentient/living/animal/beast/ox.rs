//! 牛 Ox

use bevy::prelude::*;
use murabito_movement::Locomotion;

use crate::{Beast, Model};

/// 牛: the native draft ox. Its pace is a guess at a walk, 2 shaku (about
/// 0.6 m) a second, until someone who knows says otherwise.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[require(
    Beast,
    Locomotion = Locomotion { speed: 2.0, turn_speed: 60.0 },
    Model = Model("entity_models/living/animals/ox.glb"),
)]
pub struct Ox;

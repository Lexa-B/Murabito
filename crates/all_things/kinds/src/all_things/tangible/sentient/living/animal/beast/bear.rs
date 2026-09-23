//! 熊 Bear

use bevy::prelude::*;
use murabito_movement::Locomotion;

use crate::{Beast, Model};

/// 熊: the Japanese black bear (ツキノワグマ). Its pace is a guess at a walk, 3 shaku (about
/// 0.9 m) a second, until someone who knows says otherwise.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[require(
    Beast,
    Locomotion = Locomotion { speed: 3.0, turn_speed: 90.0 },
    Model = Model("entity_models/living/animals/bear.glb"),
)]
pub struct Bear;

//! 狼 Wolf

use bevy::prelude::*;
use murabito_movement::Locomotion;

use crate::{Beast, Model};

/// 狼: the Japanese wolf (ニホンオオカミ). Its pace is a guess at a walk, 5 shaku (about
/// 1.5 m) a second, until someone who knows says otherwise.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "debug", derive(Reflect), reflect(Component))]
#[require(
    Beast,
    Locomotion = Locomotion { speed: 5.0, turn_speed: 180.0 },
    Model = Model("entity_models/living/animals/wolf.glb"),
)]
pub struct Wolf;

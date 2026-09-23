//! 鶏 Chicken

use bevy::prelude::*;
use murabito_movement::Locomotion;

use crate::{Bird, Model};

/// 鶏: the native chicken (地鶏). Its pace is a guess at a walk, 2 shaku (about
/// 0.6 m) a second, until someone who knows says otherwise.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "debug", derive(Reflect), reflect(Component))]
#[require(
    Bird,
    Locomotion = Locomotion { speed: 2.0, turn_speed: 240.0 },
    Model = Model("entity_models/living/animals/chicken.glb"),
)]
pub struct Chicken;

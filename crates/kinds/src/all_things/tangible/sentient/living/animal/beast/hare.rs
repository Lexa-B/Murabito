//! 兎 Hare

use bevy::prelude::*;
use murabito_movement::Locomotion;

use crate::{Beast, Model};

/// 兎: the Japanese hare (野兎). Its pace is a guess at a walk, 5 shaku (about
/// 1.5 m) a second, until someone who knows says otherwise.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[require(
    Beast,
    Locomotion = Locomotion { speed: 5.0, turn_speed: 240.0 },
    Model = Model("entity_models/living/animals/hare.glb"),
)]
pub struct Hare;

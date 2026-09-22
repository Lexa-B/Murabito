//! 雉 Pheasant

use bevy::prelude::*;
use murabito_movement::Locomotion;

use crate::{Bird, Model};

/// 雉: the green pheasant (キジ). Its pace is a guess at a walk, 2.5 shaku (about
/// 0.8 m) a second, until someone who knows says otherwise.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[require(
    Bird,
    Locomotion = Locomotion { speed: 2.5, turn_speed: 240.0 },
    Model = Model("entity_models/living/animals/pheasant.glb"),
)]
pub struct Pheasant;

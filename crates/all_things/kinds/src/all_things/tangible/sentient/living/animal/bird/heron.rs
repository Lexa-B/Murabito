//! 鷺 Heron

use bevy::prelude::*;
use murabito_movement::Locomotion;

use crate::{Bird, Model};

/// 鷺: the grey heron (アオサギ). Its pace is a guess at a walk, 2 shaku (about
/// 0.6 m) a second, until someone who knows says otherwise.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[require(
    Bird,
    Locomotion = Locomotion { speed: 2.0, turn_speed: 120.0 },
    Model = Model("entity_models/living/animals/heron.glb"),
)]
pub struct Heron;

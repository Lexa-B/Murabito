//! 狐 Fox

use bevy::prelude::*;
use murabito_movement::Locomotion;

use crate::{Beast, Model};

/// 狐: the red fox. A brisk walk of 4 shaku (about 1.2 m) a second, and half a turn a
/// second: the numbers the first attempt settled on by eye.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[require(
    Beast,
    Locomotion = Locomotion { speed: 4.0, turn_speed: 180.0 },
    Model = Model("entity_models/living/animals/fox.glb"),
)]
pub struct Fox;

//! 猿 Macaque

use bevy::prelude::*;
use murabito_identity::Kind;
use murabito_movement::Locomotion;

use crate::{Beast, Model};

/// 猿: the Japanese macaque (ニホンザル). Its pace is a guess at a walk, 3 shaku (about
/// 0.9 m) a second, until someone who knows says otherwise.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "debug", derive(Reflect), reflect(Component))]
#[require(
    Beast,
    Locomotion = Locomotion { speed: 3.0, turn_speed: 180.0 },
    Model = Model("entity_models/living/animals/macaque.glb"),
    Kind = Kind::at(module_path!()),
)]
pub struct Macaque;

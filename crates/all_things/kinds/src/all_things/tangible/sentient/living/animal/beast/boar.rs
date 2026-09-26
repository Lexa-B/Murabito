//! 猪 Boar

use bevy::prelude::*;
use murabito_identity::Kind;
use murabito_movement::Locomotion;

use crate::{Beast, Model};

/// 猪: the Japanese wild boar (ニホンイノシシ). Its pace is a guess at a walk, 4 shaku (about
/// 1.2 m) a second, until someone who knows says otherwise.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "debug", derive(Reflect), reflect(Component))]
#[require(
    Beast,
    Locomotion = Locomotion { speed: 4.0, turn_speed: 120.0 },
    Model = Model("entity_models/living/animals/boar.glb"),
    Kind = Kind::at(module_path!()),
)]
pub struct Boar;

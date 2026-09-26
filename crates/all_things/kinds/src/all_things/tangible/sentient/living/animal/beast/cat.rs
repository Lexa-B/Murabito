//! 猫 Cat

use bevy::prelude::*;
use murabito_identity::Kind;
use murabito_movement::Locomotion;

use crate::{Beast, Model};

/// 猫: the cat, a calico (三毛). Its pace is a guess at a walk, 3 shaku (about
/// 0.9 m) a second, until someone who knows says otherwise.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "debug", derive(Reflect), reflect(Component))]
#[require(
    Beast,
    Locomotion = Locomotion { speed: 3.0, turn_speed: 240.0 },
    Model = Model("entity_models/living/animals/cat.glb"),
    Kind = Kind::at(module_path!()),
)]
pub struct Cat;

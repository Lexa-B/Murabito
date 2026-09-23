//! 妖怪 Yokai
//!
//! A leaf tier: the kinds under it arrive as they are needed.

use bevy::prelude::*;

use crate::Spiritual;

/// 妖怪: tsukumogami, yoko, bakedanuki, ningyo, kaika, …
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[require(Spiritual)]
pub struct Yokai;

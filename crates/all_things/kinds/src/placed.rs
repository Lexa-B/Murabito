//! A tangible thing has a place and a sentient thing faces somewhere, and both are the
//! instance's, not the kind's.

use bevy::prelude::*;
use murabito_placement::{Facing, VoxelPosition};

use crate::{Sentient, Tangible};

/// The tree can't require a `VoxelPosition`, since a `require` always carries a value
/// and no kind can say where an instance of it stands. So the rule is checked at spawn
/// instead: a tangible thing spawned with no place is a spawner's mistake, and this says
/// so at once rather than letting a tree exist nowhere.
pub(crate) fn check_placed(added: On<Add, Tangible>, placed: Query<Has<VoxelPosition>>) {
    let entity = added.entity;
    if !placed.get(entity).unwrap_or(true) {
        panic!(
            "a tangible thing ({entity}) was spawned with no VoxelPosition: its place is given at spawn, beside the kind"
        );
    }
}

/// The same for which way a sentient thing faces: a kind has no direction to give, so
/// the spawner must, and a missing one is said at once.
pub(crate) fn check_facing(added: On<Add, Sentient>, facing: Query<Has<Facing>>) {
    let entity = added.entity;
    if !facing.get(entity).unwrap_or(true) {
        panic!(
            "a sentient thing ({entity}) was spawned with no Facing: which way it faces is given at spawn, beside the kind"
        );
    }
}

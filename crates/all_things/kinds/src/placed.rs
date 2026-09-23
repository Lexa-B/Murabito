//! A tangible thing has a place, and the place is the instance's, not the kind's.

use bevy::prelude::*;
use murabito_placement::VoxelPosition;

use crate::Tangible;

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

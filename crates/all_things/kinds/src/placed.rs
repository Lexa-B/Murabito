//! A tangible thing has a place and faces some way, and both are the instance's, not
//! the kind's.

use bevy::prelude::*;
use murabito_placement::{Facing, VoxelPosition};

use crate::Tangible;

/// The tree can't require a `VoxelPosition` or a `Facing`, since a `require` always
/// carries a value and no kind can say where an instance of it stands or which way it
/// faces. So the rule is checked at spawn instead: a tangible thing spawned without
/// either is a spawner's mistake, and this says so at once rather than letting a tree
/// exist nowhere. `place` needs both to put a model anywhere, so a thing missing one
/// would stand at the world origin on screen whatever its voxel said.
pub(crate) fn check_placed(
    added: On<Add, Tangible>,
    placed: Query<(Has<VoxelPosition>, Has<Facing>)>,
) {
    let entity = added.entity;
    let (has_place, has_facing) = placed.get(entity).unwrap_or((true, true));
    if !has_place {
        panic!(
            "a tangible thing ({entity}) was spawned with no VoxelPosition: its place is given at spawn, beside the kind"
        );
    }
    if !has_facing {
        panic!(
            "a tangible thing ({entity}) was spawned with no Facing: which way it faces is given at spawn, beside the kind"
        );
    }
}

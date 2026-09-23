//! Stamping a `ThingId` on every thing, at the root of the tree.

use bevy::prelude::*;
use murabito_identity::{NextThingId, ThingId};

use crate::AllThings;

/// Gives anything that just became a thing its number, unless it was spawned with one
/// already, as a loader replaying a save would: what is given wins. Runs on `AllThings`,
/// the root every kind requires, so a light or a camera never gets one.
pub(crate) fn stamp_id(
    added: On<Add, AllThings>,
    given: Query<Has<ThingId>>,
    mut counter: ResMut<NextThingId>,
    mut commands: Commands,
) {
    let entity = added.entity;
    if given.get(entity).unwrap_or(true) {
        return;
    }
    commands.entity(entity).insert(counter.mint());
}

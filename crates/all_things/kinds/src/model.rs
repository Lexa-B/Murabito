//! `Model`: which file a kind is drawn from.
//!
//! A kind names its model as a path under `assets/`, and one observer here turns that
//! into the loaded scene the moment the component lands. A `require` constructor is a
//! plain function with no world in reach, so it can't ask the asset server for a
//! handle; the path is what a kind can say about itself, and loading is the crate's job.

use bevy::prelude::*;

/// The glTF file a thing is drawn from, relative to `assets/`. Put on a kind by its
/// `require`; the entity gets a `WorldAssetRoot` for it as soon as it is spawned.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Model(pub &'static str);

/// Loads the model of anything that just got one.
pub(crate) fn load_model(
    added: On<Add, Model>,
    models: Query<&Model>,
    assets: Res<AssetServer>,
    mut commands: Commands,
) {
    let entity = added.entity;
    let Ok(model) = models.get(entity) else {
        return;
    };
    let scene = assets.load(GltfAssetLabel::Scene(0).from_asset(model.0));
    commands.entity(entity).insert(WorldAssetRoot(scene));
}

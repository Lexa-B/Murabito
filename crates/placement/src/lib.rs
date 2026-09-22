//! Where a thing stands and which way it faces.
//!
//! Two plain components any entity in the world can carry, and the one system that
//! keeps a model where they say: nothing in them says what the entity is, or whether it
//! ever moves. A tree has a `VoxelPosition` and never leaves it; the movement mechanism
//! writes to these, and the senses read them, and neither needs to know about the other.
//! `Docs/movement_readme.md`'s first section is the design this implements.
//!
//! Compass: **east is +X, north is −Z, up is +Y.** Facing is one of the twelve compass
//! directions, never up or down.

use bevy::prelude::*;
use murabito_hexcoords::{Direction, VoxelCoord};

pub struct PlacementPlugin;

impl Plugin for PlacementPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, place);
    }
}

/// The voxel an entity is in. Integer: an entity is in exactly one cell, and this
/// changes only when a step lands.
///
/// Requires a `Transform`, so `place` has somewhere to put the model.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
#[require(Transform)]
pub struct VoxelPosition(pub VoxelCoord);

/// Which of the twelve compass directions an entity faces.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Facing(pub Direction);

/// An entity whose position or facing changed since `place` last ran.
type Moved = Or<(Changed<VoxelPosition>, Changed<Facing>)>;

/// Keeps a model where its cell is: at the centre of the voxel's bottom face, turned to
/// its heading. The only thing that writes a placed entity's `Transform`, and only when
/// the position or facing changed, so a standing entity is left alone. Presentation:
/// it runs per frame, in `Update`, and the simulation never reads what it writes.
fn place(mut placed: Query<(&VoxelPosition, &Facing, &mut Transform), Moved>) {
    for (position, facing, mut transform) in &mut placed {
        *transform =
            Transform::from_translation(position.0.to_world()).with_rotation(facing.0.heading());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn voxel(q: i32, r: i32, layer: i32) -> VoxelCoord {
        VoxelCoord::new(q, r, -q - r, layer).expect("on the plane")
    }

    /// A headless app: no window, no GPU. `MinimalPlugins` brings the schedules, and
    /// placing a model needs nothing else.
    fn app_with(position: VoxelCoord, facing: Direction) -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, PlacementPlugin));
        let entity = app
            .world_mut()
            .spawn((VoxelPosition(position), Facing(facing)))
            .id();
        (app, entity)
    }

    fn transform_of(app: &App, entity: Entity) -> Transform {
        *app.world().get::<Transform>(entity).expect("a transform")
    }

    #[test]
    fn a_placed_entity_gets_a_transform_without_asking() {
        let (app, entity) = app_with(voxel(0, 0, 0), Direction::N);

        assert!(app.world().get::<Transform>(entity).is_some());
    }

    #[test]
    fn a_placed_entity_stands_at_its_voxels_centre_facing_its_heading() {
        let (mut app, entity) = app_with(voxel(1, -2, 1), Direction::N);

        app.update();

        let transform = transform_of(&app, entity);
        assert_eq!(transform.translation, voxel(1, -2, 1).to_world());
        assert!(transform.rotation.abs_diff_eq(Quat::IDENTITY, 1e-6));
    }

    #[test]
    fn turning_to_face_east_turns_the_model_to_face_east() {
        let (mut app, entity) = app_with(voxel(0, 0, 0), Direction::N);
        app.update();

        app.world_mut()
            .get_mut::<Facing>(entity)
            .expect("a facing")
            .0 = Direction::E;
        app.update();

        let faces = transform_of(&app, entity).forward();
        assert!(faces.abs_diff_eq(Vec3::X, 1e-5), "faces {faces}");
    }

    #[test]
    fn moving_to_another_voxel_moves_the_model_there() {
        let (mut app, entity) = app_with(voxel(0, 0, 0), Direction::N);
        app.update();

        app.world_mut()
            .get_mut::<VoxelPosition>(entity)
            .expect("a position")
            .0 = voxel(3, 0, 2);
        app.update();

        assert_eq!(
            transform_of(&app, entity).translation,
            voxel(3, 0, 2).to_world()
        );
    }

    #[test]
    fn a_standing_entity_is_left_alone() {
        let (mut app, entity) = app_with(voxel(0, 0, 0), Direction::N);
        app.update();
        let nudged = Transform::from_xyz(9.0, 9.0, 9.0);
        *app.world_mut()
            .get_mut::<Transform>(entity)
            .expect("a transform") = nudged;

        app.update();

        assert_eq!(transform_of(&app, entity), nudged);
    }
}

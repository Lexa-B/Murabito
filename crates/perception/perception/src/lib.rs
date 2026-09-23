//! What every sense shares, and no sense owns.
//!
//! A sense is a body's own reading of the world: vision reports what it sees, hearing
//! what it hears, each in its own shape and in its own list, and nothing here merges
//! them. What they have in common is *when* they run and *what stands where*:
//!
//! - [`Occupancy`] is which things stand in which voxel, rebuilt every tick from every
//!   `VoxelPosition`. To sight it is what blocks the view, and what was seen once a cell
//!   is in view; to hearing and smell, later, it is what attenuates.
//! - [`PerceptionSet`] is the `FixedUpdate` set the senses run in, after the mechanisms
//!   have moved things and after the map is rebuilt, so a step that lands on a tick is
//!   in the map and seen on that tick.
//!
//! Perceiving is simulation, so it ticks with the rest and freezes with it. The debts
//! this layer will take up when facets return, obscuring and heights, and blurring a
//! poorly seen thing into something vaguer, belong here too, since they apply to every
//! sense's list alike. `Docs/perception_readme.md` is the design.

use std::collections::HashMap;

use bevy::prelude::*;
use murabito_hexcoords::VoxelCoord;
use murabito_placement::VoxelPosition;
use murabito_progress::MechanismSet;

/// When perceiving happens, in `FixedUpdate`: the map of what stands where is gathered,
/// then every sense reads it. A sense adds its system with `.in_set(PerceptionSet::Sense)`;
/// whatever acts on what was sensed orders itself after that.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PerceptionSet {
    /// `Occupancy` is rebuilt. After the mechanisms, so the tick's steps have landed.
    Gather,
    /// The senses run, each writing its own list.
    Sense,
}

pub struct PerceptionPlugin;

impl Plugin for PerceptionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Occupancy>()
            .configure_sets(
                FixedUpdate,
                (PerceptionSet::Gather, PerceptionSet::Sense)
                    .chain()
                    .after(MechanismSet),
            )
            .add_systems(FixedUpdate, gather.in_set(PerceptionSet::Gather));
        #[cfg(feature = "debug")]
        app.register_type::<Occupancy>();
    }
}

/// Which things stand in which voxel. Rebuilt whole each tick from every entity with a
/// `VoxelPosition`, so nothing registers or unregisters and nothing can be forgotten.
/// A body is in exactly one voxel; a voxel may hold several things.
#[derive(Resource, Default, Debug, Clone)]
#[cfg_attr(feature = "debug", derive(Reflect), reflect(opaque))]
#[cfg_attr(feature = "debug", reflect(Resource, Serialize))]
pub struct Occupancy(HashMap<VoxelCoord, Vec<Entity>>);

/// On the debug wire: the map as a list of its entries, each `[voxel, things]`, in the
/// map's own order, which is none. JSON can't key an object by a struct, so this is the
/// nearest thing to the map as it is in memory.
#[cfg(feature = "debug")]
impl serde::Serialize for Occupancy {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(self.0.iter())
    }
}

impl Occupancy {
    /// A fixed map, for a test or for anything that needs one that is not the world's.
    pub fn from_cells(cells: impl IntoIterator<Item = (VoxelCoord, Entity)>) -> Self {
        let mut occupancy = Self::default();
        for (voxel, entity) in cells {
            occupancy.0.entry(voxel).or_default().push(entity);
        }
        occupancy
    }

    /// What stands in this voxel: nothing, one thing, or several.
    pub fn at(&self, voxel: VoxelCoord) -> &[Entity] {
        self.0.get(&voxel).map_or(&[], Vec::as_slice)
    }

    pub fn is_occupied(&self, voxel: VoxelCoord) -> bool {
        self.0.contains_key(&voxel)
    }

    /// Every occupied voxel and what stands in it, in no particular order.
    pub fn iter(&self) -> impl Iterator<Item = (VoxelCoord, &[Entity])> {
        self.0
            .iter()
            .map(|(voxel, things)| (*voxel, things.as_slice()))
    }

    /// How many voxels have something in them.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Rebuilds the map from scratch. There are a handful of things and the map is small;
/// when there are thousands and most stand still, this is where the rebuild becomes an
/// update on `Changed<VoxelPosition>` (`TODO.md`).
fn gather(mut occupancy: ResMut<Occupancy>, things: Query<(Entity, &VoxelPosition)>) {
    occupancy.0.clear();
    for (entity, position) in &things {
        occupancy.0.entry(position.0).or_default().push(entity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Through the serializer the debug server uses: the map is a list of pairs.
    #[cfg(feature = "debug")]
    #[test]
    fn on_the_wire_the_map_is_a_list_of_voxel_and_things_pairs() {
        use bevy::reflect::serde::ReflectSerializer;
        use serde_json::{Value, json};

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, ProgressPlugin, PerceptionPlugin));
        let (a, b) = (
            app.world_mut().spawn_empty().id(),
            app.world_mut().spawn_empty().id(),
        );
        let occupancy = Occupancy::from_cells([
            (voxel(1, -1, 0), a),
            (voxel(1, -1, 0), b),
            (voxel(0, 2, 1), a),
        ]);

        let registry = app.world().resource::<AppTypeRegistry>().read();
        let wire = serde_json::to_value(ReflectSerializer::new(&occupancy, &registry))
            .expect("serialises");
        println!("WIRE {wire}");
        let Value::Array(entries) = &wire["murabito_perception::Occupancy"] else {
            panic!("a list of entries under the type path: {wire}");
        };
        let mut entries = entries.clone();
        entries.sort_by_key(Value::to_string);
        let mut expected = vec![
            json!([{"q": 1, "r": -1, "layer": 0}, [a.to_bits(), b.to_bits()]]),
            json!([{"q": 0, "r": 2, "layer": 1}, [a.to_bits()]]),
        ];
        expected.sort_by_key(Value::to_string);
        assert_eq!(entries, expected);
    }
    use murabito_hexcoords::Direction;
    use murabito_movement::{Locomotion, MovementPlugin, Step};
    use murabito_placement::Facing;
    use murabito_progress::ProgressPlugin;

    fn voxel(q: i32, r: i32, layer: i32) -> VoxelCoord {
        VoxelCoord::new(q, r, -q - r, layer).expect("on the plane")
    }

    /// A headless app whose every `update` is exactly one 64 Hz tick, with the
    /// mechanisms running so a thing can step. An app's first update only starts its
    /// clock, so it is spent here.
    fn ticking_app() -> App {
        use bevy::time::TimeUpdateStrategy;

        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            ProgressPlugin,
            MovementPlugin,
            PerceptionPlugin,
        ));
        app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
        app.update();
        app
    }

    fn occupancy(app: &App) -> &Occupancy {
        app.world().resource::<Occupancy>()
    }

    #[test]
    fn a_placed_thing_is_in_its_voxel_after_a_tick() {
        let mut app = ticking_app();
        let tree = app.world_mut().spawn(VoxelPosition(voxel(2, -1, 0))).id();

        app.update();

        assert_eq!(occupancy(&app).at(voxel(2, -1, 0)), [tree]);
        assert!(occupancy(&app).is_occupied(voxel(2, -1, 0)));
        assert!(!occupancy(&app).is_occupied(voxel(0, 0, 0)));
    }

    #[test]
    fn two_things_in_one_voxel_are_both_listed() {
        let mut app = ticking_app();
        let one = app.world_mut().spawn(VoxelPosition(voxel(0, 0, 0))).id();
        let other = app.world_mut().spawn(VoxelPosition(voxel(0, 0, 0))).id();

        app.update();

        let mut there = occupancy(&app).at(voxel(0, 0, 0)).to_vec();
        there.sort();
        let mut both = [one, other];
        both.sort();
        assert_eq!(there, both);
        assert_eq!(occupancy(&app).len(), 1);
    }

    #[test]
    fn a_thing_with_no_position_is_nowhere() {
        let mut app = ticking_app();
        app.world_mut().spawn(Facing(Direction::E));

        app.update();

        assert!(occupancy(&app).is_empty());
    }

    #[test]
    fn a_stepping_body_is_in_its_new_voxel_on_the_tick_it_lands_and_not_before() {
        let mut app = ticking_app();
        let body = app
            .world_mut()
            .spawn((
                VoxelPosition(voxel(0, 0, 0)),
                Facing(Direction::E),
                Locomotion {
                    speed: 4.0,
                    turn_speed: 180.0,
                },
                Step(Direction::E),
            ))
            .id();

        // An edge step at 4 shaku/s lands on tick 16.
        (0..15).for_each(|_| app.update());
        assert_eq!(occupancy(&app).at(voxel(0, 0, 0)), [body]);
        assert!(!occupancy(&app).is_occupied(voxel(1, 0, 0)));

        app.update();

        assert_eq!(occupancy(&app).at(voxel(1, 0, 0)), [body]);
        assert!(!occupancy(&app).is_occupied(voxel(0, 0, 0)));
    }

    #[test]
    fn a_despawned_thing_is_gone_on_the_next_tick() {
        let mut app = ticking_app();
        let tree = app.world_mut().spawn(VoxelPosition(voxel(3, 0, 0))).id();
        app.update();
        assert!(occupancy(&app).is_occupied(voxel(3, 0, 0)));

        app.world_mut().despawn(tree);
        app.update();

        assert!(occupancy(&app).is_empty());
    }

    #[test]
    fn a_fixed_map_reads_the_same_as_a_gathered_one() {
        let mut world = World::new();
        let a = world.spawn_empty().id();
        let b = world.spawn_empty().id();
        let map = Occupancy::from_cells([
            (voxel(0, 0, 0), a),
            (voxel(0, 0, 0), b),
            (voxel(1, 0, 0), a),
        ]);

        assert_eq!(map.at(voxel(0, 0, 0)), [a, b]);
        assert_eq!(map.at(voxel(1, 0, 0)), [a]);
        assert_eq!(map.at(voxel(5, 5, 0)), []);
        assert_eq!(map.iter().count(), 2);
    }
}

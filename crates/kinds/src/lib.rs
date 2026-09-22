//! What a thing *is*: the tree of kinds.
//!
//! Anything in the world that is informative or interactive is a thing, and every thing
//! is of a kind. The kinds form one tree, and a kind inherits everything its parents
//! have, the way classes do: a fox is a beast is an animal is a living thing. Each kind
//! is a unit component, and `#[require(...)]` is both *extends* and the members: a
//! kind's parent comes first in its list, then what every thing of that kind has.
//! Spawning a kind inserts the whole chain, so the node itself says what it has, and
//! the compiler holds it to that.
//!
//! **One node, one file, and the folders are the tree.** A node with children is a
//! file beside a folder of the same name holding them, so `src/` reads the same as this:
//!
//! ```text
//! AllThings  諸法                              all_things.rs
//! ├─ Tangible  物                              all_things/tangible.rs
//! │  ├─ Sentient  有情     whatever takes いる   all_things/tangible/sentient.rs
//! │  │  ├─ Living  生き物                          …/sentient/living.rs
//! │  │  │  ├─ Animal  動物                            …/living/animal.rs
//! │  │  │  │  ├─ Beast  獣                               …/animal/beast.rs
//! │  │  │  │  ├─ Bird  鳥
//! │  │  │  │  ├─ Critter  虫
//! │  │  │  │  └─ Fish  魚
//! │  │  │  └─ Human  人間
//! │  │  └─ Spiritual  幽
//! │  │     ├─ Kami  神
//! │  │     ├─ Yokai  妖怪
//! │  │     ├─ Akuma  悪魔
//! │  │     └─ Rei  霊
//! │  └─ NonSentient  非情  whatever takes ある
//! │     ├─ Plant  植物
//! │     │  ├─ Tree  木
//! │     │  ├─ Bamboo  竹
//! │     │  ├─ Shrub  茂み
//! │     │  └─ Undergrowth  草
//! │     └─ Object  物体
//! │        ├─ Tool  道具
//! │        ├─ Furniture  家具
//! │        └─ Rock  岩
//! └─ Intangible  事
//! ```
//!
//! Every kind is re-exported here flat, so a caller writes `murabito_kinds::Beast` and
//! the depth of the tree is the crate's business. English keys, the kanji beside them in
//! each file; display names are the i18n catalogues' job.
//!
//! A kind overrides a member by requiring the same component directly with its own
//! constructor: Bevy takes a direct `#[require]` over an inherited one, and otherwise the
//! first found depth-first through the parents in list order. So a species' numbers sit
//! in the species' own `require`, and a tier's are the placeholder its children override.
//!
//! What a kind requires is what it *has*; what it is *at* (`murabito_placement`'s
//! `VoxelPosition`) is given
//! when it is spawned, beside the kind. That line also decides what survives
//! reclassifying a thing later with `remove_with_requires`, which takes the whole chain.
//!
//! `Sentient` is where the members sit today: whatever takes いる faces somewhere, moves
//! (`Locomotion`, at a placeholder pace a kind overrides) and takes orders; yokai move
//! as much as beasts do. The tiers below it add nothing yet.
//!
//! A kind that is drawn names its file with `Model`, and `KindsPlugin`'s one observer
//! loads it as the thing is spawned. A plant names its first version in summer; a
//! spawner that wants another version, colour or season gives its own `Model` beside
//! the kind, and what is given at spawn wins.
//!
//! This crate depends on the mechanism crates whose components the tiers require, and
//! nothing but what spawns things depends on it. A cycle in the requirements panics at
//! registration, naming the loop, so the tests spawn every kind.

pub mod all_things;
mod model;

pub use all_things::*;
pub use model::Model;

/// Loads the model of anything spawned with one. The kinds themselves are types and
/// need no plugin; this is the one system the crate has.
pub struct KindsPlugin;

impl bevy::app::Plugin for KindsPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_observer(model::load_model);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;
    use murabito_actions::ActionQueue;
    use murabito_hexcoords::Direction;
    use murabito_movement::Locomotion;
    use murabito_placement::Facing;
    use murabito_progress::Progress;

    fn has<C: Component>(world: &World, entity: Entity) -> bool {
        world.entity(entity).contains::<C>()
    }

    #[test]
    fn a_beast_is_an_animal_is_living_is_sentient_is_tangible_is_a_thing() {
        let mut world = World::new();

        let beast = world.spawn(Beast).id();

        assert!(has::<Animal>(&world, beast));
        assert!(has::<Living>(&world, beast));
        assert!(has::<Sentient>(&world, beast));
        assert!(has::<Tangible>(&world, beast));
        assert!(has::<AllThings>(&world, beast));
    }

    #[test]
    fn a_beast_is_not_a_bird_nor_a_plant_nor_intangible() {
        let mut world = World::new();

        let beast = world.spawn(Beast).id();

        assert!(!has::<Bird>(&world, beast));
        assert!(!has::<NonSentient>(&world, beast));
        assert!(!has::<Plant>(&world, beast));
        assert!(!has::<Intangible>(&world, beast));
    }

    #[test]
    fn a_sentient_thing_moves_faces_somewhere_and_takes_orders() {
        let mut world = World::new();

        let beast = world.spawn(Beast).id();

        assert!(has::<Locomotion>(&world, beast));
        assert!(has::<Progress>(&world, beast), "Locomotion brings its bar");
        assert!(has::<Facing>(&world, beast));
        assert!(has::<ActionQueue>(&world, beast));
    }

    #[test]
    fn a_tree_has_a_place_but_neither_walks_nor_takes_orders() {
        let mut world = World::new();

        let tree = world.spawn(Tree).id();

        assert!(has::<Plant>(&world, tree));
        assert!(has::<NonSentient>(&world, tree));
        assert!(has::<Tangible>(&world, tree));
        assert!(!has::<Sentient>(&world, tree));
        assert!(!has::<Locomotion>(&world, tree));
        assert!(!has::<ActionQueue>(&world, tree));
    }

    #[test]
    fn a_kami_is_sentient_but_not_living_and_moves_all_the_same() {
        let mut world = World::new();

        let kami = world.spawn(Kami).id();

        assert!(has::<Spiritual>(&world, kami));
        assert!(has::<Sentient>(&world, kami));
        assert!(!has::<Living>(&world, kami));
        assert!(has::<ActionQueue>(&world, kami));
        assert!(has::<Locomotion>(&world, kami));
    }

    #[test]
    fn what_is_given_at_spawn_is_kept_over_the_tier_default() {
        let mut world = World::new();

        let beast = world.spawn((Beast, Facing(Direction::S))).id();

        assert_eq!(
            world.entity(beast).get::<Facing>(),
            Some(&Facing(Direction::S))
        );
    }

    /// A species is a kind under a tier with its own numbers in its own `require`. This
    /// one exists only to pin the rule the tree relies on: a direct `require` wins over
    /// an inherited one.
    #[derive(Component, Default)]
    #[require(Beast, Locomotion = Locomotion { speed: 4.0, turn_speed: 180.0 })]
    struct SomeSpecies;

    #[test]
    fn a_species_own_numbers_win_over_its_tiers() {
        let mut world = World::new();

        let one = world.spawn(SomeSpecies).id();

        let locomotion = world.entity(one).get::<Locomotion>().copied();
        assert_eq!(
            locomotion,
            Some(Locomotion {
                speed: 4.0,
                turn_speed: 180.0
            })
        );
        assert!(has::<Beast>(&world, one));
    }

    #[test]
    fn a_fox_is_a_beast_with_its_own_pace() {
        let mut world = World::new();

        let fox = world.spawn(Fox).id();

        assert!(has::<Beast>(&world, fox));
        assert_eq!(
            world.entity(fox).get::<Locomotion>(),
            Some(&Locomotion {
                speed: 4.0,
                turn_speed: 180.0
            })
        );
    }

    #[test]
    fn a_fox_is_drawn_from_its_model_once_spawned() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default(), KindsPlugin))
            .init_asset::<WorldAsset>();

        let fox = app.world_mut().spawn(Fox).id();
        app.update();

        assert!(has::<WorldAssetRoot>(app.world(), fox));
    }

    /// Every kind there is, spawned once. A loop in the requirements panics on the first
    /// spawn of a kind in it, naming the loop; spawning the leaves registers every tier
    /// above them, and the leafless tiers are spawned as themselves.
    fn spawn_every_kind(world: &mut World) {
        world.spawn(Fox);
        world.spawn(Wolf);
        world.spawn(Hare);
        world.spawn(Boar);
        world.spawn(Deer);
        world.spawn(Bear);
        world.spawn(Macaque);
        world.spawn(Cat);
        world.spawn(Rat);
        world.spawn(Dog);
        world.spawn(Tanuki);
        world.spawn(Horse);
        world.spawn(Ox);
        world.spawn(Crane);
        world.spawn(Heron);
        world.spawn(Chicken);
        world.spawn(Pheasant);
        world.spawn(Critter);
        world.spawn(Fish);
        world.spawn(Human);
        world.spawn(Kami);
        world.spawn(Yokai);
        world.spawn(Akuma);
        world.spawn(Rei);
        world.spawn(Maple);
        world.spawn(Sakura);
        world.spawn(Hinoki);
        world.spawn(Redpine);
        world.spawn(Sugi);
        world.spawn(Madake);
        world.spawn(Sasa);
        world.spawn(Azalea);
        world.spawn(Aoki);
        world.spawn(Kusa);
        world.spawn(Kuzu);
        world.spawn(Shida);
        world.spawn(Tool);
        world.spawn(Furniture);
        world.spawn(Rock);
        world.spawn(Intangible);
    }

    #[test]
    fn every_kind_spawns_and_is_a_thing() {
        let mut world = World::new();

        spawn_every_kind(&mut world);

        let things = world
            .query_filtered::<(), With<AllThings>>()
            .iter(&world)
            .count();
        assert_eq!(things, 40, "one entity per spawn, each of them a thing");
    }

    #[test]
    fn every_animal_is_one() {
        let mut world = World::new();
        spawn_every_kind(&mut world);

        let animals = world
            .query_filtered::<(), With<Animal>>()
            .iter(&world)
            .count();
        let beasts = world
            .query_filtered::<(), With<Beast>>()
            .iter(&world)
            .count();
        let birds = world
            .query_filtered::<(), With<Bird>>()
            .iter(&world)
            .count();

        assert_eq!((beasts, birds), (13, 4));
        assert_eq!(animals, 13 + 4 + 2, "plus a bare critter and a bare fish");
    }

    fn how_many<Tier: Component>(world: &mut World) -> usize {
        world.query_filtered::<(), With<Tier>>().iter(world).count()
    }

    #[test]
    fn every_plant_is_one_and_none_of_them_walks() {
        let mut world = World::new();
        spawn_every_kind(&mut world);

        let tiers = (
            how_many::<Tree>(&mut world),
            how_many::<Bamboo>(&mut world),
            how_many::<Shrub>(&mut world),
            how_many::<Undergrowth>(&mut world),
        );
        let walking_plants = world
            .query_filtered::<(), (With<Plant>, With<Locomotion>)>()
            .iter(&world)
            .count();

        assert_eq!(tiers, (5, 2, 2, 3));
        assert_eq!(walking_plants, 0);
    }

    #[test]
    fn what_spawns_a_plant_may_say_which_model() {
        let mut world = World::new();

        let bare = Model("entity_models/flora/trees/sakura-02-a-winter.glb");
        let sakura = world.spawn((Sakura, bare)).id();

        assert_eq!(world.entity(sakura).get::<Model>(), Some(&bare));
    }

    /// The models are made by the art pipeline, not here. A rename there would
    /// otherwise show up only as a thing quietly missing from the screen.
    #[test]
    fn every_model_a_kind_names_is_on_disk() {
        let mut world = World::new();
        spawn_every_kind(&mut world);

        let assets = bevy::asset::io::file::FileAssetReader::get_base_path().join("assets");
        let mut named = 0;
        for model in world.query::<&Model>().iter(&world) {
            let file = assets.join(model.0);
            assert!(file.is_file(), "no model at {}", file.display());
            named += 1;
        }
        assert_eq!(named, 29, "every animal and every plant names its model");
    }
}

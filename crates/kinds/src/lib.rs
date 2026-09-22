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
//! What a kind requires is what it *has*; what it is *at* (its `VoxelPosition`) is given
//! when it is spawned, beside the kind. That line also decides what survives
//! reclassifying a thing later with `remove_with_requires`, which takes the whole chain.
//!
//! `Sentient` is where the members sit today: whatever takes いる faces somewhere, moves
//! (`Locomotion`, at a placeholder pace a kind overrides) and takes orders; yokai move
//! as much as beasts do. The tiers below it add nothing yet.
//!
//! This crate depends on the mechanism crates whose components the tiers require, and
//! nothing but what spawns things depends on it. A cycle in the requirements panics at
//! registration, naming the loop, so the tests spawn every kind.

pub mod all_things;

pub use all_things::*;

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;
    use murabito_actions::ActionQueue;
    use murabito_hexcoords::Direction;
    use murabito_movement::{Facing, Locomotion};
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

    /// Requirements are registered the first time a kind is spawned, and a loop in them
    /// panics there, naming the loop. Spawning the deepest tier of every branch registers
    /// every tier above it.
    #[test]
    fn every_tier_spawns() {
        let mut world = World::new();

        world.spawn(Beast);
        world.spawn(Bird);
        world.spawn(Critter);
        world.spawn(Fish);
        world.spawn(Human);
        world.spawn(Kami);
        world.spawn(Yokai);
        world.spawn(Akuma);
        world.spawn(Rei);
        world.spawn(Tree);
        world.spawn(Bamboo);
        world.spawn(Shrub);
        world.spawn(Undergrowth);
        world.spawn(Tool);
        world.spawn(Furniture);
        world.spawn(Rock);
        world.spawn(Intangible);

        let things = world
            .query_filtered::<(), With<AllThings>>()
            .iter(&world)
            .count();
        assert_eq!(things, 17);
    }
}

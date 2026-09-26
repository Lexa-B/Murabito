//! What a thing *is*: the tree of kinds.
//!
//! Anything in the world that is informative or interactive is a thing, and every thing
//! is of a kind. The kinds form one tree, and a kind inherits everything its parents
//! have, the way classes do: a fox is a beast is an animal is a living thing. Each kind
//! is a unit component, and `#[require(...)]` is both *extends* and the members: a
//! kind's parent comes first in its list, then what every thing of that kind has.
//! Spawning a kind inserts the whole chain, so the node itself says what it has, and
//! the compiler holds it to that. Every node also names itself: `Kind =
//! Kind::at(module_path!())` in its `require`, so a spawned thing carries its place in
//! the tree as a path, and a test walks the whole tree to hold every node to it.
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
//! What a kind requires is what it *has*; where it *is* and which way it faces
//! (`murabito_placement`'s `VoxelPosition` and `Facing`) is given when it is spawned,
//! beside the kind. That line also decides what survives reclassifying a thing later
//! with `remove_with_requires`, which takes the whole chain.
//!
//! `Sentient` is where the members sit today: whatever takes いる moves (`Locomotion`,
//! at a placeholder pace a kind overrides), looks where it faces (`Vision`, a
//! placeholder cone a kind overrides, or `Vision::BLIND`) and takes orders; yokai move
//! and see as much as beasts do. The tiers below it add nothing yet.
//!
//! Every thing gets a `ThingId` as it is spawned, stamped by an observer on `AllThings`,
//! the root, unless it was spawned with one. Every tangible thing must be spawned with a
//! `VoxelPosition` and a `Facing`: the tree can't require either, since where a thing
//! stands and which way it faces are the instance's and not the kind's, so an observer
//! on `Tangible` checks and panics if one is missing. A kind that is drawn names its
//! file with `Model`, and a third observer loads it as the thing is spawned. A plant
//! names its first version in summer; a spawner that wants another version, colour or
//! season gives its own `Model` beside
//! the kind, and what is given at spawn wins.
//!
//! This crate depends on the mechanism crates whose components the tiers require, and
//! nothing but what spawns things depends on it. A cycle in the requirements panics at
//! registration, naming the loop, so the tests spawn every kind.

pub mod all_things;
mod model;
mod ontology;
mod placed;
mod roster;
mod thing_id;

pub use all_things::*;
pub use model::Model;
pub use ontology::{KindNode, Ontology};

/// Stamps an id on every thing, checks every tangible thing was given a place and a
/// facing, loads the model of anything spawned with one, and puts the [`Ontology`], the
/// tree as the types make it, in the world. The kinds themselves are types and need no
/// plugin; three observers and one resource are all the crate runs. The ids come from
/// `murabito_identity`'s counter, so that plugin is added here if the app hasn't already.
pub struct KindsPlugin;

impl bevy::app::Plugin for KindsPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        if !app.is_plugin_added::<murabito_identity::IdentityPlugin>() {
            app.add_plugins(murabito_identity::IdentityPlugin);
        }
        app.add_observer(thing_id::stamp_id)
            .add_observer(placed::check_placed)
            .add_observer(model::load_model);
        let roster = roster::roster();
        app.insert_resource(Ontology::build(&roster));
        #[cfg(feature = "debug")]
        register_kinds(app, &roster);
    }
}

/// Every node of the tree and `Model`, on the debug wire, so that listing a thing's
/// components says what it is: its whole chain, by each node's full module path. The
/// nodes come from the roster, so a node on the tree is a node on the wire.
#[cfg(feature = "debug")]
fn register_kinds(app: &mut bevy::app::App, roster: &roster::Entry) {
    for node in roster.walk() {
        (node.entry.register_type)(app);
    }
    app.register_type::<Model>();
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every node in the tree and `Model` are on the wire: as many registrations as
    /// there are `Component` derives under `src/`.
    #[cfg(feature = "debug")]
    #[test]
    fn in_the_debug_build_every_kind_is_on_the_wire_by_its_place_in_the_tree() {
        use bevy::ecs::reflect::ReflectComponent;

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default(), KindsPlugin))
            .init_asset::<WorldAsset>();
        let registry = app.world().resource::<AppTypeRegistry>().read();

        let fox = registry
            .get_with_type_path(
                "murabito_kinds::all_things::tangible::sentient::living::animal::beast::fox::Fox",
            )
            .expect("the fox, by its place in the tree");
        assert!(fox.data::<ReflectComponent>().is_some());

        let ours = registry
            .iter()
            .filter(|registration| {
                registration
                    .type_info()
                    .type_path()
                    .starts_with("murabito_kinds::")
            })
            .filter(|registration| registration.data::<ReflectComponent>().is_some())
            .count();
        assert_eq!(ours, 56, "55 nodes and Model");
    }
    use std::any::type_name;

    use bevy::prelude::*;
    use murabito_actions::ActionQueue;
    use murabito_brainstem::Brainstem;
    use murabito_hexcoords::Direction;
    use murabito_hexcoords::VoxelCoord;
    use murabito_identity::Kind;
    use murabito_identity::{IdentityPlugin, NextThingId, ThingId};
    use murabito_movement::Locomotion;
    use murabito_placement::{Facing, VoxelPosition};
    use murabito_progress::Progress;
    use murabito_reflexes::{Reflex, Reflexes};
    use murabito_vision::{Seen, Vision};

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
    fn a_fox_startles_and_a_bare_beast_has_no_reflexes_and_a_given_repertoire_wins() {
        let mut world = World::new();
        let fox = world.spawn(Fox).id();
        let beast = world.spawn(Beast).id();
        let bob = world
            .spawn((
                Fox,
                Reflexes::new([Reflex::StartleFaceApparition.at(10)])
                    .tuned(Reflex::StartleFaceApparition, 200),
            ))
            .id();

        let repertoire = |entity| {
            world
                .get::<Reflexes>(entity)
                .expect("every sentient has one")
        };
        assert_eq!(
            repertoire(fox)
                .iter()
                .map(|wired| wired.reflex)
                .collect::<Vec<_>>(),
            [Reflex::StartleFaceApparition]
        );
        assert!(repertoire(beast).is_empty(), "a tier names none");
        assert_eq!(
            repertoire(bob).iter().next().unwrap().priority,
            200,
            "Bob is jumpy"
        );
    }

    #[test]
    fn a_sentient_thing_moves_and_takes_orders() {
        let mut world = World::new();

        let beast = world.spawn(Beast).id();

        assert!(has::<Locomotion>(&world, beast));
        assert!(has::<Progress>(&world, beast), "Locomotion brings its bar");
        assert!(has::<ActionQueue>(&world, beast));
        assert!(
            has::<Brainstem>(&world, beast),
            "and a driver for the queue"
        );
    }

    #[test]
    fn a_sentient_thing_sees_and_has_somewhere_to_put_what_it_saw() {
        let mut world = World::new();

        let beast = world.spawn(Beast).id();

        assert!(has::<Vision>(&world, beast));
        assert!(has::<Seen>(&world, beast), "Vision brings its list");
        assert!(
            !world
                .entity(beast)
                .get::<Vision>()
                .expect("eyes")
                .is_blind()
        );
    }

    #[test]
    fn a_tree_has_a_place_but_neither_walks_nor_takes_orders_nor_sees() {
        let mut world = World::new();

        let tree = world.spawn(Tree).id();

        assert!(has::<Plant>(&world, tree));
        assert!(has::<NonSentient>(&world, tree));
        assert!(has::<Tangible>(&world, tree));
        assert!(!has::<Sentient>(&world, tree));
        assert!(!has::<Locomotion>(&world, tree));
        assert!(!has::<ActionQueue>(&world, tree));
        assert!(!has::<Brainstem>(&world, tree));
        assert!(!has::<Vision>(&world, tree));
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

        let beast = world
            .spawn((
                Beast,
                Locomotion {
                    speed: 1.0,
                    turn_speed: 10.0,
                },
            ))
            .id();

        assert_eq!(
            world.entity(beast).get::<Locomotion>(),
            Some(&Locomotion {
                speed: 1.0,
                turn_speed: 10.0
            })
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

    /// A kind with no eyes at all requires `Vision::BLIND`, the way some yokai will.
    #[derive(Component, Default)]
    #[require(Yokai, Vision = Vision::BLIND)]
    struct SomeBlindYokai;

    #[test]
    fn a_species_can_be_blind_and_still_has_its_empty_list() {
        let mut world = World::new();

        let one = world.spawn(SomeBlindYokai).id();

        assert!(world.entity(one).get::<Vision>().expect("eyes").is_blind());
        assert!(has::<Seen>(&world, one));
    }

    #[test]
    fn a_fox_and_a_hare_have_their_own_eyes_a_hunter_narrow_and_far_prey_wide_and_near() {
        let mut world = World::new();

        let fox = world.spawn(Fox).id();
        let hare = world.spawn(Hare).id();

        let fox_eyes = world.entity(fox).get::<Vision>().expect("eyes").clone();
        let hare_eyes = world.entity(hare).get::<Vision>().expect("eyes").clone();
        assert!(fox_eyes.arc < hare_eyes.arc);
        assert!(fox_eyes.far_range() > hare_eyes.far_range());
        assert_eq!((fox_eyes.arc, fox_eyes.far_range()), (120.0, 48));
        assert_eq!((hare_eyes.arc, hare_eyes.far_range()), (240.0, 26));
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

    /// A headless app with the kinds' observers running. Assets, because a spawned
    /// kind with a `Model` asks for its file.
    fn app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default(), KindsPlugin))
            .init_asset::<WorldAsset>();
        app
    }

    /// Somewhere to put a thing, facing somewhere: the tests here are not about where.
    fn here() -> (VoxelPosition, Facing) {
        (
            VoxelPosition(VoxelCoord::new(0, 0, 0, 0).expect("the origin")),
            Facing(Direction::E),
        )
    }

    fn id_of(app: &App, entity: Entity) -> Option<ThingId> {
        app.world().entity(entity).get::<ThingId>().copied()
    }

    #[test]
    fn a_fox_is_drawn_from_its_model_once_spawned() {
        let mut app = app();
        let fox = app.world_mut().spawn((Fox, here())).id();
        app.update();

        assert!(has::<WorldAssetRoot>(app.world(), fox));
    }

    #[test]
    fn things_are_numbered_from_one_in_the_order_they_are_spawned() {
        let mut app = app();
        let fox = app.world_mut().spawn((Fox, here())).id();
        let sugi = app.world_mut().spawn((Sugi, here())).id();
        app.update();

        assert_eq!(id_of(&app, fox).map(ThingId::number), Some(1));
        assert_eq!(id_of(&app, sugi).map(ThingId::number), Some(2));
    }

    #[test]
    fn what_is_not_a_thing_gets_no_number() {
        let mut app = app();
        let light = app.world_mut().spawn(Transform::default()).id();
        app.update();

        assert_eq!(id_of(&app, light), None);
    }

    #[test]
    fn a_number_given_at_spawn_is_kept_and_costs_the_counter_nothing() {
        let mut app = app();
        let given = app.world_mut().resource_mut::<NextThingId>().mint();
        let hare = app.world_mut().spawn((Hare, given, here())).id();
        let fox = app.world_mut().spawn((Fox, here())).id();
        app.update();

        assert_eq!(id_of(&app, hare), Some(given));
        assert_eq!(
            id_of(&app, fox).map(ThingId::number),
            Some(given.number() + 1)
        );
    }

    #[test]
    fn a_despawned_things_number_is_never_given_again() {
        let mut app = app();
        let first = app.world_mut().spawn((Hare, here())).id();
        let gone = id_of(&app, first).expect("stamped at spawn");
        app.world_mut().despawn(first);
        let next = app.world_mut().spawn((Hare, here())).id();

        assert_eq!(
            id_of(&app, next).map(ThingId::number),
            Some(gone.number() + 1)
        );
    }

    #[test]
    fn the_identity_plugin_may_be_added_by_the_app_first() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            IdentityPlugin,
            KindsPlugin,
        ))
        .init_asset::<WorldAsset>();
        let fox = app.world_mut().spawn((Fox, here())).id();

        assert_eq!(id_of(&app, fox).map(ThingId::number), Some(1));
    }

    #[test]
    fn a_tangible_thing_spawned_with_a_place_is_fine() {
        let mut app = app();
        let sugi = app.world_mut().spawn((Sugi, here())).id();

        assert!(has::<VoxelPosition>(app.world(), sugi));
    }

    #[test]
    #[should_panic(expected = "spawned with no VoxelPosition")]
    fn a_tangible_thing_spawned_with_no_place_is_a_mistake_said_at_once() {
        let mut app = app();
        app.world_mut().spawn(Sugi);
    }

    /// A tree too: `place` needs a facing to put a model anywhere, so a tree without
    /// one would stand at the world origin whatever its voxel said.
    #[test]
    #[should_panic(expected = "spawned with no Facing")]
    fn a_tangible_thing_spawned_facing_nowhere_is_a_mistake_said_at_once() {
        let mut app = app();
        let (place, _) = here();
        app.world_mut().spawn((Sugi, place));
    }

    #[test]
    fn an_intangible_thing_needs_no_place() {
        let mut app = app();
        let happening = app.world_mut().spawn(Intangible).id();

        assert!(id_of(&app, happening).is_some());
        assert!(!has::<VoxelPosition>(app.world(), happening));
    }

    /// Every node there is, spawned once, from the roster. A loop in the requirements
    /// panics on the first spawn of a node in it, naming the loop.
    fn spawn_every_kind(world: &mut World) {
        for node in roster::roster().walk() {
            (node.entry.spawn)(world);
        }
    }

    #[test]
    fn every_kind_spawns_and_is_a_thing() {
        let mut world = World::new();

        spawn_every_kind(&mut world);

        let things = world
            .query_filtered::<(), With<AllThings>>()
            .iter(&world)
            .count();
        assert_eq!(things, 55, "one entity per node, each of them a thing");
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

        assert_eq!(
            (beasts, birds),
            (13 + 1, 4 + 1),
            "the species, and the tier itself"
        );
        assert_eq!(
            animals,
            14 + 5 + 3,
            "plus a bare animal, a bare critter and a bare fish"
        );
    }

    fn ontology() -> Ontology {
        Ontology::build(&roster::roster())
    }

    #[test]
    fn the_tree_is_fifty_five_nodes_under_one_root_all_things() {
        let tree = ontology();
        assert_eq!(tree.len(), 55);
        let root = tree.root();
        assert_eq!(root.name(), "all_things");
        assert_eq!(
            tree.get(root).unwrap().type_name(),
            type_name::<AllThings>()
        );
        assert_eq!(tree.nodes().filter(|node| node.is_root()).count(), 1);
    }

    /// Where a node's file is, as the type's path minus the type itself:
    /// `…::beast::fox::Fox` is defined in `…::beast::fox`.
    fn file_of(type_name: &str) -> &str {
        type_name.rsplit_once("::").expect("a type has a module").0
    }

    /// Every node is labelled with its own file, and sits in its parent's folder: the
    /// label is the parent's label plus the node's name. The parent comes from the
    /// `require` chain as Bevy resolved it, the label from the file, so a missing line,
    /// a file in the wrong folder or a `require` naming the wrong parent all fail here,
    /// naming the node.
    #[test]
    fn every_node_names_its_own_place_in_the_tree_under_its_parent() {
        let tree = ontology();
        for node in tree.nodes() {
            let kind = node.kind();
            assert_eq!(
                kind.path(),
                file_of(node.type_name()),
                "{}",
                node.type_name()
            );
            match node.parent() {
                None => assert_eq!(kind, tree.root()),
                Some(parent) => assert_eq!(
                    kind.path(),
                    format!("{}::{}", parent.path(), kind.name()),
                    "{kind} should sit in its parent's folder, {parent}"
                ),
            }
        }
    }

    /// The roster is the files: every `.rs` under `src/` that defines a node is on it,
    /// under the node whose folder it sits in, and nothing else is. Rust can't
    /// enumerate types, so this is what keeps the one written tree honest.
    #[test]
    fn the_roster_is_the_tree_the_files_make() {
        use std::collections::BTreeMap;
        use std::path::Path;

        /// Module path to node type name, for every file under `src/` with a node in it.
        fn node_files(dir: &Path, module: &str, found: &mut BTreeMap<String, String>) {
            for entry in std::fs::read_dir(dir).expect("src/ is readable") {
                let path = entry.expect("an entry").path();
                let stem = path.file_stem().unwrap().to_str().unwrap().to_owned();
                let module = format!("{module}::{stem}");
                if path.is_dir() {
                    node_files(&path, &module, found);
                } else if path.extension().is_some_and(|ext| ext == "rs") {
                    let text = std::fs::read_to_string(&path).expect("a source file");
                    let node = text
                        .lines()
                        .find_map(|line| line.strip_prefix("pub struct ")?.strip_suffix(';'));
                    if let Some(node) = node {
                        found.insert(module.clone(), format!("{module}::{node}"));
                    }
                }
            }
        }

        // The tree is `all_things.rs` and the folder beside it; the crate's other files
        // hold the plugin and the observers.
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut by_module = BTreeMap::new();
        node_files(
            &src.join("all_things"),
            "murabito_kinds::all_things",
            &mut by_module,
        );
        by_module.insert(
            "murabito_kinds::all_things".to_owned(),
            "murabito_kinds::all_things::AllThings".to_owned(),
        );
        // A node's parent is the node whose file its folder is named after.
        let mut in_files: Vec<(String, Option<String>)> = by_module
            .iter()
            .map(|(module, node)| {
                let folder = module.rsplit_once("::").unwrap().0;
                (node.clone(), by_module.get(folder).cloned())
            })
            .collect();
        in_files.sort();

        let roster = roster::roster();
        let walked = roster.walk();
        let mut on_roster: Vec<(String, Option<String>)> = walked
            .iter()
            .map(|node| {
                let parent = node.parent.map(|j| walked[j].entry.type_name.to_owned());
                (node.entry.type_name.to_owned(), parent)
            })
            .collect();
        on_roster.sort();

        assert_eq!(
            on_roster, in_files,
            "left: the roster; right: the files under src/"
        );
    }

    #[test]
    fn the_fox_descends_from_beast_animal_living_sentient_tangible_and_all_things() {
        let tree = ontology();
        let fox = tree
            .nodes()
            .find(|node| node.type_name() == type_name::<Fox>())
            .expect("the fox is a node")
            .kind();
        let line: Vec<&str> = tree.ancestors(fox).iter().map(|kind| kind.name()).collect();
        assert_eq!(
            line,
            [
                "beast",
                "animal",
                "living",
                "sentient",
                "tangible",
                "all_things"
            ]
        );
        let beast = tree.parent(fox).unwrap();
        assert!(tree.children(beast).any(|kind| kind == fox));
        assert!(
            tree.children(fox).next().is_none(),
            "a species has no children"
        );
    }

    #[test]
    fn the_tree_renders_one_node_a_line_root_first_children_indented_in_name_order() {
        let drawn = ontology().render();
        let lines: Vec<&str> = drawn.lines().collect();
        assert_eq!(lines.len(), 55);
        assert_eq!(lines[0], "all_things");
        assert_eq!(lines[1], "├─ intangible");
        assert_eq!(lines[2], "└─ tangible");
        assert!(drawn.contains("├─ fox\n"), "{drawn}");
    }

    #[test]
    fn a_spawned_thing_carries_its_leafs_label_not_a_tiers() {
        let mut world = World::new();
        let fox = world.spawn(Fox).id();
        let kind = *world.get::<Kind>(fox).expect("labelled");
        assert_eq!(kind.name(), "fox");
        assert_eq!(
            kind.path(),
            "murabito_kinds::all_things::tangible::sentient::living::animal::beast::fox"
        );
        let tree = ontology();
        assert_eq!(
            tree.get(kind).map(KindNode::type_name),
            Some(type_name::<Fox>())
        );
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

        assert_eq!(
            tiers,
            (5 + 1, 2 + 1, 2 + 1, 3 + 1),
            "the species, and each tier itself"
        );
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

//! The roster: the tree of kinds, written down once, as a tree.
//!
//! Rust can't enumerate types, so the tree's members have to be named somewhere, and this
//! is the one place. It is written nested, so it reads as the tree it is, and that makes
//! it a third statement of the shape beside the `require` chain in each node's file and
//! the folders the files sit in. All three are held to agree: building the [`Ontology`]
//! checks the chain against this, and a test checks this against the files.
//!
//! What is done with a node, spawning it to read its label and its chain, or putting it on
//! the debug wire, is done through its entry, so nothing else ever names the nodes again.
//!
//! [`Ontology`]: crate::Ontology

use std::any::type_name;

use bevy::ecs::component::ComponentId;
use bevy::prelude::*;

use crate::*;

/// What a node is, as a type: a unit component that spawns on its own. In the debug
/// build it can also register itself for the wire.
#[cfg(not(feature = "debug"))]
pub(crate) trait NodeType: Component + Default {}
#[cfg(not(feature = "debug"))]
impl<T: Component + Default> NodeType for T {}
#[cfg(feature = "debug")]
pub(crate) trait NodeType: Component + Default + bevy::reflect::GetTypeRegistration {}
#[cfg(feature = "debug")]
impl<T: Component + Default + bevy::reflect::GetTypeRegistration> NodeType for T {}

/// One node and its children, held as what can be done with the type without naming it
/// again.
pub(crate) struct Entry {
    /// The type's full path, `murabito_kinds::all_things::…::fox::Fox`.
    pub(crate) type_name: &'static str,
    /// Registers the component in a world and gives its id, to ask whether an entity has it.
    pub(crate) register: fn(&mut World) -> ComponentId,
    /// Spawns one, so its label and its chain can be read.
    pub(crate) spawn: fn(&mut World) -> Entity,
    /// Puts the type on the debug wire.
    #[cfg(feature = "debug")]
    pub(crate) register_type: fn(&mut App),
    pub(crate) children: Vec<Entry>,
}

/// An entry as the tree walks it: the node and the index of its parent in the walk.
pub(crate) struct Walked<'a> {
    pub(crate) entry: &'a Entry,
    pub(crate) parent: Option<usize>,
}

impl Entry {
    /// The tree flattened, parents before children, each with its parent's index.
    pub(crate) fn walk(&self) -> Vec<Walked<'_>> {
        let mut out = Vec::new();
        self.walk_into(None, &mut out);
        out
    }

    fn walk_into<'a>(&'a self, parent: Option<usize>, out: &mut Vec<Walked<'a>>) {
        let me = out.len();
        out.push(Walked {
            entry: self,
            parent,
        });
        for child in &self.children {
            child.walk_into(Some(me), out);
        }
    }
}

/// A node with children under it.
fn tier<N: NodeType>(children: Vec<Entry>) -> Entry {
    Entry {
        type_name: type_name::<N>(),
        register: |world| world.register_component::<N>(),
        spawn: |world| world.spawn(N::default()).id(),
        #[cfg(feature = "debug")]
        register_type: |app| {
            app.register_type::<N>();
        },
        children,
    }
}

/// A node with nothing under it.
fn leaf<N: NodeType>() -> Entry {
    tier::<N>(Vec::new())
}

/// The tree, root first, siblings in name order. A new node is a new file under `src/`
/// and its place here; the tests say when the two disagree.
pub(crate) fn roster() -> Entry {
    tier::<AllThings>(vec![
        tier::<Intangible>(vec![]),
        tier::<Tangible>(vec![
            tier::<NonSentient>(vec![
                tier::<Object>(vec![leaf::<Furniture>(), leaf::<Rock>(), leaf::<Tool>()]),
                tier::<Plant>(vec![
                    tier::<Bamboo>(vec![leaf::<Madake>(), leaf::<Sasa>()]),
                    tier::<Shrub>(vec![leaf::<Aoki>(), leaf::<Azalea>()]),
                    tier::<Tree>(vec![
                        leaf::<Hinoki>(),
                        leaf::<Maple>(),
                        leaf::<Redpine>(),
                        leaf::<Sakura>(),
                        leaf::<Sugi>(),
                    ]),
                    tier::<Undergrowth>(vec![leaf::<Kusa>(), leaf::<Kuzu>(), leaf::<Shida>()]),
                ]),
            ]),
            tier::<Sentient>(vec![
                tier::<Living>(vec![
                    tier::<Animal>(vec![
                        tier::<Beast>(vec![
                            leaf::<Bear>(),
                            leaf::<Boar>(),
                            leaf::<Cat>(),
                            leaf::<Deer>(),
                            leaf::<Dog>(),
                            leaf::<Fox>(),
                            leaf::<Hare>(),
                            leaf::<Horse>(),
                            leaf::<Macaque>(),
                            leaf::<Ox>(),
                            leaf::<Rat>(),
                            leaf::<Tanuki>(),
                            leaf::<Wolf>(),
                        ]),
                        tier::<Bird>(vec![
                            leaf::<Chicken>(),
                            leaf::<Crane>(),
                            leaf::<Heron>(),
                            leaf::<Pheasant>(),
                        ]),
                        leaf::<Critter>(),
                        leaf::<Fish>(),
                    ]),
                    leaf::<Human>(),
                ]),
                tier::<Spiritual>(vec![
                    leaf::<Akuma>(),
                    leaf::<Kami>(),
                    leaf::<Rei>(),
                    leaf::<Yokai>(),
                ]),
            ]),
        ]),
    ])
}

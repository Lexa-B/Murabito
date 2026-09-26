//! The tree of kinds as the running world has it.
//!
//! Built from the roster, whose nesting gives each node its parent, and checked against
//! what the types do: each node is spawned once in a scratch world, its [`Kind`] label
//! read, and the deepest other node whose component Bevy put on it, the `require` chain
//! as the engine resolved it, must be the parent the roster names. So the shape here is
//! stated once and proven once, and a test holds the labels, which are what the files
//! say, to it as well. Available as a resource wherever `KindsPlugin` is, for whatever
//! needs to walk the tree: a test of every node, a display, a mind asking what is under
//! what.

use bevy::ecs::component::ComponentId;
use bevy::prelude::*;
use murabito_identity::Kind;

use crate::roster::Entry;

/// One node of the tree: its label, its parent's, and the type behind it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KindNode {
    kind: Kind,
    parent: Option<Kind>,
    type_name: &'static str,
}

impl KindNode {
    /// The node's label, its place in the tree.
    pub fn kind(&self) -> Kind {
        self.kind
    }

    /// The parent's label; the root has none.
    pub fn parent(&self) -> Option<Kind> {
        self.parent
    }

    /// The Rust type's full path, `murabito_kinds::all_things::…::fox::Fox`.
    pub fn type_name(&self) -> &'static str {
        self.type_name
    }

    pub fn is_root(&self) -> bool {
        self.parent.is_none()
    }
}

/// The whole tree, in path order, so siblings sit together in name order.
#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct Ontology {
    nodes: Vec<KindNode>,
}

impl Ontology {
    /// Spawns every node of the roster once and reads what came of it. Panics, naming the
    /// node, on a node without a label, two nodes with one label, or a node whose
    /// `require` puts it under a different parent than the roster does: each is a mistake
    /// in a node's file, and the app should not start with it.
    pub(crate) fn build(roster: &Entry) -> Self {
        let mut world = World::new();
        let walked = roster.walk();
        let ids: Vec<ComponentId> = walked
            .iter()
            .map(|node| (node.entry.register)(&mut world))
            .collect();
        let spawned: Vec<Entity> = walked
            .iter()
            .map(|node| (node.entry.spawn)(&mut world))
            .collect();
        let kinds: Vec<Kind> = walked
            .iter()
            .zip(&spawned)
            .map(|(node, &entity)| {
                *world.get::<Kind>(entity).unwrap_or_else(|| {
                    panic!(
                        "{} has no Kind: is `Kind = Kind::at(module_path!())` in its require?",
                        node.entry.type_name
                    )
                })
            })
            .collect();

        // Which other nodes Bevy put on each one: its ancestors, by the `require` chain.
        // The parent is the deepest of them, the one with the most ancestors of its own.
        let ancestors: Vec<Vec<usize>> = (0..walked.len())
            .map(|i| {
                (0..walked.len())
                    .filter(|&j| j != i && world.get_by_id(spawned[i], ids[j]).is_some())
                    .collect()
            })
            .collect();
        for (i, node) in walked.iter().enumerate() {
            let by_chain = ancestors[i]
                .iter()
                .copied()
                .max_by_key(|&j| ancestors[j].len());
            assert!(
                by_chain == node.parent,
                "{}'s require puts it under {:?}, the roster under {:?}",
                node.entry.type_name,
                by_chain.map(|j| walked[j].entry.type_name),
                node.parent.map(|j| walked[j].entry.type_name)
            );
        }

        let mut nodes: Vec<KindNode> = walked
            .iter()
            .enumerate()
            .map(|(i, node)| KindNode {
                kind: kinds[i],
                parent: node.parent.map(|j| kinds[j]),
                type_name: node.entry.type_name,
            })
            .collect();
        nodes.sort_by_key(|node| node.kind.path());

        for pair in nodes.windows(2) {
            assert!(
                pair[0].kind != pair[1].kind,
                "two nodes are labelled {}: {} and {}",
                pair[0].kind,
                pair[0].type_name,
                pair[1].type_name
            );
        }

        Self { nodes }
    }

    /// Every node, in path order.
    pub fn nodes(&self) -> impl Iterator<Item = &KindNode> {
        self.nodes.iter()
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn get(&self, kind: Kind) -> Option<&KindNode> {
        self.nodes.iter().find(|node| node.kind == kind)
    }

    /// The one node with no parent.
    pub fn root(&self) -> Kind {
        self.nodes
            .iter()
            .find(|node| node.is_root())
            .expect("a built tree has a root")
            .kind
    }

    pub fn parent(&self, kind: Kind) -> Option<Kind> {
        self.get(kind).and_then(|node| node.parent)
    }

    /// The nodes whose parent is that one, in name order.
    pub fn children(&self, kind: Kind) -> impl Iterator<Item = Kind> + '_ {
        self.nodes
            .iter()
            .filter(move |node| node.parent == Some(kind))
            .map(|node| node.kind)
    }

    /// Parent first, then its parent, up to the root.
    pub fn ancestors(&self, kind: Kind) -> Vec<Kind> {
        let mut line = Vec::new();
        let mut current = self.parent(kind);
        while let Some(kind) = current {
            line.push(kind);
            current = self.parent(kind);
        }
        line
    }

    /// The tree drawn with box characters, one node's name per line, the root first and
    /// each node's children indented under it in name order.
    pub fn render(&self) -> String {
        let root = self.root();
        let mut out = format!("{}\n", root.name());
        self.render_children(root, "", &mut out);
        out
    }

    fn render_children(&self, parent: Kind, indent: &str, out: &mut String) {
        let children: Vec<Kind> = self.children(parent).collect();
        for (i, &child) in children.iter().enumerate() {
            let last = i + 1 == children.len();
            let (branch, below) = if last {
                ("└─ ", "   ")
            } else {
                ("├─ ", "│  ")
            };
            out.push_str(&format!("{indent}{branch}{}\n", child.name()));
            self.render_children(child, &format!("{indent}{below}"), out);
        }
    }
}

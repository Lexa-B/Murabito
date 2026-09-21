//! 諸法実相 — the world's own taxonomy, as the engine knows it to be.
//!
//! One shared tree, loaded once. What a thing *is*: 狐 is a 動物 is a 生き物 is a 者.
//! A being's private, partial copy — with impressions attached and most branches
//! missing — is 仮諦 and does not live here.
//!
//! The tree is data rather than Rust types because the whole point is walking it at
//! runtime, and types cannot be walked. Ontology keys are therefore strings, never
//! identifiers, which is also why 物 and 者 sharing a romanisation costs nothing.
//!
//! Three sources, because they are three functions:
//!
//!   assets/ontology/諸法.yaml            the tree, and only the tree
//!   assets/item_properties/属性.yaml     what facet types exist
//!   assets/item_properties/諸法/         one file per node, holding that node's facets
//!
//! The last mirrors the first: every node in the tree has exactly one file at the
//! matching path, and a disagreement is a load error rather than a shrug. Two
//! statements of the same structure are only worth having if they are held to agree.
//!
//! On 未知: it is not a node. A being's traversal stopping at 狼 and "pulling 狼's 未知"
//! is just the traversal stopping at 狼 — the generic impression hangs off the node in
//! that being's 仮諦, where impressions live. The world tree has no impressions to hang,
//! so materialising a 未知 child under all nineteen nodes would add nineteen keys that
//! nothing could ever store anything in.

use std::collections::{BTreeMap, BTreeSet};

use bevy::prelude::*;
use include_dir::{Dir, include_dir};
use serde::Deserialize;

/// Compiled in, like the locale catalogues: a missing file is a build error rather than
/// a startup failure.
const 諸法_YAML: &str = include_str!("../assets/ontology/諸法.yaml");
const 属性_YAML: &str = include_str!("../assets/item_properties/属性.yaml");
/// The per-node property files. `include_str!` cannot take a directory.
static 属性_DIR: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/assets/item_properties/諸法");

/// Reserved. Never a node in the tree; see the module note.
pub const 未知: &str = "未知";

/// Reserved. Under a node this lists that node's facets rather than naming a child.
pub const 属性: &str = "属性";

/// Loads the taxonomy and inserts it as a resource.
pub struct TaxonomyPlugin;

impl Plugin for TaxonomyPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(実相::load());
    }
}

/// One node's place in the tree. Children are sorted rather than in file order: the
/// map is a `BTreeMap`, which buys determinism in tests at the cost of a designer not
/// seeing their own ordering back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub parent: Option<String>,
    pub children: Vec<String>,
    /// 0 at the root. Also the cost of a full traversal to here.
    pub depth: usize,
}

/// The taxonomy: 諸法実相.
#[derive(Resource, Debug, Clone)]
pub struct 実相 {
    root: String,
    nodes: BTreeMap<String, Node>,
    /// Facet group to its declaration. The closed set a node may draw from.
    属性: BTreeMap<String, 軸>,
    /// Node to the facets it declares itself, before inheritance.
    own_facets: BTreeMap<String, BTreeSet<String>>,
}

/// A group of facets.
///
/// Exclusive by default, because a group whose values are not alternatives is not a
/// group but a namespace — and because the default that catches mistakes is better than
/// the one that swallows them. Arity is irrelevant: three or four values are exclusive
/// exactly as two are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct 軸 {
    /// Whether a thing may hold only one of these at a time.
    pub 排他: bool,
    pub 値: Vec<String>,
}

/// Which node of 諸法実相 an entity is an instance of.
///
/// A `String` and not a Rust type, for the same reason the tree is data: `狐` has to be
/// something `descend_while` can be handed at runtime. It is also why there is no `Fox`
/// component — that would be the taxonomy leaking back into the type system.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct 種(pub String);

impl 種 {
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }

    pub fn key(&self) -> &str {
        &self.0
    }
}

impl 実相 {
    /// Parses the compiled-in taxonomy. A tree that fails to load leaves a root and
    /// nothing else — which is a legitimate state in this design rather than a broken
    /// one: every being can then classify everything only as "something, and I know no
    /// more", which is exactly what the floor bid is for.
    pub fn load() -> Self {
        match Self::parse() {
            Ok(tree) => {
                info!("諸法: {} nodes under {}", tree.len(), tree.root());
                tree
            }
            Err(problems) => {
                for problem in &problems {
                    error!("諸法: {problem}");
                }
                Self::rootless()
            }
        }
    }

    /// Everything unknown: the root, and no way down from it.
    fn rootless() -> Self {
        let root = "諸法".to_string();
        let mut nodes = BTreeMap::new();
        nodes.insert(
            root.clone(),
            Node {
                parent: None,
                children: Vec::new(),
                depth: 0,
            },
        );
        Self {
            root,
            nodes,
            属性: BTreeMap::new(),
            own_facets: BTreeMap::new(),
        }
    }

    fn parse() -> Result<Self, Vec<String>> {
        let (root, nodes) = parse_tree(諸法_YAML)?;
        let axes = parse_axes(属性_YAML)?;

        let mut present = BTreeSet::new();
        let mut contents = BTreeMap::new();
        collect_files(&属性_DIR, &mut present, &mut contents);

        let mut problems = match_files(&nodes, &present);
        let mut own_facets = BTreeMap::new();
        for key in nodes.keys() {
            let Some(text) = contents.get(&expected_path(&nodes, key)) else {
                continue; // already reported by match_files
            };
            match serde_yaml_ng::from_str::<Option<PropertyFile>>(text) {
                Ok(file) => {
                    let facets = file.unwrap_or_default().属性;
                    if !facets.is_empty() {
                        own_facets.insert(key.clone(), facets.into_iter().collect());
                    }
                }
                Err(error) => problems.push(format!("{key}: {error}")),
            }
        }

        Self::finish(root, nodes, axes, own_facets, problems)
    }

    /// Everything but the property files, which only exist on disk. Tests hand the
    /// facets in directly rather than fabricating a directory.
    #[cfg(test)]
    fn from_parts(
        tree_yaml: &str,
        axes_yaml: &str,
        properties: &[(&str, &[&str])],
    ) -> Result<Self, Vec<String>> {
        let (root, nodes) = parse_tree(tree_yaml)?;
        let axes = parse_axes(axes_yaml)?;
        let own_facets = properties
            .iter()
            .map(|(node, facets)| {
                (
                    (*node).to_string(),
                    facets.iter().map(|f| (*f).to_string()).collect(),
                )
            })
            .collect();
        Self::finish(root, nodes, axes, own_facets, Vec::new())
    }

    /// The checks that do not care where the facets came from.
    fn finish(
        root: String,
        nodes: BTreeMap<String, Node>,
        axes: BTreeMap<String, 軸>,
        own_facets: BTreeMap<String, BTreeSet<String>>,
        mut problems: Vec<String>,
    ) -> Result<Self, Vec<String>> {
        // A value in two groups would make `group_of` a coin toss, and the groups are
        // meant to be alternatives within one axis.
        let mut seen: BTreeMap<&str, &str> = BTreeMap::new();
        for (group, axis) in &axes {
            for value in &axis.値 {
                if let Some(other) = seen.insert(value, group) {
                    problems.push(format!(
                        "facet {value} is declared in two groups: {other} and {group}"
                    ));
                }
            }
        }
        // A facet nobody declared is almost always a typo, and a silently-ignored one
        // would mean a fox quietly stops being a 捕食者.
        for (node, facets) in &own_facets {
            for facet in facets {
                if !seen.contains_key(facet.as_str()) {
                    problems.push(format!(
                        "{node} carries {facet}, which no 属性 type declares"
                    ));
                }
            }
        }
        // A node holding two values from one exclusive axis has nothing to resolve:
        // it is a contradiction at a single point, not a nearer value overriding a
        // further one, so it is refused rather than arbitrated.
        for (node, facets) in &own_facets {
            let mut claimed: BTreeMap<&str, &str> = BTreeMap::new();
            for facet in facets {
                let Some(group) = axes
                    .iter()
                    .find(|(_, axis)| axis.排他 && axis.値.contains(facet))
                    .map(|(group, _)| group.as_str())
                else {
                    continue;
                };
                if let Some(other) = claimed.insert(group, facet) {
                    problems.push(format!(
                        "{node} carries both {other} and {facet}, which are alternatives on {group}"
                    ));
                }
            }
        }

        if problems.is_empty() {
            Ok(Self {
                root,
                nodes,
                属性: axes,
                own_facets,
            })
        } else {
            Err(problems)
        }
    }

    pub fn root(&self) -> &str {
        &self.root
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn contains(&self, key: &str) -> bool {
        self.nodes.contains_key(key)
    }

    pub fn node(&self, key: &str) -> Option<&Node> {
        self.nodes.get(key)
    }

    pub fn children(&self, key: &str) -> &[String] {
        self.nodes
            .get(key)
            .map_or(&[], |node| node.children.as_slice())
    }

    pub fn parent(&self, key: &str) -> Option<&str> {
        self.nodes.get(key)?.parent.as_deref()
    }

    pub fn depth(&self, key: &str) -> Option<usize> {
        Some(self.nodes.get(key)?.depth)
    }

    /// The full traversal, root first: `狐` gives 諸法 → 物 → 者 → 生き物 → 動物 → 狐.
    pub fn path(&self, key: &str) -> Option<Vec<&str>> {
        let mut path = Vec::new();
        let mut here = self.nodes.get_key_value(key)?.0.as_str();
        loop {
            path.push(here);
            match self.parent(here) {
                Some(parent) => here = self.nodes.get_key_value(parent)?.0.as_str(),
                None => break,
            }
        }
        path.reverse();
        Some(path)
    }

    /// Whether `key` is a kind of `ancestor`. A node is a kind of itself, so
    /// `is_a("者", "者")` holds — the same way a fox is an animal and is a fox.
    pub fn is_a(&self, key: &str, ancestor: &str) -> bool {
        let mut here = key;
        loop {
            if here == ancestor {
                return true;
            }
            match self.parent(here) {
                Some(parent) => here = parent,
                None => return false,
            }
        }
    }

    /// Walks from the root toward `target`, stopping at the deepest node `known` accepts.
    ///
    /// This is the seam the recognition loop plugs into: pass a predicate reading one
    /// being's 仮諦 and you get that being's classification of the thing, which is as
    /// specific as its knowledge allows and no more. A being that knows nothing stops at
    /// the root; one that knows everything reaches the leaf. Nothing has to ask "do I
    /// know this?" — running out of knowledge is just an early return.
    pub fn descend_while(&self, target: &str, known: impl Fn(&str) -> bool) -> Option<&str> {
        let path = self.path(target)?;
        let mut deepest = *path.first()?;
        for step in path.into_iter().skip(1) {
            if !known(step) {
                break;
            }
            deepest = step;
        }
        Some(deepest)
    }

    /// The declared facet groups: the closed set a node may draw from.
    pub fn 属性(&self) -> &BTreeMap<String, 軸> {
        &self.属性
    }

    /// Which group a facet belongs to, if any.
    pub fn group_of(&self, facet: &str) -> Option<&str> {
        self.属性
            .iter()
            .find(|(_, axis)| axis.値.iter().any(|value| value == facet))
            .map(|(group, _)| group.as_str())
    }

    /// Whether a group's values are alternatives.
    pub fn is_exclusive(&self, group: &str) -> bool {
        self.属性.get(group).is_some_and(|axis| axis.排他)
    }

    /// Every facet this node carries, its own and every ancestor's.
    ///
    /// Facets inherit downward because the tree is a taxonomy: whatever is true of
    /// 動物 is true of 狐. They are still a separate axis — two nodes sharing a facet
    /// need share no branch, which is the whole point of 子供.
    pub fn facets_of(&self, key: &str) -> BTreeSet<&str> {
        let mut facets = BTreeSet::new();
        // Exclusive axes already settled by a nearer node. The walk runs from the node
        // upward, so the first value found is the nearest one, and it wins: 狐 overrides
        // 動物 the way an instance will override its 種.
        let mut claimed: BTreeSet<&str> = BTreeSet::new();
        let mut here = Some(key);
        while let Some(node) = here {
            if let Some(own) = self.own_facets.get(node) {
                for facet in own {
                    match self.group_of(facet) {
                        Some(group) if self.is_exclusive(group) => {
                            if claimed.insert(group) {
                                facets.insert(facet.as_str());
                            }
                        }
                        _ => {
                            facets.insert(facet.as_str());
                        }
                    }
                }
            }
            here = self.parent(node);
        }
        facets
    }

    /// The value this node holds on one axis, after resolution. `None` if it holds none.
    pub fn facet_in_group(&self, key: &str, group: &str) -> Option<&str> {
        self.facets_of(key)
            .into_iter()
            .find(|facet| self.group_of(facet) == Some(group))
    }

    pub fn has_facet(&self, key: &str, facet: &str) -> bool {
        self.contains(key) && self.facets_of(key).contains(facet)
    }

    /// Every node carrying `facet`, inherited ones included. Sorted, so it is stable.
    pub fn nodes_with(&self, facet: &str) -> Vec<&str> {
        self.nodes
            .keys()
            .map(String::as_str)
            .filter(|key| self.facets_of(key).contains(facet))
            .collect()
    }
}

/// The tree, flattened. Structure only: a facet appearing here is a mistake, because
/// facets are a different function and live under `assets/item_properties/`.
fn parse_tree(yaml: &str) -> Result<(String, BTreeMap<String, Node>), Vec<String>> {
    let raw: BTreeMap<String, RawEntry> =
        serde_yaml_ng::from_str(yaml).map_err(|error| vec![error.to_string()])?;

    let mut roots = raw.into_iter();
    let (root, entry) = roots
        .next()
        .ok_or_else(|| vec!["the tree is empty; there is nothing to classify".to_string()])?;
    // Two roots would make "walk from the top" ambiguous, and the recognition loop has
    // no way to choose between them.
    if let Some((extra, _)) = roots.next() {
        return Err(vec![format!(
            "the tree has more than one root ({root} and {extra}); it must have exactly one"
        )]);
    }
    let RawEntry::Node(entry) = entry else {
        return Err(vec![format!("{root} is a list; the root must be a node")]);
    };

    let mut nodes = BTreeMap::new();
    let mut problems = Vec::new();
    flatten(&root, entry, None, 0, &mut nodes, &mut problems);
    if problems.is_empty() {
        Ok((root, nodes))
    } else {
        Err(problems)
    }
}

fn parse_axes(yaml: &str) -> Result<BTreeMap<String, 軸>, Vec<String>> {
    let raw: BTreeMap<String, RawAxis> =
        serde_yaml_ng::from_str(yaml).map_err(|error| vec![error.to_string()])?;
    Ok(raw.into_iter().map(|(k, v)| (k, v.into())).collect())
}

fn flatten(
    key: &str,
    raw: RawNode,
    parent: Option<&str>,
    depth: usize,
    nodes: &mut BTreeMap<String, Node>,
    problems: &mut Vec<String>,
) {
    // 未知 is reserved: it is what a stopped traversal means, not somewhere to stop.
    if key == 未知 {
        problems.push(format!(
            "未知 is reserved and cannot be a node (found under {})",
            parent.unwrap_or("the root")
        ));
        return;
    }
    // Nodes are keyed by bare name, so the same name twice would silently drop one
    // branch — and "is 狼 a 者?" would then depend on which one won.
    if let Some(existing) = nodes.get(key) {
        problems.push(format!(
            "{key} appears twice: under {} and under {}",
            existing.parent.as_deref().unwrap_or("the root"),
            parent.unwrap_or("the root")
        ));
        return;
    }

    let mut children = BTreeMap::new();
    for (name, entry) in raw.0.unwrap_or_default() {
        match entry {
            // The separation, enforced. Silently ignoring this would leave someone
            // wondering why their facet had no effect.
            _ if name == 属性 => problems.push(format!(
                "{key}: 属性 does not belong in the tree; it goes in the node's file                  under assets/item_properties/"
            )),
            RawEntry::Node(child) => {
                children.insert(name, child);
            }
            RawEntry::List(_) => {
                problems.push(format!("{key}: {name} is a list, but the tree holds nodes"));
            }
        }
    }

    nodes.insert(
        key.to_string(),
        Node {
            parent: parent.map(str::to_string),
            children: children.keys().cloned().collect(),
            depth,
        },
    );
    for (child, grandchildren) in children {
        flatten(&child, grandchildren, Some(key), depth + 1, nodes, problems);
    }
}

/// Where a node's property file must sit, relative to `assets/item_properties/諸法/`.
///
/// A node with children is a folder holding its own `<name>.yaml`; a leaf is a
/// `<name>.yaml` in its parent's folder. The root's file sits at the top, because that
/// directory *is* the root.
fn expected_path(nodes: &BTreeMap<String, Node>, key: &str) -> String {
    let mut ancestry = Vec::new();
    let mut here = Some(key);
    while let Some(node) = here {
        ancestry.push(node);
        here = nodes.get(node).and_then(|n| n.parent.as_deref());
    }
    ancestry.reverse();

    let is_leaf = nodes.get(key).is_some_and(|n| n.children.is_empty());
    let mut parts = ancestry;
    if is_leaf {
        parts.pop();
    }
    if !parts.is_empty() {
        // The directory *is* the root, so the root never appears in a path inside it.
        parts.remove(0);
    }
    let mut path = parts.join("/");
    if !path.is_empty() {
        path.push('/');
    }
    path.push_str(key);
    path.push_str(".yaml");
    path
}

/// The two trees must agree. A node with no file, or a file with no node, is a load
/// error: the point of stating the structure twice is that they are held to match.
fn match_files(nodes: &BTreeMap<String, Node>, present: &BTreeSet<String>) -> Vec<String> {
    let mut problems = Vec::new();
    let mut expected = BTreeSet::new();
    for key in nodes.keys() {
        let path = expected_path(nodes, key);
        if !present.contains(&path) {
            problems.push(format!("{key} has no property file; expected {path}"));
        }
        expected.insert(path);
    }
    for path in present.difference(&expected) {
        problems.push(format!("{path} has no node in the tree"));
    }
    problems
}

fn collect_files(
    dir: &Dir<'_>,
    present: &mut BTreeSet<String>,
    contents: &mut BTreeMap<String, String>,
) {
    for file in dir.files() {
        let Some(path) = file.path().to_str() else {
            continue;
        };
        present.insert(path.to_string());
        if let Some(text) = file.contents_utf8() {
            contents.insert(path.to_string(), text.to_string());
        }
    }
    for sub in dir.dirs() {
        collect_files(sub, present, contents);
    }
}

/// One node's property file.
#[derive(Deserialize, Default)]
#[serde(default)]
struct PropertyFile {
    属性: Vec<String>,
}

/// A node as written in the tree: a mapping of children, or nothing at all. Both
/// `狐: {}` and a bare `狐:` mean a leaf, because one of them is what somebody will type.
#[derive(Deserialize)]
#[serde(transparent)]
struct RawNode(Option<BTreeMap<String, RawEntry>>);

/// Told apart by shape, so a list where a node belongs is reported rather than guessed.
#[derive(Deserialize)]
#[serde(untagged)]
enum RawEntry {
    /// A list where a node belongs. Only its shape matters: it is always a mistake in
    /// the tree, so the values are never read.
    List(#[allow(dead_code)] Vec<String>),
    Node(RawNode),
}

/// A bare list is an exclusive axis, which is the common case and stays terse. The long
/// form is only needed to opt out.
#[derive(Deserialize)]
#[serde(untagged)]
enum RawAxis {
    Values(Vec<String>),
    Declared {
        #[serde(default = "exclusive_by_default")]
        排他: bool,
        値: Vec<String>,
    },
}

fn exclusive_by_default() -> bool {
    true
}

impl From<RawAxis> for 軸 {
    fn from(raw: RawAxis) -> Self {
        match raw {
            RawAxis::Values(値) => Self { 排他: true, 値 },
            RawAxis::Declared { 排他, 値 } => Self { 排他, 値 },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The real thing: all three sources, as shipped.
    fn tree() -> 実相 {
        実相::parse().expect("the compiled-in ontology should load")
    }

    const 年齢: &str = "年齢: [子供, 大人]\n";
    const 視界: &str = "視界: [不透明, 半透明, 透明]\n";

    // --- the tree -----------------------------------------------------------

    #[test]
    fn the_real_ontology_loads() {
        let tree = tree();
        assert_eq!(tree.root(), "諸法");
        assert_eq!(tree.len(), 19);
    }

    #[test]
    fn a_leaf_knows_its_whole_lineage() {
        assert_eq!(
            tree().path("狐").unwrap(),
            ["諸法", "物", "者", "生き物", "動物", "狐"]
        );
    }

    #[test]
    fn is_a_walks_up_the_tree() {
        let tree = tree();
        assert!(tree.is_a("狐", "動物"));
        assert!(tree.is_a("狐", "者"));
        assert!(tree.is_a("狐", "諸法"));
        // A thing is a kind of itself, the way a fox is a fox.
        assert!(tree.is_a("狐", "狐"));
        assert!(!tree.is_a("狐", "幽"));
        assert!(!tree.is_a("者", "狐"));
    }

    /// The grammar rule, as a test: 木がある, so a tree is never a 者 and will never be
    /// given senses or an intent. 猫がいる, so an animal is.
    #[test]
    fn what_takes_aru_is_not_a_being() {
        let tree = tree();
        assert!(tree.is_a("木", "植物"));
        assert!(tree.is_a("木", "物"));
        assert!(!tree.is_a("木", "者"));
        assert!(tree.is_a("兎", "者"));
    }

    /// 幽 classifies by realm, so neither 神 nor 妖怪 ends up filed under the other.
    #[test]
    fn gods_and_yokai_are_siblings_under_the_unseen() {
        let tree = tree();
        assert_eq!(tree.parent("神"), Some("幽"));
        assert_eq!(tree.parent("妖怪"), Some("幽"));
        assert!(tree.is_a("付喪神", "者"));
    }

    /// 物 and 事 are a contrast pair. If 事 ever ends up beneath 物 the distinction is
    /// gone, and nothing else would notice.
    #[test]
    fn things_and_happenings_are_siblings() {
        let tree = tree();
        assert_eq!(tree.parent("物"), Some("諸法"));
        assert_eq!(tree.parent("事"), Some("諸法"));
        assert!(!tree.is_a("事", "物"));
    }

    // --- what the tree refuses ----------------------------------------------

    /// The separation, enforced. Ignoring a stray 属性 here would leave someone
    /// wondering why the facet they wrote had no effect.
    #[test]
    fn facets_do_not_belong_in_the_tree() {
        let problems = parse_tree("諸法:\n  狐:\n    属性: [捕食者]\n").unwrap_err();
        assert!(
            problems.iter().any(|p| p.contains("does not belong")),
            "{problems:?}"
        );
    }

    /// Nodes are keyed by bare name, so a repeated name would silently drop a branch and
    /// leave `is_a` answering according to whichever one happened to win.
    #[test]
    fn a_name_used_twice_is_rejected() {
        let problems = parse_tree("諸法:\n  物:\n    狼: {}\n  者:\n    狼: {}\n").unwrap_err();
        assert!(
            problems
                .iter()
                .any(|p| p.contains("狼") && p.contains("twice")),
            "{problems:?}"
        );
    }

    #[test]
    fn an_explicit_unknown_node_is_rejected() {
        let problems = parse_tree("諸法:\n  未知: {}\n").unwrap_err();
        assert!(
            problems.iter().any(|p| p.contains("reserved")),
            "{problems:?}"
        );
    }

    /// Two roots would make "walk from the top" ambiguous and the recognition loop has
    /// no way to pick.
    #[test]
    fn more_than_one_root_is_rejected() {
        let problems = parse_tree("諸法: {}\n他: {}\n").unwrap_err();
        assert!(
            problems.iter().any(|p| p.contains("more than one root")),
            "{problems:?}"
        );
    }

    /// Both spellings of a leaf, because somebody will type each of them.
    #[test]
    fn a_bare_leaf_and_an_empty_leaf_mean_the_same_thing() {
        let (_, nodes) = parse_tree("諸法:\n  物:\n  者: {}\n").unwrap();
        assert_eq!(nodes.len(), 3);
        assert!(nodes["物"].children.is_empty());
        assert!(nodes["者"].children.is_empty());
    }

    // --- the two trees must agree -------------------------------------------

    /// A node with children is a folder holding its own file; a leaf sits in its
    /// parent's folder; and the root never appears in a path, because the directory
    /// *is* the root.
    #[test]
    fn a_nodes_file_sits_where_its_lineage_says() {
        let tree = tree();
        assert_eq!(expected_path(&tree.nodes, "諸法"), "諸法.yaml");
        assert_eq!(expected_path(&tree.nodes, "事"), "事.yaml");
        assert_eq!(expected_path(&tree.nodes, "物"), "物/物.yaml");
        assert_eq!(
            expected_path(&tree.nodes, "狐"),
            "物/者/生き物/動物/狐.yaml"
        );
    }

    #[test]
    fn a_node_with_no_file_is_a_load_error() {
        let (_, nodes) = parse_tree("諸法:\n  物: {}\n").unwrap();
        let present = BTreeSet::from(["諸法.yaml".to_string()]);
        let problems = match_files(&nodes, &present);
        assert!(
            problems
                .iter()
                .any(|p| p.contains("物") && p.contains("no property file")),
            "{problems:?}"
        );
    }

    #[test]
    fn a_file_with_no_node_is_a_load_error() {
        let (_, nodes) = parse_tree("諸法:\n  物: {}\n").unwrap();
        let present = BTreeSet::from([
            "諸法.yaml".to_string(),
            "物.yaml".to_string(),
            "麒麟.yaml".to_string(),
        ]);
        let problems = match_files(&nodes, &present);
        assert!(
            problems
                .iter()
                .any(|p| p.contains("麒麟") && p.contains("no node")),
            "{problems:?}"
        );
    }

    #[test]
    fn the_shipped_trees_agree() {
        let tree = tree();
        let mut present = BTreeSet::new();
        let mut contents = BTreeMap::new();
        collect_files(&属性_DIR, &mut present, &mut contents);
        assert_eq!(present.len(), tree.len());
        assert!(match_files(&tree.nodes, &present).is_empty());
    }

    /// A tree that fails to load is not a broken state in this design: every being then
    /// classifies everything as "something, and I know no more", which is the floor bid.
    #[test]
    fn a_tree_that_will_not_load_leaves_a_usable_root() {
        let rootless = 実相::rootless();
        assert_eq!(rootless.root(), "諸法");
        assert_eq!(rootless.len(), 1);
        assert_eq!(rootless.path("諸法").unwrap(), ["諸法"]);
    }

    // --- the seam the recognition loop plugs into ---------------------------

    #[test]
    fn knowing_nothing_stops_at_the_root() {
        assert_eq!(tree().descend_while("狐", |_| false), Some("諸法"));
    }

    #[test]
    fn knowing_everything_reaches_the_leaf() {
        assert_eq!(tree().descend_while("狐", |_| true), Some("狐"));
    }

    /// The case the whole design turns on: a being that knows 者 and 生き物 but has never
    /// met a fox stops at 生き物 and acts on what it feels about living things in
    /// general. Nothing asked it whether it knew what a fox was.
    #[test]
    fn running_out_of_knowledge_is_just_an_early_return() {
        let known = ["物", "者", "生き物"];
        assert_eq!(
            tree().descend_while("狐", |node| known.contains(&node)),
            Some("生き物")
        );
    }

    #[test]
    fn descending_toward_something_outside_the_tree_yields_nothing() {
        assert_eq!(tree().descend_while("麒麟", |_| true), None);
    }

    // --- facets, as shipped --------------------------------------------------

    #[test]
    fn a_node_carries_the_facets_its_file_declares() {
        let tree = tree();
        assert!(tree.has_facet("狐", "捕食者"));
        assert!(tree.has_facet("狼", "捕食者"));
        assert!(tree.has_facet("兎", "被食者"));
        assert!(!tree.has_facet("兎", "捕食者"));
    }

    /// The axes are independent: 狐 and 狼 share 捕食者 without 兎 sharing it, though all
    /// three are siblings under 動物. A facet cuts across the tree rather than along it.
    #[test]
    fn facets_cut_across_the_tree_rather_than_along_it() {
        let tree = tree();
        assert_eq!(tree.parent("狐"), tree.parent("兎"));
        assert_eq!(tree.nodes_with("捕食者"), ["狐", "狼"]);
        assert_eq!(tree.nodes_with("被食者"), ["兎"]);
    }

    #[test]
    fn a_facet_knows_its_group() {
        let tree = tree();
        assert_eq!(tree.group_of("捕食者"), Some("食物連鎖"));
        assert_eq!(tree.group_of("子供"), Some("年齢"));
        assert_eq!(tree.group_of("麒麟"), None);
    }

    #[test]
    fn a_facet_is_never_a_node() {
        let tree = tree();
        assert!(!tree.contains("子供"));
        assert!(!tree.contains("属性"));
    }

    #[test]
    fn an_axis_is_exclusive_unless_it_opts_out() {
        let tree = tree();
        for axis in ["年齢", "食物連鎖", "音", "視界", "匂い"] {
            assert!(tree.is_exclusive(axis), "{axis} should be exclusive");
        }
    }

    /// Arity is irrelevant to exclusivity: 匂い has four values and is exclusive exactly
    /// as 年齢 with two is.
    #[test]
    fn exclusivity_does_not_care_how_many_values_an_axis_has() {
        let tree = tree();
        assert_eq!(tree.属性().get("匂い").unwrap().値.len(), 4);
        assert!(tree.is_exclusive("匂い"));
    }

    /// One value per axis, several axes at once. A tree hides you from sight and damps
    /// your footsteps, and those are separate facts about it.
    #[test]
    fn a_node_holds_one_value_on_each_of_several_axes() {
        let tree = tree();
        assert_eq!(tree.facet_in_group("木", "視界"), Some("不透明"));
        assert_eq!(tree.facet_in_group("木", "音"), Some("吸音"));
        assert_eq!(tree.facet_in_group("木", "匂い"), Some("保臭"));
        assert_eq!(tree.facet_in_group("草", "視界"), Some("半透明"));
        // Grass says nothing about scent, and absence is not a value.
        assert_eq!(tree.facet_in_group("草", "匂い"), None);
    }

    /// The reason the sense axes use 遮音 / 不透明 / 防臭 rather than three copies of a
    /// bare "blocks": a facet may be declared in one group only.
    #[test]
    fn the_sense_axes_share_no_vocabulary() {
        let tree = tree();
        assert_eq!(tree.group_of("吸音"), Some("音"));
        assert_eq!(tree.group_of("半透明"), Some("視界"));
        assert_eq!(tree.group_of("保臭"), Some("匂い"));
    }

    // --- resolution ----------------------------------------------------------

    /// The requirement in its canonical form: walls are opaque, a glass wall is not.
    /// Exclusivity is not enforced *down* the tree — a more precise node states a
    /// different value on the same axis and supersedes the vaguer one.
    #[test]
    fn a_more_precise_node_supersedes_a_vaguer_one() {
        let tree = 実相::from_parts(
            "諸法:\n  壁:\n    硝子壁: {}\n",
            視界,
            &[("壁", &["不透明"]), ("硝子壁", &["透明"])],
        )
        .expect("a child contradicting its parent is legal");
        assert_eq!(tree.facet_in_group("壁", "視界"), Some("不透明"));
        assert_eq!(tree.facet_in_group("硝子壁", "視界"), Some("透明"));
        assert!(!tree.has_facet("硝子壁", "不透明"));
    }

    /// Without axes this yielded *both*, silently: `facets_of` was a set union with no
    /// notion of a group.
    #[test]
    fn on_an_exclusive_axis_the_nearest_value_wins() {
        let tree = 実相::from_parts(
            "諸法:\n  人間:\n    子: {}\n",
            年齢,
            &[("人間", &["大人"]), ("子", &["子供"])],
        )
        .unwrap();
        assert_eq!(
            tree.facets_of("子").into_iter().collect::<Vec<_>>(),
            ["子供"]
        );
        assert_eq!(
            tree.facets_of("人間").into_iter().collect::<Vec<_>>(),
            ["大人"]
        );
    }

    #[test]
    fn facets_inherit_downward() {
        let tree = 実相::from_parts(
            "諸法:\n  動物:\n    兎: {}\n",
            "食物連鎖: [捕食者, 被食者]\n",
            &[("動物", &["被食者"])],
        )
        .unwrap();
        // Declared on the parent, true of the child, and the child declared nothing.
        assert!(tree.has_facet("兎", "被食者"));
        assert!(!tree.has_facet("諸法", "被食者"));
    }

    #[test]
    fn a_non_exclusive_group_keeps_every_value() {
        let tree = 実相::from_parts(
            "諸法:\n  人間:\n    子: {}\n",
            "性格:\n  排他: false\n  値: [勇敢, 慎重]\n",
            &[("人間", &["勇敢"]), ("子", &["慎重"])],
        )
        .unwrap();
        assert!(!tree.is_exclusive("性格"));
        assert_eq!(
            tree.facets_of("子").into_iter().collect::<Vec<_>>(),
            ["勇敢", "慎重"]
        );
    }

    /// Two values from one axis on a *single* node is a contradiction at one point, not
    /// a nearer value overriding a further one. There is nothing to resolve.
    #[test]
    fn one_node_holding_two_values_from_an_axis_is_rejected() {
        let problems =
            実相::from_parts("諸法:\n  人間: {}\n", 年齢, &[("人間", &["子供", "大人"])])
                .unwrap_err();
        assert!(
            problems.iter().any(|p| p.contains("alternatives")),
            "{problems:?}"
        );
    }

    /// An undeclared facet is nearly always a typo, and ignoring it silently would mean
    /// a fox quietly ceasing to be a 捕食者.
    #[test]
    fn a_facet_no_type_declares_is_rejected() {
        let problems = 実相::from_parts(
            "諸法:\n  狐: {}\n",
            "食物連鎖: [捕食者, 被食者]\n",
            &[("狐", &["補食者"])],
        )
        .unwrap_err();
        assert!(
            problems.iter().any(|p| p.contains("補食者")),
            "{problems:?}"
        );
    }

    /// Two groups claiming one value would make `group_of` a coin toss.
    #[test]
    fn a_facet_declared_in_two_groups_is_rejected() {
        let problems = 実相::from_parts(
            "諸法: {}\n",
            "年齢: [子供, 大人]\n世代: [子供, 老人]\n",
            &[],
        )
        .unwrap_err();
        assert!(
            problems.iter().any(|p| p.contains("two groups")),
            "{problems:?}"
        );
    }
}

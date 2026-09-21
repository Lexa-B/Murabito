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
//! On 未知: it is not a node. A being's traversal stopping at 狼 and "pulling 狼's 未知"
//! is just the traversal stopping at 狼 — the generic impression hangs off the node in
//! that being's 仮諦, where impressions live. The world tree has no impressions to hang,
//! so materialising a 未知 child under all nineteen nodes would add nineteen keys that
//! nothing could ever store anything in.

use std::collections::{BTreeMap, BTreeSet};

use bevy::prelude::*;
use serde::Deserialize;

/// Compiled in, like the locale catalogues: a missing file is a build error rather than
/// a startup failure.
const 諸法_YAML: &str = include_str!("../assets/ontology/諸法.yaml");

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
        match Self::parse(諸法_YAML) {
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

    fn parse(yaml: &str) -> Result<Self, Vec<String>> {
        let file: File = serde_yaml_ng::from_str(yaml).map_err(|error| vec![error.to_string()])?;

        let mut roots = file.分類.0.unwrap_or_default().into_iter();
        let (root, raw) = roots
            .next()
            .ok_or_else(|| vec!["分類 is empty; there is nothing to classify".to_string()])?;
        // Two roots would make "walk from the top" ambiguous, and the recognition loop
        // has no way to choose between them.
        if let Some((extra, _)) = roots.next() {
            return Err(vec![format!(
                "分類 has more than one root ({root} and {extra}); it must have exactly one"
            )]);
        }

        let RawEntry::Node(raw) = raw else {
            return Err(vec![format!("{root} is a list; the root must be a node")]);
        };

        let mut nodes = BTreeMap::new();
        let mut own_facets = BTreeMap::new();
        let mut problems = Vec::new();
        flatten(
            &root,
            raw,
            None,
            0,
            &mut nodes,
            &mut own_facets,
            &mut problems,
        );

        // A value in two groups would make `group_of` a coin toss, and the groups are
        // meant to be alternatives within one axis.
        let mut seen: BTreeMap<&str, &str> = BTreeMap::new();
        // Not named 属性: that is a `const` in this module, and `let 属性 = ..` would be
        // a const *pattern* rather than a binding. SCREAMING_CASE normally keeps consts
        // and locals apart by sight; kanji has no case, so the convention cannot.
        let axes: BTreeMap<String, 軸> = file
            .属性
            .into_iter()
            .map(|(group, raw)| (group, raw.into()))
            .collect();
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
                        "{node} carries {facet}, which no 属性 group declares"
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

/// Walks the parsed YAML into a flat map, complaining rather than silently losing data.
fn flatten(
    key: &str,
    raw: RawNode,
    parent: Option<&str>,
    depth: usize,
    nodes: &mut BTreeMap<String, Node>,
    own_facets: &mut BTreeMap<String, BTreeSet<String>>,
    problems: &mut Vec<String>,
) {
    // 未知 is reserved: it is what a stopped traversal means, not somewhere to stop.
    if key == 未知 {
        problems.push(format!(
            "未知 is reserved and cannot be a node (found under {})",
            parent.unwrap_or("諸法")
        ));
        return;
    }
    // Nodes are keyed by bare name, so the same name twice would silently drop one
    // branch — and "is 狼 a 者?" would then depend on which one won.
    if let Some(existing) = nodes.get(key) {
        problems.push(format!(
            "{key} appears twice: under {} and under {}",
            existing.parent.as_deref().unwrap_or("諸法"),
            parent.unwrap_or("諸法")
        ));
        return;
    }

    // Split the node's own facets out from its children before recording either.
    let mut children = BTreeMap::new();
    let mut facets = BTreeSet::new();
    for (name, entry) in raw.0.unwrap_or_default() {
        match (name.as_str() == 属性, entry) {
            (true, RawEntry::Facets(values)) => facets.extend(values),
            (true, RawEntry::Node(_)) => {
                problems.push(format!("{key}: 属性 must be a list of facets, not a node"));
            }
            (false, RawEntry::Node(child)) => {
                children.insert(name, child);
            }
            (false, RawEntry::Facets(_)) => {
                problems.push(format!("{key}: {name} is a list; only 属性 may be one"));
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
    if !facets.is_empty() {
        own_facets.insert(key.to_string(), facets);
    }
    for (child, grandchildren) in children {
        flatten(
            &child,
            grandchildren,
            Some(key),
            depth + 1,
            nodes,
            own_facets,
            problems,
        );
    }
}

#[derive(Deserialize)]
struct File {
    分類: RawNode,
    #[serde(default)]
    属性: BTreeMap<String, RawAxis>,
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

/// A node as written: a mapping of children, or nothing at all. Both `狐: {}` and a bare
/// `狐:` mean a leaf, because one of them is what somebody will type.
#[derive(Deserialize)]
#[serde(transparent)]
struct RawNode(Option<BTreeMap<String, RawEntry>>);

/// Under a node, `属性` holds a list of facets and every other key is a child. They are
/// told apart by shape, and anything of the wrong shape is reported rather than guessed.
#[derive(Deserialize)]
#[serde(untagged)]
enum RawEntry {
    Facets(Vec<String>),
    Node(RawNode),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree() -> 実相 {
        実相::parse(諸法_YAML).expect("the compiled-in taxonomy should parse")
    }

    #[test]
    fn the_real_taxonomy_loads() {
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
        assert!(tree.is_a("狐", "物"));
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
        assert!(tree.is_a("付喪神", "妖怪"));
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

    #[test]
    fn facets_are_loaded_alongside_the_tree() {
        let tree = tree();
        let 年齢 = tree.属性().get("年齢").unwrap();
        assert_eq!(年齢.値, ["子供", "大人"]);
        assert!(年齢.排他, "an axis is exclusive unless it says otherwise");
        // A facet is not a node: that is the whole point of having two axes.
        assert!(!tree.contains("子供"));
    }

    // --- what the loader refuses --------------------------------------------

    /// Nodes are keyed by bare name, so a repeated name would silently drop a branch and
    /// leave `is_a` answering according to whichever one happened to win.
    #[test]
    fn a_name_used_twice_is_rejected() {
        let yaml = "
分類:
  諸法:
    物:
      狼: {}
    者:
      狼: {}
";
        let problems = 実相::parse(yaml).unwrap_err();
        assert!(
            problems
                .iter()
                .any(|p| p.contains("狼") && p.contains("twice")),
            "{problems:?}"
        );
    }

    #[test]
    fn an_explicit_unknown_node_is_rejected() {
        let yaml = "
分類:
  諸法:
    未知: {}
";
        let problems = 実相::parse(yaml).unwrap_err();
        assert!(
            problems.iter().any(|p| p.contains("reserved")),
            "{problems:?}"
        );
    }

    /// Two roots would make "walk from the top" ambiguous and the recognition loop has
    /// no way to pick.
    #[test]
    fn more_than_one_root_is_rejected() {
        let yaml = "
分類:
  諸法: {}
  他: {}
";
        let problems = 実相::parse(yaml).unwrap_err();
        assert!(
            problems.iter().any(|p| p.contains("more than one root")),
            "{problems:?}"
        );
    }

    /// Both spellings of a leaf, because somebody will type each of them.
    #[test]
    fn a_bare_leaf_and_an_empty_leaf_mean_the_same_thing() {
        let yaml = "
分類:
  諸法:
    物:
    者: {}
";
        let tree = 実相::parse(yaml).unwrap();
        assert_eq!(tree.len(), 3);
        assert!(tree.children("物").is_empty());
        assert!(tree.children("者").is_empty());
    }

    /// A taxonomy that fails to load is not a broken state in this design: every being
    /// then classifies everything as "something, and I know no more", which is what the
    /// floor bid is for.
    #[test]
    fn a_tree_that_will_not_load_leaves_a_usable_root() {
        let rootless = 実相::rootless();
        assert_eq!(rootless.root(), "諸法");
        assert_eq!(rootless.len(), 1);
        assert_eq!(rootless.path("諸法").unwrap(), ["諸法"]);
        assert!(rootless.descend_while("諸法", |_| true).is_some());
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
        let tree = tree();
        let known = ["物", "者", "生き物"];
        assert_eq!(
            tree.descend_while("狐", |node| known.contains(&node)),
            Some("生き物")
        );
        // The same being, meeting a person it knows by kind but not by name.
        let knows_people = ["物", "者", "生き物", "人間"];
        assert_eq!(
            tree.descend_while("人間", |node| knows_people.contains(&node)),
            Some("人間")
        );
    }

    #[test]
    fn descending_toward_something_outside_the_tree_yields_nothing() {
        assert_eq!(tree().descend_while("麒麟", |_| true), None);
    }

    // --- 属性: the second axis ----------------------------------------------

    #[test]
    fn a_node_carries_the_facets_it_declares() {
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
    fn facets_inherit_downward() {
        let mut yaml = String::from(
            "
分類:
  諸法:
    動物:
      属性: [被食者]
      兎: {}
属性:
  食物連鎖: [捕食者, 被食者]
",
        );
        yaml.push('\n');
        let tree = 実相::parse(&yaml).unwrap();
        // Declared on the parent, true of the child, and the child declared nothing.
        assert!(tree.has_facet("動物", "被食者"));
        assert!(tree.has_facet("兎", "被食者"));
        assert!(!tree.has_facet("諸法", "被食者"));
    }

    #[test]
    fn a_facet_knows_its_group() {
        let tree = tree();
        assert_eq!(tree.group_of("捕食者"), Some("食物連鎖"));
        assert_eq!(tree.group_of("子供"), Some("年齢"));
        assert_eq!(tree.group_of("麒麟"), None);
    }

    /// An undeclared facet is nearly always a typo, and ignoring it silently would mean
    /// a fox quietly ceasing to be a 捕食者.
    #[test]
    fn a_facet_no_group_declares_is_rejected() {
        let yaml = "
分類:
  諸法:
    狐:
      属性: [補食者]
属性:
  食物連鎖: [捕食者, 被食者]
";
        let problems = 実相::parse(yaml).unwrap_err();
        assert!(
            problems.iter().any(|p| p.contains("補食者")),
            "{problems:?}"
        );
    }

    /// Two groups claiming one value would make `group_of` a coin toss.
    #[test]
    fn a_facet_declared_in_two_groups_is_rejected() {
        let yaml = "
分類:
  諸法: {}
属性:
  年齢: [子供, 大人]
  世代: [子供, 老人]
";
        let problems = 実相::parse(yaml).unwrap_err();
        assert!(
            problems.iter().any(|p| p.contains("two groups")),
            "{problems:?}"
        );
    }

    /// 属性 under a node is metadata, never a child, so it must not appear in the tree.
    #[test]
    fn the_reserved_facet_key_is_not_a_node() {
        let tree = tree();
        assert!(!tree.contains("属性"));
        assert!(!tree.children("狐").contains(&"属性".to_string()));
        assert!(tree.children("狐").is_empty());
    }

    // --- 軸: exclusivity and resolution -------------------------------------

    /// The bug this machinery exists for: without axes, a child declaring 子供 under a
    /// parent declaring 大人 yielded *both*, silently, and nothing complained.
    #[test]
    fn on_an_exclusive_axis_the_nearest_value_wins() {
        let yaml = "
分類:
  諸法:
    人間:
      属性: [大人]
      子:
        属性: [子供]
属性:
  年齢: [子供, 大人]
";
        let tree = 実相::parse(yaml).unwrap();
        assert_eq!(
            tree.facets_of("子").into_iter().collect::<Vec<_>>(),
            ["子供"]
        );
        assert_eq!(
            tree.facets_of("人間").into_iter().collect::<Vec<_>>(),
            ["大人"]
        );
        assert!(
            !tree.has_facet("子", "大人"),
            "the further value should be overridden"
        );
    }

    #[test]
    fn a_non_exclusive_group_keeps_every_value() {
        let yaml = "
分類:
  諸法:
    人間:
      属性: [勇敢]
      子:
        属性: [慎重]
属性:
  性格:
    排他: false
    値: [勇敢, 慎重]
";
        let tree = 実相::parse(yaml).unwrap();
        assert!(!tree.is_exclusive("性格"));
        assert_eq!(
            tree.facets_of("子").into_iter().collect::<Vec<_>>(),
            ["勇敢", "慎重"]
        );
    }

    /// Two values from one axis on a *single* node is a contradiction at one point, not
    /// a nearer value overriding a further one. There is nothing to resolve, so it is
    /// refused rather than arbitrated.
    #[test]
    fn one_node_holding_two_values_from_an_axis_is_rejected() {
        let yaml = "
分類:
  諸法:
    人間:
      属性: [子供, 大人]
属性:
  年齢: [子供, 大人]
";
        let problems = 実相::parse(yaml).unwrap_err();
        assert!(
            problems.iter().any(|p| p.contains("alternatives")),
            "{problems:?}"
        );
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
    /// bare "blocks": a facet may be declared only once across all groups.
    #[test]
    fn the_sense_axes_share_no_vocabulary() {
        let tree = tree();
        assert_eq!(tree.group_of("吸音"), Some("音"));
        assert_eq!(tree.group_of("半透明"), Some("視界"));
        assert_eq!(tree.group_of("保臭"), Some("匂い"));
    }

    #[test]
    fn a_list_where_a_node_belongs_is_reported() {
        let yaml = "
分類:
  諸法:
    狐: [捕食者]
属性: {}
";
        let problems = 実相::parse(yaml).unwrap_err();
        assert!(
            problems.iter().any(|p| p.contains("only 属性")),
            "{problems:?}"
        );
    }
}

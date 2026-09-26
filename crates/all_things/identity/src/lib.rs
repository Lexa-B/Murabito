//! Which thing this is: a number every thing in the world carries for its whole life,
//! and that a save keeps.
//!
//! Bevy's own `Entity` is a handle, unique within one run: a despawned thing's index is
//! reused with a new generation, and a save and a load hand out new ones. [`ThingId`] is
//! ours: a serial number from one counter, [`NextThingId`], never reused, and stable
//! across saves as long as the counter is saved with the world. The engine acts on
//! things by `Entity`; whatever remembers things across ticks and saves keys on this.
//!
//! Beside the number, a thing carries its [`Kind`]: the name of its node in the kinds'
//! tree, so that whatever thinks about a thing can ask what it is. This crate owns the
//! name's type and knows no tree; each node names itself in its own file.
//!
//! This crate mints ids and holds names, nothing more. Stamping an id onto every thing
//! is the kinds' business, at the root of their tree, so that lights, cameras and UI
//! never get one.

use std::fmt;

use bevy::prelude::*;

pub struct IdentityPlugin;

impl Plugin for IdentityPlugin {
    fn build(&self, app: &mut App) {
        // `init_resource`, so a counter already there, restored from a save before the
        // plugin was added, is kept.
        app.init_resource::<NextThingId>();
        #[cfg(feature = "debug")]
        app.register_type::<ThingId>()
            .register_type::<NextThingId>()
            .register_type::<Kind>();
    }
}

/// A thing's number, for life. There is no unassigned value: an entity has one or it is
/// not a thing. Minted only by [`NextThingId::mint`]; [`ThingId::restored`] names a number
/// already minted, arriving from a wire or a file. Reads as `#7`.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "debug", derive(Reflect), reflect(Component))]
pub struct ThingId(u64);

impl ThingId {
    /// The number itself, for a display or a file.
    pub fn number(self) -> u64 {
        self.0
    }

    /// A number already minted, coming back from a wire or a file. Mints nothing: a
    /// number nothing has is a name for nothing, and whoever looks it up finds no thing.
    pub fn restored(number: u64) -> Self {
        Self(number)
    }
}

impl fmt::Display for ThingId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}

/// The counter every id comes from: the next number to hand out. World state, not any
/// thing's, and the one piece of this crate a save must keep, or the next spawn after a
/// load would mint a number something already has.
#[derive(Resource, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "debug", derive(Reflect), reflect(Resource))]
pub struct NextThingId(u64);

impl Default for NextThingId {
    /// Numbers start at 1, so that a 0 in a file or a display always means "not set".
    fn default() -> Self {
        Self(1)
    }
}

impl NextThingId {
    /// Hands out the next number and moves on. Never gives the same one twice.
    pub fn mint(&mut self) -> ThingId {
        let id = ThingId(self.0);
        self.0 += 1;
        id
    }
}

/// Which kind of thing this is: its node's place in the kinds' tree, spelled as the path
/// of the module that defines the node, `murabito_kinds::all_things::…::beast::fox`.
/// Every node gives itself one, `Kind = Kind::at(module_path!())` in its own `require`,
/// and a direct requirement wins over an inherited one, so a spawned fox is labelled
/// `…::fox`, never `…::beast`. The path is the whole ancestry: match on the last
/// segment for the leaf, or on a prefix for "some animal", "something sentient".
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "debug", derive(Reflect), reflect(Component))]
pub struct Kind(&'static str);

impl Kind {
    /// The kind at that place in the tree. In a node's file this is
    /// `Kind::at(module_path!())`, so the file's place under `src/` is the label.
    pub const fn at(path: &'static str) -> Self {
        Self(path)
    }

    /// The whole path, root to node.
    pub fn path(self) -> &'static str {
        self.0
    }

    /// The node's own name, the last segment: `fox`.
    pub fn name(self) -> &'static str {
        self.0.rsplit("::").next().unwrap_or(self.0)
    }

    /// Whether that kind is an ancestor of this one. `…::beast::fox` is under
    /// `…::beast` and under `…::animal`; nothing is under itself.
    pub fn is_under(self, ancestor: Kind) -> bool {
        self.0
            .strip_prefix(ancestor.0)
            .is_some_and(|rest| rest.starts_with("::"))
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BEAST: Kind = Kind::at("kinds::animal::beast");
    const FOX: Kind = Kind::at("kinds::animal::beast::fox");

    #[test]
    fn a_kinds_name_is_the_last_segment_of_its_path() {
        assert_eq!(FOX.name(), "fox");
        assert_eq!(FOX.path(), "kinds::animal::beast::fox");
        assert_eq!(Kind::at("kinds").name(), "kinds");
    }

    #[test]
    fn a_kind_is_under_each_of_its_ancestors_and_nothing_else() {
        assert!(FOX.is_under(BEAST));
        assert!(FOX.is_under(Kind::at("kinds::animal")));
        assert!(!FOX.is_under(FOX), "nothing is under itself");
        assert!(!BEAST.is_under(FOX), "a parent is not under its child");
        assert!(
            !FOX.is_under(Kind::at("kinds::animal::bea")),
            "a prefix of a name is not an ancestor"
        );
    }

    #[test]
    fn a_kind_reads_as_its_path() {
        assert_eq!(FOX.to_string(), "kinds::animal::beast::fox");
    }

    #[cfg(feature = "debug")]
    #[test]
    fn in_the_debug_build_the_id_is_a_component_and_the_counter_a_resource_on_the_wire() {
        use bevy::ecs::reflect::{ReflectComponent, ReflectResource};

        let mut app = App::new();
        app.add_plugins(IdentityPlugin);
        let registry = app.world().resource::<AppTypeRegistry>().read();
        let id = registry
            .get_with_type_path("murabito_identity::ThingId")
            .expect("ThingId");
        assert!(id.data::<ReflectComponent>().is_some());
        let counter = registry
            .get_with_type_path("murabito_identity::NextThingId")
            .expect("NextThingId");
        assert!(counter.data::<ReflectResource>().is_some());
        let kind = registry
            .get_with_type_path("murabito_identity::Kind")
            .expect("Kind");
        assert!(kind.data::<ReflectComponent>().is_some());
    }

    #[test]
    fn a_restored_number_is_the_same_id_that_was_minted() {
        let mut counter = NextThingId::default();
        counter.mint();
        let minted = counter.mint();
        assert_eq!(ThingId::restored(minted.number()), minted);
        assert_eq!(ThingId::restored(2).number(), 2);
    }

    #[test]
    fn the_first_id_minted_is_one() {
        let mut counter = NextThingId::default();
        assert_eq!(counter.mint().number(), 1);
    }

    #[test]
    fn ids_come_out_in_order_and_are_never_repeated() {
        let mut counter = NextThingId::default();
        let minted: Vec<u64> = (0..5).map(|_| counter.mint().number()).collect();
        assert_eq!(minted, [1, 2, 3, 4, 5]);
    }

    #[test]
    fn an_id_reads_as_a_hash_and_its_number() {
        let mut counter = NextThingId::default();
        counter.mint();
        assert_eq!(counter.mint().to_string(), "#2");
    }

    #[test]
    fn the_plugin_puts_the_counter_in_the_world() {
        let mut app = App::new();
        app.add_plugins(IdentityPlugin);
        assert_eq!(
            app.world().resource::<NextThingId>(),
            &NextThingId::default()
        );
    }

    #[test]
    fn a_counter_already_in_the_world_is_kept() {
        let mut app = App::new();
        let mut restored = NextThingId::default();
        for _ in 0..41 {
            restored.mint();
        }
        app.insert_resource(restored.clone());
        app.add_plugins(IdentityPlugin);
        assert_eq!(app.world().resource::<NextThingId>(), &restored);
        assert_eq!(
            app.world_mut()
                .resource_mut::<NextThingId>()
                .mint()
                .number(),
            42
        );
    }
}

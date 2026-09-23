//! Which thing this is: a number every thing in the world carries for its whole life,
//! and that a save keeps.
//!
//! Bevy's own `Entity` is a handle, unique within one run: a despawned thing's index is
//! reused with a new generation, and a save and a load hand out new ones. [`ThingId`] is
//! ours: a serial number from one counter, [`NextThingId`], never reused, and stable
//! across saves as long as the counter is saved with the world. The engine acts on
//! things by `Entity`; whatever remembers things across ticks and saves keys on this.
//!
//! This crate mints ids and nothing more. Stamping one onto every thing is the kinds'
//! business, at the root of their tree, so that lights, cameras and UI never get one.

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
            .register_type::<NextThingId>();
    }
}

/// A thing's number, for life. There is no unassigned value: an entity has one or it is
/// not a thing. Made only by [`NextThingId::mint`]. Reads as `#7`.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "debug", derive(Reflect), reflect(Component))]
pub struct ThingId(u64);

impl ThingId {
    /// The number itself, for a display or a file.
    pub fn number(self) -> u64 {
        self.0
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

#[cfg(test)]
mod tests {
    use super::*;

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

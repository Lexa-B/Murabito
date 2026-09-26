//! The one accumulation bar: how far along an entity is with whatever sustained thing it
//! is doing. Stepping, turning, breaking a block, drawing a bow: every mechanism
//! accumulates here, by the rules in `docs/actions_readme.md`, and this crate knows none
//! of them.
//!
//! Units are the mechanism's, not time: shaku for a step, degrees for a turn. Time comes
//! in through the rate a mechanism advances by each tick, so how many ticks an action
//! takes falls out rather than being rounded up front.

use std::any::TypeId;

use bevy::prelude::*;

/// The set every mechanism's tick system belongs to, so the sweep that follows them sees
/// the tick's work done. A mechanism adds its system with `.in_set(MechanismSet)`.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MechanismSet;

pub struct ProgressPlugin;

impl Plugin for ProgressPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, sweep.after(MechanismSet));
    }
}

/// How far along an entity is with the sustained action in flight, if any. One per
/// entity: an entity does one sustained thing at a time.
///
/// `done` accumulates toward `of`, the action's cost, both in the starting mechanism's
/// units. `kind` is the intent that started it, so a leftover carries only into another
/// action of the same kind.
#[derive(Component, Debug, Default, Clone, PartialEq)]
pub struct Progress {
    done: f32,
    of: f32,
    kind: Option<TypeId>,
    /// Whether a mechanism touched this on the current tick. The sweep clears it, and
    /// zeroes `done` when a whole tick passed with nothing touched and nothing in flight.
    touched: bool,
}

impl Progress {
    /// Begins an action of `cost` for the intent `K`. A leftover from an action of the
    /// same kind is kept, so the overshoot of one step starts the next; any other
    /// leftover is dropped.
    pub fn start<K: 'static>(&mut self, cost: f32) {
        let kind = Some(TypeId::of::<K>());
        if self.kind != kind {
            self.done = 0.0;
        }
        self.kind = kind;
        self.of = cost;
        self.touched = true;
    }

    /// Adds a tick's worth of work. `true` once `done` has reached the cost.
    pub fn advance(&mut self, amount: f32) -> bool {
        self.done += amount;
        self.touched = true;
        self.in_flight() && self.done >= self.of
    }

    /// Ends the action in flight, keeping whatever `done` overshot the cost by.
    pub fn finish(&mut self) {
        self.done -= self.of;
        self.of = 0.0;
        self.touched = true;
    }

    /// Whether an action is in flight.
    pub fn in_flight(&self) -> bool {
        self.of > 0.0
    }

    /// Whether the action in flight, or the one just finished, was started by `K`.
    pub fn kind_is<K: 'static>(&self) -> bool {
        self.kind == Some(TypeId::of::<K>())
    }

    /// How far along the action in flight is, from 0 to 1: the bar. 0 when nothing is.
    pub fn fraction(&self) -> f32 {
        if self.in_flight() {
            (self.done / self.of).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }
}

/// After every mechanism has had its tick: an entity that was left alone all tick, with
/// nothing in flight, is at rest, and rest forgets any leftover.
fn sweep(mut all: Query<&mut Progress>) {
    for mut progress in &mut all {
        if !progress.touched && !progress.in_flight() {
            progress.done = 0.0;
            progress.kind = None;
        }
        progress.touched = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Intents, as a mechanism would declare them. Any type will do: only its identity is
    /// used.
    struct Step;
    struct Turn;

    const SHAKU_PER_TICK: f32 = 4.0 / 64.0;

    #[test]
    fn nothing_in_flight_to_begin_with() {
        let progress = Progress::default();

        assert!(!progress.in_flight());
        assert_eq!(progress.fraction(), 0.0);
    }

    #[test]
    fn an_action_is_done_when_the_work_adds_up_to_its_cost() {
        let mut progress = Progress::default();
        progress.start::<Step>(1.0);

        let ticks = (1..).find(|_| progress.advance(SHAKU_PER_TICK));

        assert_eq!(ticks, Some(16));
    }

    #[test]
    fn the_bar_fills_from_nothing_to_full() {
        let mut progress = Progress::default();
        progress.start::<Step>(2.0);

        progress.advance(0.5);

        assert!((progress.fraction() - 0.25).abs() < 1e-6);
    }

    #[test]
    fn the_overshoot_carries_into_the_next_action_of_the_same_kind() {
        let mut progress = Progress::default();
        progress.start::<Step>(1.0);
        progress.advance(1.25);
        progress.finish();

        progress.start::<Step>(1.0);

        assert!((progress.fraction() - 0.25).abs() < 1e-6, "{progress:?}");
    }

    #[test]
    fn the_overshoot_never_carries_into_a_different_kind() {
        let mut progress = Progress::default();
        progress.start::<Step>(1.0);
        progress.advance(1.25);
        progress.finish();

        progress.start::<Turn>(30.0);

        assert_eq!(progress.fraction(), 0.0);
    }

    #[test]
    fn finishing_clears_what_is_in_flight_but_remembers_the_kind() {
        let mut progress = Progress::default();
        progress.start::<Step>(1.0);
        progress.advance(1.0);

        progress.finish();

        assert!(!progress.in_flight());
        assert!(progress.kind_is::<Step>());
        assert!(!progress.kind_is::<Turn>());
    }

    #[test]
    fn advancing_with_nothing_in_flight_is_never_done() {
        let mut progress = Progress::default();

        assert!(!progress.advance(100.0));
    }

    /// A headless app whose every `update` runs `FixedUpdate` exactly once. An app's
    /// first update only starts its clock, with no time elapsed and so no fixed tick, so
    /// that one is spent here.
    fn app_ticking_once_per_update() -> App {
        use bevy::time::TimeUpdateStrategy;

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, ProgressPlugin));
        app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
        app.update();
        app
    }

    #[test]
    fn every_update_of_the_test_app_is_exactly_one_tick() {
        #[derive(Resource, Default)]
        struct Ticks(u32);
        let mut app = app_ticking_once_per_update();
        app.init_resource::<Ticks>();
        app.add_systems(FixedUpdate, |mut ticks: ResMut<Ticks>| ticks.0 += 1);

        app.update();
        app.update();

        assert_eq!(app.world().resource::<Ticks>().0, 2);
    }

    fn progress_of(app: &App, entity: Entity) -> &Progress {
        app.world().get::<Progress>(entity).expect("a progress")
    }

    #[test]
    fn a_leftover_survives_the_tick_it_was_made_on() {
        let mut app = app_ticking_once_per_update();
        let entity = app.world_mut().spawn(Progress::default()).id();
        let mut progress = app
            .world_mut()
            .get_mut::<Progress>(entity)
            .expect("a progress");
        progress.start::<Step>(1.0);
        progress.advance(1.25);
        progress.finish();

        app.update();

        let mut progress = progress_of(&app, entity).clone();
        progress.start::<Step>(1.0);
        assert!((progress.fraction() - 0.25).abs() < 1e-6, "{progress:?}");
    }

    #[test]
    fn a_tick_at_rest_forgets_the_leftover() {
        let mut app = app_ticking_once_per_update();
        let entity = app.world_mut().spawn(Progress::default()).id();
        let mut progress = app
            .world_mut()
            .get_mut::<Progress>(entity)
            .expect("a progress");
        progress.start::<Step>(1.0);
        progress.advance(1.25);
        progress.finish();
        app.update();

        app.update();

        let mut progress = progress_of(&app, entity).clone();
        progress.start::<Step>(1.0);
        assert_eq!(progress.fraction(), 0.0);
        assert!(!progress_of(&app, entity).kind_is::<Step>());
    }

    #[test]
    fn an_action_in_flight_is_not_swept() {
        let mut app = app_ticking_once_per_update();
        let entity = app.world_mut().spawn(Progress::default()).id();
        let mut progress = app
            .world_mut()
            .get_mut::<Progress>(entity)
            .expect("a progress");
        progress.start::<Step>(1.0);
        progress.advance(0.5);

        app.update();
        app.update();

        assert!((progress_of(&app, entity).fraction() - 0.5).abs() < 1e-6);
    }
}

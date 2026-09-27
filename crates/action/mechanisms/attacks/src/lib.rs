//! How a body attacks: the mechanics of a bite, and later whatever else has teeth, claws
//! or a bow. Nothing here decides *whom* to attack; that is the actions layer above,
//! which puts a `Bite` on an entity and this crate carries it out. A stub for now: a bite
//! takes its time on the bar and lands on nothing, since there is nothing yet to bite,
//! no jaws to bite with and no hurt to do. It holds the word's place in the queue and on
//! the wire until those systems exist.

use bevy::prelude::*;
use murabito_progress::{MechanismSet, Progress};

/// How long a bite takes, in seconds, until a body has jaws of its own to say.
const BITE_SECONDS: f32 = 0.25;

pub struct AttacksPlugin;

impl Plugin for AttacksPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, bite.in_set(MechanismSet));
    }
}

/// The intent to bite whatever is in the cell the body faces. Put it on a body with a
/// `Progress` and `bite` carries it out over ticks, then removes it. One at a time: the
/// actions layer never issues another while one is in flight. Bites nothing yet.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Bite;

/// Carries out every `Bite` in flight, a tick at a time on a bar measured in seconds, and
/// lands it once the time is spent: nothing happens but a line in the log, for now.
fn bite(
    time: Res<Time>,
    mut commands: Commands,
    mut bodies: Query<(Entity, &mut Progress), With<Bite>>,
) {
    for (body, mut progress) in &mut bodies {
        if !progress.in_flight() {
            progress.start::<Bite>(BITE_SECONDS);
        }
        if progress.advance(time.delta_secs()) {
            debug!("{body}: bites, at nothing yet");
            progress.finish();
            commands.entity(body).remove::<Bite>();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use murabito_progress::ProgressPlugin;

    /// A headless app whose every `update` is exactly one 64 Hz tick, with one body in
    /// it. An app's first update only starts its clock, so it is spent here.
    fn body() -> (App, Entity) {
        use bevy::time::TimeUpdateStrategy;

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, ProgressPlugin, AttacksPlugin));
        app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
        app.update();
        let body = app.world_mut().spawn(Progress::default()).id();
        (app, body)
    }

    fn is_biting(app: &App, body: Entity) -> bool {
        app.world().get::<Bite>(body).is_some()
    }

    fn in_flight(app: &App, body: Entity) -> bool {
        app.world()
            .get::<Progress>(body)
            .expect("a bar")
            .in_flight()
    }

    #[test]
    fn a_bite_takes_a_quarter_of_a_second_of_ticks_and_is_then_gone() {
        let (mut app, body) = body();
        app.world_mut().entity_mut(body).insert(Bite);

        for tick in 1..16 {
            app.update();
            assert!(is_biting(&app, body), "done early, on tick {tick}");
            assert!(in_flight(&app, body), "idle on tick {tick}");
        }
        app.update();

        assert!(!is_biting(&app, body));
        assert!(!in_flight(&app, body));
    }

    #[test]
    fn the_bar_reads_as_a_bite_while_one_is_in_flight() {
        let (mut app, body) = body();
        app.world_mut().entity_mut(body).insert(Bite);

        app.update();

        let bar = app.world().get::<Progress>(body).expect("a bar");
        assert!(bar.kind_is::<Bite>());
        assert!((bar.fraction() - 1.0 / 16.0).abs() < 1e-6);
    }
}

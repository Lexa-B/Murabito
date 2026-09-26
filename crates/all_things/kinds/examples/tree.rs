//! Prints the tree of kinds as the running world has it, from the roster and each node's
//! `require`: `cargo run -p murabito_kinds --example tree`, or `scripts/tree.sh`.

use bevy::prelude::*;
use murabito_kinds::{KindsPlugin, Ontology};

fn main() {
    let mut app = App::new();
    app.add_plugins(KindsPlugin);
    print!("{}", app.world().resource::<Ontology>().render());
}

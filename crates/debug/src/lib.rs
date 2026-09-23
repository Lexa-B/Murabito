//! The debug build's window into the running world: Bevy's remote protocol, served over
//! HTTP on this machine only, so that something outside the process can read the
//! world's components and resources as they stand at the end of each frame.
//!
//! All of it is behind the `debug` Cargo feature. Without it [`DebugPlugin`] does
//! nothing and the engine's `bevy_remote` feature is never turned on, so the everyday
//! build carries no server. With it, every module that registered its types with the
//! reflection registry is readable by their type path; this crate registers nothing
//! itself and depends on no module. The world is read while paused too: the server
//! runs after `Last`, whether or not a tick happened this frame.
//!
//! The protocol is JSON-RPC over HTTP (`world.query`, `world.get_components`,
//! `world.get_resources`, and their `+watch` forms). A local page will read it later;
//! the response headers already allow that.

use bevy::prelude::*;

/// Where the server listens: Bevy's default, on the loopback address only.
#[cfg(feature = "debug")]
pub const PORT: u16 = bevy::remote::http::DEFAULT_PORT;

pub struct DebugPlugin;

#[cfg(feature = "debug")]
impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        use bevy::remote::{
            RemotePlugin,
            http::{Headers, RemoteHttpPlugin},
        };

        // A page served from a file or another port is another origin to a browser,
        // which asks before reading across origins; these headers say yes.
        let allow_a_page = Headers::new()
            .insert("Access-Control-Allow-Origin", "*")
            .insert("Access-Control-Allow-Headers", "Content-Type");
        app.add_plugins((
            RemotePlugin::default(),
            RemoteHttpPlugin::default()
                .with_port(PORT)
                .with_headers(allow_a_page),
        ));
    }
}

#[cfg(not(feature = "debug"))]
impl Plugin for DebugPlugin {
    fn build(&self, _app: &mut App) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(feature = "debug"))]
    #[test]
    fn without_the_feature_the_plugin_adds_nothing() {
        let mut app = App::new();
        let before = app.world().iter_resources().count();
        app.add_plugins(DebugPlugin);
        assert_eq!(app.world().iter_resources().count(), before);
    }

    #[cfg(feature = "debug")]
    #[test]
    fn with_the_feature_the_server_is_set_up_on_the_loopback_port() {
        let mut app = App::new();
        app.add_plugins(DebugPlugin);
        assert!(
            app.world()
                .contains_resource::<bevy::remote::RemoteMethods>(),
            "the remote methods are registered"
        );
        assert_eq!(PORT, 15702);
    }
}

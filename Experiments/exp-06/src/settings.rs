//! Player-facing preferences.
//!
//! Nothing writes to these yet — a settings page will. Until then every field keeps its
//! `Default`, which is the value tuned by hand while building the feature it belongs to.

use bevy::prelude::*;

/// Registers every settings resource. Added before the plugins that read them.
pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraSettings>();
    }
}

/// Camera preferences. A `Resource`, not a component: there is one set of them for the
/// whole app, independent of how many camera rigs exist.
#[derive(Resource)]
pub struct CameraSettings {
    /// Multiplies the whole pan-speed curve. 1.0 is the tuned default.
    pub pan_speed_scale: f32,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            pan_speed_scale: 1.0,
        }
    }
}

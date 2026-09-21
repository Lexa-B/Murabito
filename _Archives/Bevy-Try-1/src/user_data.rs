//! Where everything belonging to the player lives.
//!
//! One directory — `$XDG_CONFIG_HOME/murabito`, in practice `~/.config/murabito` — holds
//! all of it, and this is the only place that decides where that is. Settings sit there
//! now; saves and anything else the player accumulates go in the same place later, each
//! module joining its own filename onto [`UserData::root`] rather than working out the
//! directory for itself.
//!
//! Having it in one place is also what makes any of it testable: a test points
//! [`UserData::at`] at a temporary directory and the code under test cannot touch the
//! player's real files.

use std::path::{Path, PathBuf};

use bevy::prelude::*;

/// The directory name under the platform's config directory.
const APP_DIR: &str = "murabito";

/// Inserts [`UserData`] so that everything else can find the player's directory.
///
/// Add it before any plugin that reads or writes player data: those read the resource
/// while the app is still being built, not while it runs.
pub struct UserDataPlugin(Option<UserData>);

impl UserDataPlugin {
    /// The real thing: the player's directory under the platform's config directory.
    pub fn from_config_dir() -> Self {
        Self(UserData::from_config_dir())
    }

    /// Everything under `root`. For tests, which must never write to the real one.
    pub fn at(root: impl Into<PathBuf>) -> Self {
        Self(Some(UserData::at(root)))
    }
}

impl Plugin for UserDataPlugin {
    fn build(&self, app: &mut App) {
        match &self.0 {
            Some(user_data) => {
                app.insert_resource(user_data.clone());
            }
            // A platform with nowhere to put player data is not worth refusing to start
            // over. The resource is simply absent, and everything that persists sees
            // that it has nowhere to write — see `Option<Res<UserData>>`.
            None => warn!("no config directory on this platform; nothing will be saved"),
        }
    }
}

/// The player's directory. Absent as a resource when the platform has no config
/// directory, which is the only reason it can be missing.
#[derive(Resource, Clone, Debug, PartialEq, Eq)]
pub struct UserData {
    root: PathBuf,
}

impl UserData {
    /// `$XDG_CONFIG_HOME/murabito`, or `None` where the platform has no config directory.
    pub fn from_config_dir() -> Option<Self> {
        Some(Self {
            root: dirs::config_dir()?.join(APP_DIR),
        })
    }

    /// Everything under `root`, as given. For tests.
    pub fn at(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// The directory itself. Join a filename onto it; it is not guaranteed to exist yet,
    /// and whoever writes a file creates it.
    pub fn root(&self) -> &Path {
        &self.root
    }
}

//! Where everything belonging to the player lives.
//!
//! One directory, `~/.config/murabito` (the platform's config directory plus `murabito`),
//! holds all of it, and this is the only place that decides where that is. Settings sit
//! there first; saves and anything else the player accumulates go in the same place,
//! each module joining its own filename onto [`UserData::root`] rather than working out
//! the directory for itself.
//!
//! Having it in one place is also what makes any of it testable: a test points
//! [`UserDataPlugin::at`] at a temporary directory, and the code under test cannot touch
//! the player's real files.

use std::path::{Path, PathBuf};

use bevy::prelude::*;

/// The directory name under the platform's config directory.
const APP_DIR: &str = "murabito";

/// The player's directory. Absent as a resource only when the platform has no config
/// directory; everything that persists asks for `Option<Res<UserData>>` and does
/// nothing when it is `None`.
#[derive(Resource, Clone, Debug, PartialEq, Eq)]
pub struct UserData {
    root: PathBuf,
}

impl UserData {
    /// `~/.config/murabito`, or `None` where the platform has no config directory.
    pub fn from_config_dir() -> Option<Self> {
        Some(Self {
            root: dirs::config_dir()?.join(APP_DIR),
        })
    }

    /// Everything under `root`, as given. For tests.
    pub fn at(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// The directory itself. Join a filename onto it. It is not guaranteed to exist
    /// yet: whoever writes a file creates it.
    pub fn root(&self) -> &Path {
        &self.root
    }
}

/// Inserts [`UserData`] so that everything else can find the player's directory.
///
/// Add it before any plugin that reads or writes player data: those read the resource
/// while the app is still being built, not once it runs.
pub struct UserDataPlugin(Option<UserData>);

impl UserDataPlugin {
    /// The real thing: the player's directory under the platform's config directory.
    pub fn from_config_dir() -> Self {
        Self(UserData::from_config_dir())
    }

    /// Everything under `root`. For tests, which must never touch the real directory.
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
            // over. The resource is simply absent, and nothing gets saved.
            None => warn!("no config directory on this platform; nothing will be saved"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_directory_given_directly_is_used_as_given() {
        let user_data = UserData::at("/somewhere/particular");

        assert_eq!(user_data.root(), Path::new("/somewhere/particular"));
    }

    #[test]
    fn the_real_directory_is_named_after_the_game_inside_the_config_directory() {
        let Some(user_data) = UserData::from_config_dir() else {
            return; // a platform with no config directory; nothing to check
        };

        assert_eq!(user_data.root().file_name().unwrap(), APP_DIR);
        assert_eq!(user_data.root().parent(), dirs::config_dir().as_deref());
    }

    #[test]
    fn the_plugin_makes_the_directory_a_resource_as_the_app_is_built() {
        let mut app = App::new();

        app.add_plugins(UserDataPlugin::at("/scratch"));

        assert_eq!(
            app.world().resource::<UserData>(),
            &UserData::at("/scratch")
        );
    }

    #[test]
    fn with_no_config_directory_there_is_no_resource() {
        let mut app = App::new();

        app.add_plugins(UserDataPlugin(None));

        assert!(app.world().get_resource::<UserData>().is_none());
    }
}

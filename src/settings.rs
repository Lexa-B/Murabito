//! Player-facing preferences, and the file they persist to.
//!
//! The file is `settings.yaml` in the player's directory (see [`crate::user_data`]),
//! one top-level key per group of settings, so later groups can be added without
//! disturbing what is already there.
//!
//! It is meant to be hand-editable: a missing file means "use the defaults", and a file
//! that fails to parse is reported and ignored rather than fatal. Losing a tweak to a
//! typo is a nuisance; refusing to start over one is worse.

use std::fs;
use std::path::PathBuf;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::i18n::Language;
use crate::user_data::UserData;

/// What the file is called inside the player's directory. Where that directory is
/// belongs to `user_data`, not here.
const CONFIG_FILE: &str = "settings.yaml";

/// Loads settings at startup and writes them back whenever they change.
///
/// Add it after [`crate::user_data::UserDataPlugin`], whose resource it reads here,
/// while the app is still being built.
pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        // No player directory means nowhere to read from or write to: the defaults
        // stand, and nothing is persisted. Rare enough to be worth handling quietly
        // rather than refusing to start.
        let Some(user_data) = app.world().get_resource::<UserData>().cloned() else {
            app.init_resource::<CameraSettings>()
                .init_resource::<Language>();
            return;
        };

        let (file, outcome) = SettingsFile::load(&user_data);

        // Only a genuinely missing file is created here. A file that failed to parse is
        // left exactly as it is: the player can fix the typo and keep the rest of their
        // settings, instead of finding them silently replaced with defaults.
        if outcome == LoadOutcome::Missing {
            file.save(&user_data);
        }

        app.insert_resource(file.camera)
            // The language is a resource in its own right: `i18n` reads it every frame
            // and nothing there needs to know it came from a file.
            .insert_resource(file.ui.language)
            // Runs in `Last` so a change made anywhere this frame is saved once, after
            // everything that might have touched it has run.
            .add_systems(Last, save_on_change);
    }
}

/// What reading the settings file turned up. Drives whether a file gets written back.
#[derive(PartialEq, Debug)]
enum LoadOutcome {
    /// Read and parsed. Nothing to write.
    Loaded,
    /// No file yet: a first run. One gets written so it can be found and edited.
    Missing,
    /// Present but unreadable or invalid. Left untouched so it can be repaired.
    Unusable,
}

/// The whole file. One field per group; each is a resource in its own right.
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
struct SettingsFile {
    camera: CameraSettings,
    ui: UiSettings,
}

/// Settings about the interface itself, as opposed to how the camera behaves.
#[derive(Serialize, Deserialize, Default, Clone, Copy, PartialEq, Debug)]
#[serde(default)]
pub struct UiSettings {
    /// Which language the UI is in. `en` or `ja` in the file.
    pub language: Language,
}

/// Camera preferences. A `Resource`, not a component: there is one set of them for the
/// whole app, independent of how many camera rigs exist.
#[derive(Resource, Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
#[serde(default)]
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

impl SettingsFile {
    /// Reads the file, falling back to defaults for anything missing or unreadable.
    fn load(user_data: &UserData) -> (Self, LoadOutcome) {
        let path = settings_path(user_data);

        match fs::read_to_string(&path) {
            Ok(text) => match serde_yaml_ng::from_str::<Self>(&text) {
                Ok(file) => {
                    info!("loaded settings from {}", path.display());
                    (file, LoadOutcome::Loaded)
                }
                Err(error) => {
                    // Deliberately not fatal, and deliberately not overwritten either:
                    // the file stays as the player left it so the typo can be fixed.
                    warn!(
                        "{} is not valid settings YAML ({error}); using defaults",
                        path.display()
                    );
                    (Self::default(), LoadOutcome::Unusable)
                }
            },
            // Missing is the normal first-run case, not a problem worth reporting.
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                (Self::default(), LoadOutcome::Missing)
            }
            Err(error) => {
                warn!(
                    "could not read {} ({error}); using defaults",
                    path.display()
                );
                (Self::default(), LoadOutcome::Unusable)
            }
        }
    }

    /// Writes the file, creating the directory if this is the first run.
    fn save(&self, user_data: &UserData) {
        let path = settings_path(user_data);
        if let Some(parent) = path.parent()
            && let Err(error) = fs::create_dir_all(parent)
        {
            warn!(
                "could not create {} ({error}); settings not saved",
                parent.display()
            );
            return;
        }

        let text = match serde_yaml_ng::to_string(self) {
            Ok(text) => text,
            Err(error) => {
                warn!("could not serialise settings ({error})");
                return;
            }
        };

        // Write beside the real file and rename over it: a crash mid-write then leaves
        // the old settings intact rather than a half-written file.
        let temporary = path.with_extension("yaml.tmp");
        if let Err(error) = fs::write(&temporary, text) {
            warn!(
                "could not write {} ({error}); settings not saved",
                temporary.display()
            );
            return;
        }
        if let Err(error) = fs::rename(&temporary, &path) {
            warn!(
                "could not replace {} ({error}); settings not saved",
                path.display()
            );
        }
    }
}

/// The full path to the settings file.
fn settings_path(user_data: &UserData) -> PathBuf {
    user_data.root().join(CONFIG_FILE)
}

/// Saves whenever any settings resource changed this frame. The frame one is inserted
/// counts as a change, so those are skipped: writing the file at startup is the plugin's
/// job, and only when there was no file to begin with.
fn save_on_change(camera: Res<CameraSettings>, language: Res<Language>, user_data: Res<UserData>) {
    if camera.is_added() || language.is_added() {
        return;
    }
    if !camera.is_changed() && !language.is_changed() {
        return;
    }
    SettingsFile {
        camera: *camera,
        ui: UiSettings {
            language: *language,
        },
    }
    .save(&user_data);
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// A player directory of our own. Every test gets one: nothing here may read or
    /// write the real `~/.config/murabito`.
    fn scratch() -> (TempDir, UserData) {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let user_data = UserData::at(dir.path());
        (dir, user_data)
    }

    fn write_settings(user_data: &UserData, text: &str) {
        fs::create_dir_all(user_data.root()).expect("the scratch directory");
        fs::write(settings_path(user_data), text).expect("writing the settings file");
    }

    #[test]
    fn a_missing_file_reports_missing_and_yields_defaults() {
        let (_dir, user_data) = scratch();
        let (file, outcome) = SettingsFile::load(&user_data);
        assert_eq!(outcome, LoadOutcome::Missing);
        assert_eq!(file.camera, CameraSettings::default());
    }

    #[test]
    fn a_valid_file_is_read_back() {
        let (_dir, user_data) = scratch();
        write_settings(
            &user_data,
            "camera:\n  pan_speed_scale: 2.5\nui:\n  language: ja\n",
        );
        let (file, outcome) = SettingsFile::load(&user_data);
        assert_eq!(outcome, LoadOutcome::Loaded);
        assert_eq!(file.camera.pan_speed_scale, 2.5);
        assert_eq!(file.ui.language, Language::Japanese);
    }

    /// A group the file doesn't mention falls back to its defaults, which is what lets a
    /// new group of settings be added without invalidating everyone's existing file.
    #[test]
    fn a_partial_file_keeps_the_defaults_for_what_it_omits() {
        let (_dir, user_data) = scratch();
        write_settings(&user_data, "camera:\n  pan_speed_scale: 0.5\n");
        let (file, outcome) = SettingsFile::load(&user_data);
        assert_eq!(outcome, LoadOutcome::Loaded);
        assert_eq!(file.camera.pan_speed_scale, 0.5);
        assert_eq!(file.ui.language, Language::default());
    }

    #[test]
    fn an_unparseable_file_reports_unusable_and_yields_defaults() {
        let (_dir, user_data) = scratch();
        write_settings(&user_data, "camera: [this is not a mapping\n");
        let (file, outcome) = SettingsFile::load(&user_data);
        assert_eq!(outcome, LoadOutcome::Unusable);
        assert_eq!(file.camera, CameraSettings::default());
    }

    #[test]
    fn saving_creates_the_directory_and_round_trips() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        // Deliberately a directory that does not exist yet: the first run has to create
        // the player's directory, not assume it.
        let user_data = UserData::at(dir.path().join("not-yet"));
        let written = SettingsFile {
            camera: CameraSettings {
                pan_speed_scale: 1.75,
            },
            ui: UiSettings {
                language: Language::Japanese,
            },
        };
        written.save(&user_data);

        let (read_back, outcome) = SettingsFile::load(&user_data);
        assert_eq!(outcome, LoadOutcome::Loaded);
        assert_eq!(read_back.camera, written.camera);
        assert_eq!(read_back.ui.language, written.ui.language);
    }

    /// The write goes to a temporary file and is renamed over the real one, so a crash
    /// leaves the old settings rather than half a file. Nothing should be left behind.
    #[test]
    fn saving_leaves_no_temporary_file_behind() {
        let (_dir, user_data) = scratch();
        SettingsFile::default().save(&user_data);
        let leftovers: Vec<_> = fs::read_dir(user_data.root())
            .expect("the scratch directory")
            .filter_map(Result::ok)
            .map(|entry| entry.file_name())
            .filter(|name| name != CONFIG_FILE)
            .collect();
        assert!(leftovers.is_empty(), "left behind {leftovers:?}");
    }
}

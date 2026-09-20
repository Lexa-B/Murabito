//! Player-facing preferences, and the file they persist to.
//!
//! The file is YAML at `$XDG_CONFIG_HOME/murabito/settings.yaml` (in practice
//! `~/.config/murabito/settings.yaml`), one top-level key per group of settings,
//! so later groups can be added without disturbing what is already there.
//!
//! It is meant to be hand-editable: a missing file means "use the defaults", and a file
//! that fails to parse is reported and ignored rather than fatal. Losing a tweak to a
//! typo is a nuisance; refusing to start over one is worse.

use std::fs;
use std::path::PathBuf;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::i18n::Language;

/// Where the file lives under the platform's config directory.
const CONFIG_SUBDIR: &str = "murabito";
const CONFIG_FILE: &str = "settings.yaml";

/// Loads settings at startup and writes them back whenever they change.
pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        let (file, outcome) = SettingsFile::load();

        // Only a genuinely missing file is created here. A file that failed to parse is
        // left exactly as it is: the player can fix the typo and keep the rest of their
        // settings, instead of finding them silently replaced with defaults.
        if outcome == LoadOutcome::Missing {
            file.save();
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
#[derive(PartialEq)]
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
#[derive(Serialize, Deserialize, Default, Clone, Copy, PartialEq)]
#[serde(default)]
pub struct UiSettings {
    /// Which language the UI is in. `en` or `ja` in the file.
    pub language: Language,
}

/// Camera preferences. A `Resource`, not a component: there is one set of them for the
/// whole app, independent of how many camera rigs exist.
#[derive(Resource, Serialize, Deserialize, Clone, Copy, PartialEq)]
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
    fn load() -> (Self, LoadOutcome) {
        let Some(path) = settings_path() else {
            warn!("no config directory for this platform; using default settings");
            return (Self::default(), LoadOutcome::Unusable);
        };

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
    fn save(&self) {
        let Some(path) = settings_path() else {
            return;
        };
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

/// The full path to the settings file, or `None` if the platform has no config dir.
fn settings_path() -> Option<PathBuf> {
    Some(dirs::config_dir()?.join(CONFIG_SUBDIR).join(CONFIG_FILE))
}

/// Saves whenever any settings resource changed this frame. The frame one is inserted
/// counts as a change, so those are skipped: writing the file at startup is the plugin's
/// job, and only when there was no file to begin with.
fn save_on_change(camera: Res<CameraSettings>, language: Res<Language>) {
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
    .save();
}

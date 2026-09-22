//! Settings that persist: one file, `settings.yaml` in the player's directory, with one
//! top-level key per registered resource.
//!
//! This crate names no setting. The module that uses a setting owns it as a resource;
//! the app registers that resource here with [`Persist::persist`], and from then on it
//! is loaded at startup and written back on the frame it changes. Nothing but the app
//! depends on this crate.
//!
//! The file is meant to be hand-edited. A missing file means the defaults, and one is
//! written so it can be found. A file, or a section of one, that can't be read is
//! copied to `settings-<moment>.bak` first, then replaced by what the game could make
//! of it: losing a tweak to a typo is a nuisance, losing the whole file to it is worse,
//! and refusing to start over it is worst. Sections nothing registered ride along
//! untouched, so an older or newer version's key is never dropped.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use bevy::prelude::*;
use murabito_user_data::UserData;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_yaml_ng::Value;

/// What the file is called inside the player's directory. Where that directory is
/// belongs to `murabito_user_data`, not here.
const SETTINGS_FILE: &str = "settings.yaml";

/// Reads the settings file as the app is built, and writes it back whenever a
/// registered resource changes.
///
/// Add it after `UserDataPlugin`, whose resource it reads while the app is built, and
/// before any call to [`Persist::persist`].
pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        let file = match app.world().get_resource::<UserData>() {
            Some(user_data) => SettingsFile::load(user_data.root().join(SETTINGS_FILE)),
            None => {
                warn!("no player directory; settings will not be saved");
                SettingsFile::unsaved()
            }
        };
        app.insert_resource(file)
            // Every snapshot lands before the one write, so a frame's changes are
            // written once, together.
            .configure_sets(Last, (SettingsSet::Snapshot, SettingsSet::Write).chain())
            .add_systems(Last, write_if_dirty.in_set(SettingsSet::Write));
    }
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
enum SettingsSet {
    Snapshot,
    Write,
}

/// Registers a resource for persistence.
pub trait Persist {
    /// Loads `T` from the section `key` of the settings file, or its default when the
    /// file has no such section, inserts it as a resource, and writes it back on any
    /// frame it changes. The module's own plugin may be added before or after: its
    /// `init_resource` keeps a value already present, and this replaces a default.
    ///
    /// # Panics
    ///
    /// If [`SettingsPlugin`] has not been added yet.
    fn persist<T>(&mut self, key: &'static str) -> &mut Self
    where
        T: Resource + Serialize + DeserializeOwned + Default;
}

impl Persist for App {
    fn persist<T>(&mut self, key: &'static str) -> &mut Self
    where
        T: Resource + Serialize + DeserializeOwned + Default,
    {
        let value = self
            .world_mut()
            .get_resource_mut::<SettingsFile>()
            .expect("add SettingsPlugin before persisting anything")
            .section::<T>(key);
        self.insert_resource(value);
        self.add_systems(Last, snapshot::<T>(key).in_set(SettingsSet::Snapshot))
    }
}

/// The system that notices `T` changing and records it. A closure, so that the key it
/// was registered under travels with it.
fn snapshot<T>(key: &'static str) -> impl FnMut(Res<T>, ResMut<SettingsFile>)
where
    T: Resource + Serialize,
{
    move |setting: Res<T>, mut file: ResMut<SettingsFile>| {
        // The frame it was inserted counts as a change, and writing at startup is the
        // plugin's decision, not the resource's.
        if setting.is_added() || !setting.is_changed() {
            return;
        }
        file.set(key, &*setting);
    }
}

fn write_if_dirty(mut file: ResMut<SettingsFile>) {
    if file.dirty {
        file.write();
    }
}

/// The file as the game holds it: every top-level section as raw YAML, registered or
/// not, and whether what is held differs from what is on disk.
#[derive(Resource)]
struct SettingsFile {
    /// `None` when there is nowhere to write: no player directory, or the file could
    /// not be read for a reason other than not existing.
    path: Option<PathBuf>,
    sections: BTreeMap<String, Value>,
    dirty: bool,
    /// A file is backed up once per run, however many of its sections turn out bad.
    backed_up: bool,
}

impl SettingsFile {
    fn unsaved() -> Self {
        Self {
            path: None,
            sections: BTreeMap::new(),
            dirty: false,
            backed_up: false,
        }
    }

    fn load(path: PathBuf) -> Self {
        let mut file = Self {
            path: Some(path.clone()),
            ..Self::unsaved()
        };
        match fs::read_to_string(&path) {
            Ok(text) => match serde_yaml_ng::from_str::<BTreeMap<String, Value>>(&text) {
                Ok(sections) => {
                    info!("loaded settings from {}", path.display());
                    file.sections = sections;
                }
                Err(error) => {
                    warn!("{} is not valid settings YAML ({error})", path.display());
                    file.back_up();
                    file.dirty = true;
                }
            },
            // The normal first run. A file is written at the end of the first frame.
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => file.dirty = true,
            Err(error) => {
                warn!(
                    "could not read {} ({error}); settings will not be saved",
                    path.display()
                );
                file.path = None;
            }
        }
        file
    }

    /// The section `key` as a `T`, or the default when it is missing or unreadable.
    /// Either way the section is then held as the value handed back, so the file gains
    /// what it lacked when next written.
    fn section<T: DeserializeOwned + Serialize + Default>(&mut self, key: &str) -> T {
        let value = match self.sections.get(key) {
            None => T::default(),
            Some(raw) => match serde_yaml_ng::from_value(raw.clone()) {
                Ok(value) => value,
                Err(error) => {
                    warn!("settings section `{key}` could not be read ({error}); using defaults");
                    self.back_up();
                    T::default()
                }
            },
        };
        self.set(key, &value);
        value
    }

    /// Records a section's current value. Dirty only if it differs from what is held,
    /// so a clean load writes nothing.
    fn set<T: Serialize>(&mut self, key: &str, value: &T) {
        let raw = match serde_yaml_ng::to_value(value) {
            Ok(raw) => raw,
            Err(error) => {
                warn!("settings section `{key}` could not be serialised ({error})");
                return;
            }
        };
        if self.sections.get(key) != Some(&raw) {
            self.sections.insert(key.to_string(), raw);
            self.dirty = true;
        }
    }

    /// Copies the file as it is to `settings-<moment>.bak`, before it is replaced.
    fn back_up(&mut self) {
        let Some(path) = &self.path else { return };
        if self.backed_up {
            return;
        }
        self.backed_up = true;
        let backup = backup_path(path);
        match fs::copy(path, &backup) {
            Ok(_) => warn!("the file as it was is kept at {}", backup.display()),
            Err(error) => warn!("could not back up {} ({error})", path.display()),
        }
    }

    /// Writes every section. To a temporary file first, renamed over the real one, so
    /// a crash mid-write leaves the old file rather than half of a new one.
    fn write(&mut self) {
        // Whatever happens, one attempt per change: a failing disk is warned about
        // once, not every frame.
        self.dirty = false;
        let Some(path) = &self.path else { return };
        let text = match serde_yaml_ng::to_string(&self.sections) {
            Ok(text) => text,
            Err(error) => return warn!("could not serialise settings ({error})"),
        };
        if let Err(error) = write_atomically(path, &text) {
            warn!(
                "could not write {} ({error}); settings not saved",
                path.display()
            );
        }
    }
}

fn write_atomically(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension("yaml.tmp");
    fs::write(&temporary, text)?;
    fs::rename(&temporary, path)
}

/// `settings-20260922T154012Z.bak` beside the file, in UTC so the name needs no
/// timezone to read. A second backup in the same second gets a counter.
fn backup_path(path: &Path) -> PathBuf {
    let moment = time::OffsetDateTime::now_utc()
        .format(&time::macros::format_description!(
            "[year][month][day]T[hour][minute][second]Z"
        ))
        .unwrap_or_else(|_| "unknown-time".to_string());
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let mut candidate = path.with_file_name(format!("{stem}-{moment}.bak"));
    let mut n = 2;
    while candidate.exists() {
        candidate = path.with_file_name(format!("{stem}-{moment}-{n}.bak"));
        n += 1;
    }
    candidate
}

#[cfg(test)]
mod tests {
    use super::*;
    use murabito_user_data::UserDataPlugin;
    use serde::Deserialize;
    use tempfile::TempDir;

    /// A setting of some module's, as far as this crate is concerned: a resource that
    /// serialises and has a default.
    #[derive(Resource, Serialize, Deserialize, Default, Clone, Debug, PartialEq)]
    struct Volume {
        level: u8,
    }

    /// A player directory of our own, and the path the file would have in it. Nothing
    /// here may read or write the real one.
    fn scratch() -> (TempDir, PathBuf) {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join(SETTINGS_FILE);
        (dir, path)
    }

    /// An app persisting `Volume` from `dir`, run for its first frame.
    fn app_in(dir: &Path) -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, UserDataPlugin::at(dir), SettingsPlugin))
            .persist::<Volume>("volume");
        app.update();
        app
    }

    fn write_file(path: &Path, text: &str) {
        fs::write(path, text).expect("writing the settings file");
    }

    fn read_file(path: &Path) -> String {
        fs::read_to_string(path).expect("reading the settings file")
    }

    fn backups_in(dir: &Path) -> Vec<PathBuf> {
        let mut found: Vec<PathBuf> = fs::read_dir(dir)
            .expect("the scratch directory")
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|p| p.extension().is_some_and(|e| e == "bak"))
            .collect();
        found.sort();
        found
    }

    fn set_level(app: &mut App, level: u8) {
        app.world_mut().resource_mut::<Volume>().level = level;
    }

    #[test]
    fn a_missing_file_means_defaults_and_one_is_written_on_the_first_frame() {
        let (dir, path) = scratch();

        let app = app_in(dir.path());

        assert_eq!(app.world().resource::<Volume>(), &Volume::default());
        assert_eq!(read_file(&path), "volume:\n  level: 0\n");
    }

    #[test]
    fn a_setting_in_the_file_is_what_the_resource_starts_as() {
        let (dir, path) = scratch();
        write_file(&path, "volume:\n  level: 7\n");

        let app = app_in(dir.path());

        assert_eq!(app.world().resource::<Volume>().level, 7);
    }

    #[test]
    fn a_change_is_written_by_the_end_of_its_frame() {
        let (dir, path) = scratch();
        let mut app = app_in(dir.path());

        set_level(&mut app, 3);
        app.update();

        assert_eq!(read_file(&path), "volume:\n  level: 3\n");
    }

    #[test]
    fn a_frame_without_a_change_writes_nothing() {
        let (dir, path) = scratch();
        let mut app = app_in(dir.path());
        fs::remove_file(&path).expect("the file written at startup");

        app.update();

        assert!(
            !path.exists(),
            "the file was rewritten with nothing changed"
        );
    }

    #[test]
    fn a_section_nothing_registered_survives_a_save() {
        let (dir, path) = scratch();
        write_file(&path, "mystery:\n  answer: 42\n");
        let mut app = app_in(dir.path());

        set_level(&mut app, 1);
        app.update();

        assert_eq!(
            read_file(&path),
            "mystery:\n  answer: 42\nvolume:\n  level: 1\n"
        );
    }

    #[test]
    fn an_unreadable_file_is_backed_up_then_replaced_with_defaults() {
        let (dir, path) = scratch();
        write_file(&path, "volume: [this is not\n");

        let app = app_in(dir.path());

        assert_eq!(app.world().resource::<Volume>(), &Volume::default());
        let backups = backups_in(dir.path());
        assert_eq!(backups.len(), 1, "backups: {backups:?}");
        assert_eq!(read_file(&backups[0]), "volume: [this is not\n");
        assert_eq!(read_file(&path), "volume:\n  level: 0\n");
    }

    #[test]
    fn a_bad_section_is_backed_up_and_defaulted_while_the_rest_is_kept() {
        let (dir, path) = scratch();
        write_file(&path, "mystery: 42\nvolume: [1, 2]\n");

        let app = app_in(dir.path());

        assert_eq!(app.world().resource::<Volume>(), &Volume::default());
        assert_eq!(backups_in(dir.path()).len(), 1);
        assert_eq!(read_file(&path), "mystery: 42\nvolume:\n  level: 0\n");
    }

    #[test]
    fn the_backup_is_named_after_the_moment_it_was_taken() {
        let (dir, path) = scratch();
        write_file(&path, "not yaml: [\n");

        let _app = app_in(dir.path());

        let name = backups_in(dir.path())[0]
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        assert!(
            name.starts_with("settings-20") && name.ends_with("Z.bak") && name.len() == 29,
            "{name}"
        );
    }

    #[test]
    fn a_second_backup_in_the_same_moment_does_not_overwrite_the_first() {
        let (_dir, path) = scratch();
        write_file(&path, "first\n");
        let first = backup_path(&path);
        fs::copy(&path, &first).unwrap();

        let second = backup_path(&path);

        assert_ne!(first, second);
        assert!(second.to_string_lossy().ends_with("-2.bak"), "{second:?}");
    }

    #[test]
    fn nothing_is_left_beside_the_file_after_a_write() {
        let (dir, _path) = scratch();
        let mut app = app_in(dir.path());
        set_level(&mut app, 9);
        app.update();

        let names: Vec<String> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, [SETTINGS_FILE]);
    }

    #[test]
    #[should_panic(expected = "SettingsPlugin")]
    fn persisting_before_the_plugin_is_a_wiring_mistake() {
        let (dir, _path) = scratch();
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, UserDataPlugin::at(dir.path())))
            .persist::<Volume>("volume");
    }

    #[test]
    fn with_no_player_directory_the_defaults_stand_and_nothing_is_written() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, SettingsPlugin))
            .persist::<Volume>("volume");
        app.update();
        set_level(&mut app, 5);

        app.update();

        assert_eq!(app.world().resource::<Volume>().level, 5);
    }

    #[test]
    fn a_module_plugin_added_after_keeps_the_loaded_value() {
        let (dir, path) = scratch();
        write_file(&path, "volume:\n  level: 7\n");
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            UserDataPlugin::at(dir.path()),
            SettingsPlugin,
        ))
        .persist::<Volume>("volume")
        .init_resource::<Volume>();

        app.update();

        assert_eq!(app.world().resource::<Volume>().level, 7);
    }
}

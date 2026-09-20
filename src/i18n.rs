//! UI strings in English and Japanese.
//!
//! Text is never written into the UI as a literal. A text entity carries a [`Localized`]
//! key instead, and the systems here fill in the string for the current [`Language`] —
//! when the entity appears, and again whenever the language changes, so switching
//! language is live rather than needing every screen rebuilt.
//!
//! The catalogues are compiled in with `include_str!`, so a missing file is a build
//! error rather than a startup failure. Editing a translation needs a rebuild.

use std::collections::HashMap;

use bevy::platform::collections::HashSet;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// The catalogues, as they sit in `assets/locales/`.
const EN_YAML: &str = include_str!("../assets/locales/en.yaml");
const JA_YAML: &str = include_str!("../assets/locales/ja.yaml");

pub struct I18nPlugin;

impl Plugin for I18nPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Language>()
            .insert_resource(Catalogues::load())
            .add_systems(
                Update,
                (
                    // New text gets its string the moment it appears...
                    localize_new,
                    // ...and everything is rewritten when the language changes.
                    localize_all.run_if(resource_changed::<Language>),
                ),
            );
    }
}

/// Which language the UI is in.
#[derive(Resource, Serialize, Deserialize, Clone, Copy, Default, PartialEq, Eq, Hash, Debug)]
pub enum Language {
    #[default]
    #[serde(rename = "en")]
    English,
    #[serde(rename = "ja")]
    Japanese,
}

/// Marks a text entity as holding a translated string. The value is the catalogue key.
#[derive(Component, Clone, Copy)]
pub struct Localized(pub &'static str);

/// Every language's strings, parsed once at startup.
#[derive(Resource)]
struct Catalogues(HashMap<Language, HashMap<String, String>>);

impl Catalogues {
    fn load() -> Self {
        let mut catalogues = HashMap::new();
        for (language, yaml) in [(Language::English, EN_YAML), (Language::Japanese, JA_YAML)] {
            match serde_yaml_ng::from_str::<HashMap<String, String>>(yaml) {
                Ok(strings) => {
                    catalogues.insert(language, strings);
                }
                Err(error) => {
                    // Compiled in, so this means the file in the repo is malformed: loud,
                    // but not worth refusing to start over.
                    error!("{language:?} catalogue is not valid YAML ({error})");
                    catalogues.insert(language, HashMap::new());
                }
            }
        }

        let catalogues = Self(catalogues);
        catalogues.warn_about_gaps();
        catalogues
    }

    /// The string for `key`, or the key itself when it is missing — which shows up on
    /// screen as `menu.quit` rather than as nothing at all.
    fn get(&self, language: Language, key: &str) -> String {
        self.0
            .get(&language)
            .and_then(|strings| strings.get(key))
            .cloned()
            .unwrap_or_else(|| key.to_string())
    }

    /// Reports keys that exist in one language but not another, once, at startup. Cheap
    /// insurance against a translation quietly falling behind as strings are added.
    fn warn_about_gaps(&self) {
        let all: HashSet<&String> = self.0.values().flat_map(|strings| strings.keys()).collect();
        for (language, strings) in &self.0 {
            let missing: Vec<&&String> = all
                .iter()
                .filter(|key| !strings.contains_key(**key))
                .collect();
            if !missing.is_empty() {
                warn!("{language:?} catalogue is missing {missing:?}");
            }
        }
    }
}

fn localize_new(
    language: Res<Language>,
    catalogues: Res<Catalogues>,
    mut texts: Query<(&Localized, &mut Text), Added<Localized>>,
) {
    for (localized, mut text) in &mut texts {
        **text = catalogues.get(*language, localized.0);
    }
}

fn localize_all(
    language: Res<Language>,
    catalogues: Res<Catalogues>,
    mut texts: Query<(&Localized, &mut Text)>,
) {
    for (localized, mut text) in &mut texts {
        **text = catalogues.get(*language, localized.0);
    }
}

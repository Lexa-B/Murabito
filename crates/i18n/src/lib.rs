//! UI strings in English and Japanese.
//!
//! Text is never written into the UI as a literal. A text entity carries a [`Localized`]
//! key instead, and the systems here fill in the string for the current [`Language`]:
//! when the entity appears, and again for every entity whenever the language changes,
//! so switching language is live rather than needing every screen rebuilt.
//!
//! The catalogues, `assets/locales/en.yaml` and `ja.yaml`, are compiled in with
//! `include_str!`, so a missing file is a build error rather than a startup failure.
//! Editing a translation needs a rebuild. A key with no entry renders as the key
//! itself, which on screen beats nothing at all; keys one catalogue has and another
//! lacks are warned about at startup, and a test on the real files fails.
//!
//! `Language` is a setting. It is owned here, and the app persists it.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// The catalogues, as they sit in `assets/locales/` at the repo root.
const EN_YAML: &str = include_str!("../../../assets/locales/en.yaml");
const JA_YAML: &str = include_str!("../../../assets/locales/ja.yaml");

pub struct I18nPlugin;

impl Plugin for I18nPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Language>()
            .insert_resource(Catalogues::compiled_in())
            .add_systems(
                Update,
                (
                    // New text gets its string the frame it appears...
                    localize_new,
                    // ...and everything is rewritten when the language changes.
                    localize_all.run_if(resource_changed::<Language>),
                ),
            );
    }
}

/// Which language the UI is in. `en` or `ja` in the settings file.
#[derive(
    Resource,
    Serialize,
    Deserialize,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Debug,
)]
pub enum Language {
    #[default]
    #[serde(rename = "en")]
    English,
    #[serde(rename = "ja")]
    Japanese,
}

impl Language {
    pub const ALL: [Language; 2] = [Language::English, Language::Japanese];

    /// The language's name in its own script. Never translated: someone who can't read
    /// the current language still recognises their own.
    pub fn own_name(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::Japanese => "日本語",
        }
    }
}

/// Marks a text entity as holding a translated string. The value is the catalogue key.
///
/// `Cow` rather than `&'static str` because not every key will be known at compile
/// time: a debug screen may name a thing by a key read from data at startup. A literal
/// key still costs no allocation.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct Localized(pub Cow<'static, str>);

impl Localized {
    pub fn new(key: impl Into<Cow<'static, str>>) -> Self {
        Self(key.into())
    }
}

/// Every language's strings, parsed once at startup.
#[derive(Resource, Debug, PartialEq, Eq)]
struct Catalogues(BTreeMap<Language, BTreeMap<String, String>>);

impl Catalogues {
    fn compiled_in() -> Self {
        let catalogues = Self::parse([(Language::English, EN_YAML), (Language::Japanese, JA_YAML)]);
        for (language, missing) in catalogues.gaps() {
            warn!("the {language:?} catalogue is missing {missing:?}");
        }
        catalogues
    }

    /// Parses each language's YAML. A catalogue that won't parse is reported and left
    /// empty: compiled in, so it means the file in the repo is malformed, which is loud
    /// but not worth refusing to start over.
    fn parse<'a>(sources: impl IntoIterator<Item = (Language, &'a str)>) -> Self {
        let mut catalogues = BTreeMap::new();
        for (language, yaml) in sources {
            // A file that is only comments parses as nothing at all, not as an empty map.
            let parsed = serde_yaml_ng::from_str::<Option<BTreeMap<String, String>>>(yaml);
            let strings = match parsed {
                Ok(strings) => strings.unwrap_or_default(),
                Err(error) => {
                    error!("the {language:?} catalogue is not valid YAML ({error})");
                    BTreeMap::new()
                }
            };
            catalogues.insert(language, strings);
        }
        Self(catalogues)
    }

    /// The string for `key`, or the key itself when there is none.
    fn get(&self, language: Language, key: &str) -> String {
        self.0
            .get(&language)
            .and_then(|strings| strings.get(key))
            .cloned()
            .unwrap_or_else(|| key.to_string())
    }

    /// For each language, the keys some other language has and it lacks. Empty when
    /// every catalogue carries every key.
    fn gaps(&self) -> BTreeMap<Language, BTreeSet<String>> {
        let every_key: BTreeSet<&String> = self.0.values().flat_map(|s| s.keys()).collect();
        self.0
            .iter()
            .map(|(language, strings)| {
                let missing = every_key
                    .iter()
                    .filter(|key| !strings.contains_key(**key))
                    .map(|key| (*key).clone())
                    .collect::<BTreeSet<String>>();
                (*language, missing)
            })
            .filter(|(_, missing)| !missing.is_empty())
            .collect()
    }
}

fn localize_new(
    language: Res<Language>,
    catalogues: Res<Catalogues>,
    mut texts: Query<(&Localized, &mut Text), Added<Localized>>,
) {
    for (localized, mut text) in &mut texts {
        **text = catalogues.get(*language, &localized.0);
    }
}

fn localize_all(
    language: Res<Language>,
    catalogues: Res<Catalogues>,
    mut texts: Query<(&Localized, &mut Text)>,
) {
    for (localized, mut text) in &mut texts {
        **text = catalogues.get(*language, &localized.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EN: &str = "menu.quit: Quit\nmenu.resume: Resume\n";
    const JA: &str = "menu.quit: 終了\nmenu.resume: 再開\n";

    fn two_languages() -> Catalogues {
        Catalogues::parse([(Language::English, EN), (Language::Japanese, JA)])
    }

    /// A headless app with these catalogues instead of the compiled-in ones, run for
    /// its first frame.
    fn app_with(catalogues: Catalogues) -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, I18nPlugin));
        app.insert_resource(catalogues);
        app.update();
        app
    }

    fn text_of(app: &mut App, entity: Entity) -> String {
        app.world().get::<Text>(entity).expect("a text").0.clone()
    }

    #[test]
    fn the_language_is_two_letters_in_a_file() {
        assert_eq!(
            serde_yaml_ng::to_string(&Language::Japanese).unwrap(),
            "ja\n"
        );
        assert_eq!(
            serde_yaml_ng::from_str::<Language>("en").unwrap(),
            Language::English
        );
    }

    #[test]
    fn each_language_names_itself_in_its_own_script() {
        assert_eq!(Language::English.own_name(), "English");
        assert_eq!(Language::Japanese.own_name(), "日本語");
    }

    #[test]
    fn a_text_gets_its_string_the_frame_it_appears() {
        let mut app = app_with(two_languages());
        let quit = app
            .world_mut()
            .spawn((Localized::new("menu.quit"), Text::default()))
            .id();

        app.update();

        assert_eq!(text_of(&mut app, quit), "Quit");
    }

    #[test]
    fn changing_the_language_rewrites_every_text() {
        let mut app = app_with(two_languages());
        let quit = app
            .world_mut()
            .spawn((Localized::new("menu.quit"), Text::default()))
            .id();
        let resume = app
            .world_mut()
            .spawn((Localized::new("menu.resume"), Text::default()))
            .id();
        app.update();

        *app.world_mut().resource_mut::<Language>() = Language::Japanese;
        app.update();

        assert_eq!(text_of(&mut app, quit), "終了");
        assert_eq!(text_of(&mut app, resume), "再開");
    }

    #[test]
    fn a_key_with_no_entry_shows_as_itself() {
        let mut app = app_with(two_languages());
        let odd = app
            .world_mut()
            .spawn((Localized::new("menu.nothing"), Text::default()))
            .id();

        app.update();

        assert_eq!(text_of(&mut app, odd), "menu.nothing");
    }

    #[test]
    fn a_key_one_language_lacks_is_a_gap_named_against_that_language() {
        let catalogues = Catalogues::parse([
            (Language::English, "a: A\nb: B\n"),
            (Language::Japanese, "a: あ\n"),
        ]);

        let gaps = catalogues.gaps();

        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[&Language::Japanese], BTreeSet::from(["b".to_string()]));
    }

    #[test]
    fn a_catalogue_of_only_comments_is_empty_not_an_error() {
        let catalogues = Catalogues::parse([(Language::English, "# nothing yet\n")]);

        assert_eq!(catalogues.0[&Language::English].len(), 0);
    }

    #[test]
    fn the_real_catalogues_parse_and_carry_the_same_keys() {
        let catalogues = Catalogues::compiled_in();

        assert_eq!(catalogues.0.len(), 2);
        assert!(
            catalogues.gaps().is_empty(),
            "gaps: {:?}",
            catalogues.gaps()
        );
    }
}

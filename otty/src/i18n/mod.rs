//! Interface localization.
//!
//! Translations live in JSON files under `locales/` and are embedded into the
//! binary at build time. The active locale is process-global so that view
//! functions can call [`t`] without threading a locale parameter through every
//! widget signature.

mod key;

use std::collections::HashMap;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU8, Ordering};

pub(crate) use key::Key;
use serde::Deserialize;

/// Number of configurable palette colors, mirroring `PALETTE_FIELDS`.
const PALETTE_LABEL_COUNT: usize = 29;

/// Active locale, encoded as [`Locale::index`].
static CURRENT_LOCALE: AtomicU8 = AtomicU8::new(Locale::En.index() as u8);

/// Parsed contents of one locale JSON file.
#[derive(Debug, Deserialize)]
struct Catalog {
    /// Palette color labels in palette field order.
    palette_labels: Vec<String>,
    /// All translatable strings, addressed by [`Key`].
    strings: HashMap<Key, String>,
}

impl Catalog {
    /// Parse an embedded catalog, panicking on malformed input.
    ///
    /// The inputs are compile-time constants, so a failure here is a build
    /// defect rather than a runtime condition.
    fn parse(name: &str, json: &str) -> Self {
        serde_json::from_str(json).unwrap_or_else(|err| {
            panic!("locale catalog {name} is invalid: {err}")
        })
    }
}

/// Interface locales shipped with the application.
///
/// Adding a language means adding a variant here, a JSON file under
/// `locales/`, and an entry in [`Locale::ALL`]; no view code changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Locale {
    /// English.
    En,
    /// Simplified Chinese.
    ZhCn,
    /// Traditional Chinese.
    ZhTw,
    /// Japanese.
    Ja,
    /// Korean.
    Ko,
    /// French.
    Fr,
    /// German.
    De,
    /// Spanish.
    Es,
    /// Brazilian Portuguese.
    PtBr,
    /// Russian.
    Ru,
}

impl Locale {
    /// Every shipped locale, in the order shown by the language selector.
    pub(crate) const ALL: [Self; 10] = [
        Self::En,
        Self::ZhCn,
        Self::ZhTw,
        Self::Ja,
        Self::Ko,
        Self::Fr,
        Self::De,
        Self::Es,
        Self::PtBr,
        Self::Ru,
    ];

    /// BCP 47 tag persisted in the settings file.
    pub(crate) fn tag(self) -> &'static str {
        match self {
            Self::ZhCn => "zh-CN",
            Self::ZhTw => "zh-TW",
            Self::Ja => "ja",
            Self::Ko => "ko",
            Self::Fr => "fr",
            Self::De => "de",
            Self::Es => "es",
            Self::PtBr => "pt-BR",
            Self::Ru => "ru",
            _ => "en",
        }
    }

    /// Language name written in that language, as shown in the selector.
    ///
    /// Endonyms are used so a user can find their language without already
    /// reading the current interface language.
    pub(crate) fn native_name(self) -> &'static str {
        match self {
            Self::ZhCn => "简体中文",
            Self::ZhTw => "繁體中文",
            Self::Ja => "日本語",
            Self::Ko => "한국어",
            Self::Fr => "Français",
            Self::De => "Deutsch",
            Self::Es => "Español",
            Self::PtBr => "Português (Brasil)",
            Self::Ru => "Русский",
            _ => "English",
        }
    }

    /// Resolve a locale from a BCP 47 tag or POSIX locale string.
    ///
    /// Accepts forms such as `zh_TW.UTF-8` and `pt-BR`. Unknown or malformed
    /// values fall back to English rather than failing.
    pub(crate) fn from_tag(tag: &str) -> Self {
        let normalized = tag
            .split(['.', '@'])
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase()
            .replace('_', "-");

        // Chinese splits by script rather than by primary subtag, so the
        // traditional-script regions and an explicit `Hant` map to zh-TW.
        if normalized.starts_with("zh") {
            let traditional = normalized.contains("hant")
                || normalized.contains("-tw")
                || normalized.contains("-hk")
                || normalized.contains("-mo");
            return if traditional { Self::ZhTw } else { Self::ZhCn };
        }

        match normalized.split('-').next().unwrap_or_default() {
            "ja" => Self::Ja,
            "ko" => Self::Ko,
            "fr" => Self::Fr,
            "de" => Self::De,
            "es" => Self::Es,
            "pt" => Self::PtBr,
            "ru" => Self::Ru,
            _ => Self::En,
        }
    }

    /// Embedded catalog source for this locale.
    fn json(self) -> &'static str {
        match self {
            Self::ZhCn => include_str!("locales/zh-CN.json"),
            Self::ZhTw => include_str!("locales/zh-TW.json"),
            Self::Ja => include_str!("locales/ja.json"),
            Self::Ko => include_str!("locales/ko.json"),
            Self::Fr => include_str!("locales/fr.json"),
            Self::De => include_str!("locales/de.json"),
            Self::Es => include_str!("locales/es.json"),
            Self::PtBr => include_str!("locales/pt-BR.json"),
            Self::Ru => include_str!("locales/ru.json"),
            _ => include_str!("locales/en.json"),
        }
    }

    /// Position in [`Locale::ALL`], used to index the catalog cache and to
    /// encode the locale into the global atomic.
    const fn index(self) -> usize {
        match self {
            Self::ZhCn => 1,
            Self::ZhTw => 2,
            Self::Ja => 3,
            Self::Ko => 4,
            Self::Fr => 5,
            Self::De => 6,
            Self::Es => 7,
            Self::PtBr => 8,
            Self::Ru => 9,
            _ => 0,
        }
    }

    /// Decode a locale previously encoded by [`Locale::index`].
    fn from_index(value: usize) -> Self {
        match Self::ALL.get(value) {
            Some(locale) => *locale,
            _ => Self::En,
        }
    }
}

/// Return the active interface locale.
pub(crate) fn current_locale() -> Locale {
    Locale::from_index(CURRENT_LOCALE.load(Ordering::Relaxed) as usize)
}

/// Set the active interface locale.
///
/// Callers must re-render afterwards; this only swaps the catalog used by
/// subsequent [`t`] calls.
pub(crate) fn set_locale(locale: Locale) {
    CURRENT_LOCALE.store(locale.index() as u8, Ordering::Relaxed);
}

/// Detect the preferred locale from the environment.
///
/// Checks the POSIX locale variables in the order the C library uses; an unset
/// or unrecognized environment yields [`Locale::En`].
pub(crate) fn detect_system_locale() -> Locale {
    for name in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        let Ok(value) = std::env::var(name) else {
            continue;
        };
        if !value.is_empty() {
            return Locale::from_tag(&value);
        }
    }

    Locale::En
}

/// Return the translation of `key` in the active locale.
pub(crate) fn t(key: Key) -> &'static str {
    text_in(current_locale(), key)
}

/// Return the label for the palette color at `index`.
pub(crate) fn palette_label(index: usize) -> String {
    palette_label_in(current_locale(), index)
}

/// Build the tab title shown when editing an existing quick launch.
pub(crate) fn edit_tab_title(command_title: &str) -> String {
    fill_in(
        current_locale(),
        Key::TplEditTabTitle,
        &[("title", command_title)],
    )
}

/// Build the error tab title shown when a command fails to launch.
pub(crate) fn launch_failed_title(command_title: &str) -> String {
    fill_in(
        current_locale(),
        Key::TplLaunchFailedTitle,
        &[("title", command_title)],
    )
}

/// Build the error tab body shown when a command fails to launch.
pub(crate) fn launch_failed_body(command_title: &str, error: &str) -> String {
    fill_in(
        current_locale(),
        Key::TplLaunchFailedBody,
        &[("command", command_title), ("error", error)],
    )
}

/// Build the message shown when a terminal tab fails to initialize.
pub(crate) fn terminal_init_failed(error: &str) -> String {
    fill_in(
        current_locale(),
        Key::TplTerminalInitFailed,
        &[("error", error)],
    )
}

/// Return the translation of `key` in `locale`.
///
/// Falls back to English when a key is absent, so a partial catalog degrades
/// to readable text instead of blank UI.
fn text_in(locale: Locale, key: Key) -> &'static str {
    if let Some(value) = catalog(locale).strings.get(&key) {
        return value.as_str();
    }

    catalog(Locale::En)
        .strings
        .get(&key)
        .map_or("", String::as_str)
}

/// Return the label for the palette color at `index` in `locale`.
fn palette_label_in(locale: Locale, index: usize) -> String {
    if let Some(label) = catalog(locale).palette_labels.get(index) {
        return label.clone();
    }

    let display_index = (index + 1).to_string();
    fill_in(
        locale,
        Key::TplPaletteFallbackLabel,
        &[("index", &display_index)],
    )
}

/// Substitute `{name}` placeholders in the `locale` template behind `key`.
fn fill_in(locale: Locale, key: Key, arguments: &[(&str, &str)]) -> String {
    let mut text = text_in(locale, key).to_string();
    for (name, value) in arguments {
        text = text.replace(&format!("{{{name}}}"), value);
    }

    text
}

/// Return the parsed catalog for `locale`, parsing it on first use.
///
/// Catalogs are cached per locale, so only the languages actually displayed
/// are ever parsed.
fn catalog(locale: Locale) -> &'static Catalog {
    static CATALOGS: [OnceLock<Catalog>; Locale::ALL.len()] =
        [const { OnceLock::new() }; Locale::ALL.len()];

    // `index` is defined as the position in `ALL`, which
    // `locale_indices_match_all_positions` verifies.
    CATALOGS[locale.index()]
        .get_or_init(|| Catalog::parse(locale.tag(), locale.json()))
}

#[cfg(test)]
mod tests {
    use super::{
        Catalog, Key, Locale, PALETTE_LABEL_COUNT, catalog, fill_in,
        palette_label_in, text_in,
    };

    #[test]
    fn given_every_key_when_catalogs_are_read_then_all_are_translated() {
        for locale in Locale::ALL {
            let catalog = catalog(locale);
            for key in Key::ALL {
                assert!(
                    catalog.strings.contains_key(&key),
                    "{} is missing translation for {key:?}",
                    locale.tag()
                );
            }
        }
    }

    #[test]
    fn given_catalogs_when_counted_then_they_contain_no_extra_keys() {
        for locale in Locale::ALL {
            assert_eq!(
                catalog(locale).strings.len(),
                Key::ALL.len(),
                "{} has entries not listed in Key::ALL",
                locale.tag()
            );
        }
    }

    #[test]
    fn given_catalogs_when_palette_labels_read_then_counts_match_palette() {
        for locale in Locale::ALL {
            assert_eq!(
                catalog(locale).palette_labels.len(),
                PALETTE_LABEL_COUNT,
                "{} has the wrong palette label count",
                locale.tag()
            );
        }
    }

    #[test]
    fn given_chinese_tags_when_parsed_then_script_selects_the_variant() {
        assert_eq!(Locale::from_tag("zh_CN.UTF-8"), Locale::ZhCn);
        assert_eq!(Locale::from_tag("zh-Hans"), Locale::ZhCn);
        assert_eq!(Locale::from_tag("zh"), Locale::ZhCn);
        assert_eq!(Locale::from_tag("ZH-TW"), Locale::ZhTw);
        assert_eq!(Locale::from_tag("zh_HK"), Locale::ZhTw);
        assert_eq!(Locale::from_tag("zh-Hant"), Locale::ZhTw);
    }

    #[test]
    fn given_posix_locale_strings_when_parsed_then_language_is_recognized() {
        assert_eq!(Locale::from_tag("ja_JP.UTF-8"), Locale::Ja);
        assert_eq!(Locale::from_tag("ko_KR.UTF-8"), Locale::Ko);
        assert_eq!(Locale::from_tag("fr_CA"), Locale::Fr);
        assert_eq!(Locale::from_tag("de_AT@euro"), Locale::De);
        assert_eq!(Locale::from_tag("es_MX.UTF-8"), Locale::Es);
        assert_eq!(Locale::from_tag("pt_PT"), Locale::PtBr);
        assert_eq!(Locale::from_tag("ru_RU.UTF-8"), Locale::Ru);
        assert_eq!(Locale::from_tag("en_US.UTF-8"), Locale::En);
    }

    #[test]
    fn given_unknown_tags_when_parsed_then_english_is_used() {
        assert_eq!(Locale::from_tag("tlh"), Locale::En);
        assert_eq!(Locale::from_tag("C"), Locale::En);
        assert_eq!(Locale::from_tag(""), Locale::En);
    }

    #[test]
    fn given_locales_when_indexed_then_positions_match_all_order() {
        for (position, locale) in Locale::ALL.into_iter().enumerate() {
            assert_eq!(
                locale.index(),
                position,
                "{} has an index that does not match its position in ALL",
                locale.tag()
            );
        }
    }

    #[test]
    fn given_locales_when_encoded_then_the_atomic_round_trips() {
        for locale in Locale::ALL {
            assert_eq!(Locale::from_index(locale.index()), locale);
        }
    }

    #[test]
    fn given_locales_when_listed_then_tags_and_names_are_unique() {
        let mut tags: Vec<&str> = Locale::ALL.iter().map(|l| l.tag()).collect();
        tags.sort_unstable();
        let unique_tags = tags.len();
        tags.dedup();
        assert_eq!(tags.len(), unique_tags, "duplicate locale tag");

        let mut names: Vec<&str> =
            Locale::ALL.iter().map(|l| l.native_name()).collect();
        names.sort_unstable();
        let unique_names = names.len();
        names.dedup();
        assert_eq!(names.len(), unique_names, "duplicate locale name");
    }

    #[test]
    fn given_locale_when_round_tripped_through_tag_then_value_is_preserved() {
        for locale in Locale::ALL {
            assert_eq!(Locale::from_tag(locale.tag()), locale);
        }
    }

    #[test]
    fn given_a_locale_when_translating_then_that_locale_text_is_returned() {
        assert_eq!(text_in(Locale::ZhCn, Key::ButtonSave), "保存");
        assert_eq!(text_in(Locale::ZhCn, Key::SectionAppearance), "外观");
        assert_eq!(text_in(Locale::En, Key::ButtonSave), "Save");
        assert_eq!(text_in(Locale::En, Key::SectionAppearance), "Appearance");
    }

    #[test]
    fn given_templates_when_filled_then_placeholders_are_substituted() {
        assert_eq!(
            fill_in(Locale::En, Key::TplEditTabTitle, &[("title", "deploy")]),
            "Edit: deploy"
        );
        assert_eq!(
            fill_in(
                Locale::En,
                Key::TplLaunchFailedTitle,
                &[("title", "deploy")]
            ),
            "Failed to launch \"deploy\""
        );
        assert_eq!(
            fill_in(
                Locale::En,
                Key::TplLaunchFailedBody,
                &[("command", "deploy"), ("error", "boom")]
            ),
            "Command: deploy\nError: boom"
        );
        assert_eq!(
            fill_in(
                Locale::En,
                Key::TplTerminalInitFailed,
                &[("error", "boom")]
            ),
            "Terminal tab initialization failed: boom"
        );

        assert_eq!(
            fill_in(Locale::ZhCn, Key::TplEditTabTitle, &[("title", "deploy")]),
            "编辑：deploy"
        );
        assert_eq!(
            fill_in(
                Locale::ZhCn,
                Key::TplLaunchFailedTitle,
                &[("title", "deploy")]
            ),
            "启动“deploy”失败"
        );
        assert_eq!(
            fill_in(
                Locale::ZhCn,
                Key::TplLaunchFailedBody,
                &[("command", "deploy"), ("error", "boom")]
            ),
            "命令：deploy\n错误：boom"
        );
    }

    #[test]
    fn given_palette_index_when_labelled_then_named_and_fallback_labels_apply()
    {
        assert_eq!(palette_label_in(Locale::ZhCn, 0), "前景色");
        assert_eq!(
            palette_label_in(Locale::ZhCn, PALETTE_LABEL_COUNT),
            "颜色 30"
        );

        assert_eq!(palette_label_in(Locale::En, 0), "Foreground");
        assert_eq!(
            palette_label_in(Locale::En, PALETTE_LABEL_COUNT),
            "Color 30"
        );
    }

    #[test]
    fn given_every_locale_when_templates_read_then_placeholders_survive() {
        let required: [(Key, &[&str]); 5] = [
            (Key::TplEditTabTitle, &["{title}"]),
            (Key::TplLaunchFailedTitle, &["{title}"]),
            (Key::TplLaunchFailedBody, &["{command}", "{error}"]),
            (Key::TplTerminalInitFailed, &["{error}"]),
            (Key::TplPaletteFallbackLabel, &["{index}"]),
        ];

        for locale in Locale::ALL {
            for (key, placeholders) in required {
                let template = text_in(locale, key);
                for placeholder in placeholders {
                    assert!(
                        template.contains(placeholder),
                        "{} template {key:?} lost {placeholder}",
                        locale.tag()
                    );
                }
            }
        }
    }

    #[test]
    fn given_every_locale_when_catalog_read_then_no_entry_is_blank() {
        for locale in Locale::ALL {
            let catalog = catalog(locale);

            for (key, value) in &catalog.strings {
                assert!(
                    !value.trim().is_empty(),
                    "{} has a blank translation for {key:?}",
                    locale.tag()
                );
            }

            for (index, label) in catalog.palette_labels.iter().enumerate() {
                assert!(
                    !label.trim().is_empty(),
                    "{} has a blank palette label at {index}",
                    locale.tag()
                );
            }
        }
    }

    #[test]
    fn given_embedded_catalogs_when_parsed_then_they_deserialize() {
        for locale in Locale::ALL {
            let catalog = Catalog::parse(locale.tag(), locale.json());

            assert!(
                !catalog.strings.is_empty(),
                "{} parsed to an empty catalog",
                locale.tag()
            );
        }
    }
}

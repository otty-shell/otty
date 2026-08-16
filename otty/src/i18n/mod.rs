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

/// Embedded English catalog.
const EN_JSON: &str = include_str!("locales/en.json");
/// Embedded Simplified Chinese catalog.
const ZH_CN_JSON: &str = include_str!("locales/zh-CN.json");

/// Number of configurable palette colors, mirroring `PALETTE_FIELDS`.
const PALETTE_LABEL_COUNT: usize = 29;

/// Active locale, encoded as [`Locale::as_u8`].
static CURRENT_LOCALE: AtomicU8 = AtomicU8::new(Locale::En.as_u8());

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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Locale {
    /// English.
    En,
    /// Simplified Chinese.
    ZhCn,
}

impl Locale {
    /// BCP 47 tag persisted in the settings file.
    pub(crate) fn tag(self) -> &'static str {
        match self {
            Self::ZhCn => "zh-CN",
            _ => "en",
        }
    }

    /// Resolve a locale from a BCP 47 tag or POSIX locale string.
    ///
    /// Anything that is not recognizably Chinese falls back to English, which
    /// keeps unknown or malformed values usable instead of failing.
    pub(crate) fn from_tag(tag: &str) -> Self {
        let normalized = tag.to_ascii_lowercase().replace('_', "-");
        if normalized.starts_with("zh") {
            return Self::ZhCn;
        }

        Self::En
    }

    /// Encode the locale for storage in the global atomic.
    const fn as_u8(self) -> u8 {
        match self {
            Self::En => 0,
            Self::ZhCn => 1,
        }
    }

    /// Decode a locale previously encoded by [`Locale::as_u8`].
    fn from_u8(value: u8) -> Self {
        match value {
            1 => Self::ZhCn,
            _ => Self::En,
        }
    }
}

/// Return the active interface locale.
pub(crate) fn current_locale() -> Locale {
    Locale::from_u8(CURRENT_LOCALE.load(Ordering::Relaxed))
}

/// Set the active interface locale.
///
/// Callers must re-render afterwards; this only swaps the catalog used by
/// subsequent [`t`] calls.
pub(crate) fn set_locale(locale: Locale) {
    CURRENT_LOCALE.store(locale.as_u8(), Ordering::Relaxed);
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
fn catalog(locale: Locale) -> &'static Catalog {
    static EN: OnceLock<Catalog> = OnceLock::new();
    static ZH_CN: OnceLock<Catalog> = OnceLock::new();

    match locale {
        Locale::ZhCn => {
            ZH_CN.get_or_init(|| Catalog::parse("zh-CN", ZH_CN_JSON))
        },
        _ => EN.get_or_init(|| Catalog::parse("en", EN_JSON)),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Catalog, Key, Locale, PALETTE_LABEL_COUNT, catalog, fill_in,
        palette_label_in, text_in,
    };

    #[test]
    fn given_every_key_when_catalogs_are_read_then_all_are_translated() {
        for locale in [Locale::En, Locale::ZhCn] {
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
        for locale in [Locale::En, Locale::ZhCn] {
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
        for locale in [Locale::En, Locale::ZhCn] {
            assert_eq!(
                catalog(locale).palette_labels.len(),
                PALETTE_LABEL_COUNT,
                "{} has the wrong palette label count",
                locale.tag()
            );
        }
    }

    #[test]
    fn given_locale_tags_when_parsed_then_chinese_variants_map_to_zh_cn() {
        assert_eq!(Locale::from_tag("zh_CN.UTF-8"), Locale::ZhCn);
        assert_eq!(Locale::from_tag("zh-Hans"), Locale::ZhCn);
        assert_eq!(Locale::from_tag("ZH-TW"), Locale::ZhCn);
        assert_eq!(Locale::from_tag("en_US.UTF-8"), Locale::En);
        assert_eq!(Locale::from_tag("de_DE"), Locale::En);
        assert_eq!(Locale::from_tag(""), Locale::En);
    }

    #[test]
    fn given_locale_when_round_tripped_through_tag_then_value_is_preserved() {
        for locale in [Locale::En, Locale::ZhCn] {
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
    fn given_embedded_catalogs_when_parsed_then_they_deserialize() {
        let en = Catalog::parse("en", super::EN_JSON);
        let zh = Catalog::parse("zh-CN", super::ZH_CN_JSON);

        assert!(!en.strings.is_empty());
        assert!(!zh.strings.is_empty());
    }
}

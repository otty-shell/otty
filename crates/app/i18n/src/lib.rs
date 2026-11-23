//! Interface localization of the OTTY app.
//!
//! Translations live in JSON files under `locales/` and are embedded into the
//! binary at build time. The active locale is process-global so that view
//! functions can call [`t`] without threading a locale parameter through every
//! widget signature.
//!
//! This crate has no iced and no tokio. Its only I/O is asking the system
//! which language it prefers ([`system_locale`]), because that is a question
//! of the translations. It does not read the settings: the binary reads them
//! and passes the language in with [`set_locale`].
//!
//! Callers fill a template with [`filled`], [`with_count`], [`with_error`]
//! or [`one_or_many`]. A few functions stay for one template whose numbers
//! are not a single `{count}`, such as an exit code or "Block 5 of 6": they
//! write the numbers as text in one place, so no caller has to.
//!
//! # Examples
//!
//! ```
//! use otty_app_i18n::{Key, Locale, text_in};
//!
//! assert_eq!(text_in(Locale::En, Key::ButtonSave), "Save");
//! assert_eq!(
//!     text_in(Locale::from_tag("ru_RU.UTF-8"), Key::ButtonSave),
//!     "Сохранить"
//! );
//! ```

mod age;
mod key;

use std::collections::HashMap;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU8, Ordering};

pub use age::{short_age, time_ago};
pub use key::Key;
use serde::Deserialize;

/// Active locale, encoded as [`Locale::index`].
static CURRENT_LOCALE: AtomicU8 = AtomicU8::new(Locale::En.index() as u8);

/// The days of the week, Monday first.
const WEEKDAY_KEYS: [Key; 7] = [
    Key::AgentsWeekdayMonday,
    Key::AgentsWeekdayTuesday,
    Key::AgentsWeekdayWednesday,
    Key::AgentsWeekdayThursday,
    Key::AgentsWeekdayFriday,
    Key::AgentsWeekdaySaturday,
    Key::AgentsWeekdaySunday,
];

/// The months, January first.
const MONTH_KEYS: [Key; 12] = [
    Key::AgentsMonthJan,
    Key::AgentsMonthFeb,
    Key::AgentsMonthMar,
    Key::AgentsMonthApr,
    Key::AgentsMonthMay,
    Key::AgentsMonthJun,
    Key::AgentsMonthJul,
    Key::AgentsMonthAug,
    Key::AgentsMonthSep,
    Key::AgentsMonthOct,
    Key::AgentsMonthNov,
    Key::AgentsMonthDec,
];

/// Parsed contents of one locale JSON file.
#[derive(Debug, Deserialize)]
struct Catalog {
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
pub enum Locale {
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
    pub const ALL: [Self; 10] = [
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
    pub fn tag(self) -> &'static str {
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
    pub fn native_name(self) -> &'static str {
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
    pub fn from_tag(tag: &str) -> Self {
        let normalized = tag
            .split(['.', '@'])
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase()
            .replace('_', "-");

        // Chinese splits by script rather than by primary subtag. An explicit
        // script wins; otherwise traditional-script regions map to zh-TW.
        if normalized.split('-').next() == Some("zh") {
            if normalized.split('-').any(|subtag| subtag == "hans") {
                return Self::ZhCn;
            }
            if normalized.split('-').any(|subtag| subtag == "hant") {
                return Self::ZhTw;
            }

            let traditional = normalized
                .split('-')
                .any(|subtag| matches!(subtag, "tw" | "hk" | "mo"));
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
pub fn current_locale() -> Locale {
    Locale::from_index(CURRENT_LOCALE.load(Ordering::Relaxed) as usize)
}

/// Set the active interface locale.
///
/// Callers must re-render afterwards; this only swaps the catalog used by
/// subsequent [`t`] calls.
pub fn set_locale(locale: Locale) {
    CURRENT_LOCALE.store(locale.index() as u8, Ordering::Relaxed);
}

/// Resolve the locale from the system's preferred language, if it has one.
///
/// [`system_locale`] reads the preference and passes it in; `None` gives
/// English.
///
/// # Examples
///
/// ```
/// use otty_app_i18n::{Locale, locale_from_system_preference};
///
/// assert_eq!(
///     locale_from_system_preference(Some("ja_JP.UTF-8")),
///     Locale::Ja
/// );
/// assert_eq!(locale_from_system_preference(None), Locale::En);
/// ```
pub fn locale_from_system_preference(preference: Option<&str>) -> Locale {
    preference.map_or(Locale::En, Locale::from_tag)
}

/// Return the language the system prefers; English when it says none.
///
/// # Examples
///
/// ```
/// use otty_app_i18n::{Locale, set_locale, system_locale};
///
/// set_locale(system_locale());
/// ```
pub fn system_locale() -> Locale {
    locale_from_system_preference(sys_locale::get_locale().as_deref())
}

/// Return the language that decides the regional shapes of Han letters: the
/// app's when it writes them itself, otherwise the computer's, whose files and
/// names still come up in a program in English.
pub fn text_locale_tag(app: Locale, computer: Locale) -> &'static str {
    match app {
        Locale::ZhCn | Locale::ZhTw | Locale::Ja | Locale::Ko => app.tag(),
        _ => computer.tag(),
    }
}

/// Return the translation of `key` in the active locale.
pub fn t(key: Key) -> &'static str {
    text_in(current_locale(), key)
}

/// Build the status bar pane count, e.g. "2 panes".
pub fn status_panes(count: usize) -> String {
    one_or_many(Key::StatusPaneOne, Key::TplStatusPanes, count as u64)
}

/// Build the status bar session count, e.g. "3 sessions".
pub fn status_sessions(count: usize) -> String {
    one_or_many(Key::StatusSessionOne, Key::TplStatusSessions, count as u64)
}

/// Build the Agents mode status bar count of projects, e.g. "7 projects".
pub fn agents_status_projects(count: usize) -> String {
    one_or_many(
        Key::AgentsStatusOneProject,
        Key::TplAgentsStatusProjects,
        count as u64,
    )
}

/// Build the Agents mode status bar count of agent sessions, e.g. "3
/// agent sessions".
pub fn agents_status_sessions(count: usize) -> String {
    one_or_many(
        Key::AgentsStatusOneSession,
        Key::TplAgentsStatusSessions,
        count as u64,
    )
}

/// Build the status bar counter of the selected command block: the block
/// `number` of `count`, e.g. "Block 5 of 6".
pub fn status_block_place(number: usize, count: usize) -> String {
    status_block_place_in(current_locale(), number, count)
}

/// Build the note after the Pinned label of the All modes menu, e.g.
/// "1 of 4".
pub fn modes_pinned_note(count: usize, limit: usize) -> String {
    modes_pinned_note_in(current_locale(), count, limit)
}

/// Build a count: the text `one` for a single item, else the template
/// `many` with the count, e.g. "1 image" or "7 images".
///
/// # Examples
///
/// ```
/// use otty_app_i18n::{Key, Locale, one_or_many, set_locale};
///
/// set_locale(Locale::En);
///
/// let images = |count| {
///     one_or_many(
///         Key::ContainersStatusOneImage,
///         Key::TplContainersStatusImages,
///         count,
///     )
/// };
///
/// assert_eq!(images(1), "1 image");
/// assert_eq!(images(7), "7 images");
/// ```
pub fn one_or_many(one: Key, many: Key, count: u64) -> String {
    one_or_many_in(current_locale(), one, many, count)
}

/// Fill the `{count}` placeholder of the template behind `key`, e.g.
/// "2 running" or "3 h".
pub fn with_count(key: Key, count: u64) -> String {
    fill_in(current_locale(), key, &[("count", &format!("{count}"))])
}

/// Fill the `{error}` placeholder of the template behind `key`, e.g. "Could
/// not save the pinned modes: disk full".
pub fn with_error(key: Key, error: &str) -> String {
    fill_in(current_locale(), key, &[("error", error)])
}

/// Fill the named `{placeholders}` of the template behind `key` with their
/// values, e.g. the title "Remove container web-1?" from `{name}`.
///
/// # Examples
///
/// ```
/// use otty_app_i18n::{Key, Locale, filled, set_locale};
///
/// set_locale(Locale::En);
///
/// assert_eq!(
///     filled(Key::TplAgentsRemoveTitle, &[("name", "Coder")]),
///     "Remove Coder?"
/// );
/// ```
pub fn filled(key: Key, values: &[(&str, &str)]) -> String {
    fill_in(current_locale(), key, values)
}

/// Build the badge word of a container that exited with `code`, e.g.
/// "Exited (3)".
pub fn containers_exited(code: i64) -> String {
    fill_in(
        current_locale(),
        Key::TplContainersStateExitedCode,
        &[("code", &format!("{code}"))],
    )
}

/// Build the count of log lines that wait while Follow is off, e.g.
/// "1 new line" or "12 new lines".
pub fn containers_logs_new_lines(count: usize) -> String {
    containers_logs_new_lines_in(current_locale(), count)
}

/// Build the note under the search field of the projects while text is
/// typed: `shown` of the `total` projects match, such as "2 of 7 projects
/// match".
pub fn agents_projects_match(shown: usize, total: usize) -> String {
    agents_projects_count(Key::TplAgentsProjectsMatch, shown, total)
}

/// Build the note under the empty search field of the projects while a
/// filter is on, such as "4 of 7 projects shown".
pub fn agents_projects_shown(shown: usize, total: usize) -> String {
    agents_projects_count(Key::TplAgentsProjectsShown, shown, total)
}

/// The name of the day of the week `weekday`, from 1 for Monday to 7 for
/// Sunday, as a reset is written with it, such as "Thursday"; empty out of
/// that range.
///
/// # Examples
///
/// ```
/// use otty_app_i18n::{Locale, agents_weekday, set_locale};
///
/// set_locale(Locale::En);
///
/// assert_eq!(agents_weekday(4), "Thursday");
/// ```
pub fn agents_weekday(weekday: i8) -> &'static str {
    let key = usize::try_from(weekday)
        .ok()
        .and_then(|number| number.checked_sub(1))
        .and_then(|index| WEEKDAY_KEYS.get(index));

    key.map_or("", |key| t(*key))
}

/// The short name of the month `month`, from 1 for January to 12, as a
/// reset date is written with it, such as "Nov"; empty out of that range.
///
/// # Examples
///
/// ```
/// use otty_app_i18n::{Locale, agents_month, set_locale};
///
/// set_locale(Locale::En);
///
/// assert_eq!(agents_month(11), "Nov");
/// ```
pub fn agents_month(month: i8) -> &'static str {
    let key = usize::try_from(month)
        .ok()
        .and_then(|number| number.checked_sub(1))
        .and_then(|index| MONTH_KEYS.get(index));

    key.map_or("", |key| t(*key))
}

/// Build the banner shown on a kept-open pane once its process exited.
///
/// `exit_code` is `None` when the process was stopped by a signal.
pub fn process_exited(exit_code: Option<i32>) -> String {
    process_exited_in(current_locale(), exit_code)
}

/// Build the status of a block whose command ended with `code`, e.g.
/// "exit 101".
pub fn block_exit_code(code: i32) -> String {
    block_exit_code_in(current_locale(), code)
}

/// Build the badge after the status of a folded block that hides `count`
/// lines of output, e.g. "20 lines hidden".
pub fn block_lines_hidden(count: usize) -> String {
    block_lines_hidden_in(current_locale(), count)
}

/// Build the note in the sticky header of a block that has `count` lines
/// above the pane, e.g. "412 lines above".
pub fn block_lines_above(count: usize) -> String {
    block_lines_above_in(current_locale(), count)
}

/// Return the translation of `key` in `locale`.
///
/// Falls back to English when a key is absent, so a partial catalog degrades
/// to readable text instead of blank UI.
pub fn text_in(locale: Locale, key: Key) -> &'static str {
    if let Some(value) = catalog(locale).strings.get(&key) {
        return value.as_str();
    }

    catalog(Locale::En)
        .strings
        .get(&key)
        .map_or("", String::as_str)
}

fn process_exited_in(locale: Locale, exit_code: Option<i32>) -> String {
    let Some(code) = exit_code else {
        return text_in(locale, Key::ProcessTerminated).to_string();
    };

    fill_in(
        locale,
        Key::TplProcessExited,
        &[("code", &format!("{code}"))],
    )
}

fn block_exit_code_in(locale: Locale, code: i32) -> String {
    fill_in(
        locale,
        Key::TplBlockExitCode,
        &[("code", &format!("{code}"))],
    )
}

fn block_lines_hidden_in(locale: Locale, count: usize) -> String {
    one_or_many_in(
        locale,
        Key::BlockOneLineHidden,
        Key::TplBlockLinesHidden,
        count as u64,
    )
}

fn block_lines_above_in(locale: Locale, count: usize) -> String {
    one_or_many_in(
        locale,
        Key::BlockOneLineAbove,
        Key::TplBlockLinesAbove,
        count as u64,
    )
}

fn containers_logs_new_lines_in(locale: Locale, count: usize) -> String {
    one_or_many_in(
        locale,
        Key::ContainersLogsOneNewLine,
        Key::TplContainersLogsNewLines,
        count as u64,
    )
}

fn status_block_place_in(
    locale: Locale,
    number: usize,
    count: usize,
) -> String {
    fill_in(
        locale,
        Key::TplStatusBlockPlace,
        &[
            ("number", &format!("{number}")),
            ("count", &format!("{count}")),
        ],
    )
}

fn modes_pinned_note_in(locale: Locale, count: usize, limit: usize) -> String {
    fill_in(
        locale,
        Key::TplModesPinnedNote,
        &[
            ("count", &format!("{count}")),
            ("limit", &format!("{limit}")),
        ],
    )
}

/// Fill the `{shown}` and `{total}` placeholders of the template behind
/// `key` in the active locale.
fn agents_projects_count(key: Key, shown: usize, total: usize) -> String {
    let shown = shown.to_string();
    let total = total.to_string();

    fill_in(
        current_locale(),
        key,
        &[("shown", &shown), ("total", &total)],
    )
}

/// `one` in `locale` for a single item, else the template `many` filled
/// with `count`, e.g. "1 project" or "7 projects".
fn one_or_many_in(locale: Locale, one: Key, many: Key, count: u64) -> String {
    if count == 1 {
        return text_in(locale, one).to_string();
    }

    fill_in(locale, many, &[("count", &format!("{count}"))])
}

/// Substitute `{name}` placeholders in the `locale` template behind `key`.
fn fill_in(locale: Locale, key: Key, arguments: &[(&str, &str)]) -> String {
    let template = text_in(locale, key);
    let mut remaining = template;
    let mut text = String::with_capacity(template.len());

    while let Some((before, after_open)) = remaining.split_once('{') {
        text.push_str(before);

        let Some((name, after_close)) = after_open.split_once('}') else {
            text.push('{');
            text.push_str(after_open);
            return text;
        };

        if let Some((_, value)) =
            arguments.iter().find(|(argument, _)| *argument == name)
        {
            text.push_str(value);
        } else {
            text.push('{');
            text.push_str(name);
            text.push('}');
        }

        remaining = after_close;
    }

    text.push_str(remaining);

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
    use std::collections::BTreeSet;

    use super::{
        Catalog, Key, Locale, MONTH_KEYS, WEEKDAY_KEYS, agents_month,
        agents_status_projects, agents_weekday, block_exit_code_in,
        block_lines_above_in, block_lines_hidden_in, catalog,
        containers_logs_new_lines_in, fill_in, locale_from_system_preference,
        modes_pinned_note_in, process_exited_in, status_block_place_in,
        system_locale, text_in, text_locale_tag,
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

    /// Owner, 2026-11-22: sentences of the UI words take commas or a new
    /// sentence, never a semicolon, also not the full-width one of Chinese
    /// and Japanese.
    #[test]
    fn no_text_of_any_locale_holds_a_semicolon() {
        for locale in Locale::ALL {
            for key in Key::ALL {
                let text = text_in(locale, key);

                assert!(
                    !text.contains([';', '；']),
                    "{} {key:?}: {text}",
                    locale.tag()
                );
            }
        }
    }

    #[test]
    fn given_chinese_tags_when_parsed_then_script_selects_the_variant() {
        assert_eq!(Locale::from_tag("zh_CN.UTF-8"), Locale::ZhCn);
        assert_eq!(Locale::from_tag("zh-Hans"), Locale::ZhCn);
        assert_eq!(Locale::from_tag("zh-Hans_TW"), Locale::ZhCn);
        assert_eq!(Locale::from_tag("zh"), Locale::ZhCn);
        assert_eq!(Locale::from_tag("ZH-TW"), Locale::ZhTw);
        assert_eq!(Locale::from_tag("zh_HK"), Locale::ZhTw);
        assert_eq!(Locale::from_tag("zh-Hant"), Locale::ZhTw);
        assert_eq!(Locale::from_tag("zh-Hant_CN"), Locale::ZhTw);
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
    fn given_platform_preference_when_system_locale_resolved_then_platform_value_is_used()
     {
        assert_eq!(
            locale_from_system_preference(Some("zh-Hans_TW")),
            Locale::ZhCn
        );
        assert_eq!(locale_from_system_preference(None), Locale::En);
    }

    /// The system language is the one the computer prefers, read the way
    /// the pure resolution expects it.
    #[test]
    fn the_system_language_is_the_preference_of_the_computer() {
        let preference = sys_locale::get_locale();

        assert_eq!(
            system_locale(),
            locale_from_system_preference(preference.as_deref())
        );
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

    /// The mode is named after its first list, in the words the tree uses.
    #[test]
    fn the_containers_mode_has_the_name_of_the_containers_list() {
        assert_eq!(text_in(Locale::En, Key::ModeContainers), "Containers");

        for locale in Locale::ALL {
            assert_eq!(
                text_in(locale, Key::ModeContainers),
                text_in(locale, Key::ContainersContainers),
                "{}",
                locale.tag()
            );
        }
    }

    #[test]
    fn the_texts_that_name_the_mode_use_its_new_name_in_every_locale() {
        for locale in Locale::ALL {
            let tag = locale.tag();
            let name = text_in(locale, Key::ModeContainers);

            for key in [
                Key::ContainersStatusMode,
                Key::PaletteSwitchToContainersMode,
            ] {
                let text = text_in(locale, key);

                assert!(text.contains(name), "{tag} {key:?}: {text}");
                assert!(!text.contains("Docker"), "{tag} {key:?}: {text}");
            }

            let about = text_in(locale, Key::ContainersNotInstalledText);
            assert!(about.contains(name), "{tag}: {about}");
        }
    }

    /// "Docker" and "Podman" still name the engines.
    #[test]
    fn the_texts_that_name_an_engine_keep_its_brand_in_every_locale() {
        for locale in Locale::ALL {
            let tag = locale.tag();

            for key in [
                Key::ContainersNotInstalledTitle,
                Key::ContainersNotInstalledText,
                Key::ContainersInstallDocker,
                Key::ContainersDockerStoppedTitle,
                Key::ContainersDockerStoppedText,
                Key::ContainersDockerNoAccessTitle,
                Key::ContainersPanelDockerStopped,
                Key::ContainersPanelDockerNoAccess,
                Key::ContainersBothOffTitle,
                Key::ContainersPanelBothOff,
                Key::ContainersStatusDockerStopped,
                Key::ContainersStatusBothOff,
            ] {
                let text = text_in(locale, key).to_lowercase();

                assert!(text.contains("docker"), "{tag} {key:?}: {text}");
            }
        }
    }

    /// The card of a denied socket with the other engine off speaks of both
    /// engines, or it reads as if only one were there. The status bar text
    /// names the socket by its path, so there it names the other engine.
    #[test]
    fn the_texts_of_a_denied_socket_with_the_other_off_name_both_engines() {
        for locale in Locale::ALL {
            let tag = locale.tag();

            for key in [
                Key::ContainersDockerNoAccessPodmanOffTitle,
                Key::TplContainersDockerNoAccessPodmanOffText,
                Key::ContainersPanelDockerNoAccessPodmanOff,
                Key::ContainersPodmanNoAccessDockerOffTitle,
                Key::TplContainersPodmanNoAccessDockerOffText,
                Key::ContainersPanelPodmanNoAccessDockerOff,
            ] {
                let text = text_in(locale, key).to_lowercase();

                assert!(text.contains("docker"), "{tag} {key:?}: {text}");
                assert!(text.contains("podman"), "{tag} {key:?}: {text}");
            }

            let podman_off = text_in(
                locale,
                Key::TplContainersStatusDockerNoAccessPodmanOff,
            );
            let docker_off = text_in(
                locale,
                Key::TplContainersStatusPodmanNoAccessDockerOff,
            );
            assert!(podman_off.to_lowercase().contains("podman"), "{tag}");
            assert!(docker_off.to_lowercase().contains("docker"), "{tag}");
        }
    }

    /// The texts of the kit's form fields come from the catalog, so a language
    /// must not show English there. German spells `NAME` the same.
    #[test]
    fn the_empty_env_row_and_its_cross_read_in_every_language() {
        assert_eq!(text_in(Locale::En, Key::EnvKeyPlaceholder), "NAME");
        assert_eq!(text_in(Locale::En, Key::EnvValuePlaceholder), "value");
        assert_eq!(
            text_in(Locale::En, Key::ButtonRemoveEnv),
            "Remove variable"
        );

        for locale in Locale::ALL {
            if locale == Locale::En {
                continue;
            }
            for key in [
                Key::EnvValuePlaceholder,
                Key::ButtonRemoveEnv,
                Key::ButtonRemove,
            ] {
                assert_ne!(
                    text_in(locale, key),
                    text_in(Locale::En, key),
                    "{locale:?}: {key:?}"
                );
            }
            if locale != Locale::De {
                assert_ne!(
                    text_in(locale, Key::EnvKeyPlaceholder),
                    "NAME",
                    "{locale:?}"
                );
            }
        }
    }

    #[test]
    fn an_app_that_writes_han_or_hangul_gives_the_font_system_its_own_language()
    {
        for (locale, tag) in [
            (Locale::Ja, "ja"),
            (Locale::Ko, "ko"),
            (Locale::ZhCn, "zh-CN"),
            (Locale::ZhTw, "zh-TW"),
        ] {
            assert_eq!(text_locale_tag(locale, Locale::En), tag);
            assert_eq!(text_locale_tag(locale, Locale::Ru), tag);
        }
    }

    /// Names in Han still come up in a program in English, and on a Japanese
    /// computer they are Japanese names.
    #[test]
    fn an_app_in_another_language_gives_the_font_system_the_computers_language()
    {
        for locale in [
            Locale::En,
            Locale::Fr,
            Locale::De,
            Locale::Es,
            Locale::PtBr,
            Locale::Ru,
        ] {
            assert_eq!(text_locale_tag(locale, Locale::Ja), "ja");
            assert_eq!(text_locale_tag(locale, Locale::ZhTw), "zh-TW");
            assert_eq!(text_locale_tag(locale, Locale::De), "de");
        }
    }

    /// German is the one language that spells the word as English does.
    #[test]
    fn given_the_optional_marker_then_each_language_has_its_own_word() {
        for locale in Locale::ALL {
            let is_same_word = matches!(locale, Locale::En | Locale::De);
            let marker = text_in(locale, Key::FieldOptional);

            assert_eq!(
                marker == "optional",
                is_same_word,
                "{} reads {marker}",
                locale.tag()
            );
        }
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
                Key::TplLaunchFailed,
                &[("title", "deploy"), ("error", "boom")]
            ),
            "Could not launch \"deploy\": boom"
        );
        assert_eq!(
            fill_in(
                Locale::En,
                Key::TplTerminalInitFailed,
                &[("error", "boom")]
            ),
            "Could not open a terminal: boom"
        );

        assert_eq!(
            fill_in(Locale::ZhCn, Key::TplEditTabTitle, &[("title", "deploy")]),
            "编辑：deploy"
        );
        assert_eq!(
            fill_in(
                Locale::ZhCn,
                Key::TplLaunchFailed,
                &[("title", "deploy"), ("error", "boom")]
            ),
            "无法启动“deploy”：boom"
        );
    }

    #[test]
    fn given_exit_code_when_describing_exit_then_code_is_shown() {
        assert_eq!(
            process_exited_in(Locale::En, Some(2)),
            "Process exited with code 2. Press any key to close."
        );
        assert_eq!(
            process_exited_in(Locale::Ru, Some(0)),
            "Процесс завершился с кодом 0. Нажмите любую клавишу, чтобы закрыть."
        );
    }

    #[test]
    fn the_exit_code_of_a_block_is_filled_in_every_locale() {
        assert_eq!(block_exit_code_in(Locale::En, 101), "exit 101");
        for locale in Locale::ALL {
            let text = block_exit_code_in(locale, 101);

            assert!(text.contains("101"), "{}: {text}", locale.tag());
            assert!(!text.contains("{code}"), "{}: {text}", locale.tag());
        }
    }

    #[test]
    fn the_hidden_lines_of_a_folded_block_are_counted_with_one_and_many_forms()
    {
        assert_eq!(block_lines_hidden_in(Locale::En, 20), "20 lines hidden");
        assert_eq!(block_lines_hidden_in(Locale::En, 1), "1 line hidden");
        for locale in Locale::ALL {
            let many = block_lines_hidden_in(locale, 20);
            let one = block_lines_hidden_in(locale, 1);

            assert!(many.contains("20"), "{}: {many}", locale.tag());
            assert!(!many.contains("{count}"), "{}: {many}", locale.tag());
            assert!(one.contains('1'), "{}: {one}", locale.tag());
        }
    }

    #[test]
    fn the_lines_above_the_pane_in_the_sticky_header_are_counted_with_one_and_many_forms()
     {
        assert_eq!(block_lines_above_in(Locale::En, 412), "412 lines above");
        assert_eq!(block_lines_above_in(Locale::En, 1), "1 line above");
        for locale in Locale::ALL {
            let many = block_lines_above_in(locale, 412);
            let one = block_lines_above_in(locale, 1);

            assert!(many.contains("412"), "{}: {many}", locale.tag());
            assert!(!many.contains("{count}"), "{}: {many}", locale.tag());
            assert!(one.contains('1'), "{}: {one}", locale.tag());
        }
    }

    #[test]
    fn the_block_counter_shows_the_place_and_the_count_in_every_locale() {
        assert_eq!(status_block_place_in(Locale::En, 5, 6), "Block 5 of 6");
        for locale in Locale::ALL {
            let counter = status_block_place_in(locale, 5, 6);

            assert!(counter.contains('5'), "{}: {counter}", locale.tag());
            assert!(counter.contains('6'), "{}: {counter}", locale.tag());
            assert!(!counter.contains('{'), "{}: {counter}", locale.tag());
        }
    }

    #[test]
    fn the_looked_for_line_names_path_and_the_install_folders_in_every_locale()
    {
        let expected = [
            (
                Locale::En,
                "OTTY looked for /run/docker.sock, and for docker and podman on PATH and in the usual install folders.",
            ),
            (
                Locale::De,
                "OTTY hat gesucht nach /run/docker.sock sowie nach docker und podman in PATH und in den üblichen Installationsordnern.",
            ),
            (
                Locale::Es,
                "OTTY buscó /run/docker.sock, y docker y podman en PATH y en las carpetas de instalación habituales.",
            ),
            (
                Locale::Fr,
                "OTTY a cherché /run/docker.sock, ainsi que docker et podman dans le PATH et dans les dossiers d'installation habituels.",
            ),
            (
                Locale::Ja,
                "OTTY は /run/docker.sock と、PATH 上と一般的なインストール先フォルダーにある docker と podman を探しました。",
            ),
            (
                Locale::Ko,
                "OTTY는 /run/docker.sock와(과) PATH 및 일반적인 설치 폴더의 docker, podman을 찾았습니다.",
            ),
            (
                Locale::PtBr,
                "O OTTY procurou /run/docker.sock, e docker e podman no PATH e nas pastas de instalação habituais.",
            ),
            (
                Locale::Ru,
                "OTTY искал /run/docker.sock, а также docker и podman в PATH и в обычных папках установки.",
            ),
            (
                Locale::ZhCn,
                "OTTY 查找了 /run/docker.sock，以及 PATH 和常用安装文件夹中的 docker 和 podman。",
            ),
            (
                Locale::ZhTw,
                "OTTY 已尋找 /run/docker.sock，以及 PATH 與常用安裝資料夾中的 docker 與 podman。",
            ),
        ];
        assert_eq!(expected.len(), Locale::ALL.len());

        for (locale, text) in expected {
            let filled = fill_in(
                locale,
                Key::TplContainersLookedFor,
                &[("places", "/run/docker.sock")],
            );

            assert_eq!(filled, text, "{}", locale.tag());
        }
    }

    #[test]
    fn a_container_that_exited_shows_its_code() {
        assert_eq!(
            fill_in(
                Locale::En,
                Key::TplContainersStateExitedCode,
                &[("code", "3")]
            ),
            "Exited (3)"
        );
    }

    #[test]
    fn the_limits_say_where_they_were_recorded() {
        assert_eq!(
            fill_in(
                Locale::En,
                Key::TplAgentsLimitsRecorded,
                &[("time", "30 min ago")]
            ),
            "Recorded in a session 30 min ago"
        );
    }

    /// One project is written in the singular; any other count fills the
    /// template.
    #[test]
    fn the_agents_status_bar_counts_one_project_in_the_singular() {
        let locale = super::current_locale();

        assert_eq!(
            agents_status_projects(1),
            text_in(locale, Key::AgentsStatusOneProject)
        );
        assert_eq!(
            agents_status_projects(7),
            fill_in(locale, Key::TplAgentsStatusProjects, &[("count", "7")])
        );
        assert_eq!(
            agents_status_projects(0),
            fill_in(locale, Key::TplAgentsStatusProjects, &[("count", "0")])
        );
    }

    #[test]
    fn the_left_panel_of_agents_mode_counts_projects_and_amounts() {
        let counts = [("shown", "2"), ("total", "7")];

        assert_eq!(
            fill_in(Locale::En, Key::TplAgentsProjectsMatch, &counts),
            "2 of 7 projects match"
        );
        assert_eq!(
            fill_in(
                Locale::En,
                Key::TplAgentsAmountToday,
                &[("amount", "$12.40")]
            ),
            "$12.40 today"
        );
        assert_eq!(
            fill_in(
                Locale::En,
                Key::TplAgentsAmountToday,
                &[("amount", "1.8M")]
            ),
            "1.8M today"
        );
    }

    #[test]
    fn a_containers_action_failure_names_the_row_and_the_error() {
        assert_eq!(
            fill_in(
                Locale::En,
                Key::TplContainersStopFailed,
                &[("name", "web-1"), ("error", "busy")]
            ),
            "Could not stop web-1: busy"
        );
    }

    #[test]
    fn a_notification_ends_with_its_error() {
        assert_eq!(
            fill_in(
                Locale::En,
                Key::TplPinnedModesSaveFailed,
                &[("error", "disk full")]
            ),
            "Could not save the pinned modes: disk full"
        );
    }

    #[test]
    fn the_removal_of_a_profile_names_it_and_its_folder() {
        assert_eq!(
            fill_in(
                Locale::En,
                Key::TplAgentsRemoveTitle,
                &[("name", "Coder · Work")]
            ),
            "Remove Coder · Work?"
        );
        assert_eq!(
            fill_in(
                Locale::En,
                Key::TplAgentsRemoveText,
                &[("path", "/home/user/.coder-work")]
            ),
            "OTTY forgets this profile. The folder /home/user/.coder-work and \
             its sessions stay on disk."
        );
    }

    /// A reset is written with its exact time, in English as the spec
    /// shows it (spec Decision 10).
    #[test]
    fn a_reset_reads_as_the_spec_writes_it_in_english() {
        let filled = |key: Key, arguments: &[(&str, &str)]| {
            fill_in(Locale::En, key, arguments)
        };

        assert_eq!(text_in(Locale::En, Key::AgentsUsage), "Usage");
        assert_eq!(
            filled(Key::TplAgentsUpdated, &[("time", "2 min ago")]),
            "Updated 2 min ago"
        );
        assert_eq!(
            filled(
                Key::TplAgentsResetsInAt,
                &[("duration", "2 h 10 min"), ("time", "15:30")]
            ),
            "Resets in 2 h 10 min, at 15:30"
        );
        assert_eq!(
            filled(
                Key::TplAgentsResetsOnDay,
                &[("day", "Thursday"), ("time", "09:00")]
            ),
            "Resets on Thursday at 09:00"
        );
        assert_eq!(
            filled(
                Key::TplAgentsResetsOnDate,
                &[("date", "Nov 23"), ("time", "09:00")]
            ),
            "Resets on Nov 23 at 09:00"
        );
        assert_eq!(
            filled(Key::TplAgentsMonthDay, &[("month", "Nov"), ("day", "23")]),
            "Nov 23"
        );
        assert_eq!(
            filled(Key::TplAgentsTimeUtc, &[("time", "15:30")]),
            "15:30 UTC"
        );
    }

    /// Every language has its own seven day names and twelve month names,
    /// so a reset is written in the user's language.
    #[test]
    fn every_locale_names_seven_days_and_twelve_months_each_differently() {
        for locale in Locale::ALL {
            for names in [&WEEKDAY_KEYS[..], &MONTH_KEYS[..]] {
                let mut words: Vec<&str> =
                    names.iter().map(|key| text_in(locale, *key)).collect();
                words.sort_unstable();
                words.dedup();

                assert_eq!(words.len(), names.len(), "{}", locale.tag());
                assert!(
                    words.iter().all(|word| !word.trim().is_empty()),
                    "{}",
                    locale.tag()
                );
            }
        }
        assert_eq!(text_in(Locale::En, WEEKDAY_KEYS[3]), "Thursday");
        assert_eq!(text_in(Locale::En, MONTH_KEYS[10]), "Nov");
        assert_eq!(
            agents_weekday(1),
            text_in(super::current_locale(), WEEKDAY_KEYS[0])
        );
        assert_eq!(
            agents_weekday(7),
            text_in(super::current_locale(), WEEKDAY_KEYS[6])
        );
        assert_eq!(
            agents_month(1),
            text_in(super::current_locale(), MONTH_KEYS[0])
        );
        assert_eq!(
            agents_month(12),
            text_in(super::current_locale(), MONTH_KEYS[11])
        );
        assert_eq!(agents_weekday(0), "", "a day out of range has no name");
        assert_eq!(agents_month(13), "", "a month out of range has no name");
    }

    /// The sources of the list prices have plain names, as the owner
    /// approved them on 2026-11-18 (spec Decisions 10 and 18): a failure
    /// names them.
    #[test]
    fn every_price_source_has_a_name_a_person_reads() {
        let names = [
            (Key::AgentsSourceLitellmPrices, "LiteLLM prices"),
            (Key::AgentsSourceKeptPrices, "LiteLLM prices cache"),
        ];

        for (key, english) in names {
            assert_eq!(text_in(Locale::En, key), english, "{key:?}");
        }
    }

    /// The blocks of an account are named after what they show, and money
    /// estimated at the list prices says so with a short mark (issue 19 of
    /// spec 2026-11-19-agent-plugins).
    #[test]
    fn every_number_has_a_name_and_an_estimate_its_mark() {
        let texts = [
            (Key::AgentsNumberLimits, "Limits"),
            (Key::AgentsTokens, "Tokens"),
            (Key::AgentsNumberMoney, "Money"),
            (Key::AgentsEstimateMark, "estimate, list prices"),
        ];

        for (key, english) in texts {
            assert_eq!(text_in(Locale::En, key), english, "{key:?}");
        }
    }

    /// The profiles of Agents mode are called profiles, and the config
    /// folder of a new one is a typed path or a new folder; the mode keeps
    /// its name (spec Decisions 10 and 11).
    #[test]
    fn the_profiles_are_called_profiles_and_the_mode_keeps_its_name() {
        let names = [
            (Key::AgentsSectionProfiles, "Profiles"),
            (Key::AgentsAddProfile, "Add profile"),
            (Key::AgentsAddProfile, "Add profile"),
            (Key::AgentsEditProfileTitle, "Edit profile"),
            (Key::AgentsFolderTyped, "Type a path"),
            (Key::AgentsFolderCreate, "New folder"),
            (Key::ModeAgents, "Agents"),
        ];

        for (key, english) in names {
            assert_eq!(text_in(Locale::En, key), english, "{key:?}");
        }
        assert_eq!(text_in(Locale::Ru, Key::AgentsSectionProfiles), "Профили");
        assert_eq!(text_in(Locale::ZhCn, Key::AgentsSectionProfiles), "配置");
        assert_eq!(text_in(Locale::ZhTw, Key::AgentsSectionProfiles), "設定檔");
    }

    /// A failed launch names the launch as well as the error, so the toast
    /// says which of several launches failed.
    #[test]
    fn a_launch_failure_names_the_launch_and_its_error() {
        assert_eq!(
            fill_in(
                Locale::En,
                Key::TplLaunchFailed,
                &[("title", "deploy"), ("error", "disk full")]
            ),
            "Could not launch \"deploy\": disk full"
        );
    }

    /// A folder that would not open is named, so the toast says which of
    /// the pressed folders failed.
    #[test]
    fn a_folder_failure_names_the_folder_and_its_error() {
        assert_eq!(
            fill_in(
                Locale::En,
                Key::TplFolderOpenFailed,
                &[("name", "src"), ("error", "no access")]
            ),
            "Could not open the folder \"src\": no access"
        );
    }

    /// The delete question in Files names the entry, so the user sees what
    /// goes.
    #[test]
    fn the_delete_question_of_files_names_the_entry() {
        assert_eq!(
            fill_in(Locale::En, Key::TplFilesDeleteTitle, &[("name", "src")]),
            "Delete \"src\"?"
        );
    }

    #[test]
    fn waiting_log_lines_are_counted_with_one_and_many_forms() {
        assert_eq!(containers_logs_new_lines_in(Locale::En, 1), "1 new line");
        assert_eq!(
            containers_logs_new_lines_in(Locale::En, 12),
            "12 new lines"
        );
        assert_eq!(
            containers_logs_new_lines_in(Locale::Ru, 1),
            "1 новая строка"
        );
        assert_eq!(
            containers_logs_new_lines_in(Locale::Ru, 12),
            "Новых строк: 12"
        );
    }

    #[test]
    fn the_skipped_output_line_is_one_translated_line_in_every_locale() {
        let english = text_in(Locale::En, Key::ContainersLogsSkippedOutput);

        assert_eq!(english, "… some output was skipped");
        for locale in Locale::ALL {
            let text = text_in(locale, Key::ContainersLogsSkippedOutput);

            assert!(text.starts_with('…'), "{}: {text}", locale.tag());
            assert!(!text.contains('\n'), "{}: {text}", locale.tag());
            if locale != Locale::En {
                assert_ne!(text, english, "{}", locale.tag());
            }
        }
    }

    #[test]
    fn given_pinned_count_and_limit_when_building_the_pinned_note_then_both_show()
     {
        assert_eq!(modes_pinned_note_in(Locale::En, 1, 4), "1 of 4");
        assert_eq!(modes_pinned_note_in(Locale::En, 0, 4), "0 of 4");
    }

    #[test]
    fn given_no_exit_code_when_describing_exit_then_termination_is_shown() {
        assert_eq!(
            process_exited_in(Locale::En, None),
            "Process was terminated. Press any key to close."
        );
    }

    #[test]
    fn given_placeholder_text_in_argument_when_filled_then_argument_remains_verbatim()
     {
        assert_eq!(
            fill_in(
                Locale::En,
                Key::TplLaunchFailed,
                &[("title", "deploy {error}"), ("error", "boom")]
            ),
            "Could not launch \"deploy {error}\": boom"
        );
    }

    #[test]
    fn given_quick_launch_diagnostics_when_translated_then_visible_text_is_localized()
     {
        assert_eq!(
            text_in(Locale::ZhCn, Key::ErrSshPortPositive),
            "SSH 端口必须大于 0。"
        );
        assert_eq!(
            text_in(Locale::ZhCn, Key::ErrMissingTargetFolder),
            "找不到目标文件夹。"
        );
        assert_eq!(
            fill_in(
                Locale::ZhCn,
                Key::TplWorkingDirectoryNotFound,
                &[("path", "/tmp/project")]
            ),
            "找不到工作目录：/tmp/project"
        );
        assert_eq!(
            fill_in(
                Locale::ZhCn,
                Key::TplProgramNotExecutable,
                &[("program", "deploy")]
            ),
            "程序不可执行：deploy"
        );
    }

    /// Every language has the placeholders of English in each text, so no
    /// translation drops, adds or renames one.
    #[test]
    fn every_locale_has_the_placeholders_of_english_in_each_text() {
        for key in Key::ALL {
            let english = placeholders(text_in(Locale::En, key));

            for locale in Locale::ALL {
                let text = text_in(locale, key);

                assert_eq!(
                    placeholders(text),
                    english,
                    "{} {key:?}: {text}",
                    locale.tag()
                );
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

    /// The names of the `{placeholders}` of `text`.
    fn placeholders(text: &str) -> BTreeSet<&str> {
        text.split('{')
            .skip(1)
            .filter_map(|part| part.split_once('}'))
            .map(|(name, _)| name)
            .collect()
    }
}

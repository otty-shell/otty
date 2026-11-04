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

/// Active locale, encoded as [`Locale::index`].
static CURRENT_LOCALE: AtomicU8 = AtomicU8::new(Locale::En.index() as u8);

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

/// Detect the preferred locale through the current platform's native source.
pub(crate) fn detect_system_locale() -> Locale {
    let preference = sys_locale::get_locale();
    locale_from_system_preference(preference.as_deref())
}

/// Return the translation of `key` in the active locale.
pub(crate) fn t(key: Key) -> &'static str {
    text_in(current_locale(), key)
}

/// Build the status bar pane count, e.g. "2 panes".
pub(crate) fn status_panes(count: usize) -> String {
    if count == 1 {
        return t(Key::StatusPaneOne).to_string();
    }

    let count = count.to_string();
    fill_in(current_locale(), Key::TplStatusPanes, &[("count", &count)])
}

/// Build the status bar session count, e.g. "3 sessions".
pub(crate) fn status_sessions(count: usize) -> String {
    if count == 1 {
        return t(Key::StatusSessionOne).to_string();
    }

    let count = count.to_string();
    fill_in(
        current_locale(),
        Key::TplStatusSessions,
        &[("count", &count)],
    )
}

/// Build the note after the Pinned label of the All modes menu, e.g.
/// "1 of 4".
pub(crate) fn modes_pinned_note(count: usize, limit: usize) -> String {
    modes_pinned_note_in(current_locale(), count, limit)
}

/// Fill the `{count}` placeholder of the template behind `key`, e.g.
/// "2 running" or "3 h".
pub(crate) fn with_count(key: Key, count: u64) -> String {
    fill_in(current_locale(), key, &[("count", &format!("{count}"))])
}

/// Build the badge word of a container that exited with `code`, e.g.
/// "Exited (3)".
pub(crate) fn docker_exited(code: i64) -> String {
    fill_in(
        current_locale(),
        Key::TplDockerStateExitedCode,
        &[("code", &format!("{code}"))],
    )
}

/// Build the text of the no access card for the denied `socket`.
pub(crate) fn docker_no_access_text(socket: &str) -> String {
    fill_in(
        current_locale(),
        Key::TplDockerNoAccessText,
        &[("socket", socket)],
    )
}

/// Build the text of the Podman no access card for the denied `socket`.
pub(crate) fn docker_podman_no_access_text(socket: &str) -> String {
    fill_in(
        current_locale(),
        Key::TplDockerPodmanNoAccessText,
        &[("socket", socket)],
    )
}

/// Build the text of the card for a denied Docker `socket` with Podman off.
pub(crate) fn docker_no_access_podman_off_text(socket: &str) -> String {
    fill_in(
        current_locale(),
        Key::TplDockerNoAccessPodmanOffText,
        &[("socket", socket)],
    )
}

/// Build the text of the card for a denied Podman `socket` with Docker off.
pub(crate) fn docker_podman_no_access_docker_off_text(socket: &str) -> String {
    fill_in(
        current_locale(),
        Key::TplDockerPodmanNoAccessDockerOffText,
        &[("socket", socket)],
    )
}

/// Build the line under the no engine cards from the `places` OTTY tried.
pub(crate) fn docker_looked_for(places: &str) -> String {
    fill_in(
        current_locale(),
        Key::TplDockerLookedFor,
        &[("places", places)],
    )
}

/// Build the status bar text for a denied `socket`.
pub(crate) fn docker_status_no_access(socket: &str) -> String {
    fill_in(
        current_locale(),
        Key::TplDockerStatusNoAccess,
        &[("socket", socket)],
    )
}

/// Build the status bar text for a denied Docker `socket` with Podman off.
pub(crate) fn docker_status_no_access_podman_off(socket: &str) -> String {
    fill_in(
        current_locale(),
        Key::TplDockerStatusNoAccessPodmanOff,
        &[("socket", socket)],
    )
}

/// Build the status bar text for a denied Podman `socket` with Docker off.
pub(crate) fn docker_status_podman_no_access_docker_off(
    socket: &str,
) -> String {
    fill_in(
        current_locale(),
        Key::TplDockerStatusPodmanNoAccessDockerOff,
        &[("socket", socket)],
    )
}

/// Build a status bar count: `one` for a single item, else `many` with the
/// count, e.g. "1 image" or "7 images".
pub(crate) fn docker_status_count(one: Key, many: Key, count: usize) -> String {
    if count == 1 {
        return t(one).to_string();
    }

    with_count(many, count as u64)
}

/// Build the notice shown above a table whose list failed with `error`.
pub(crate) fn docker_list_failed(error: &str) -> String {
    fill_in(
        current_locale(),
        Key::TplDockerListFailed,
        &[("error", error)],
    )
}

/// Build the notice shown above the logs when they could not be read.
pub(crate) fn docker_logs_failed(error: &str) -> String {
    fill_in(
        current_locale(),
        Key::TplDockerLogsFailed,
        &[("error", error)],
    )
}

/// Build the count of log lines that wait while Follow is off, e.g.
/// "1 new line" or "12 new lines".
pub(crate) fn docker_logs_new_lines(count: usize) -> String {
    docker_logs_new_lines_in(current_locale(), count)
}

/// Build the notice shown in the Inspect tab when the JSON did not load.
pub(crate) fn docker_inspect_failed(error: &str) -> String {
    fill_in(
        current_locale(),
        Key::TplDockerInspectFailed,
        &[("error", error)],
    )
}

/// Build the inspector status of a volume used by `names`.
pub(crate) fn docker_used_by(names: &str) -> String {
    fill_in(current_locale(), Key::TplDockerUsedBy, &[("names", names)])
}

/// Fill the `{name}` placeholder of a Docker template, e.g. the title
/// "Remove container web-1?".
pub(crate) fn docker_named(key: Key, name: &str) -> String {
    fill_in(current_locale(), key, &[("name", name)])
}

/// Build the notice of a failed Docker action from its template, the row
/// `name` and the engine's `error`, e.g. "Could not stop web-1: busy".
pub(crate) fn docker_action_failed(
    key: Key,
    name: &str,
    error: &str,
) -> String {
    fill_in(current_locale(), key, &[("name", name), ("error", error)])
}

/// Build the hint under the disabled Remove of a volume used by `names`.
pub(crate) fn docker_volume_in_use(names: &str) -> String {
    fill_in(
        current_locale(),
        Key::TplDockerVolumeInUseHint,
        &[("names", names)],
    )
}

/// Fill the one `{placeholder}` of a Docker template with `value`, e.g.
/// "Starts in the background on Docker 29.9.1." from `TplDockerRunReady`.
pub(crate) fn docker_filled(
    key: Key,
    placeholder: &str,
    value: &str,
) -> String {
    fill_in(current_locale(), key, &[(placeholder, value)])
}

/// Build the progress text of a running pull of `image`.
pub(crate) fn docker_pulling(image: &str) -> String {
    fill_in(current_locale(), Key::TplDockerPulling, &[("image", image)])
}

/// Build the bytes of a running pull, e.g. "15.0 MB of 47.0 MB".
pub(crate) fn docker_pull_amount(done: &str, total: &str) -> String {
    fill_in(
        current_locale(),
        Key::TplDockerPullAmount,
        &[("done", done), ("total", total)],
    )
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

/// Build the message shown when an SSH connection cannot be established.
pub(crate) fn ssh_connection_failed(error: &str) -> String {
    fill_in(
        current_locale(),
        Key::TplSshConnectionFailed,
        &[("error", error)],
    )
}

/// Build the message shown when a working directory cannot be found.
pub(crate) fn working_directory_not_found(path: &str) -> String {
    fill_in(
        current_locale(),
        Key::TplWorkingDirectoryNotFound,
        &[("path", path)],
    )
}

/// Build the message shown when a working path is not a directory.
pub(crate) fn working_directory_not_directory(path: &str) -> String {
    fill_in(
        current_locale(),
        Key::TplWorkingDirectoryNotDirectory,
        &[("path", path)],
    )
}

/// Build the message shown when an SSH identity file cannot be found.
pub(crate) fn identity_file_not_found(path: &str) -> String {
    fill_in(
        current_locale(),
        Key::TplIdentityFileNotFound,
        &[("path", path)],
    )
}

/// Build the message shown when an SSH identity path is not a file.
pub(crate) fn identity_file_not_file(path: &str) -> String {
    fill_in(
        current_locale(),
        Key::TplIdentityFileNotFile,
        &[("path", path)],
    )
}

/// Build the SSH secret dialog title asking for a key passphrase.
pub(crate) fn secret_prompt_passphrase_title(path: &str) -> String {
    fill_in(
        current_locale(),
        Key::TplSecretPromptPassphraseTitle,
        &[("path", path)],
    )
}

/// Build the SSH secret dialog title asking for a password.
pub(crate) fn secret_prompt_password_title(target: &str) -> String {
    fill_in(
        current_locale(),
        Key::TplSecretPromptPasswordTitle,
        &[("target", target)],
    )
}

/// Build the muted line explaining why a key passphrase is needed.
pub(crate) fn secret_prompt_passphrase_hint(
    launch: &str,
    account: &str,
) -> String {
    fill_in(
        current_locale(),
        Key::TplSecretPromptPassphraseHint,
        &[("launch", launch), ("account", account)],
    )
}

/// Build the muted line explaining why a password is needed.
pub(crate) fn secret_prompt_password_hint(
    launch: &str,
    account: &str,
) -> String {
    fill_in(
        current_locale(),
        Key::TplSecretPromptPasswordHint,
        &[("launch", launch), ("account", account)],
    )
}

/// Build the message shown when a program cannot be found in `PATH`.
pub(crate) fn program_not_found_in_path(program: &str) -> String {
    fill_in(
        current_locale(),
        Key::TplProgramNotFoundInPath,
        &[("program", program)],
    )
}

/// Build the message shown when an explicit program path cannot be found.
pub(crate) fn program_not_found(program: &str) -> String {
    fill_in(
        current_locale(),
        Key::TplProgramNotFound,
        &[("program", program)],
    )
}

/// Build the message shown when a program path is a directory.
pub(crate) fn program_is_directory(program: &str) -> String {
    fill_in(
        current_locale(),
        Key::TplProgramIsDirectory,
        &[("program", program)],
    )
}

/// Build the message shown when a program path is not executable.
pub(crate) fn program_not_executable(program: &str) -> String {
    fill_in(
        current_locale(),
        Key::TplProgramNotExecutable,
        &[("program", program)],
    )
}

/// Build the banner shown on a kept-open pane once its process exited.
///
/// `exit_code` is `None` when the process was stopped by a signal.
pub(crate) fn process_exited(exit_code: Option<i32>) -> String {
    process_exited_in(current_locale(), exit_code)
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

fn docker_logs_new_lines_in(locale: Locale, count: usize) -> String {
    if count == 1 {
        return text_in(locale, Key::DockerLogsOneNewLine).to_string();
    }

    fill_in(
        locale,
        Key::TplDockerLogsNewLines,
        &[("count", &format!("{count}"))],
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

/// Return the translation of `key` in `locale`.
///
/// Falls back to English when a key is absent, so a partial catalog degrades
/// to readable text instead of blank UI.
pub(crate) fn text_in(locale: Locale, key: Key) -> &'static str {
    if let Some(value) = catalog(locale).strings.get(&key) {
        return value.as_str();
    }

    catalog(Locale::En)
        .strings
        .get(&key)
        .map_or("", String::as_str)
}

fn locale_from_system_preference(preference: Option<&str>) -> Locale {
    preference.map_or(Locale::En, Locale::from_tag)
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
    use otty_ui_kit::components::switch::Switch;
    use otty_ui_kit::palette::UiPalette;
    use otty_ui_kit::{size, spacing};

    use super::{
        Catalog, Key, Locale, catalog, docker_logs_new_lines_in, fill_in,
        locale_from_system_preference, modes_pinned_note_in, process_exited_in,
        text_in,
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
        assert_eq!(text_in(Locale::En, Key::ModeDocker), "Containers");

        for locale in Locale::ALL {
            assert_eq!(
                text_in(locale, Key::ModeDocker),
                text_in(locale, Key::DockerContainers),
                "{}",
                locale.tag()
            );
        }
    }

    #[test]
    fn the_texts_that_name_the_mode_use_its_new_name_in_every_locale() {
        for locale in Locale::ALL {
            let tag = locale.tag();
            let name = text_in(locale, Key::ModeDocker);

            for key in [Key::DockerStatusMode, Key::PaletteSwitchToDockerMode] {
                let text = text_in(locale, key);

                assert!(text.contains(name), "{tag} {key:?}: {text}");
                assert!(!text.contains("Docker"), "{tag} {key:?}: {text}");
            }

            let about = text_in(locale, Key::DockerNotInstalledText);
            assert!(about.contains(name), "{tag}: {about}");
        }
    }

    /// "Docker" and "Podman" still name the engines.
    #[test]
    fn the_texts_that_name_an_engine_keep_its_brand_in_every_locale() {
        for locale in Locale::ALL {
            let tag = locale.tag();

            for key in [
                Key::DockerSearching,
                Key::DockerNotInstalledTitle,
                Key::DockerNotInstalledText,
                Key::DockerInstallDocker,
                Key::DockerStoppedTitle,
                Key::DockerStoppedText,
                Key::DockerNoAccessTitle,
                Key::DockerPanelStopped,
                Key::DockerPanelNoAccess,
                Key::DockerBothOffTitle,
                Key::DockerPanelBothOff,
                Key::DockerStatusStopped,
                Key::DockerStatusBothOff,
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
                Key::DockerNoAccessPodmanOffTitle,
                Key::TplDockerNoAccessPodmanOffText,
                Key::DockerPanelNoAccessPodmanOff,
                Key::DockerPodmanNoAccessDockerOffTitle,
                Key::TplDockerPodmanNoAccessDockerOffText,
                Key::DockerPanelPodmanNoAccessDockerOff,
            ] {
                let text = text_in(locale, key).to_lowercase();

                assert!(text.contains("docker"), "{tag} {key:?}: {text}");
                assert!(text.contains("podman"), "{tag} {key:?}: {text}");
            }

            let podman_off =
                text_in(locale, Key::TplDockerStatusNoAccessPodmanOff);
            let docker_off =
                text_in(locale, Key::TplDockerStatusPodmanNoAccessDockerOff);
            assert!(podman_off.to_lowercase().contains("podman"), "{tag}");
            assert!(docker_off.to_lowercase().contains("docker"), "{tag}");
        }
    }

    /// A dialog switch stays on one line, so its label has to fit whole: it
    /// says what a removal will force. The Chinese, Japanese and Korean labels
    /// are short, and a test machine may have no font to measure them with.
    #[test]
    fn given_a_confirm_dialog_switch_then_its_label_fits_the_dialog() {
        let palette = UiPalette::otty_dark();
        let room = size::DIALOG - 2.0 * spacing::SPACE_5;

        for locale in [
            Locale::En,
            Locale::Fr,
            Locale::De,
            Locale::Es,
            Locale::PtBr,
            Locale::Ru,
        ] {
            for key in [Key::DockerStopItFirst, Key::DockerForceRemoveImage] {
                let label = text_in(locale, key);
                let switch: Switch<'_, ()> =
                    Switch::new(true, &palette).label(label);

                assert!(
                    switch.width() <= room,
                    "{label} in {} is {} wide, the dialog has {room}",
                    locale.tag(),
                    switch.width(),
                );
            }
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
    fn given_modes_pinned_note_when_filled_then_every_locale_shows_both_numbers()
     {
        for locale in Locale::ALL {
            let note = fill_in(
                locale,
                Key::TplModesPinnedNote,
                &[("count", "1"), ("limit", "4")],
            );

            assert!(
                note.contains('1') && note.contains('4') && !note.contains('{'),
                "{} pinned note is {note:?}",
                locale.tag()
            );
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
                Key::TplDockerLookedFor,
                &[("places", "/run/docker.sock")],
            );

            assert_eq!(filled, text, "{}", locale.tag());
        }
    }

    #[test]
    fn docker_templates_fill_their_placeholders_in_every_locale() {
        let fills = [
            (Key::TplDockerRunning, ("count", "2")),
            (Key::TplDockerAgeHours, ("count", "3")),
            (Key::TplDockerStateExitedCode, ("code", "137")),
            (Key::TplDockerListFailed, ("error", "boom")),
            (Key::TplDockerUsedBy, ("names", "db-1")),
            (Key::TplDockerStatusNoAccess, ("socket", "/run/docker.sock")),
            (
                Key::TplDockerNoAccessPodmanOffText,
                ("socket", "/run/docker.sock"),
            ),
            (
                Key::TplDockerPodmanNoAccessDockerOffText,
                ("socket", "/run/podman.sock"),
            ),
            (
                Key::TplDockerStatusNoAccessPodmanOff,
                ("socket", "/run/docker.sock"),
            ),
            (
                Key::TplDockerStatusPodmanNoAccessDockerOff,
                ("socket", "/run/podman.sock"),
            ),
            (Key::TplDockerStatusContainers, ("count", "7")),
            (Key::TplDockerStatusImages, ("count", "7")),
            (Key::TplDockerStatusVolumes, ("count", "7")),
            (Key::TplDockerStatusNetworks, ("count", "7")),
            (Key::TplDockerRemoveContainerTitle, ("name", "web-1")),
            (Key::TplDockerPruneContainersText, ("count", "3")),
            (Key::TplDockerRemoveImageTitle, ("name", "nginx:1.27")),
            (Key::TplDockerRemoveTagTitle, ("name", "nginx:1.27")),
            (Key::TplDockerRemoveVolumeTitle, ("name", "pgdata")),
            (Key::TplDockerRemoveNetworkTitle, ("name", "web_default")),
            (Key::TplDockerPruneImagesText, ("count", "3")),
            (Key::TplDockerPruneVolumesText, ("count", "3")),
            (Key::TplDockerPruneAnonymousVolumesText, ("count", "3")),
            (Key::TplDockerPruneNetworksText, ("count", "3")),
            (Key::TplDockerVolumeInUseHint, ("names", "db-1")),
            (Key::TplDockerPulling, ("image", "nginx:latest")),
            (Key::TplDockerLogsNewLines, ("count", "12")),
            (Key::TplDockerRunCommandHint, ("command", "nginx -g")),
            (Key::TplDockerRunImageListens, ("ports", "80/tcp")),
            (Key::TplDockerRunReady, ("engine", "Docker 29.9.1")),
            (Key::TplDockerImageHere, ("size", "192.0 MB")),
            (Key::TplDockerImageLookupFailed, ("error", "boom")),
        ];

        for locale in Locale::ALL {
            for (key, (name, value)) in fills {
                let text = fill_in(locale, key, &[(name, value)]);

                assert!(
                    text.contains(value) && !text.contains('{'),
                    "{} {key:?}: {text}",
                    locale.tag()
                );
            }
        }
        assert_eq!(
            fill_in(
                Locale::En,
                Key::TplDockerStateExitedCode,
                &[("code", "3")]
            ),
            "Exited (3)"
        );
    }

    #[test]
    fn docker_action_failures_name_the_row_and_the_error_in_every_locale() {
        let failures = [
            Key::TplDockerStartFailed,
            Key::TplDockerStopFailed,
            Key::TplDockerRestartFailed,
            Key::TplDockerPauseFailed,
            Key::TplDockerUnpauseFailed,
            Key::TplDockerRemoveContainerFailed,
            Key::TplDockerPullFailed,
            Key::TplDockerRemoveImageFailed,
            Key::TplDockerRemoveVolumeFailed,
            Key::TplDockerCreateNetworkFailed,
            Key::TplDockerRemoveNetworkFailed,
            Key::TplDockerRunFailed,
        ];
        let without_name = [
            Key::TplDockerPruneContainersFailed,
            Key::TplDockerPruneImagesFailed,
            Key::TplDockerCreateVolumeFailed,
            Key::TplDockerPruneVolumesFailed,
            Key::TplDockerPruneNetworksFailed,
        ];

        for locale in Locale::ALL {
            for key in failures {
                let text = fill_in(
                    locale,
                    key,
                    &[("name", "web-1"), ("error", "engine said no")],
                );

                assert!(
                    text.contains("web-1")
                        && text.contains("engine said no")
                        && !text.contains('{'),
                    "{} {key:?}: {text}",
                    locale.tag()
                );
            }
            for key in without_name {
                let text = fill_in(locale, key, &[("error", "engine said no")]);

                assert!(
                    text.contains("engine said no") && !text.contains('{'),
                    "{} {key:?}: {text}",
                    locale.tag()
                );
            }
            let amount = fill_in(
                locale,
                Key::TplDockerPullAmount,
                &[("done", "15.0 MB"), ("total", "47.0 MB")],
            );
            assert!(
                amount.contains("15.0 MB")
                    && amount.contains("47.0 MB")
                    && !amount.contains('{'),
                "{} amount: {amount}",
                locale.tag()
            );
        }
        assert_eq!(
            fill_in(
                Locale::En,
                Key::TplDockerStopFailed,
                &[("name", "web-1"), ("error", "busy")]
            ),
            "Could not stop web-1: busy"
        );
    }

    #[test]
    fn waiting_log_lines_are_counted_with_one_and_many_forms() {
        assert_eq!(docker_logs_new_lines_in(Locale::En, 1), "1 new line");
        assert_eq!(docker_logs_new_lines_in(Locale::En, 12), "12 new lines");
        assert_eq!(docker_logs_new_lines_in(Locale::Ru, 1), "1 новая строка");
        assert_eq!(docker_logs_new_lines_in(Locale::Ru, 12), "Новых строк: 12");
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
                Key::TplLaunchFailedBody,
                &[("command", "deploy {error}"), ("error", "boom")]
            ),
            "Command: deploy {error}\nError: boom"
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

    #[test]
    fn given_every_locale_when_templates_read_then_placeholders_survive() {
        let required: [(Key, &[&str]); 15] = [
            (Key::TplEditTabTitle, &["{title}"]),
            (Key::TplLaunchFailedTitle, &["{title}"]),
            (Key::TplLaunchFailedBody, &["{command}", "{error}"]),
            (Key::TplTerminalInitFailed, &["{error}"]),
            (Key::TplSshConnectionFailed, &["{error}"]),
            (Key::TplWorkingDirectoryNotFound, &["{path}"]),
            (Key::TplWorkingDirectoryNotDirectory, &["{path}"]),
            (Key::TplIdentityFileNotFound, &["{path}"]),
            (Key::TplIdentityFileNotFile, &["{path}"]),
            (Key::TplProgramNotFoundInPath, &["{program}"]),
            (Key::TplProgramNotFound, &["{program}"]),
            (Key::TplProgramIsDirectory, &["{program}"]),
            (Key::TplProgramNotExecutable, &["{program}"]),
            (Key::TplStatusPanes, &["{count}"]),
            (Key::TplStatusSessions, &["{count}"]),
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

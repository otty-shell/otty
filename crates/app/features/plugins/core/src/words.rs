//! The words of Plugins mode in the user's language: each reason an
//! install or an update did not happen, the kind of a part and how far a
//! plugin is trusted. Text that came from outside, such as the system's
//! reason a file could not be written, is put in as it came. The
//! permissions of a part are places in groups (`model::permissions`).

use otty_app_i18n::{self as i18n, Key};
use otty_plugin_manifest::{PartKind, PartRef};
use otty_plugin_releases::{InstallError, Trust};

/// `error` in the user's language.
pub(crate) fn error_text(error: &InstallError) -> String {
    match error {
        InstallError::BadUrl(url) => {
            i18n::filled(Key::TplPluginsErrorBadUrl, &[("url", url)])
        },
        InstallError::RateLimited => {
            i18n::t(Key::PluginsErrorRateLimited).to_string()
        },
        InstallError::RepositoryNotFound(url) => {
            i18n::filled(Key::TplPluginsErrorNotFound, &[("url", url)])
        },
        InstallError::Network(reason) => {
            i18n::filled(Key::TplPluginsErrorNetwork, &[("error", reason)])
        },
        InstallError::UnexpectedAnswer(reason) => {
            i18n::filled(Key::TplPluginsErrorUnexpected, &[("error", reason)])
        },
        InstallError::NoRelease => {
            i18n::t(Key::PluginsErrorNoRelease).to_string()
        },
        InstallError::NeedsNewerOtty(reason) => needs_newer_otty(reason),
        InstallError::Manifest(error) => i18n::filled(
            Key::TplPluginsErrorManifest,
            &[("error", &error.to_string())],
        ),
        InstallError::VersionIsNotTag { tag, version } => i18n::filled(
            Key::TplPluginsErrorVersionIsNotTag,
            &[("tag", tag), ("version", version)],
        ),
        InstallError::MissingFile(name) => {
            i18n::filled(Key::TplPluginsErrorMissingFile, &[("name", name)])
        },
        InstallError::Download { name, reason } => i18n::filled(
            Key::TplPluginsErrorDownload,
            &[("name", name), ("error", reason)],
        ),
        InstallError::TooLarge { name, limit } => i18n::filled(
            Key::TplPluginsErrorTooLarge,
            &[("name", name), ("limit", &limit.to_string())],
        ),
        InstallError::BadIcon { name, problem } => i18n::filled(
            Key::TplPluginsErrorBadIcon,
            &[("name", name), ("error", &problem.to_string())],
        ),
        InstallError::IdTaken { id, origin } => i18n::filled(
            Key::TplPluginsErrorIdTaken,
            &[("id", id), ("origin", origin)],
        ),
        InstallError::AlreadyInstalled(id) => {
            i18n::filled(Key::TplPluginsErrorAlreadyInstalled, &[("id", id)])
        },
        InstallError::ReleaseChanged { tag, name } => i18n::filled(
            Key::TplPluginsErrorReleaseChanged,
            &[("tag", tag), ("name", name)],
        ),
        InstallError::OtherPlugin { expected, found } => i18n::filled(
            Key::TplPluginsErrorOtherPlugin,
            &[("expected", expected), ("found", found)],
        ),
        InstallError::Compile(reason) => {
            i18n::filled(Key::TplPluginsErrorCompile, &[("error", reason)])
        },
        InstallError::NotInstalled(id) => {
            i18n::filled(Key::TplPluginsErrorNotInstalled, &[("id", id)])
        },
        InstallError::NoSuchPart { plugin, part } => i18n::filled(
            Key::TplPluginsErrorNoSuchPart,
            &[("id", plugin), ("part", part.id())],
        ),
        InstallError::NotLocal(id) => {
            i18n::filled(Key::TplPluginsErrorNotLocal, &[("id", id)])
        },
        _ => error.to_string(),
    }
}

/// "Needs a newer OTTY (agents.agent contract 0.2)", from the English reason
/// the manifest gives, such as "Needs a newer OTTY (agents.agent contract 0.2)".
pub(crate) fn needs_newer_otty(reason: &str) -> String {
    let detail = reason
        .split_once('(')
        .and_then(|(_, rest)| rest.split_once(')'))
        .map(|(detail, _)| detail);

    match detail {
        Some(detail) => i18n::filled(
            Key::TplPluginsErrorNeedsNewerOtty,
            &[("detail", detail)],
        ),
        None => i18n::t(Key::PluginsNeedsNewerOtty).to_string(),
    }
}

/// The kind of `part` as its tag shows it: "Agent" or "Provider".
pub(crate) fn kind_text(part: &PartRef) -> &'static str {
    match part.kind() {
        PartKind::Agent => i18n::t(Key::PluginsPartAgent),
        PartKind::Provider => i18n::t(Key::PluginsPartProvider),
    }
}

/// How far a plugin is trusted, as its row marks it: "Official",
/// "Community" or "Local".
pub(crate) fn trust_text(trust: Trust) -> &'static str {
    match trust {
        Trust::Official => i18n::t(Key::PluginsOfficial),
        Trust::Community => i18n::t(Key::PluginsCommunity),
        Trust::Local => i18n::t(Key::PluginsLocal),
    }
}

/// The line of the install dialog about `trust`.
pub(crate) fn trust_note(trust: Trust) -> &'static str {
    match trust {
        Trust::Official => i18n::t(Key::PluginsOfficialNote),
        Trust::Community => i18n::t(Key::PluginsCommunityNote),
        Trust::Local => i18n::t(Key::PluginsLocalNote),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn needs_a_newer_otty_names_the_contract() {
        let text =
            needs_newer_otty("Needs a newer OTTY (agents.agent contract 0.2)");

        assert_eq!(
            text,
            i18n::filled(
                Key::TplPluginsErrorNeedsNewerOtty,
                &[("detail", "agents.agent contract 0.2")]
            )
        );
    }

    /// Issue 35: the card of a part tags its kind.
    #[test]
    fn each_kind_of_part_has_its_tag() {
        assert_eq!(
            kind_text(&PartRef::agent("coder")),
            i18n::t(Key::PluginsPartAgent)
        );
        assert_eq!(
            kind_text(&PartRef::provider("coder")),
            i18n::t(Key::PluginsPartProvider)
        );
    }

    #[test]
    fn each_trust_has_its_mark() {
        assert_eq!(trust_text(Trust::Official), i18n::t(Key::PluginsOfficial));
        assert_eq!(
            trust_text(Trust::Community),
            i18n::t(Key::PluginsCommunity)
        );
        assert_eq!(trust_text(Trust::Local), i18n::t(Key::PluginsLocal));
    }
}

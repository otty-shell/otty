//! The words of Plugins mode in the user's language: each permission of a
//! part, each reason an install or an update did not happen, the kind of
//! a part and how far a plugin is trusted. Text that came from outside,
//! such as the system's reason a file could not be written, is put in as
//! it came.

use otty_app_i18n::{self as i18n, Key};
use otty_plugin_manifest::{
    Domain, Manifest, PartKind, PartRef, PermissionLine,
};
use otty_plugin_releases::{InstallError, Trust};

/// `line` in the user's language, such as "Reads files in
/// `~/.coder*/**`".
pub(crate) fn permission_text(line: &PermissionLine) -> String {
    match line {
        PermissionLine::ReadsFiles(glob) => {
            i18n::filled(Key::TplPluginsPermReadsFiles, &[("glob", glob)])
        },
        PermissionLine::WritesFiles(glob) => {
            i18n::filled(Key::TplPluginsPermWritesFiles, &[("glob", glob)])
        },
        PermissionLine::QueriesDatabases(glob) => {
            i18n::filled(Key::TplPluginsPermQueriesDatabases, &[("glob", glob)])
        },
        PermissionLine::ReadsVariable(name) => {
            i18n::filled(Key::TplPluginsPermReadsVariable, &[("name", name)])
        },
        PermissionLine::RunsProgram(command) => i18n::filled(
            Key::TplPluginsPermRunsProgram,
            &[("command", command)],
        ),
        PermissionLine::StartsHiddenTerminal(command) => i18n::filled(
            Key::TplPluginsPermHiddenTerminal,
            &[("command", command)],
        ),
        PermissionLine::ReadsKeyring { service, user } => i18n::filled(
            Key::TplPluginsPermReadsKeyring,
            &[("service", service), ("user", user)],
        ),
        PermissionLine::ReadsLoginsInFiles(glob) => i18n::filled(
            Key::TplPluginsPermReadsLoginsInFiles,
            &[("glob", glob)],
        ),
        PermissionLine::ReadsLoginsInDatabase {
            databases,
            table,
            column,
        } => i18n::filled(
            Key::TplPluginsPermReadsLoginsInDatabase,
            &[
                ("databases", databases),
                ("table", table),
                ("column", column),
            ],
        ),
        PermissionLine::ReadsKeyInVariable(name) => i18n::filled(
            Key::TplPluginsPermReadsKeyInVariable,
            &[("name", name)],
        ),
        PermissionLine::SendsRequests(host) => {
            i18n::filled(Key::TplPluginsPermSendsRequests, &[("host", host)])
        },
        PermissionLine::GetsProviderKeys(provider) => i18n::filled(
            Key::TplPluginsPermGetsProviderKeys,
            &[("provider", provider)],
        ),
    }
}

/// The permissions of the part `part` of `manifest` in the user's
/// language: its own lines and, for a provider part, each host whose keys
/// it gets.
pub(crate) fn part_permissions(
    manifest: &Manifest,
    part: &PartRef,
) -> Vec<String> {
    match part.kind() {
        PartKind::Agent => manifest
            .agents()
            .iter()
            .find(|agent| agent.id() == part.id())
            .map(|agent| {
                agent
                    .permission_lines()
                    .iter()
                    .map(permission_text)
                    .collect()
            })
            .unwrap_or_default(),
        PartKind::Provider => {
            let Some(provider) = manifest
                .providers()
                .iter()
                .find(|provider| provider.id() == part.id())
            else {
                return Vec::new();
            };

            let mut lines: Vec<String> = provider
                .permission_lines()
                .iter()
                .map(permission_text)
                .collect();
            lines.extend(provider.hosts().iter().map(|host| host_keys(host)));

            lines
        },
    }
}

/// A line of the permissions the user granted, as `plugins.json` keeps it
/// in English, in the user's language: the same line of `manifest` when
/// it has it, else as it was kept.
pub(crate) fn granted_text(line: &str, manifest: Option<&Manifest>) -> String {
    let Some(manifest) = manifest else {
        return line.to_string();
    };

    for part in manifest.parts() {
        let typed = typed_lines(manifest, &part);
        if let Some((_, text)) =
            typed.into_iter().find(|(english, _)| english == line)
        {
            return text;
        }
    }

    line.to_string()
}

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

/// The kind of `part`: "agent" or "provider".
pub(crate) fn kind_text(part: &PartRef) -> &'static str {
    match part.kind() {
        PartKind::Agent => i18n::t(Key::PluginsPartAgent),
        PartKind::Provider => i18n::t(Key::PluginsPartProvider),
    }
}

/// The domain of `part` as a plain word, such as "Agents", after the mode
/// of OTTY the domain fills (issue 21).
pub(crate) fn domain_text(part: &PartRef) -> &'static str {
    match part.kind().domain() {
        Domain::Agents => i18n::t(Key::ModeAgents),
    }
}

/// A part with its domain and kind, such as "Coder (Agents: agent)".
pub(crate) fn part_title(name: &str, part: &PartRef) -> String {
    i18n::filled(
        Key::TplPluginsPart,
        &[
            ("name", name),
            ("domain", domain_text(part)),
            ("kind", kind_text(part)),
        ],
    )
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

/// "Gets the keys found for `example.com`".
fn host_keys(host: &str) -> String {
    i18n::filled(Key::TplPluginsPermGetsHostKeys, &[("host", host)])
}

/// Each permission of `part` of `manifest` as `plugins.json` keeps it, in
/// English, with its text in the user's language.
fn typed_lines(manifest: &Manifest, part: &PartRef) -> Vec<(String, String)> {
    let lines = match part.kind() {
        PartKind::Agent => manifest
            .agents()
            .iter()
            .find(|agent| agent.id() == part.id())
            .map(|agent| agent.permission_lines())
            .unwrap_or_default(),
        PartKind::Provider => manifest
            .providers()
            .iter()
            .find(|provider| provider.id() == part.id())
            .map(|provider| provider.permission_lines())
            .unwrap_or_default(),
    };
    let mut typed: Vec<(String, String)> = lines
        .iter()
        .map(|line| (line.to_string(), permission_text(line)))
        .collect();

    if part.kind() == PartKind::Provider
        && let Some(provider) = manifest
            .providers()
            .iter()
            .find(|provider| provider.id() == part.id())
    {
        typed.extend(provider.hosts().iter().map(|host| {
            (format!("Gets the keys found for `{host}`"), host_keys(host))
        }));
    }

    typed
}

#[cfg(test)]
mod tests {
    use otty_plugin_releases::fixtures::example_manifest;

    use super::*;

    fn manifest() -> Manifest {
        Manifest::parse(&example_manifest("0.1.0")).expect("valid")
    }

    #[test]
    fn a_provider_part_lists_its_requests_and_the_hosts_of_its_keys() {
        let lines =
            part_permissions(&manifest(), &PartRef::provider("example-cloud"));

        assert_eq!(
            lines,
            [
                i18n::filled(
                    Key::TplPluginsPermSendsRequests,
                    &[("host", "api.example.com")]
                ),
                i18n::filled(
                    Key::TplPluginsPermGetsProviderKeys,
                    &[("provider", "Example Cloud")]
                ),
                i18n::filled(
                    Key::TplPluginsPermGetsHostKeys,
                    &[("host", "example.com")]
                ),
            ]
        );
    }

    /// The lines `plugins.json` keeps in English are shown in the user's
    /// language when the manifest has them; any other stays as kept.
    #[test]
    fn a_granted_line_is_written_in_the_users_language() {
        let manifest = manifest();

        let known = granted_text(
            "Sends requests to `api.example.com`",
            Some(&manifest),
        );
        let unknown =
            granted_text("Reads files in `~/.old/**`", Some(&manifest));

        assert_eq!(
            known,
            i18n::filled(
                Key::TplPluginsPermSendsRequests,
                &[("host", "api.example.com")]
            )
        );
        assert_eq!(unknown, "Reads files in `~/.old/**`");
    }

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

    /// Issue 21: the list of a plugin's parts names the domain of each,
    /// after the mode of OTTY it fills.
    #[test]
    fn a_part_title_names_its_domain_and_kind() {
        let title = part_title("Coder", &PartRef::agent("coder"));

        assert_eq!(
            title,
            i18n::filled(
                Key::TplPluginsPart,
                &[
                    ("name", "Coder"),
                    ("domain", i18n::t(Key::ModeAgents)),
                    ("kind", i18n::t(Key::PluginsPartAgent)),
                ]
            )
        );
        assert!(title.contains(i18n::t(Key::ModeAgents)), "{title}");
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

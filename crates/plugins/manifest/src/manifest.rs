//! The plugin manifest, `otty-plugin.toml` (spec 2026-11-19-agent-plugins,
//! "What a plugin is"): the plugin, the contracts it uses, and its parts,
//! each with its own permissions. It is checked as a whole when read.

mod error;
pub(crate) mod raw;

use std::collections::BTreeSet;

use crate::agent_program::{ProgramInstall, ProgramPlace};
use crate::contract::Api;
pub use crate::manifest::error::ManifestError;
use crate::manifest::raw::{RawAgent, RawManifest, RawPlugin, RawProvider};
use crate::part::{PartKind, PartRef, is_valid_id};
use crate::permissions::{
    LoginRule, PartPermissions, PermissionLine, check_host,
};
use crate::version::PluginVersion;

/// A checked manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    plugin: PluginInfo,
    api: Api,
    agents: Vec<AgentPart>,
    providers: Vec<ProviderPart>,
}

impl Manifest {
    /// `[plugin]`.
    pub fn plugin(&self) -> &PluginInfo {
        &self.plugin
    }

    /// `[api]`: the version of each contract the plugin exports.
    pub fn api(&self) -> &Api {
        &self.api
    }

    /// The `[[agents.agent]]` parts.
    pub fn agents(&self) -> &[AgentPart] {
        &self.agents
    }

    /// The `[[agents.provider]]` parts.
    pub fn providers(&self) -> &[ProviderPart] {
        &self.providers
    }

    /// Every part, agents first.
    pub fn parts(&self) -> Vec<PartRef> {
        let agents = self.agents.iter().map(|part| PartRef::agent(part.id()));
        let providers = self
            .providers
            .iter()
            .map(|part| PartRef::provider(part.id()));

        agents.chain(providers).collect()
    }

    /// The permissions of the part `part`, if the manifest lists it.
    pub fn permissions_of(&self, part: &PartRef) -> Option<&PartPermissions> {
        match part.kind() {
            PartKind::Agent => self
                .agents
                .iter()
                .find(|agent| agent.id() == part.id())
                .map(AgentPart::permissions),
            PartKind::Provider => self
                .providers
                .iter()
                .find(|provider| provider.id() == part.id())
                .map(ProviderPart::permissions),
        }
    }

    /// The manifest in `text`, checked as a whole.
    ///
    /// ```
    /// use otty_plugin_manifest::Manifest;
    ///
    /// let manifest = Manifest::parse(
    ///     r#"
    ///     [plugin]
    ///     id = "example"
    ///     name = "Example"
    ///     version = "0.1.0"
    ///     wasm = "example.wasm"
    ///
    ///     [api]
    ///     agents.agent = "0.1"
    ///
    ///     [[agents.agent]]
    ///     id = "example-agent"
    ///     name = "Example agent"
    ///     icon = "example.svg"
    ///
    ///     [agents.agent.permissions]
    ///     read = ["~/.example-agent/**"]
    ///     "#,
    /// )
    /// .expect("a valid manifest");
    ///
    /// assert_eq!(manifest.plugin().id(), "example");
    /// assert_eq!(manifest.agents().len(), 1);
    /// ```
    pub fn parse(text: &str) -> Result<Self, ManifestError> {
        let raw: RawManifest = toml::from_str(text).map_err(|error| {
            ManifestError::Shape(error.message().to_string())
        })?;

        let plugin = PluginInfo::of_raw(&raw.plugin)?;
        let api = Api::of_manifest(&raw.api)?;
        let agents = raw
            .agents
            .agent
            .iter()
            .map(AgentPart::of_raw)
            .collect::<Result<Vec<_>, _>>()?;
        let providers = raw
            .agents
            .provider
            .iter()
            .map(|provider| ProviderPart::of_raw(provider, &agents))
            .collect::<Result<Vec<_>, _>>()?;

        let manifest = Self {
            plugin,
            api,
            agents,
            providers,
        };
        manifest.check_parts()?;

        Ok(manifest)
    }

    /// Checks the parts against each other and against `[api]`.
    fn check_parts(&self) -> Result<(), ManifestError> {
        let parts = self.parts();
        if parts.is_empty() {
            return Err(ManifestError::NoParts);
        }

        let mut seen = BTreeSet::new();
        for part in &parts {
            if !seen.insert((part.kind(), part.id())) {
                return Err(ManifestError::DuplicatePart {
                    kind: part.kind(),
                    id: part.id().to_string(),
                });
            }
            if self.api.version_of(part.kind().contract()).is_none() {
                return Err(ManifestError::PartWithoutContract(part.clone()));
            }
        }

        let part_kinds = self
            .api
            .iter()
            .filter_map(|(contract, _)| contract.part_kind());
        for kind in part_kinds {
            if !parts.iter().any(|part| part.kind() == kind) {
                return Err(ManifestError::ContractWithoutPart(kind));
            }
        }

        Ok(())
    }
}

/// `[plugin]`: what the plugin is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginInfo {
    id: String,
    name: String,
    version: PluginVersion,
    description: Option<String>,
    repository: Option<String>,
    license: Option<String>,
    authors: Vec<String>,
    wasm: String,
}

impl PluginInfo {
    /// The plugin id; a new id is a new plugin.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The name shown in Plugins mode.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The release version; it equals the release tag without `v`.
    pub fn version(&self) -> &PluginVersion {
        &self.version
    }

    /// One line on what the plugin reads.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// The plugin's repository.
    pub fn repository(&self) -> Option<&str> {
        self.repository.as_deref()
    }

    /// The plugin's licence.
    pub fn license(&self) -> Option<&str> {
        self.license.as_deref()
    }

    /// The plugin's authors.
    pub fn authors(&self) -> &[String] {
        &self.authors
    }

    /// The file name of the component, next to the manifest.
    pub fn wasm(&self) -> &str {
        &self.wasm
    }

    fn of_raw(raw: &RawPlugin) -> Result<Self, ManifestError> {
        if !is_valid_id(&raw.id) {
            return Err(ManifestError::BadPluginId(raw.id.clone()));
        }
        if raw.name.trim().is_empty() {
            return Err(ManifestError::MissingName("plugin".to_string()));
        }
        let Some(version) = PluginVersion::parse(&raw.version) else {
            return Err(ManifestError::BadVersion(raw.version.clone()));
        };
        if !is_plain_file_name(&raw.wasm, ".wasm") {
            return Err(ManifestError::BadWasmName(raw.wasm.clone()));
        }

        Ok(Self {
            id: raw.id.clone(),
            name: raw.name.clone(),
            version,
            description: raw.description.clone(),
            repository: raw.repository.clone(),
            license: raw.license.clone(),
            authors: raw.authors.clone(),
            wasm: raw.wasm.clone(),
        })
    }
}

/// One `[[agents.agent]]` part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentPart {
    id: String,
    name: String,
    description: Option<String>,
    icon: String,
    program: Option<String>,
    program_places: Vec<ProgramPlace>,
    tested_version: Option<String>,
    install: Option<ProgramInstall>,
    permissions: PartPermissions,
}

impl AgentPart {
    /// The agent id, saved in `agents.json`; it never changes.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The agent's name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// One sentence on what the part does, shown on its card in Plugins
    /// mode.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// The file name of the agent's SVG icon, next to the manifest.
    pub fn icon(&self) -> &str {
        &self.icon
    }

    /// The agent's program, looked up on `PATH` to tell whether it is
    /// installed.
    pub fn program(&self) -> Option<&str> {
        self.program.as_deref()
    }

    /// The places the agent's official installer puts its program, which
    /// OTTY checks when the login shell does not find it by its name.
    pub fn program_places(&self) -> &[ProgramPlace] {
        &self.program_places
    }

    /// How to install the agent's program, if the part says.
    pub fn install(&self) -> Option<&ProgramInstall> {
        self.install.as_ref()
    }

    /// The newest agent version this release was checked with.
    pub fn tested_version(&self) -> Option<&str> {
        self.tested_version.as_deref()
    }

    /// What this part may do.
    pub fn permissions(&self) -> &PartPermissions {
        &self.permissions
    }

    /// The part's permissions in plain words, as the install dialog shows
    /// them.
    pub fn permission_lines(&self) -> Vec<PermissionLine> {
        self.permissions.lines()
    }

    fn of_raw(raw: &RawAgent) -> Result<Self, ManifestError> {
        let part = PartRef::agent(&raw.id);
        check_part_id(&part)?;
        check_name(&raw.name, &part)?;
        let icon = checked_icon(raw.icon.as_deref(), &part)?;
        let program_places = raw
            .program_places
            .iter()
            .map(|place| {
                ProgramPlace::parse(place, &raw.permissions.env).map_err(
                    |problem| ManifestError::BadPart {
                        part: part.clone(),
                        reason: format!(
                            "has the program place `{place}`, which {problem}"
                        ),
                    },
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let install = raw
            .install
            .as_ref()
            .map(|install| {
                ProgramInstall::new(&install.command, install.page.as_deref())
            })
            .transpose()
            .map_err(|reason| ManifestError::BadPart {
                part: part.clone(),
                reason,
            })?;

        Ok(Self {
            id: raw.id.clone(),
            name: raw.name.clone(),
            description: raw.description.clone(),
            icon,
            program: raw.program.clone(),
            program_places,
            tested_version: raw.tested_version.clone(),
            install,
            permissions: PartPermissions::of_agent(&raw.permissions, &part)?,
        })
    }
}

/// One `[[agents.provider]]` part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderPart {
    id: String,
    name: String,
    description: Option<String>,
    icon: String,
    hosts: Vec<String>,
    usage_page: Option<String>,
    plan_usage_page: Option<String>,
    priced_as: Option<String>,
    permissions: PartPermissions,
}

impl ProviderPart {
    /// The provider id; provider ids and agent ids are separate lists.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The provider's name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// One sentence on what the part does, shown on its card in Plugins
    /// mode.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// The file name of the provider's SVG icon, next to the manifest.
    pub fn icon(&self) -> &str {
        &self.icon
    }

    /// The hosts of the provider: a key's host is one of these or ends
    /// with `.` and one of these.
    pub fn hosts(&self) -> &[String] {
        &self.hosts
    }

    /// The provider's usage page: of a key or an organization, and of a
    /// subscription when the part names no page of its plan.
    pub fn usage_page(&self) -> Option<&str> {
        self.usage_page.as_deref()
    }

    /// The usage page of a subscription's plan, such as ChatGPT's page of
    /// the Codex limits, when it is another page than [`Self::usage_page`].
    pub fn plan_usage_page(&self) -> Option<&str> {
        self.plan_usage_page.as_deref()
    }

    /// The name the LiteLLM price table knows the provider by, when it is
    /// not the id, such as `gemini` for the id `google`.
    pub fn priced_as(&self) -> Option<&str> {
        self.priced_as.as_deref()
    }

    /// What this part may do.
    pub fn permissions(&self) -> &PartPermissions {
        &self.permissions
    }

    /// The agent parts of the plugin whose profiles the part reads, with
    /// what it reads inside each profile folder.
    pub fn logins(&self) -> &[LoginRule] {
        self.permissions.logins()
    }

    /// The part's permissions in plain words, as the install dialog shows
    /// them, with the keys every provider part gets.
    pub fn permission_lines(&self) -> Vec<PermissionLine> {
        let mut lines = self.permissions.lines();
        lines.push(PermissionLine::GetsProviderKeys(self.name.clone()));

        lines
    }

    fn of_raw(
        raw: &RawProvider,
        agents: &[AgentPart],
    ) -> Result<Self, ManifestError> {
        let part = PartRef::provider(&raw.id);
        check_part_id(&part)?;
        check_name(&raw.name, &part)?;
        let icon = checked_icon(raw.icon.as_deref(), &part)?;
        if raw.hosts.is_empty() {
            return Err(ManifestError::BadPart {
                part,
                reason: "lists no host".to_string(),
            });
        }
        for host in &raw.hosts {
            check_host(host, &part)?;
        }
        for page in raw.usage_page.iter().chain(&raw.plan_usage_page) {
            if !page.starts_with("https://") {
                return Err(ManifestError::BadPart {
                    part,
                    reason: format!(
                        "has the usage page `{page}`, not an https:// link"
                    ),
                });
            }
        }
        if raw
            .priced_as
            .as_ref()
            .is_some_and(|name| name.trim().is_empty())
        {
            return Err(ManifestError::BadPart {
                part,
                reason: "has an empty `priced_as`".to_string(),
            });
        }
        Ok(Self {
            id: raw.id.clone(),
            name: raw.name.clone(),
            description: raw.description.clone(),
            icon,
            hosts: raw.hosts.clone(),
            usage_page: raw.usage_page.clone(),
            plan_usage_page: raw.plan_usage_page.clone(),
            priced_as: raw.priced_as.clone(),
            permissions: PartPermissions::of_provider(
                &raw.permissions,
                &raw.logins,
                agents,
                &part,
            )?,
        })
    }
}

fn check_part_id(part: &PartRef) -> Result<(), ManifestError> {
    if is_valid_id(part.id()) {
        return Ok(());
    }

    Err(ManifestError::BadPartId {
        kind: part.kind(),
        id: part.id().to_string(),
    })
}

fn check_name(name: &str, part: &PartRef) -> Result<(), ManifestError> {
    if name.trim().is_empty() {
        return Err(ManifestError::MissingName(part.to_string()));
    }

    Ok(())
}

fn checked_icon(
    icon: Option<&str>,
    part: &PartRef,
) -> Result<String, ManifestError> {
    let Some(icon) = icon.filter(|icon| !icon.is_empty()) else {
        return Err(ManifestError::MissingIcon(part.clone()));
    };
    if !is_plain_file_name(icon, ".svg") {
        return Err(ManifestError::BadIconName {
            part: part.clone(),
            icon: icon.to_string(),
        });
    }

    Ok(icon.to_string())
}

/// Whether `name` is a file name with no folder, not hidden, ending in
/// `extension`.
fn is_plain_file_name(name: &str, extension: &str) -> bool {
    name.len() > extension.len()
        && name.ends_with(extension)
        && !name.starts_with('.')
        && !name.contains(['/', '\\'])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_program::ProgramPlace;
    use crate::path_glob::{FolderGlob, GlobProblem};
    use crate::testing::{
        OPENAI_MANIFEST, OPENCODE_MANIFEST, edited, with_codex_logins,
    };

    fn refusal(text: &str) -> ManifestError {
        Manifest::parse(text).expect_err("refused")
    }

    /// Issue 36: each part may say in one sentence what it does; a part
    /// that says nothing has no description.
    #[test]
    fn a_part_may_say_what_it_does() {
        let text = edited(
            OPENAI_MANIFEST,
            r#"icon = "codex.svg""#,
            r#"icon = "codex.svg"
description = "Finds Codex profiles, sessions, tokens and logins.""#,
        );

        let provider = edited(
            OPENAI_MANIFEST,
            r#"icon = "openai.svg""#,
            r#"icon = "openai.svg"
description = "Reads ChatGPT limits and OpenAI API spending.""#,
        );

        let manifest = Manifest::parse(&text).expect("valid");
        let with_provider = Manifest::parse(&provider).expect("valid");

        assert_eq!(
            manifest.agents()[0].description(),
            Some("Finds Codex profiles, sessions, tokens and logins.")
        );
        assert_eq!(manifest.providers()[0].description(), None);
        assert_eq!(
            with_provider.providers()[0].description(),
            Some("Reads ChatGPT limits and OpenAI API spending.")
        );
        assert_eq!(with_provider.agents()[0].description(), None);
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decision 5: a
    /// provider part names the agent parts of its plugin whose profiles
    /// it reads, with what it reads inside each profile folder.
    #[test]
    fn a_provider_part_names_the_logins_it_reads_in_profiles() {
        let manifest = Manifest::parse(&with_codex_logins()).expect("valid");

        let logins = manifest.providers()[0].logins();

        assert_eq!(logins.len(), 1);
        assert_eq!(logins[0].agent(), "codex");
        assert_eq!(logins[0].agent_name(), "Codex");
        assert_eq!(logins[0].folder_env(), Some("CODEX_HOME"));
        let read: Vec<&str> =
            logins[0].read().iter().map(FolderGlob::as_str).collect();
        assert_eq!(read, ["sessions/**"]);
        let keys = &logins[0].keys()[0];
        let files: Vec<&str> =
            keys.files().iter().map(FolderGlob::as_str).collect();
        let fields: Vec<&str> =
            keys.fields().iter().map(|field| field.as_str()).collect();
        assert_eq!(files, ["auth.json"]);
        assert_eq!(fields, ["/OPENAI_API_KEY", "/tokens/access_token"]);
        assert!(
            Manifest::parse(OPENAI_MANIFEST).expect("valid").providers()[0]
                .logins()
                .is_empty()
        );
    }

    /// Decision 5: a provider part never reads the profiles of another
    /// plugin, so `agent` names an agent part of the same manifest.
    #[test]
    fn logins_of_no_agent_part_of_the_plugin_are_refused() {
        for agent in ["claude-code", "openai", ""] {
            let error = refusal(&edited(
                &with_codex_logins(),
                r#"agent = "codex""#,
                &format!(r#"agent = "{agent}""#),
            ));

            assert!(
                matches!(
                    &error,
                    ManifestError::BadPart { part, reason }
                        if *part == PartRef::provider("openai")
                            && reason.contains(&format!("`{agent}`"))
                ),
                "{agent}: {error:?}"
            );
        }
    }

    /// The paths of a `logins` entry are inside the profile folder: no
    /// leading `/` or `~`, no `..` and no variable.
    #[test]
    fn a_login_path_outside_the_profile_folder_is_refused() {
        let cases = [
            ("/etc/passwd", GlobProblem::NotInsideFolder),
            ("~/.codex/auth.json", GlobProblem::NotInsideFolder),
            ("", GlobProblem::NotInsideFolder),
            ("../.ssh/*", GlobProblem::ParentFolder),
            ("sessions/../../x", GlobProblem::ParentFolder),
            (
                "{CODEX_HOME}/auth.json",
                GlobProblem::VariableInside("CODEX_HOME".to_string()),
            ),
        ];

        for (path, problem) in cases {
            let read = refusal(&edited(
                &with_codex_logins(),
                r#"read = ["sessions/**"]"#,
                &format!(r#"read = ["{path}"]"#),
            ));
            let keys = refusal(&edited(
                &with_codex_logins(),
                r#"files = ["auth.json"]"#,
                &format!(r#"files = ["{path}"]"#),
            ));

            for error in [read, keys] {
                assert_eq!(
                    error,
                    ManifestError::BadGlob {
                        part: PartRef::provider("openai"),
                        glob: path.to_string(),
                        problem: problem.clone(),
                    },
                    "{path}"
                );
            }
        }
    }

    /// The key fields of a login file are JSON pointers, as in the `keys`
    /// of an agent part, and a key file names at least one.
    #[test]
    fn a_login_key_field_that_is_no_json_pointer_is_refused() {
        let not_pointer = refusal(&edited(
            &with_codex_logins(),
            r#"fields = ["/OPENAI_API_KEY", "/tokens/access_token"]"#,
            r#"fields = ["api_key"]"#,
        ));
        let none = refusal(&edited(
            &with_codex_logins(),
            r#"fields = ["/OPENAI_API_KEY", "/tokens/access_token"]"#,
            "fields = []",
        ));

        assert!(
            matches!(&not_pointer, ManifestError::BadPart { reason, .. } if reason.contains("JSON pointer")),
            "{not_pointer:?}"
        );
        assert!(
            matches!(&none, ManifestError::BadPart { reason, .. } if reason.contains("no `fields`")),
            "{none:?}"
        );
    }

    #[test]
    fn a_login_folder_variable_that_is_no_variable_name_is_refused() {
        let error = refusal(&edited(
            &with_codex_logins(),
            r#"folder_env = "CODEX_HOME""#,
            r#"folder_env = "CODEX-HOME""#,
        ));

        assert!(
            matches!(&error, ManifestError::BadPart { reason, .. } if reason.contains("`CODEX-HOME`")),
            "{error:?}"
        );
    }

    #[test]
    fn a_manifest_with_two_parts_is_read() {
        let manifest = Manifest::parse(OPENAI_MANIFEST).expect("valid");

        assert_eq!(manifest.plugin().id(), "openai");
        assert_eq!(manifest.plugin().version().to_string(), "0.4.0");
        assert_eq!(
            manifest.parts(),
            vec![PartRef::agent("codex"), PartRef::provider("openai"),]
        );
        assert_eq!(manifest.agents()[0].permissions().env(), ["CODEX_HOME"]);
        assert_eq!(
            manifest.providers()[0].permissions().http(),
            ["chatgpt.com", "api.openai.com"]
        );
    }

    /// Issue 29: a provider part reads an account any way it can, also by
    /// asking the agent in a profile the account was found in, so it may
    /// declare files, variables, programs and hidden terminals; keys,
    /// databases and the keyring stay the agent parts'.
    #[test]
    fn a_provider_part_may_declare_files_programs_and_terminals() {
        let with_ways = edited(
            OPENAI_MANIFEST,
            r#"http = ["chatgpt.com", "api.openai.com"]"#,
            r#"http = ["chatgpt.com", "api.openai.com"]
read = ["~/.codex*/sessions/**"]
write = ["~/.codex*/probe/*.jsonl"]
env = ["CODEX_HOME"]

[[agents.provider.permissions.run]]
program = "codex"
args = ["app-server"]

[[agents.provider.permissions.terminal]]
program = "codex"
args = ["--usage"]"#,
        );
        let with_keys = edited(
            OPENAI_MANIFEST,
            r#"http = ["chatgpt.com", "api.openai.com"]"#,
            r#"http = ["chatgpt.com"]
sqlite = ["~/.codex/state.db"]"#,
        );

        let manifest = Manifest::parse(&with_ways).expect("valid");
        let refused = refusal(&with_keys);

        let provider = manifest.providers()[0].permissions();
        let lines: Vec<String> =
            provider.lines().iter().map(ToString::to_string).collect();
        assert_eq!(
            lines,
            [
                "Reads files in `~/.codex*/sessions/**`",
                "Writes and deletes files in `~/.codex*/probe/*.jsonl`",
                "Reads the environment variable `CODEX_HOME`",
                "Runs `codex app-server`",
                "Runs `codex --usage` in a hidden terminal",
                "Sends requests to `chatgpt.com`",
                "Sends requests to `api.openai.com`",
            ]
        );
        assert!(
            matches!(&refused, ManifestError::Shape(text) if text.contains("unknown field")),
            "{refused:?}"
        );
    }

    /// Issue 21: the parts belong to the `agents` domain, so a part
    /// table at the top level is an unknown key.
    #[test]
    fn a_part_outside_its_domain_is_an_unknown_key() {
        let agent = refusal(
            &OPENAI_MANIFEST
                .replace("[[agents.agent", "[[agent")
                .replace("[agents.agent", "[agent"),
        );
        let provider = refusal(
            &OPENAI_MANIFEST
                .replace("[[agents.provider", "[[provider")
                .replace("[agents.provider", "[provider"),
        );

        for error in [agent, provider] {
            assert!(
                matches!(&error, ManifestError::Shape(text) if text.contains("unknown field")),
                "{error:?}"
            );
        }
    }

    /// Issue 21: an extension point the `agents` domain does not have is
    /// refused.
    #[test]
    fn an_unknown_point_of_the_domain_is_refused() {
        let error = refusal(&edited(
            OPENAI_MANIFEST,
            "[[agents.provider]]",
            "[[agents.panel]]",
        ));

        assert!(matches!(error, ManifestError::Shape(_)), "{error:?}");
    }

    #[test]
    fn a_bad_plugin_id_is_refused() {
        let error = refusal(&edited(
            OPENAI_MANIFEST,
            r#"id = "openai""#,
            r#"id = "Open_AI""#,
        ));

        assert_eq!(error, ManifestError::BadPluginId("Open_AI".to_string()));
        assert!(
            error
                .to_string()
                .contains("lowercase letters, digits and dashes")
        );
    }

    #[test]
    fn a_bad_part_id_is_refused() {
        let error = refusal(&edited(
            OPENAI_MANIFEST,
            r#"id = "codex""#,
            r#"id = "co dex""#,
        ));

        assert!(matches!(
            error,
            ManifestError::BadPartId {
                kind: PartKind::Agent,
                ..
            }
        ));
    }

    #[test]
    fn a_version_that_is_not_semver_is_refused() {
        let error = refusal(&edited(
            OPENAI_MANIFEST,
            r#"version = "0.4.0""#,
            r#"version = "v0.4""#,
        ));

        assert_eq!(error, ManifestError::BadVersion("v0.4".to_string()));
    }

    #[test]
    fn a_glob_with_a_parent_folder_is_refused() {
        let error = refusal(&edited(
            OPENAI_MANIFEST,
            r#""~/.codex*/**""#,
            r#""~/.codex/../.ssh/**""#,
        ));

        assert_eq!(
            error,
            ManifestError::BadGlob {
                part: PartRef::agent("codex"),
                glob: "~/.codex/../.ssh/**".to_string(),
                problem: GlobProblem::ParentFolder,
            }
        );
        assert_eq!(
            error.to_string(),
            "The glob `~/.codex/../.ssh/**` of the agent part `codex` contains `..`"
        );
    }

    #[test]
    fn a_variable_not_in_the_parts_env_is_refused() {
        let error = refusal(&edited(
            OPENAI_MANIFEST,
            r#"env = ["CODEX_HOME"]"#,
            r#"env = []"#,
        ));

        assert!(matches!(
            error,
            ManifestError::BadGlob {
                problem: GlobProblem::UnlistedVariable(_),
                ..
            }
        ));
    }

    #[test]
    fn a_missing_icon_is_refused() {
        let error =
            refusal(&edited(OPENAI_MANIFEST, "icon = \"codex.svg\"\n", ""));

        assert_eq!(error, ManifestError::MissingIcon(PartRef::agent("codex")));
        assert_eq!(error.to_string(), "The agent part `codex` has no icon");
    }

    #[test]
    fn an_icon_in_another_folder_is_refused() {
        let error = refusal(&edited(
            OPENAI_MANIFEST,
            r#""codex.svg""#,
            r#""../codex.svg""#,
        ));

        assert!(matches!(error, ManifestError::BadIconName { .. }));
    }

    #[test]
    fn a_part_with_no_contract_in_api_is_refused() {
        let error = refusal(&edited(
            OPENAI_MANIFEST,
            "agents.provider = \"0.1\"\n",
            "",
        ));

        assert_eq!(
            error,
            ManifestError::PartWithoutContract(PartRef::provider("openai"))
        );
        assert_eq!(
            error.to_string(),
            "The provider part `openai` needs `agents.provider` in [api]"
        );
    }

    #[test]
    fn a_contract_with_no_part_is_refused() {
        let without_provider = OPENAI_MANIFEST
            .split("[[agents.provider]]")
            .next()
            .expect("the agent half");

        let error = refusal(without_provider);

        assert_eq!(
            error,
            ManifestError::ContractWithoutPart(PartKind::Provider)
        );
        assert_eq!(
            error.to_string(),
            "[api] names `agents.provider`, but the plugin has no provider part"
        );
    }

    /// Issue 20: the language receiver belongs to no part, so a plugin
    /// that exports it needs no part of its own for it.
    #[test]
    fn the_language_receiver_needs_no_part() {
        let manifest = Manifest::parse(&edited(
            OPENCODE_MANIFEST,
            r#"agents.agent = "0.1""#,
            "agents.agent = \"0.1\"\nlanguage = \"0.1\"",
        ))
        .expect("valid");

        assert_eq!(
            manifest.api().to_string(),
            "agents.agent 0.1, language 0.1"
        );
        assert_eq!(manifest.parts().len(), 1);
    }

    #[test]
    fn an_unknown_contract_version_needs_a_newer_otty() {
        let error = refusal(&edited(
            OPENAI_MANIFEST,
            r#"agents.agent = "0.1""#,
            r#"agents.agent = "0.2""#,
        ));

        assert_eq!(
            error.to_string(),
            "Needs a newer OTTY (agents.agent contract 0.2)"
        );
    }

    #[test]
    fn a_manifest_with_no_part_is_refused() {
        let text = r#"
            [plugin]
            id = "empty"
            name = "Empty"
            version = "0.1.0"
            wasm = "empty.wasm"
        "#;

        assert_eq!(refusal(text), ManifestError::NoParts);
    }

    #[test]
    fn two_parts_of_one_kind_with_one_id_are_refused() {
        let provider = OPENAI_MANIFEST
            .split("[[agents.provider]]")
            .nth(1)
            .expect("the provider half");
        let text = format!("{OPENAI_MANIFEST}\n[[agents.provider]]{provider}");

        assert!(matches!(
            refusal(&text),
            ManifestError::DuplicatePart { .. }
        ));
    }

    #[test]
    fn a_misspelt_permission_is_refused() {
        let error = refusal(&edited(OPENAI_MANIFEST, "read = [", "raed = ["));

        assert!(matches!(error, ManifestError::Shape(_)));
    }

    /// Spec 2026-11-23, Decision 10: accounts are no longer added by hand,
    /// so a provider part that offers it is refused as an unknown field.
    #[test]
    fn a_provider_that_offers_add_by_hand_is_refused() {
        let error = refusal(&edited(
            OPENAI_MANIFEST,
            "usage_page = ",
            "add_by_hand = true\nusage_page = ",
        ));

        assert!(matches!(error, ManifestError::Shape(_)), "{error:?}");
    }

    /// Issue 15: a subscription opens the usage page of its plan, a key
    /// or an organization the usage page of the provider's console.
    #[test]
    fn a_provider_may_name_the_usage_page_of_a_plan() {
        let with_plan = edited(
            OPENAI_MANIFEST,
            "usage_page = ",
            "plan_usage_page = \"https://chatgpt.com/codex/settings/usage\"\nusage_page = ",
        );

        let manifest = Manifest::parse(&with_plan).expect("valid");
        let without = Manifest::parse(OPENAI_MANIFEST).expect("valid");
        let error = refusal(&edited(
            &with_plan,
            "https://chatgpt.com/codex",
            "http://chatgpt.com/codex",
        ));

        assert_eq!(
            manifest.providers()[0].plan_usage_page(),
            Some("https://chatgpt.com/codex/settings/usage")
        );
        assert_eq!(without.providers()[0].plan_usage_page(), None);
        assert!(matches!(error, ManifestError::BadPart { .. }), "{error:?}");
    }

    /// Issue 23: a provider whose name in the LiteLLM price table is not
    /// its id gives that name.
    #[test]
    fn a_provider_may_name_the_name_the_price_table_knows_it_by() {
        let priced = edited(
            OPENAI_MANIFEST,
            "usage_page = ",
            "priced_as = \"example_ai\"\nusage_page = ",
        );

        let manifest = Manifest::parse(&priced).expect("valid");
        let without = Manifest::parse(OPENAI_MANIFEST).expect("valid");
        let error = refusal(&edited(&priced, "\"example_ai\"", "\" \""));

        assert_eq!(manifest.providers()[0].priced_as(), Some("example_ai"));
        assert_eq!(without.providers()[0].priced_as(), None);
        assert!(matches!(error, ManifestError::BadPart { .. }), "{error:?}");
    }

    #[test]
    fn a_provider_with_a_bad_host_is_refused() {
        let error = refusal(&edited(
            OPENAI_MANIFEST,
            r#""chatgpt.com"]"#,
            r#""https://chatgpt.com"]"#,
        ));

        assert!(matches!(error, ManifestError::BadHost { .. }));
    }

    /// Issue 32: OpenCode's installer puts the program in a folder the
    /// login shell may not have on its `PATH`.
    fn with_places_and_install(install: &str) -> String {
        edited(
            OPENCODE_MANIFEST,
            "program = \"opencode\"\n",
            &format!(
                "program = \"opencode\"\nprogram_places = \
                 [\"~/.opencode/bin/opencode\", \
                 \"{{XDG_DATA_HOME}}/opencode/bin/opencode\", \
                 \"/opt/opencode/bin/opencode\"]\n\n{install}\n"
            ),
        )
    }

    /// Issue 32: an agent part may name the places its official installer
    /// puts its program, from home, a variable it lists or the root, and
    /// how to install it: a command, a page, or both.
    #[test]
    fn an_agent_part_may_name_the_places_and_the_install_of_its_program() {
        let text = with_places_and_install(
            "[agents.agent.install]\ncommand = [\"sh\", \"-c\", \"curl -fsSL \
             https://example.com/install | sh\"]\npage = \
             \"https://example.com/docs\"",
        );
        let page_only = with_places_and_install(
            "[agents.agent.install]\npage = \"https://example.com/docs\"",
        );

        let manifest = Manifest::parse(&text).expect("valid");
        let page_only = Manifest::parse(&page_only).expect("valid");
        let plain = Manifest::parse(OPENCODE_MANIFEST).expect("valid");

        let part = &manifest.agents()[0];
        let places: Vec<&str> = part
            .program_places()
            .iter()
            .map(ProgramPlace::as_str)
            .collect();
        assert_eq!(
            places,
            [
                "~/.opencode/bin/opencode",
                "{XDG_DATA_HOME}/opencode/bin/opencode",
                "/opt/opencode/bin/opencode"
            ]
        );
        let install = part.install().expect("an install");
        assert_eq!(
            install.command(),
            ["sh", "-c", "curl -fsSL https://example.com/install | sh"]
        );
        assert_eq!(install.page(), Some("https://example.com/docs"));
        let page_only = page_only.agents()[0].install().expect("an install");
        assert!(page_only.command().is_empty());
        assert_eq!(page_only.page(), Some("https://example.com/docs"));
        assert!(plain.agents()[0].program_places().is_empty());
        assert_eq!(plain.agents()[0].install(), None);
    }

    /// Issue 32: a program place is one file at a place OTTY can tell
    /// without asking the disk.
    #[test]
    fn a_program_place_that_is_not_one_plain_file_is_refused() {
        let bad = [
            ("~/.opencode/../bin/opencode", "contains `..`"),
            ("bin/opencode", "must start with"),
            ("~/.opencode/*/opencode", "names one file"),
            ("{OPENCODE_HOME}/bin/opencode", "does not list in `env`"),
            ("~/.opencode/bin/", "names no file"),
        ];

        for (place, why) in bad {
            let text = edited(
                &with_places_and_install(""),
                "\"~/.opencode/bin/opencode\"",
                &format!("\"{place}\""),
            );

            let error = refusal(&text);

            assert!(
                matches!(&error, ManifestError::BadPart { part, .. }
                    if *part == PartRef::agent("opencode")),
                "{place}: {error:?}"
            );
            assert!(error.to_string().contains(why), "{place}: {error}");
        }
    }

    /// Issue 32: the install runs a program with its arguments in a tab
    /// the user sees, or opens an `https` page; it says at least one.
    #[test]
    fn an_install_with_no_way_or_a_bad_one_is_refused() {
        let bad = [
            ("[agents.agent.install]", "no command and no page"),
            (
                "[agents.agent.install]\ncommand = []",
                "no command and no page",
            ),
            ("[agents.agent.install]\ncommand = [\"\"]", "empty program"),
            (
                "[agents.agent.install]\npage = \"http://example.com/docs\"",
                "not an https:// link",
            ),
        ];

        for (install, why) in bad {
            let error = refusal(&with_places_and_install(install));

            assert!(
                matches!(error, ManifestError::BadPart { .. }),
                "{install}: {error:?}"
            );
            assert!(error.to_string().contains(why), "{install}: {error}");
        }

        let misspelt = refusal(&with_places_and_install(
            "[agents.agent.install]\npgae = \"https://example.com/docs\"",
        ));
        assert!(matches!(misspelt, ManifestError::Shape(_)), "{misspelt:?}");
    }

    #[test]
    fn a_key_place_must_name_its_fields() {
        let error = refusal(&edited(
            OPENAI_MANIFEST,
            "fields = [\"/OPENAI_API_KEY\", \"/tokens/access_token\"]\n",
            "",
        ));

        assert!(matches!(error, ManifestError::BadPart { .. }));
    }
}

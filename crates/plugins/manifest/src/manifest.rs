//! The plugin manifest, `otty-plugin.toml` (spec 2026-11-19-agent-plugins,
//! "What a plugin is"): the plugin, the contracts it uses, and its parts,
//! each with its own permissions. It is checked as a whole when read.

mod error;
pub(crate) mod raw;

use std::collections::BTreeSet;

use crate::contract::Api;
pub use crate::manifest::error::ManifestError;
use crate::manifest::raw::{RawAgent, RawManifest, RawPlugin, RawProvider};
use crate::part::{PartKind, PartRef, is_valid_id};
use crate::permissions::{PartPermissions, PermissionLine, check_host};
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

    /// `[api]`: the contract version of each kind of part.
    pub fn api(&self) -> &Api {
        &self.api
    }

    /// The `[[agent]]` parts.
    pub fn agents(&self) -> &[AgentPart] {
        &self.agents
    }

    /// The `[[provider]]` parts.
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
    ///     agent = "0.1"
    ///
    ///     [[agent]]
    ///     id = "example-agent"
    ///     name = "Example agent"
    ///     icon = "example.svg"
    ///
    ///     [agent.permissions]
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
            .agent
            .iter()
            .map(AgentPart::of_raw)
            .collect::<Result<Vec<_>, _>>()?;
        let providers = raw
            .provider
            .iter()
            .map(ProviderPart::of_raw)
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
            if self.api.version_of(part.kind()).is_none() {
                return Err(ManifestError::PartWithoutContract(part.clone()));
            }
        }

        for (kind, _) in self.api.iter() {
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

    /// The name shown on the Plugins page.
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

/// One `[[agent]]` part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentPart {
    id: String,
    name: String,
    icon: String,
    program: Option<String>,
    tested_version: Option<String>,
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

    /// The file name of the agent's SVG icon, next to the manifest.
    pub fn icon(&self) -> &str {
        &self.icon
    }

    /// The agent's program, looked up on `PATH` to tell whether it is
    /// installed.
    pub fn program(&self) -> Option<&str> {
        self.program.as_deref()
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

        Ok(Self {
            id: raw.id.clone(),
            name: raw.name.clone(),
            icon,
            program: raw.program.clone(),
            tested_version: raw.tested_version.clone(),
            permissions: PartPermissions::of_agent(&raw.permissions, &part)?,
        })
    }
}

/// One `[[provider]]` part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderPart {
    id: String,
    name: String,
    icon: String,
    hosts: Vec<String>,
    usage_page: Option<String>,
    plan_usage_page: Option<String>,
    add_by_hand: Option<AddByHand>,
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

    /// What Add account offers for this provider, if anything.
    pub fn add_by_hand(&self) -> Option<AddByHand> {
        self.add_by_hand
    }

    /// What this part may do.
    pub fn permissions(&self) -> &PartPermissions {
        &self.permissions
    }

    /// The part's permissions in plain words, as the install dialog shows
    /// them, with the keys every provider part gets.
    pub fn permission_lines(&self) -> Vec<PermissionLine> {
        let mut lines = self.permissions.lines();
        lines.push(PermissionLine::GetsProviderKeys(self.name.clone()));

        lines
    }

    fn of_raw(raw: &RawProvider) -> Result<Self, ManifestError> {
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
        let add_by_hand = match raw.add_by_hand.as_deref() {
            None => None,
            Some("key") => Some(AddByHand::Key),
            Some("admin-key") => Some(AddByHand::AdminKey),
            Some(other) => {
                return Err(ManifestError::BadPart {
                    part,
                    reason: format!(
                        "has `add_by_hand = \"{other}\"`; use \"key\" or \"admin-key\""
                    ),
                });
            },
        };

        Ok(Self {
            id: raw.id.clone(),
            name: raw.name.clone(),
            icon,
            hosts: raw.hosts.clone(),
            usage_page: raw.usage_page.clone(),
            plan_usage_page: raw.plan_usage_page.clone(),
            add_by_hand,
            permissions: PartPermissions::of_provider(&raw.permissions, &part)?,
        })
    }
}

/// What Add account offers for a provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddByHand {
    /// An API key.
    Key,
    /// An admin key that reads the organization's bill.
    AdminKey,
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
    use crate::path_glob::GlobProblem;
    use crate::testing::{OPENAI_MANIFEST, edited};

    fn refusal(text: &str) -> ManifestError {
        Manifest::parse(text).expect_err("refused")
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
        assert_eq!(
            manifest.providers()[0].add_by_hand(),
            Some(AddByHand::AdminKey)
        );
        assert_eq!(manifest.agents()[0].permissions().env(), ["CODEX_HOME"]);
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
        let error =
            refusal(&edited(OPENAI_MANIFEST, "provider = \"0.1\"\n", ""));

        assert_eq!(
            error,
            ManifestError::PartWithoutContract(PartRef::provider("openai"))
        );
        assert_eq!(
            error.to_string(),
            "The provider part `openai` needs `provider` in [api]"
        );
    }

    #[test]
    fn a_contract_with_no_part_is_refused() {
        let without_provider = OPENAI_MANIFEST
            .split("[[provider]]")
            .next()
            .expect("the agent half");

        let error = refusal(without_provider);

        assert_eq!(
            error,
            ManifestError::ContractWithoutPart(PartKind::Provider)
        );
        assert_eq!(
            error.to_string(),
            "[api] names `provider`, but the plugin has no provider part"
        );
    }

    #[test]
    fn an_unknown_contract_version_needs_a_newer_otty() {
        let error = refusal(&edited(
            OPENAI_MANIFEST,
            r#"agent = "0.1""#,
            r#"agent = "0.2""#,
        ));

        assert_eq!(
            error.to_string(),
            "Needs a newer OTTY (agent contract 0.2)"
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
            .split("[[provider]]")
            .nth(1)
            .expect("the provider half");
        let text = format!("{OPENAI_MANIFEST}\n[[provider]]{provider}");

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

    /// Issue 15: a subscription opens the usage page of its plan, a key
    /// or an organization the usage page of the provider's console.
    #[test]
    fn a_provider_may_name_the_usage_page_of_a_plan() {
        let with_plan = edited(
            OPENAI_MANIFEST,
            "add_by_hand",
            "plan_usage_page = \"https://chatgpt.com/codex/settings/usage\"\nadd_by_hand",
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

    #[test]
    fn a_provider_with_a_bad_host_is_refused() {
        let error = refusal(&edited(
            OPENAI_MANIFEST,
            r#""chatgpt.com"]"#,
            r#""https://chatgpt.com"]"#,
        ));

        assert!(matches!(error, ManifestError::BadHost { .. }));
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

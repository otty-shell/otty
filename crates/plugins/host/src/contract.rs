//! The contracts a component exports, read from its export names and
//! checked against the parts of its manifest, and the one place that maps
//! each exported contract version to the bindings this OTTY keeps for it.
//! The manifest names no contract: the SDK a plugin is built with decides
//! the versions, and they are in the `.wasm`.

use std::collections::BTreeMap;
use std::fmt;

use otty_plugin_manifest::{
    Contract, ContractVersion, Manifest, PartKind, PartRef,
};
use semver::Version;
use wasmtime::component::InstancePre;
use wasmtime::wasmparser::{BinaryReaderError, Parser, Payload};

use crate::agents::agent::AgentExports;
use crate::agents::provider::ProviderExports;
use crate::calls::CallState;
use crate::engine::LoadError;
use crate::language::LanguageExports;

/// A version of the agent contract this OTTY has bindings for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AgentVersion {
    /// `otty:agents-agent@0.1`, [`crate::agents::agent::v0_1`].
    V0_1,
}

impl AgentVersion {
    /// The bindings of `version`, as the component exports it: the one
    /// place to add a new version of the agent contract.
    pub(crate) fn of(version: ContractVersion) -> Option<Self> {
        match (version.major(), version.minor()) {
            (0, 1) => Some(AgentVersion::V0_1),
            _ => None,
        }
    }
}

/// A version of the provider contract this OTTY has bindings for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProviderVersion {
    /// `otty:agents-provider@0.1`, [`crate::agents::provider::v0_1`].
    V0_1,
}

impl ProviderVersion {
    /// The bindings of `version`, as the component exports it: the one
    /// place to add a new version of the provider contract.
    pub(crate) fn of(version: ContractVersion) -> Option<Self> {
        match (version.major(), version.minor()) {
            (0, 1) => Some(ProviderVersion::V0_1),
            _ => None,
        }
    }
}

/// A version of the language receiver this OTTY has bindings for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LanguageVersion {
    /// `otty:language@0.1`, [`crate::language::v0_1`].
    V0_1,
}

impl LanguageVersion {
    /// The bindings of `version`, as the component exports it: the one
    /// place to add a new version of the language receiver.
    pub(crate) fn of(version: ContractVersion) -> Option<Self> {
        match (version.major(), version.minor()) {
            (0, 1) => Some(LanguageVersion::V0_1),
            _ => None,
        }
    }
}

/// The bindings of the contracts a plugin's component exports, each in
/// the version it exports.
pub(crate) struct ContractExports {
    agent: Option<AgentExports>,
    provider: Option<ProviderExports>,
    language: Option<LanguageExports>,
}

impl ContractExports {
    pub(crate) fn agent(&self) -> Option<&AgentExports> {
        self.agent.as_ref()
    }

    pub(crate) fn provider(&self) -> Option<&ProviderExports> {
        self.provider.as_ref()
    }

    pub(crate) fn language(&self) -> Option<&LanguageExports> {
        self.language.as_ref()
    }

    /// The exports of the component of `instance_pre` for each contract
    /// of `exported`. Refused for a contract version this OTTY has no
    /// bindings for, or an export that is not of its contract's shape.
    pub(crate) fn of_exported(
        exported: &ExportedApi,
        instance_pre: &InstancePre<CallState>,
    ) -> Result<Self, LoadError> {
        let link_error =
            |error: wasmtime::Error| LoadError::Link(error.to_string());
        let no_bindings = |contract: Contract, version: ContractVersion| {
            LoadError::Link(format!(
                "no bindings of the {contract} contract {version}"
            ))
        };

        let agent = match exported.version_of(Contract::Agent) {
            None => None,
            Some(version) => {
                let binding = AgentVersion::of(version)
                    .ok_or_else(|| no_bindings(Contract::Agent, version))?;

                Some(
                    AgentExports::new(binding, instance_pre)
                        .map_err(link_error)?,
                )
            },
        };
        let provider = match exported.version_of(Contract::Provider) {
            None => None,
            Some(version) => {
                let binding = ProviderVersion::of(version)
                    .ok_or_else(|| no_bindings(Contract::Provider, version))?;

                Some(
                    ProviderExports::new(binding, instance_pre)
                        .map_err(link_error)?,
                )
            },
        };
        let language = match exported.version_of(Contract::Language) {
            None => None,
            Some(version) => {
                let binding = LanguageVersion::of(version)
                    .ok_or_else(|| no_bindings(Contract::Language, version))?;

                Some(
                    LanguageExports::new(binding, instance_pre)
                        .map_err(link_error)?,
                )
            },
        };

        Ok(Self {
            agent,
            provider,
            language,
        })
    }
}

/// The contracts a component exports, each in the version it exports,
/// read from its export names such as `otty:agents-agent/agent@0.1.0`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExportedApi {
    versions: BTreeMap<Contract, ContractVersion>,
}

impl ExportedApi {
    /// The exported version of `contract`, if the component exports it.
    pub fn version_of(&self, contract: Contract) -> Option<ContractVersion> {
        self.versions.get(&contract).copied()
    }

    /// The contracts among the export names `names`; other exports are not
    /// contracts and are left out. Refused when one contract is exported
    /// twice or with a version that is not semver.
    ///
    /// ```
    /// use otty_plugin_host::{Contract, ExportedApi};
    ///
    /// let exported = ExportedApi::of_exports(
    ///     ["otty:agents-agent/agent@0.1.3", "wasi:cli/run@0.2.0"].into_iter(),
    /// )
    /// .expect("one version per contract");
    ///
    /// assert_eq!(exported.to_string(), "agents.agent 0.1");
    /// assert_eq!(exported.version_of(Contract::Provider), None);
    /// ```
    pub fn of_exports<'a>(
        names: impl Iterator<Item = &'a str>,
    ) -> Result<Self, ContractError> {
        let mut versions = BTreeMap::new();

        for name in names {
            let Some((interface, version)) = name.split_once('@') else {
                continue;
            };
            let contract = Contract::ALL
                .into_iter()
                .find(|contract| contract.export_name() == interface);
            let Some(contract) = contract else {
                continue;
            };
            let Ok(parsed) = Version::parse(version) else {
                return Err(ContractError::BadExportVersion {
                    contract,
                    version: version.to_string(),
                });
            };

            let exported = ContractVersion::of_export(&parsed);
            if versions.insert(contract, exported).is_some() {
                return Err(ContractError::ExportedTwice(contract));
            }
        }

        Ok(Self { versions })
    }

    /// The contracts the component in the bytes `wasm` exports, read
    /// without compiling it: an install or an update check reads a
    /// release's `.wasm` before anything else is done with it.
    pub fn of_wasm(wasm: &[u8]) -> Result<Self, ContractError> {
        let unreadable = |error: BinaryReaderError| {
            ContractError::Unreadable(error.to_string())
        };
        let mut names = Vec::new();
        // 1 in the component itself, more in the modules and components
        // it holds, whose exports are not the plugin's.
        let mut depth = 0_usize;

        for payload in Parser::new(0).parse_all(wasm) {
            match payload.map_err(unreadable)? {
                Payload::Version { .. } => depth += 1,
                Payload::End(_) => depth = depth.saturating_sub(1),
                Payload::ComponentExportSection(exports) if depth == 1 => {
                    for export in exports {
                        names.push(export.map_err(unreadable)?.name.name);
                    }
                },
                _ => {},
            }
        }

        Self::of_exports(names.into_iter())
    }

    /// Checks the exports against the parts of `manifest`: this OTTY
    /// knows every exported version, every part has its contract
    /// exported, and every exported contract of a kind of part has a
    /// part. A newer version is told first: such a plugin is made for a
    /// newer OTTY, whatever else does not fit.
    pub fn check(&self, manifest: &Manifest) -> Result<(), ContractError> {
        for (contract, version) in &self.versions {
            if !version.is_known(*contract) {
                return Err(ContractError::NeedsNewerOtty {
                    contract: *contract,
                    version: *version,
                });
            }
        }

        let parts = manifest.parts();
        for part in &parts {
            if !self.versions.contains_key(&part.kind().contract()) {
                return Err(ContractError::PartWithoutContract(part.clone()));
            }
        }

        let part_kinds = self
            .versions
            .keys()
            .filter_map(|contract| contract.part_kind());
        for kind in part_kinds {
            if !parts.iter().any(|part| part.kind() == kind) {
                return Err(ContractError::ContractWithoutPart(kind));
            }
        }

        Ok(())
    }
}

impl fmt::Display for ExportedApi {
    /// Writes `agents.agent 0.1, agents.provider 0.1`, or `nothing`.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let listed: Vec<String> = self
            .versions
            .iter()
            .map(|(contract, version)| format!("{contract} {version}"))
            .collect();

        if listed.is_empty() {
            return formatter.write_str("nothing");
        }

        formatter.write_str(&listed.join(", "))
    }
}

/// Why the contracts a plugin's `.wasm` exports are refused, in plain
/// words for Plugins mode.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ContractError {
    /// The bytes are not a component.
    #[error("The plugin's .wasm can not be read as a component: {0}")]
    Unreadable(String),
    /// One contract is exported in two versions.
    #[error("The .wasm exports the `{0}` contract twice")]
    ExportedTwice(Contract),
    /// A contract is exported with a version that is not semver.
    #[error(
        "The .wasm exports the `{contract}` contract as `{version}`, which \
         is not a version such as 0.1.0"
    )]
    BadExportVersion {
        /// The contract.
        contract: Contract,
        /// The version as the export name writes it.
        version: String,
    },
    /// A contract is exported in a version this OTTY does not know.
    #[error("Needs a newer OTTY ({contract} contract {version})")]
    NeedsNewerOtty {
        /// The contract.
        contract: Contract,
        /// The exported version, without its patch.
        version: ContractVersion,
    },
    /// A part's contract is not exported.
    #[error(
        "The {0} needs the `{key}` contract, but the .wasm does not export it",
        key = .0.kind().key()
    )]
    PartWithoutContract(PartRef),
    /// A contract of a kind of part is exported, but no part of that kind
    /// is declared.
    #[error(
        "The .wasm exports the `{key}` contract, but the plugin has no {0} part",
        key = .0.key()
    )]
    ContractWithoutPart(PartKind),
}

#[cfg(test)]
mod tests {
    use otty_plugin_manifest::{Manifest, PartKind, PartRef, known_versions};

    use super::*;
    use crate::testing::{
        TEST_PLUGIN_MANIFEST, TempDir, example_wasm, fixture,
        fixture_with_agent_0_2,
    };

    fn exports(names: &[&str]) -> ExportedApi {
        ExportedApi::of_exports(names.iter().copied()).expect("exports")
    }

    fn agent_only() -> Manifest {
        Manifest::parse(TEST_PLUGIN_MANIFEST).expect("valid")
    }

    fn agent_and_provider() -> Manifest {
        let text = format!(
            "{TEST_PLUGIN_MANIFEST}\n[[agents.provider]]\nid = \
             \"test-provider\"\nname = \"Test provider\"\nicon = \
             \"test.svg\"\nhosts = [\"example.com\"]\n"
        );

        Manifest::parse(&text).expect("valid")
    }

    fn version(major: u64, minor: u64) -> ContractVersion {
        ContractVersion::of_export(&Version::new(major, minor, 0))
    }

    #[test]
    fn every_known_contract_version_has_bindings() {
        for known in known_versions(Contract::Agent) {
            let binding = AgentVersion::of(ContractVersion::of_export(&known));

            assert!(binding.is_some(), "agent {known}");
        }
        for known in known_versions(Contract::Provider) {
            let binding =
                ProviderVersion::of(ContractVersion::of_export(&known));

            assert!(binding.is_some(), "provider {known}");
        }
        for known in known_versions(Contract::Language) {
            let binding =
                LanguageVersion::of(ContractVersion::of_export(&known));

            assert!(binding.is_some(), "language {known}");
        }
    }

    /// Issue 15 of spec 2026-11-23-accounts-apart-from-profiles: the
    /// versions of a plugin's contracts come from its exports, and they
    /// pick the bindings.
    #[test]
    fn a_plugins_contract_versions_come_from_its_exports() {
        let exported = exports(&[
            "otty:agents-agent/agent@0.1.3",
            "otty:language/language@0.1.0",
        ]);

        let agent = exported
            .version_of(Contract::Agent)
            .and_then(AgentVersion::of);
        let provider = exported
            .version_of(Contract::Provider)
            .and_then(ProviderVersion::of);

        assert_eq!(exported.version_of(Contract::Agent), Some(version(0, 1)));
        assert_eq!(agent, Some(AgentVersion::V0_1));
        assert_eq!(provider, None);
        assert_eq!(exported.to_string(), "agents.agent 0.1, language 0.1");
    }

    #[test]
    fn a_version_with_no_bindings_maps_to_none() {
        assert_eq!(AgentVersion::of(version(0, 2)), None);
        assert_eq!(ProviderVersion::of(version(0, 2)), None);
        assert_eq!(LanguageVersion::of(version(0, 2)), None);
    }

    #[test]
    fn exports_give_their_contracts_and_skip_other_exports() {
        let exported = exports(&[
            "otty:agents-agent/agent@0.1.3",
            "otty:test/misbehave@0.1.0",
            "otty:language/language@0.1.0",
            "otty:agents-provider/provider@0.1.0",
            "otty:agent/agent@0.1.0",
        ]);

        assert_eq!(
            exported.to_string(),
            "agents.agent 0.1, agents.provider 0.1, language 0.1"
        );
    }

    #[test]
    fn a_contract_exported_twice_is_refused() {
        let exported = ExportedApi::of_exports(
            [
                "otty:agents-agent/agent@0.1.0",
                "otty:agents-agent/agent@0.2.0",
            ]
            .into_iter(),
        );

        assert_eq!(
            exported,
            Err(ContractError::ExportedTwice(Contract::Agent))
        );
    }

    #[test]
    fn a_contract_version_that_is_not_semver_is_refused() {
        let exported = ExportedApi::of_exports(
            ["otty:agents-agent/agent@latest"].into_iter(),
        );

        assert_eq!(
            exported,
            Err(ContractError::BadExportVersion {
                contract: Contract::Agent,
                version: "latest".to_string(),
            })
        );
    }

    #[test]
    fn exports_that_fit_the_parts_are_kept() {
        let checked =
            exports(&["otty:agents-agent/agent@0.1.2"]).check(&agent_only());

        assert_eq!(checked, Ok(()));
    }

    #[test]
    fn an_unknown_exported_contract_version_needs_a_newer_otty() {
        let checked =
            exports(&["otty:agents-agent/agent@0.2.0"]).check(&agent_only());

        let error = checked.expect_err("refused");
        assert_eq!(
            error,
            ContractError::NeedsNewerOtty {
                contract: Contract::Agent,
                version: version(0, 2),
            }
        );
        assert_eq!(
            error.to_string(),
            "Needs a newer OTTY (agents.agent contract 0.2)"
        );
    }

    /// A newer contract version tells first, before any part that does
    /// not fit: the plugin is made for a newer OTTY.
    #[test]
    fn a_newer_contract_needs_a_newer_otty_before_the_parts_are_checked() {
        let checked = exports(&["otty:agents-provider/provider@0.3.1"])
            .check(&agent_only());

        assert_eq!(
            checked,
            Err(ContractError::NeedsNewerOtty {
                contract: Contract::Provider,
                version: version(0, 3),
            })
        );
    }

    #[test]
    fn a_declared_part_whose_contract_is_not_exported_is_refused() {
        let checked = exports(&["otty:agents-agent/agent@0.1.0"])
            .check(&agent_and_provider());

        let error = checked.expect_err("refused");
        assert_eq!(
            error,
            ContractError::PartWithoutContract(PartRef::provider(
                "test-provider"
            ))
        );
        assert_eq!(
            error.to_string(),
            "The provider part `test-provider` needs the `agents.provider` \
             contract, but the .wasm does not export it"
        );
    }

    #[test]
    fn an_exported_contract_with_no_part_is_refused() {
        let checked = exports(&[
            "otty:agents-agent/agent@0.1.0",
            "otty:agents-provider/provider@0.1.0",
        ])
        .check(&agent_only());

        let error = checked.expect_err("refused");
        assert_eq!(
            error,
            ContractError::ContractWithoutPart(PartKind::Provider)
        );
        assert_eq!(
            error.to_string(),
            "The .wasm exports the `agents.provider` contract, but the \
             plugin has no provider part"
        );
    }

    /// Issue 20: the language receiver belongs to no part, so a plugin
    /// that exports it needs no part of its own for it.
    #[test]
    fn the_language_receiver_needs_no_part() {
        let checked = exports(&[
            "otty:agents-agent/agent@0.1.0",
            "otty:language/language@0.1.0",
        ])
        .check(&agent_only());

        assert_eq!(checked, Ok(()));
    }

    /// The exports are read from the bytes of the `.wasm`, without
    /// compiling it, as an install or an update check needs.
    #[test]
    fn the_contracts_of_a_wasm_are_read_from_its_bytes() {
        let example = std::fs::read(example_wasm()).expect("example");
        let minimal = std::fs::read(fixture("minimal")).expect("minimal");

        let example = ExportedApi::of_wasm(&example).expect("a component");
        let minimal = ExportedApi::of_wasm(&minimal).expect("a component");

        assert_eq!(
            example.to_string(),
            "agents.agent 0.1, agents.provider 0.1, language 0.1"
        );
        assert_eq!(minimal.to_string(), "agents.agent 0.1");
    }

    #[test]
    fn a_wasm_that_exports_a_newer_agent_contract_reads_as_one() {
        let folder = TempDir::new();
        let path = fixture_with_agent_0_2("minimal", folder.path());
        let bytes = std::fs::read(path).expect("the test plugin");

        let exported = ExportedApi::of_wasm(&bytes).expect("a component");

        assert_eq!(exported.version_of(Contract::Agent), Some(version(0, 2)));
    }

    #[test]
    fn a_file_that_is_not_a_component_has_no_contracts() {
        let exported = ExportedApi::of_wasm(b"not wasm");

        assert!(
            matches!(exported, Err(ContractError::Unreadable(_))),
            "{exported:?}"
        );
    }
}

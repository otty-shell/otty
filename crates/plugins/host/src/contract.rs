//! The contracts a component exports, checked against the manifest's
//! `[api]`, and the one place that maps each contract version of `[api]`
//! to the bindings this OTTY keeps for it.

use std::collections::BTreeMap;
use std::fmt;

use otty_plugin_manifest::{Api, ContractVersion, PartKind};
use semver::Version;
use wasmtime::component::InstancePre;

use crate::agent::AgentExports;
use crate::calls::CallState;
use crate::engine::LoadError;
use crate::provider::ProviderExports;

/// A version of the agent contract this OTTY has bindings for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AgentVersion {
    /// `otty:agent@0.1`, [`crate::agent::v0_1`].
    V0_1,
}

impl AgentVersion {
    /// The bindings of `version` of `[api]`: the one place to add a new
    /// version of the agent contract.
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
    /// `otty:provider@0.1`, [`crate::provider::v0_1`].
    V0_1,
}

impl ProviderVersion {
    /// The bindings of `version` of `[api]`: the one place to add a new
    /// version of the provider contract.
    pub(crate) fn of(version: ContractVersion) -> Option<Self> {
        match (version.major(), version.minor()) {
            (0, 1) => Some(ProviderVersion::V0_1),
            _ => None,
        }
    }
}

/// The bindings of the contracts a plugin's `[api]` names, each in the
/// version `[api]` names.
pub(crate) struct ContractExports {
    agent: Option<AgentExports>,
    provider: Option<ProviderExports>,
}

impl ContractExports {
    pub(crate) fn agent(&self) -> Option<&AgentExports> {
        self.agent.as_ref()
    }

    pub(crate) fn provider(&self) -> Option<&ProviderExports> {
        self.provider.as_ref()
    }

    /// The exports of the component of `instance_pre` for each contract
    /// of `api`. Refused for a contract version this OTTY has no bindings
    /// for, or an export that is not of its contract's shape.
    pub(crate) fn of_api(
        api: &Api,
        instance_pre: &InstancePre<CallState>,
    ) -> Result<Self, LoadError> {
        let link_error =
            |error: wasmtime::Error| LoadError::Link(error.to_string());
        let no_bindings = |kind: PartKind, version: ContractVersion| {
            LoadError::Link(format!(
                "no bindings of the {kind} contract {version}"
            ))
        };

        let agent = match api.version_of(PartKind::Agent) {
            None => None,
            Some(version) => {
                let binding = AgentVersion::of(version)
                    .ok_or_else(|| no_bindings(PartKind::Agent, version))?;

                Some(
                    AgentExports::new(binding, instance_pre)
                        .map_err(link_error)?,
                )
            },
        };
        let provider = match api.version_of(PartKind::Provider) {
            None => None,
            Some(version) => {
                let binding = ProviderVersion::of(version)
                    .ok_or_else(|| no_bindings(PartKind::Provider, version))?;

                Some(
                    ProviderExports::new(binding, instance_pre)
                        .map_err(link_error)?,
                )
            },
        };

        Ok(Self { agent, provider })
    }
}

/// The contracts a component exports, read from its export names such as
/// `otty:agent/agent@0.1.0`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExportedApi {
    versions: BTreeMap<PartKind, Version>,
}

impl ExportedApi {
    /// The exported version of the contract of `kind`, if any.
    pub fn version_of(&self, kind: PartKind) -> Option<&Version> {
        self.versions.get(&kind)
    }

    /// The contracts among the export names `names`; other exports are not
    /// contracts and are left out. `None` when one contract is exported
    /// twice or with a version that is not semver.
    pub(crate) fn of_exports<'a>(
        names: impl Iterator<Item = &'a str>,
    ) -> Option<Self> {
        let mut versions = BTreeMap::new();

        for name in names {
            let Some((interface, version)) = name.split_once('@') else {
                continue;
            };
            let kind = PartKind::ALL
                .into_iter()
                .find(|kind| kind.export_name() == interface);
            let Some(kind) = kind else {
                continue;
            };
            let version = Version::parse(version).ok()?;

            if versions.insert(kind, version).is_some() {
                return None;
            }
        }

        Some(Self { versions })
    }

    /// Whether the component exports exactly the contracts of `api`, each
    /// in a version it accepts.
    pub(crate) fn matches(&self, api: &Api) -> bool {
        let same_kinds = api
            .iter()
            .map(|(kind, _)| kind)
            .eq(self.versions.keys().copied());

        same_kinds
            && api.iter().all(|(kind, version)| {
                self.versions
                    .get(&kind)
                    .is_some_and(|exported| version.accepts(exported))
            })
    }
}

impl fmt::Display for ExportedApi {
    /// Writes `agent 0.1.0, provider 0.1.0`, or `nothing`.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let listed: Vec<String> = self
            .versions
            .iter()
            .map(|(kind, version)| format!("{kind} {version}"))
            .collect();

        if listed.is_empty() {
            return formatter.write_str("nothing");
        }

        formatter.write_str(&listed.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use otty_plugin_manifest::{Manifest, known_versions};

    use super::*;
    use crate::testing::TEST_PLUGIN_MANIFEST;

    fn exports(names: &[&str]) -> ExportedApi {
        ExportedApi::of_exports(names.iter().copied()).expect("exports")
    }

    fn contract_version(version: &Version) -> ContractVersion {
        let text = format!("{}.{}", version.major, version.minor);

        ContractVersion::parse(&text).expect("a known version")
    }

    #[test]
    fn every_known_contract_version_has_bindings() {
        for version in known_versions(PartKind::Agent) {
            let binding = AgentVersion::of(contract_version(&version));

            assert!(binding.is_some(), "agent {version}");
        }
        for version in known_versions(PartKind::Provider) {
            let binding = ProviderVersion::of(contract_version(&version));

            assert!(binding.is_some(), "provider {version}");
        }
    }

    #[test]
    fn a_version_with_no_bindings_maps_to_none() {
        let version = ContractVersion::parse("0.2").expect("a version");

        assert_eq!(AgentVersion::of(version), None);
        assert_eq!(ProviderVersion::of(version), None);
    }

    #[test]
    fn exports_give_their_contracts_and_skip_other_exports() {
        let exported = exports(&[
            "otty:agent/agent@0.1.3",
            "otty:test/misbehave@0.1.0",
            "otty:provider/provider@0.1.0",
        ]);

        assert_eq!(exported.to_string(), "agent 0.1.3, provider 0.1.0");
    }

    #[test]
    fn a_contract_exported_twice_is_not_one_api() {
        let exported = ExportedApi::of_exports(
            ["otty:agent/agent@0.1.0", "otty:agent/agent@0.2.0"].into_iter(),
        );

        assert_eq!(exported, None);
    }

    #[test]
    fn api_matches_exactly_the_exported_contracts() {
        let manifest = Manifest::parse(TEST_PLUGIN_MANIFEST).expect("valid");
        let api = manifest.api();

        assert!(exports(&["otty:agent/agent@0.1.2"]).matches(api));
        assert!(
            !exports(&[
                "otty:agent/agent@0.1.0",
                "otty:provider/provider@0.1.0",
            ])
            .matches(api)
        );
        assert!(!exports(&["otty:provider/provider@0.1.0"]).matches(api));
        assert!(!exports(&[]).matches(api));
    }
}

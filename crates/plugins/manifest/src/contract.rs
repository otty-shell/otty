//! The versioned contracts between OTTY and a plugin: one per part kind,
//! each an extension point of a domain, and the optional ones of no
//! domain a plugin may export besides, such as the language receiver, and
//! the one list of the versions this OTTY knows.

use std::collections::BTreeMap;
use std::fmt;

use semver::{Comparator, Op, Prerelease, Version, VersionReq};

use crate::manifest::ManifestError;
use crate::manifest::raw::RawApiEntry;
use crate::part::{Domain, PartKind};

/// The contract versions this OTTY knows: the list to change when a
/// version is added (spec 2026-11-19-agent-plugins, Decision 3).
/// `otty-plugin-host` maps each of them to its bindings, and its tests
/// check that none is missing.
const KNOWN_VERSIONS: [(Contract, Version); 3] = [
    (Contract::Agent, Version::new(0, 1, 0)),
    (Contract::Provider, Version::new(0, 1, 0)),
    (Contract::Language, Version::new(0, 1, 0)),
];

/// One contract a plugin may export, each with its own versioned WIT
/// package: the contract of a kind of part, an extension point of a
/// domain (issue 21), or the language receiver, which belongs to no part
/// and no domain (issue 20).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Contract {
    /// `otty:agents-agent`, served by the `[[agents.agent]]` parts.
    Agent,
    /// `otty:agents-provider`, served by the `[[agents.provider]]` parts.
    Provider,
    /// `otty:language`: the plugin hears when the language of OTTY's
    /// interface changes.
    Language,
}

impl Contract {
    /// Every contract, in the order `[api]` lists them.
    pub const ALL: [Contract; 3] =
        [Contract::Agent, Contract::Provider, Contract::Language];

    /// The key of the contract in `[api]`: the domain and the point,
    /// such as `agents.agent = "0.1"`, or the contract alone for one of
    /// no domain, such as `language = "0.1"`.
    pub fn key(self) -> &'static str {
        match self {
            Contract::Agent => "agents.agent",
            Contract::Provider => "agents.provider",
            Contract::Language => "language",
        }
    }

    /// The domain whose extension point the contract is; `None` for a
    /// contract of no domain, such as the language receiver.
    pub fn domain(self) -> Option<Domain> {
        self.part_kind().map(PartKind::domain)
    }

    /// The contract whose `[api]` key is `key`.
    ///
    /// ```
    /// use otty_plugin_manifest::Contract;
    ///
    /// assert_eq!(Contract::from_key("language"), Some(Contract::Language));
    /// assert_eq!(Contract::from_key("panel"), None);
    /// ```
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|contract| contract.key() == key)
    }

    /// The name of the interface a component exports for this contract,
    /// without its version, such as `otty:agents-agent/agent`.
    pub fn export_name(self) -> &'static str {
        match self {
            Contract::Agent => "otty:agents-agent/agent",
            Contract::Provider => "otty:agents-provider/provider",
            Contract::Language => "otty:language/language",
        }
    }

    /// The kind of part that serves the contract; `None` for a contract
    /// of the whole plugin, such as the language receiver.
    pub fn part_kind(self) -> Option<PartKind> {
        match self {
            Contract::Agent => Some(PartKind::Agent),
            Contract::Provider => Some(PartKind::Provider),
            Contract::Language => None,
        }
    }
}

impl fmt::Display for Contract {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.key())
    }
}

/// The version of one contract that `[api]` names, `MAJOR.MINOR` such as
/// `0.1`. It accepts the export versions semver counts as compatible:
/// `0.1` accepts `0.1.0` and `0.1.3`, not `0.2.0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContractVersion {
    major: u64,
    minor: u64,
}

impl ContractVersion {
    /// The major number.
    pub fn major(self) -> u64 {
        self.major
    }

    /// The minor number.
    pub fn minor(self) -> u64 {
        self.minor
    }

    /// The version in `text`, exactly `MAJOR.MINOR` as `[api]` writes it.
    ///
    /// ```
    /// use otty_plugin_manifest::ContractVersion;
    ///
    /// let version = ContractVersion::parse("0.1").expect("a version");
    ///
    /// assert_eq!((version.major(), version.minor()), (0, 1));
    /// assert_eq!(ContractVersion::parse("0.1.0"), None);
    /// assert_eq!(ContractVersion::parse("^0.1"), None);
    /// ```
    pub fn parse(text: &str) -> Option<Self> {
        if !text.starts_with(|first: char| first.is_ascii_digit()) {
            return None;
        }

        let requirement = VersionReq::parse(text).ok()?;
        let [comparator] = requirement.comparators.as_slice() else {
            return None;
        };
        let major_and_minor_only = comparator.op == Op::Caret
            && comparator.patch.is_none()
            && comparator.pre.is_empty();
        if !major_and_minor_only {
            return None;
        }

        Some(Self {
            major: comparator.major,
            minor: comparator.minor?,
        })
    }

    /// Whether a component that exports `version` of the contract keeps
    /// this contract version.
    pub fn accepts(self, version: &Version) -> bool {
        self.requirement().matches(version)
    }

    /// Whether this OTTY knows this version of `contract`.
    pub fn is_known(self, contract: Contract) -> bool {
        KNOWN_VERSIONS.iter().any(|(known_contract, known)| {
            *known_contract == contract && self.accepts(known)
        })
    }

    fn requirement(self) -> VersionReq {
        VersionReq {
            comparators: vec![Comparator {
                op: Op::Caret,
                major: self.major,
                minor: Some(self.minor),
                patch: None,
                pre: Prerelease::EMPTY,
            }],
        }
    }
}

impl fmt::Display for ContractVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{}", self.major, self.minor)
    }
}

/// The contracts a plugin uses, one version per contract: the manifest's
/// `[api]`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Api {
    versions: BTreeMap<Contract, ContractVersion>,
}

impl Api {
    /// The version of `contract`, if the plugin uses it.
    pub fn version_of(&self, contract: Contract) -> Option<ContractVersion> {
        self.versions.get(&contract).copied()
    }

    /// Every contract with its version, in the order of [`Contract`].
    pub fn iter(
        &self,
    ) -> impl Iterator<Item = (Contract, ContractVersion)> + '_ {
        self.versions
            .iter()
            .map(|(contract, version)| (*contract, *version))
    }

    /// The manifest's `[api]` table, checked: one known `MAJOR.MINOR` per
    /// contract. A contract or a version this OTTY does not know gives
    /// "Needs a newer OTTY" for the whole plugin.
    pub(crate) fn of_manifest(
        table: &BTreeMap<String, RawApiEntry>,
    ) -> Result<Self, ManifestError> {
        let mut versions = BTreeMap::new();

        for (key, text) in contract_keys(table) {
            let needs_newer_otty = || ManifestError::NeedsNewerOtty {
                contract: key.clone(),
                version: text.clone(),
            };
            let Some(contract) = Contract::from_key(&key) else {
                return Err(needs_newer_otty());
            };
            let Some(version) = ContractVersion::parse(&text) else {
                return Err(ManifestError::BadContractVersion {
                    contract: key.clone(),
                    version: text.clone(),
                });
            };
            if !version.is_known(contract) {
                return Err(needs_newer_otty());
            }

            versions.insert(contract, version);
        }

        Ok(Self { versions })
    }
}

impl fmt::Display for Api {
    /// Writes `agents.agent 0.1, agents.provider 0.1`, or `nothing`.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let listed: Vec<String> = self
            .iter()
            .map(|(contract, version)| format!("{contract} {version}"))
            .collect();

        write_list(formatter, &listed)
    }
}

/// The entries of `[api]` as `(key, version)`, a domain's points keyed
/// as `domain.point`, such as `agents.agent`.
fn contract_keys(
    table: &BTreeMap<String, RawApiEntry>,
) -> Vec<(String, String)> {
    let mut keys = Vec::new();

    for (key, entry) in table {
        match entry {
            RawApiEntry::Version(version) => {
                keys.push((key.clone(), version.clone()));
            },
            RawApiEntry::Domain(points) => {
                for (point, version) in points {
                    keys.push((format!("{key}.{point}"), version.clone()));
                }
            },
        }
    }

    keys
}

/// The versions of `contract` this OTTY knows, oldest first.
pub fn known_versions(contract: Contract) -> impl Iterator<Item = Version> {
    KNOWN_VERSIONS
        .into_iter()
        .filter(move |(known, _)| *known == contract)
        .map(|(_, version)| version)
}

fn write_list(
    formatter: &mut fmt::Formatter<'_>,
    listed: &[String],
) -> fmt::Result {
    if listed.is_empty() {
        return formatter.write_str("nothing");
    }

    formatter.write_str(&listed.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(text: &str) -> BTreeMap<String, RawApiEntry> {
        #[derive(serde::Deserialize)]
        struct Manifest {
            api: BTreeMap<String, RawApiEntry>,
        }

        let manifest: Manifest =
            toml::from_str(&format!("[api]\n{text}")).expect("an [api] table");

        manifest.api
    }

    #[test]
    fn known_versions_of_api_are_kept() {
        let api = Api::of_manifest(&table(
            "agents.agent = \"0.1\"\nagents.provider = \"0.1\"",
        ))
        .expect("known versions");

        assert_eq!(
            api.version_of(Contract::Agent),
            ContractVersion::parse("0.1")
        );
        assert_eq!(api.to_string(), "agents.agent 0.1, agents.provider 0.1");
    }

    /// Issue 21: the contracts of a domain are keyed by the domain and the
    /// extension point, `agents.agent`; the language receiver belongs to no
    /// domain.
    #[test]
    fn a_contract_is_keyed_by_its_domain_and_point() {
        assert_eq!(Contract::Agent.key(), "agents.agent");
        assert_eq!(Contract::Provider.key(), "agents.provider");
        assert_eq!(Contract::Language.key(), "language");
        assert_eq!(Contract::Agent.domain(), Some(Domain::Agents));
        assert_eq!(Contract::Language.domain(), None);
        assert_eq!(Contract::Agent.export_name(), "otty:agents-agent/agent");
        assert_eq!(
            Contract::Provider.export_name(),
            "otty:agents-provider/provider"
        );
    }

    /// Issue 21: `agent` alone, outside its domain, is not a contract.
    #[test]
    fn a_point_outside_its_domain_needs_a_newer_otty() {
        let error = Api::of_manifest(&table("agent = \"0.1\""))
            .expect_err("no such contract");

        assert!(matches!(error, ManifestError::NeedsNewerOtty { .. }));
    }

    /// Issue 20: the language receiver is a contract of its own in
    /// `[api]`, of no kind of part.
    #[test]
    fn the_language_receiver_is_a_contract_of_no_part() {
        let api = Api::of_manifest(&table(
            "language = \"0.1\"\nagents.agent = \"0.1\"",
        ))
        .expect("known versions");

        assert_eq!(
            api.version_of(Contract::Language),
            ContractVersion::parse("0.1")
        );
        assert_eq!(api.to_string(), "agents.agent 0.1, language 0.1");
        assert_eq!(Contract::Language.part_kind(), None);
        assert_eq!(Contract::Agent.part_kind(), Some(PartKind::Agent));
    }

    #[test]
    fn an_unknown_contract_version_needs_a_newer_otty() {
        let error = Api::of_manifest(&table("agents.agent = \"0.2\""))
            .expect_err("unknown version");

        assert_eq!(
            error.to_string(),
            "Needs a newer OTTY (agents.agent contract 0.2)"
        );
    }

    #[test]
    fn an_unknown_contract_needs_a_newer_otty() {
        for text in [
            "panel = \"0.1\"",
            "agents.panel = \"0.1\"",
            "boxes.provider = \"0.1\"",
        ] {
            let error =
                Api::of_manifest(&table(text)).expect_err("unknown contract");

            assert!(
                matches!(error, ManifestError::NeedsNewerOtty { .. }),
                "{text}"
            );
        }
    }

    #[test]
    fn a_contract_version_is_major_and_minor_only() {
        for version in ["0.1.0", "^0.1", "=0.1", "0", "0.1-beta", ">=0.1"] {
            let error = Api::of_manifest(&table(&format!(
                "agents.agent = \"{version}\""
            )))
            .expect_err("not MAJOR.MINOR");

            assert!(
                matches!(error, ManifestError::BadContractVersion { .. }),
                "{version}"
            );
        }
    }

    #[test]
    fn a_contract_version_accepts_the_patches_of_its_minor_only() {
        let version = ContractVersion::parse("0.1").expect("a version");

        assert!(version.accepts(&Version::new(0, 1, 0)));
        assert!(version.accepts(&Version::new(0, 1, 3)));
        assert!(!version.accepts(&Version::new(0, 2, 0)));
    }
}

//! The versioned contracts between OTTY and a plugin: one per part kind,
//! each an extension point of a domain, and the optional ones of no
//! domain a plugin may export besides, such as the language receiver, and
//! the one list of the versions this OTTY knows. The manifest names no
//! contract: the SDK a plugin is built with decides the versions, and
//! `otty-plugin-host` reads them from the `.wasm`.

use std::fmt;

use semver::{Comparator, Op, Prerelease, Version, VersionReq};

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
    /// Every contract.
    pub const ALL: [Contract; 3] =
        [Contract::Agent, Contract::Provider, Contract::Language];

    /// The name of the contract in plain reasons: the domain and the
    /// point, such as `agents.agent`, or the contract alone for one of no
    /// domain, such as `language`.
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

/// The version of one contract, `MAJOR.MINOR` such as `0.1`: the version
/// a component exports without its patch. It accepts the export versions
/// semver counts as compatible: `0.1` accepts `0.1.0` and `0.1.3`, not
/// `0.2.0`.
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

    /// The contract version of an export of `version`, such as `0.1` of
    /// `otty:agents-agent/agent@0.1.3`.
    ///
    /// ```
    /// use otty_plugin_manifest::ContractVersion;
    /// use semver::Version;
    ///
    /// let version = ContractVersion::of_export(&Version::new(0, 1, 3));
    ///
    /// assert_eq!((version.major(), version.minor()), (0, 1));
    /// assert_eq!(version.to_string(), "0.1");
    /// ```
    pub fn of_export(version: &Version) -> Self {
        Self {
            major: version.major,
            minor: version.minor,
        }
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

/// The versions of `contract` this OTTY knows, oldest first.
pub fn known_versions(contract: Contract) -> impl Iterator<Item = Version> {
    KNOWN_VERSIONS
        .into_iter()
        .filter(move |(known, _)| *known == contract)
        .map(|(_, version)| version)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Issue 21: the contracts of a domain are named by the domain and the
    /// extension point, `agents.agent`; the language receiver belongs to no
    /// domain.
    #[test]
    fn a_contract_is_named_by_its_domain_and_point() {
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

    /// Issue 20: the language receiver is a contract of no kind of part.
    #[test]
    fn the_language_receiver_is_a_contract_of_no_part() {
        assert_eq!(Contract::Language.part_kind(), None);
        assert_eq!(Contract::Agent.part_kind(), Some(PartKind::Agent));
    }

    /// Issue 15 of spec 2026-11-23-accounts-apart-from-profiles: the
    /// version of an export, such as `0.1.3`, is its contract version
    /// `0.1`.
    #[test]
    fn the_version_of_an_export_is_its_major_and_minor() {
        let version = ContractVersion::of_export(&Version::new(0, 1, 3));

        assert_eq!((version.major(), version.minor()), (0, 1));
        assert_eq!(version.to_string(), "0.1");
    }

    #[test]
    fn only_the_known_contract_versions_are_known() {
        let current = ContractVersion::of_export(&Version::new(0, 1, 3));
        let newer = ContractVersion::of_export(&Version::new(0, 2, 0));

        assert!(current.is_known(Contract::Agent));
        assert!(current.is_known(Contract::Language));
        assert!(!newer.is_known(Contract::Agent));
    }

    #[test]
    fn a_contract_version_accepts_the_patches_of_its_minor_only() {
        let version = ContractVersion::of_export(&Version::new(0, 1, 0));

        assert!(version.accepts(&Version::new(0, 1, 0)));
        assert!(version.accepts(&Version::new(0, 1, 3)));
        assert!(!version.accepts(&Version::new(0, 2, 0)));
    }
}

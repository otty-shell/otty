//! The versioned contracts between OTTY and a plugin, one per part kind,
//! and the one list of the versions this OTTY knows.

use std::collections::BTreeMap;
use std::fmt;

use semver::{Comparator, Op, Prerelease, Version, VersionReq};

use crate::manifest::ManifestError;
use crate::part::PartKind;

/// The contract versions this OTTY knows: the list to change when a
/// version is added (spec 2026-11-19-agent-plugins, Decision 3).
/// `otty-plugin-host` maps each of them to its bindings, and its tests
/// check that none is missing.
const KNOWN_VERSIONS: [(PartKind, Version); 2] = [
    (PartKind::Agent, Version::new(0, 1, 0)),
    (PartKind::Provider, Version::new(0, 1, 0)),
];

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

    /// Whether this OTTY knows this version of the contract of `kind`.
    pub fn is_known(self, kind: PartKind) -> bool {
        KNOWN_VERSIONS.iter().any(|(known_kind, known)| {
            *known_kind == kind && self.accepts(known)
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

/// The contracts a plugin uses, one version per part kind: the manifest's
/// `[api]`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Api {
    versions: BTreeMap<PartKind, ContractVersion>,
}

impl Api {
    /// The version of the contract of `kind`, if the plugin uses it.
    pub fn version_of(&self, kind: PartKind) -> Option<ContractVersion> {
        self.versions.get(&kind).copied()
    }

    /// Every contract with its version, in the order of [`PartKind`].
    pub fn iter(
        &self,
    ) -> impl Iterator<Item = (PartKind, ContractVersion)> + '_ {
        self.versions
            .iter()
            .map(|(kind, version)| (*kind, *version))
    }

    /// The manifest's `[api]` table, checked: one known `MAJOR.MINOR` per
    /// contract. A contract or a version this OTTY does not know gives
    /// "Needs a newer OTTY" for the whole plugin.
    pub(crate) fn of_manifest(
        table: &BTreeMap<String, String>,
    ) -> Result<Self, ManifestError> {
        let mut versions = BTreeMap::new();

        for (key, text) in table {
            let needs_newer_otty = || ManifestError::NeedsNewerOtty {
                contract: key.clone(),
                version: text.clone(),
            };
            let Some(kind) = PartKind::from_key(key) else {
                return Err(needs_newer_otty());
            };
            let Some(version) = ContractVersion::parse(text) else {
                return Err(ManifestError::BadContractVersion {
                    contract: key.clone(),
                    version: text.clone(),
                });
            };
            if !version.is_known(kind) {
                return Err(needs_newer_otty());
            }

            versions.insert(kind, version);
        }

        Ok(Self { versions })
    }
}

impl fmt::Display for Api {
    /// Writes `agent 0.1, provider 0.1`, or `nothing`.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let listed: Vec<String> = self
            .iter()
            .map(|(kind, version)| format!("{kind} {version}"))
            .collect();

        write_list(formatter, &listed)
    }
}

/// The versions of the contract of `kind` this OTTY knows, oldest first.
pub fn known_versions(kind: PartKind) -> impl Iterator<Item = Version> {
    KNOWN_VERSIONS
        .into_iter()
        .filter(move |(known_kind, _)| *known_kind == kind)
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

    fn table(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
        entries
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect()
    }

    #[test]
    fn known_versions_of_api_are_kept() {
        let api =
            Api::of_manifest(&table(&[("agent", "0.1"), ("provider", "0.1")]))
                .expect("known versions");

        assert_eq!(
            api.version_of(PartKind::Agent),
            ContractVersion::parse("0.1")
        );
        assert_eq!(api.to_string(), "agent 0.1, provider 0.1");
    }

    #[test]
    fn an_unknown_contract_version_needs_a_newer_otty() {
        let error = Api::of_manifest(&table(&[("agent", "0.2")]))
            .expect_err("unknown version");

        assert_eq!(
            error.to_string(),
            "Needs a newer OTTY (agent contract 0.2)"
        );
    }

    #[test]
    fn an_unknown_contract_needs_a_newer_otty() {
        let error = Api::of_manifest(&table(&[("panel", "0.1")]))
            .expect_err("unknown contract");

        assert!(matches!(error, ManifestError::NeedsNewerOtty { .. }));
    }

    #[test]
    fn a_contract_version_is_major_and_minor_only() {
        for text in ["0.1.0", "^0.1", "=0.1", "0", "0.1-beta", ">=0.1"] {
            let error = Api::of_manifest(&table(&[("agent", text)]))
                .expect_err("not MAJOR.MINOR");

            assert!(
                matches!(error, ManifestError::BadContractVersion { .. }),
                "{text}"
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

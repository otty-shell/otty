//! The architecture rules, grouped by what they look at.

mod attributes;
mod calls;
mod contents;
mod files;
mod imports;
mod naming;
mod routes;
mod visibility;

use super::finding::Finding;
use super::source::SourceTree;

/// Run every rule on `tree`.
pub(crate) fn all(tree: &SourceTree) -> Vec<Finding> {
    let mut findings = Vec::new();

    findings.extend(files::check(tree));
    findings.extend(imports::check(tree));
    findings.extend(visibility::check(tree));
    findings.extend(contents::check(tree));
    findings.extend(calls::check(tree));
    findings.extend(attributes::check(tree));
    findings.extend(naming::check(tree));
    findings.extend(routes::check(tree));

    findings
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::super::finding::{Finding, Rule};

    /// The `(file, key)` pairs of the findings for `rule`, sorted and
    /// without repeats.
    pub(crate) fn found(
        findings: &[Finding],
        rule: Rule,
    ) -> Vec<(String, String)> {
        let mut pairs: Vec<(String, String)> = findings
            .iter()
            .filter(|finding| finding.rule() == rule)
            .map(|finding| {
                (finding.file().to_string(), finding.key().to_string())
            })
            .collect();

        pairs.sort();
        pairs.dedup();
        pairs
    }

    /// One `(file, key)` pair, for comparing with [`found`].
    pub(crate) fn pair(file: &str, key: &str) -> (String, String) {
        (file.to_string(), key.to_string())
    }
}

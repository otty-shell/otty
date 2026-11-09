//! The architecture check of the `app` crate.
//!
//! The check reports every place in `app/src` that breaks a rule. There are
//! no rules for now: they were removed while the architecture of `app`
//! changes (`specs/2026-11-09-split-router/spec.md`) and are written again
//! once it settles, so the check finds nothing.
//! [`Comparison`] then sets the findings against
//! [`allowlist::ALLOWLIST`]: a finding that is not listed, or an entry that
//! no longer matches anything, fails the test. [`report::render`] turns the
//! comparison into the readable report that both `cargo xtask arch` and the
//! failing test print.
//!
//! The rules are the source of truth for the `app` architecture.
//! `docs/internal/architecture/app/` describes them: a rule changes here
//! first, with a test that plants the violation, and the document follows.

pub mod allowlist;
pub mod finding;
pub mod report;

use std::path::Path;
use std::{fs, io};

use allowlist::Allowed;
use finding::Finding;

/// A finding that an allowlist entry accepts.
#[derive(Debug)]
pub struct Accepted {
    finding: Finding,
    entry: &'static Allowed,
}

impl Accepted {
    /// The place in `app/src`, with its line.
    pub fn finding(&self) -> &Finding {
        &self.finding
    }

    /// The allowlist entry that accepts it.
    pub fn entry(&self) -> &'static Allowed {
        self.entry
    }
}

/// The findings of a check set against an allowlist.
#[derive(Debug)]
pub struct Comparison {
    accepted: Vec<Accepted>,
    new_findings: Vec<Finding>,
    stale_entries: Vec<&'static Allowed>,
}

impl Comparison {
    /// Findings that an allowlist entry accepts, each with its entry.
    pub fn accepted(&self) -> &[Accepted] {
        &self.accepted
    }

    /// Findings that no allowlist entry accepts.
    pub fn new_findings(&self) -> &[Finding] {
        &self.new_findings
    }

    /// Allowlist entries that match no finding any more.
    pub fn stale_entries(&self) -> &[&'static Allowed] {
        &self.stale_entries
    }

    /// Whether every finding is listed and every entry is still needed.
    pub fn is_clean(&self) -> bool {
        self.new_findings.is_empty() && self.stale_entries.is_empty()
    }

    /// Set `findings` against `allowlist`.
    pub fn compare(
        findings: &[Finding],
        allowlist: &'static [Allowed],
    ) -> Self {
        let mut accepted = Vec::new();
        let mut new_findings = Vec::new();
        for finding in findings {
            match allowlist.iter().find(|entry| entry.matches(finding)) {
                Some(entry) => accepted.push(Accepted {
                    finding: finding.clone(),
                    entry,
                }),
                None => new_findings.push(finding.clone()),
            }
        }

        let stale_entries = allowlist
            .iter()
            .filter(|entry| {
                !findings.iter().any(|finding| entry.matches(finding))
            })
            .collect();

        Self {
            accepted,
            new_findings,
            stale_entries,
        }
    }
}

/// Check the `app` sources in `app_src` and return every finding.
///
/// There are no rules for now, so the check only makes sure that `app_src`
/// can be read and finds nothing.
///
/// ```no_run
/// use std::path::Path;
///
/// use otty_xtask::architecture::allowlist::ALLOWLIST;
/// use otty_xtask::architecture::report::{Output, render};
/// use otty_xtask::architecture::{Comparison, check};
///
/// let findings = check(Path::new("app/src"))?;
/// let comparison = Comparison::compare(&findings, ALLOWLIST);
/// println!("{}", render(&comparison, Output::Plain));
/// # Ok::<(), std::io::Error>(())
/// ```
pub fn check(app_src: &Path) -> io::Result<Vec<Finding>> {
    fs::read_dir(app_src)?;

    Ok(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::finding::Rule;
    use super::*;

    const LIST: &[Allowed] = &[
        Allowed::known(
            Rule::ALPHA,
            "widgets/notes/view/panel.rs",
            "widgets::notes::state",
            "the panel still reads the state",
        ),
        Allowed::exception(
            Rule::BETA,
            "dispatch/command.rs",
            "route",
            "asks AppCommand::is_available",
        ),
    ];

    fn listed() -> Finding {
        Finding::new(
            Rule::ALPHA,
            "widgets/notes/view/panel.rs",
            "widgets::notes::state",
            7,
        )
    }

    fn unlisted() -> Finding {
        Finding::new(Rule::DELTA, "widgets/tabs/model.rs", "widgets::modes", 3)
    }

    #[test]
    fn a_listed_finding_is_accepted_and_an_unused_entry_is_stale() {
        let comparison = Comparison::compare(&[listed()], LIST);

        assert!(comparison.new_findings().is_empty());
        assert_eq!(comparison.stale_entries(), [&LIST[1]]);
        assert!(!comparison.is_clean());
    }

    #[test]
    fn an_accepted_finding_keeps_its_line_and_its_entry() {
        let comparison = Comparison::compare(&[listed()], LIST);

        let accepted: Vec<(&Finding, &Allowed)> = comparison
            .accepted()
            .iter()
            .map(|place| (place.finding(), place.entry()))
            .collect();

        assert_eq!(accepted, [(&listed(), &LIST[0])]);
    }

    #[test]
    fn an_unlisted_finding_is_new() {
        let all = [
            listed(),
            unlisted(),
            Finding::new(Rule::BETA, "dispatch/command.rs", "route", 24),
        ];

        let comparison = Comparison::compare(&all, LIST);

        assert_eq!(comparison.new_findings(), [unlisted()]);
        assert!(comparison.stale_entries().is_empty());
    }
}

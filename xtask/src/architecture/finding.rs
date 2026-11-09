//! What the architecture check reports: the rules and the places that break
//! them.
//!
//! There are no rules for now. They were removed while the architecture of
//! `app` changes (`specs/2026-11-09-split-router/spec.md`) and are written
//! again once it settles, each as a constant of [`Rule`] listed in
//! [`Rule::ALL`]. The rules that exist only in tests let the tests of the
//! allowlist and the report run without real rules.

/// One architecture rule of the `app` crate: a short name and what it
/// requires.
///
/// ```
/// use otty_xtask::architecture::finding::Rule;
///
/// let rule = Rule::new("view-imports-state", "A view never imports state.");
///
/// assert_eq!(rule.name(), "view-imports-state");
/// assert_eq!(rule.constant(), "VIEW_IMPORTS_STATE");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Rule {
    name: &'static str,
    description: &'static str,
}

impl Rule {
    /// Every rule, in the order of reports. Empty while there are no rules.
    #[cfg(not(test))]
    pub const ALL: &'static [Rule] = &[];

    /// Every rule, in the order of reports.
    #[cfg(test)]
    pub const ALL: &'static [Rule] =
        &[Rule::ALPHA, Rule::BETA, Rule::GAMMA, Rule::DELTA];

    /// An example rule for the tests of the check itself.
    #[cfg(test)]
    pub(crate) const ALPHA: Rule =
        Rule::new("alpha", "Alpha is an example rule.");

    /// An example rule for the tests of the check itself.
    #[cfg(test)]
    pub(crate) const BETA: Rule = Rule::new("beta", "Beta is an example rule.");

    /// An example rule for the tests of the check itself.
    #[cfg(test)]
    pub(crate) const GAMMA: Rule =
        Rule::new("gamma", "Gamma is an example rule.");

    /// An example rule for the tests of the check itself.
    #[cfg(test)]
    pub(crate) const DELTA: Rule =
        Rule::new("delta", "Delta is an example rule.");

    /// Short, stable name of the rule, used in reports, such as
    /// `view-imports-state`.
    pub fn name(self) -> &'static str {
        self.name
    }

    /// What the rule requires, in one plain sentence, shown above its places
    /// in the report.
    pub fn description(self) -> &'static str {
        self.description
    }

    /// The name of the rule's constant on [`Rule`], as an allowlist entry
    /// writes it: `view-imports-state` becomes `VIEW_IMPORTS_STATE`.
    pub fn constant(self) -> String {
        self.name.to_uppercase().replace('-', "_")
    }

    /// A rule named `name` that requires `description`.
    pub const fn new(name: &'static str, description: &'static str) -> Self {
        Self { name, description }
    }
}

/// One place in `app/src` that breaks a rule.
///
/// `key` names what breaks the rule (an imported module, a function, a
/// file), so a finding stays the same when lines move.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    rule: Rule,
    file: String,
    key: String,
    line: usize,
}

impl Finding {
    /// The rule this place breaks.
    pub fn rule(&self) -> Rule {
        self.rule
    }

    /// The file, relative to `app/src`, with `/` between folders.
    pub fn file(&self) -> &str {
        &self.file
    }

    /// What breaks the rule in that file.
    pub fn key(&self) -> &str {
        &self.key
    }

    /// The first line where it happens, counting from 1; 0 for a missing
    /// file.
    pub fn line(&self) -> usize {
        self.line
    }

    /// Record that `file` breaks `rule` through `key` at `line`. Only tests
    /// build findings while there are no rules.
    #[cfg(test)]
    pub(crate) fn new(rule: Rule, file: &str, key: &str, line: usize) -> Self {
        Self {
            rule,
            file: file.to_string(),
            key: key.to_string(),
            line,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn every_rule_has_its_own_name() {
        let names: HashSet<&str> =
            Rule::ALL.iter().map(|rule| rule.name()).collect();

        assert_eq!(names.len(), Rule::ALL.len());
    }

    #[test]
    fn every_rule_has_a_one_sentence_description() {
        for rule in Rule::ALL {
            let description = rule.description();

            assert!(description.ends_with('.'), "{rule:?}: {description}");
            assert!(
                description.chars().next().is_some_and(char::is_uppercase),
                "{rule:?}: {description}"
            );
        }
    }

    #[test]
    fn the_constant_is_the_name_in_capitals_with_underscores() {
        let rule =
            Rule::new("view-imports-state", "A view never imports state.");

        assert_eq!(rule.constant(), "VIEW_IMPORTS_STATE");
    }
}

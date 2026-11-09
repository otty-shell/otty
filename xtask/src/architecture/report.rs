//! The readable report of an architecture check.
//!
//! It starts with a summary line and the places that need action (new
//! findings and stale allowlist entries), then a table of counts per rule,
//! then every accepted place grouped by rule with its reason, and ends with
//! what to do next.

use std::collections::BTreeMap;

use super::allowlist::{AllowKind, Allowed};
use super::finding::{Finding, Rule};
use super::{Accepted, Comparison};

/// Where accepted places are listed, named in the advice of the report.
const ALLOWLIST_FILE: &str = "xtask/src/architecture/allowlist.rs";

/// ANSI colour codes for terminal output.
const RED: &str = "1;31";
const YELLOW: &str = "1;33";
const CYAN: &str = "36";
const DIM: &str = "2";
const BOLD: &str = "1";

/// Where the report is printed, which decides whether it uses colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Output {
    /// Plain text, as in a test failure message.
    Plain,
    /// A terminal: tags and headings get ANSI colours.
    Terminal,
}

/// How a place stands against the allowlist, shown as its tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tag {
    New,
    Stale,
    Known,
    Exception,
}

impl Tag {
    /// The tag of an accepted place.
    fn of(kind: AllowKind) -> Self {
        match kind {
            AllowKind::KnownViolation => Tag::Known,
            AllowKind::Exception => Tag::Exception,
        }
    }

    /// The tag text with its brackets, coloured for a terminal.
    fn label(self, output: Output) -> String {
        let (text, colour) = match self {
            Tag::New => ("NEW", RED),
            Tag::Stale => ("STALE", YELLOW),
            Tag::Known => ("known", DIM),
            Tag::Exception => ("exception", CYAN),
        };

        format!("[{}]", paint(text, colour, output))
    }
}

/// How many places of one rule the table shows in each column.
#[derive(Debug, Default)]
struct RuleCounts {
    known: usize,
    exception: usize,
    new: usize,
    stale: usize,
}

/// Build the readable report of `comparison`.
///
/// The report lists, in order: a summary line, the places that need action
/// (new findings and stale entries), a table of counts per rule, every
/// accepted place grouped by rule with its reason, and what to do next.
pub fn render(comparison: &Comparison, output: Output) -> String {
    let counts = counts_per_rule(comparison);

    let mut report = summary(comparison, &counts);
    if !comparison.is_clean() {
        report.push_str(&action_block(comparison, output));
    }
    report.push_str(&table(&counts));
    report.push_str(&accepted_sections(comparison, output));
    report.push_str(&advice(comparison));

    report
}

/// The process exit code for `comparison`: 1 when a place is new or an
/// entry is stale, 0 otherwise.
pub fn exit_code(comparison: &Comparison) -> u8 {
    if comparison.is_clean() { 0 } else { 1 }
}

/// Count the places of every rule that has any, in rule order.
fn counts_per_rule(comparison: &Comparison) -> BTreeMap<Rule, RuleCounts> {
    let mut counts: BTreeMap<Rule, RuleCounts> = BTreeMap::new();

    for place in comparison.accepted() {
        let rule_counts = counts.entry(place.finding().rule()).or_default();
        match place.entry().kind() {
            AllowKind::KnownViolation => rule_counts.known += 1,
            AllowKind::Exception => rule_counts.exception += 1,
        }
    }
    for finding in comparison.new_findings() {
        counts.entry(finding.rule()).or_default().new += 1;
    }
    for entry in comparison.stale_entries() {
        counts.entry(entry.rule()).or_default().stale += 1;
    }

    counts
}

/// The first line: rules, accepted places by kind, new and stale.
fn summary(
    comparison: &Comparison,
    counts: &BTreeMap<Rule, RuleCounts>,
) -> String {
    let rules = Rule::ALL.len();
    let known: usize = counts.values().map(|count| count.known).sum();
    let exceptions: usize = counts.values().map(|count| count.exception).sum();
    let accepted = comparison.accepted().len();
    let new = comparison.new_findings().len();
    let stale = comparison.stale_entries().len();

    let accepted = plural(accepted, "accepted place", "accepted places");
    let known = plural(known, "known violation", "known violations");
    let exceptions = plural(exceptions, "exception", "exceptions");

    format!(
        "Architecture: {rules} rules, {accepted} ({known}, {exceptions}), \
         {new} new, {stale} stale\n"
    )
}

/// The places that need action, grouped by rule: new findings with a ready
/// allowlist line, then stale entries with their kind and reason.
fn action_block(comparison: &Comparison, output: Output) -> String {
    let mut new_by_rule: BTreeMap<Rule, Vec<&Finding>> = BTreeMap::new();
    for finding in comparison.new_findings() {
        new_by_rule.entry(finding.rule()).or_default().push(finding);
    }
    let mut stale_by_rule: BTreeMap<Rule, Vec<&Allowed>> = BTreeMap::new();
    for entry in comparison.stale_entries() {
        stale_by_rule.entry(entry.rule()).or_default().push(entry);
    }

    let mut block = format!("\n{}\n", paint("Needs action", BOLD, output));
    for &rule in Rule::ALL {
        let new = new_by_rule.remove(&rule).unwrap_or_default();
        let stale = stale_by_rule.remove(&rule).unwrap_or_default();
        if new.is_empty() && stale.is_empty() {
            continue;
        }

        block.push_str(&rule_heading(rule, output));
        for finding in sorted_findings(new) {
            block.push_str(&place_line(finding, Tag::New, output));
            block.push_str(&format!(
                "    if it is accepted, add to {ALLOWLIST_FILE}: {}\n",
                ready_entry(finding)
            ));
        }
        for entry in sorted_entries(stale) {
            let (file, key) = (entry.file(), entry.key());
            let tag = Tag::Stale.label(output);
            let kind = kind_label(entry.kind());
            let reason = entry.reason();
            block.push_str(&format!("  {file}  {key}  {tag}\n"));
            block.push_str(&format!("    {kind}: {reason}\n"));
        }
    }

    block
}

/// One row per rule that has places, and a line for the rules without any.
fn table(counts: &BTreeMap<Rule, RuleCounts>) -> String {
    let width = counts
        .keys()
        .map(|rule| rule.name().len())
        .max()
        .unwrap_or(0)
        .max("rule".len());

    let mut table =
        format!("\n{:<width$}  known  exception  new  stale\n", "rule");
    for (rule, count) in counts {
        let name = rule.name();
        let RuleCounts {
            known,
            exception,
            new,
            stale,
        } = count;
        table.push_str(&format!(
            "{name:<width$}  {known:>5}  {exception:>9}  {new:>3}  {stale:>5}\n"
        ));
    }

    let without = Rule::ALL.len() - counts.len();
    let without = match without {
        1 => "1 rule has no findings".to_string(),
        count => format!("{count} rules have no findings"),
    };
    table.push_str(&format!("{without}\n"));

    table
}

/// Every accepted place, grouped by rule and sorted by file, with the
/// reason it is accepted.
fn accepted_sections(comparison: &Comparison, output: Output) -> String {
    let mut by_rule: BTreeMap<Rule, Vec<&Accepted>> = BTreeMap::new();
    for place in comparison.accepted() {
        by_rule
            .entry(place.finding().rule())
            .or_default()
            .push(place);
    }

    let mut sections = String::new();
    for (rule, mut places) in by_rule {
        places.sort_by(|left, right| {
            let (left, right) = (left.finding(), right.finding());
            (left.file(), left.line()).cmp(&(right.file(), right.line()))
        });

        sections.push_str(&rule_heading(rule, output));
        for place in places {
            let tag = Tag::of(place.entry().kind());
            let reason = place.entry().reason();
            sections.push_str(&place_line(place.finding(), tag, output));
            sections.push_str(&format!("    {reason}\n"));
        }
    }

    sections
}

/// The last line: what to do about new and stale places, or that nothing
/// needs action.
fn advice(comparison: &Comparison) -> String {
    if comparison.is_clean() {
        return format!(
            "\nNothing new or stale. When you fix a known place, delete its \
             entry in {ALLOWLIST_FILE}.\n"
        );
    }

    format!(
        "\nNext: fix the NEW places (or, if one is accepted, add its entry to \
         {ALLOWLIST_FILE} with the reason) and delete the STALE entries.\n"
    )
}

/// The heading of a rule: its name and what it requires.
fn rule_heading(rule: Rule, output: Output) -> String {
    let name = paint(rule.name(), BOLD, output);
    let description = rule.description();

    format!("\n{name}: {description}\n")
}

/// One place as `file:line  key  [tag]`; a missing file has no line.
fn place_line(finding: &Finding, tag: Tag, output: Output) -> String {
    let (file, line, key) = (finding.file(), finding.line(), finding.key());
    let tag = tag.label(output);

    match line {
        0 => format!("  {file}  {key}  {tag}\n"),
        line => format!("  {file}:{line}  {key}  {tag}\n"),
    }
}

/// The allowlist line that would accept `finding`, with a reason to fill in.
fn ready_entry(finding: &Finding) -> String {
    let (rule, file, key) =
        (finding.rule().constant(), finding.file(), finding.key());

    format!("Allowed::known(Rule::{rule}, \"{file}\", \"{key}\", \"<why>\"),")
}

/// The word for an allowlist kind in a report.
fn kind_label(kind: AllowKind) -> &'static str {
    match kind {
        AllowKind::Exception => "exception",
        AllowKind::KnownViolation => "known violation",
    }
}

fn sorted_findings(mut findings: Vec<&Finding>) -> Vec<&Finding> {
    findings.sort_by(|left, right| {
        (left.file(), left.line()).cmp(&(right.file(), right.line()))
    });

    findings
}

fn sorted_entries(mut entries: Vec<&Allowed>) -> Vec<&Allowed> {
    entries.sort_by(|left, right| {
        (left.file(), left.key()).cmp(&(right.file(), right.key()))
    });

    entries
}

/// `count` with the singular or plural word.
fn plural(count: usize, one: &str, many: &str) -> String {
    match count {
        1 => format!("1 {one}"),
        count => format!("{count} {many}"),
    }
}

/// Wrap `text` in an ANSI colour for a terminal; plain output is unchanged.
fn paint(text: &str, colour: &str, output: Output) -> String {
    match output {
        Output::Terminal => format!("\u{1b}[{colour}m{text}\u{1b}[0m"),
        Output::Plain => text.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::super::allowlist::Allowed;
    use super::super::finding::{Finding, Rule};
    use super::*;

    const LIST: &[Allowed] = &[
        Allowed::known(
            Rule::ALPHA,
            "widgets/tabs/view/list.rs",
            "widgets::tabs::state",
            "the list still reads the state",
        ),
        Allowed::known(
            Rule::ALPHA,
            "widgets/notes/view/panel.rs",
            "widgets::notes::state",
            "the panel still reads the state",
        ),
        Allowed::exception(
            Rule::BETA,
            "router/command.rs",
            "route",
            "asks AppCommand::is_available",
        ),
        Allowed::known(
            Rule::GAMMA,
            "widgets/tabs/reducer.rs",
            "event",
            "rename the parameter to intent",
        ),
    ];

    fn accepted_findings() -> Vec<Finding> {
        vec![
            Finding::new(
                Rule::ALPHA,
                "widgets/tabs/view/list.rs",
                "widgets::tabs::state",
                4,
            ),
            Finding::new(
                Rule::ALPHA,
                "widgets/notes/view/panel.rs",
                "widgets::notes::state",
                7,
            ),
            Finding::new(Rule::BETA, "router/command.rs", "route", 24),
            Finding::new(Rule::GAMMA, "widgets/tabs/reducer.rs", "event", 20),
        ]
    }

    fn new_finding() -> Finding {
        Finding::new(Rule::DELTA, "widgets/tabs/model.rs", "widgets::modes", 3)
    }

    /// One new finding, and the `Gamma` entry gone stale.
    fn with_action_needed() -> Comparison {
        let mut findings = accepted_findings();
        findings.retain(|finding| finding.rule() != Rule::GAMMA);
        findings.push(new_finding());

        Comparison::compare(&findings, LIST)
    }

    fn clean() -> Comparison {
        Comparison::compare(&accepted_findings(), LIST)
    }

    fn position(report: &str, needle: &str) -> usize {
        report
            .find(needle)
            .unwrap_or_else(|| panic!("{needle:?} is missing from:\n{report}"))
    }

    #[test]
    fn the_first_line_sums_up_the_counts() {
        let report = render(&with_action_needed(), Output::Plain);

        let first_line = report.lines().next().unwrap_or_default();

        assert_eq!(
            first_line,
            format!(
                "Architecture: {} rules, 3 accepted places (2 known violations, \
                 1 exception), 1 new, 1 stale",
                Rule::ALL.len()
            )
        );
    }

    #[test]
    fn a_clean_summary_uses_singular_words_for_one() {
        let comparison =
            Comparison::compare(&accepted_findings()[2..3], &LIST[2..3]);

        let report = render(&comparison, Output::Plain);

        assert!(
            report.starts_with(&format!(
                "Architecture: {} rules, 1 accepted place (0 known violations, \
                 1 exception), 0 new, 0 stale\n",
                Rule::ALL.len()
            )),
            "{report}"
        );
    }

    #[test]
    fn new_and_stale_places_come_before_the_table_and_the_known_places() {
        let report = render(&with_action_needed(), Output::Plain);

        let action = position(&report, "Needs action");
        let new =
            position(&report, "widgets/tabs/model.rs:3  widgets::modes  [NEW]");
        let stale =
            position(&report, "widgets/tabs/reducer.rs  event  [STALE]");
        let table = position(&report, "rule ");
        let known = position(&report, "[known]");

        assert!(action < new && new < table, "{report}");
        assert!(stale < table && table < known, "{report}");
    }

    #[test]
    fn a_new_place_comes_with_a_ready_allowlist_line() {
        let report = render(&with_action_needed(), Output::Plain);

        assert!(
            report.contains(
                "Allowed::known(Rule::DELTA, \"widgets/tabs/model.rs\", \
                 \"widgets::modes\", \"<why>\"),"
            ),
            "{report}"
        );
    }

    #[test]
    fn a_stale_entry_shows_its_kind_and_reason() {
        let report = render(&with_action_needed(), Output::Plain);

        let stale = position(&report, "[STALE]");
        let reason = position(
            &report,
            "known violation: rename the parameter to intent",
        );

        assert!(stale < reason, "{report}");
    }

    #[test]
    fn places_are_grouped_by_rule_and_sorted_by_file() {
        let report = render(&clean(), Output::Plain);

        let heading =
            position(&report, &format!("alpha: {}", Rule::ALPHA.description()));
        let notes = position(
            &report,
            "widgets/notes/view/panel.rs:7  widgets::notes::state  [known]",
        );
        let tabs = position(
            &report,
            "widgets/tabs/view/list.rs:4  widgets::tabs::state  [known]",
        );
        let reason = position(&report, "the panel still reads the state");

        assert!(
            heading < notes && notes < reason && reason < tabs,
            "{report}"
        );
    }

    #[test]
    fn an_exception_is_tagged_as_one() {
        let report = render(&clean(), Output::Plain);

        assert!(
            report.contains("router/command.rs:24  route  [exception]"),
            "{report}"
        );
    }

    #[test]
    fn the_table_counts_each_rule_and_leaves_out_rules_without_findings() {
        let report = render(&with_action_needed(), Output::Plain);

        let row = |name: &str| {
            report
                .lines()
                .find(|line| line.starts_with(name) && !line.contains(':'))
                .map(|line| line.split_whitespace().skip(1).collect::<Vec<_>>())
        };

        assert_eq!(row("alpha"), Some(vec!["2", "0", "0", "0"]));
        assert_eq!(row("beta"), Some(vec!["0", "1", "0", "0"]));
        assert_eq!(row("delta"), Some(vec!["0", "0", "1", "0"]));
        assert_eq!(row("gamma"), Some(vec!["0", "0", "0", "1"]));
        assert!(
            report.contains(&format!(
                "{} rules have no findings",
                Rule::ALL.len() - 4
            )),
            "{report}"
        );
    }

    #[test]
    fn the_last_line_says_what_to_do() {
        let needs_action = render(&with_action_needed(), Output::Plain);
        let clean = render(&clean(), Output::Plain);

        let last = |report: &str| {
            report.lines().last().unwrap_or_default().to_string()
        };

        assert!(
            last(&needs_action).contains("fix the NEW places"),
            "{needs_action}"
        );
        assert!(
            last(&needs_action).contains("delete the STALE entries"),
            "{needs_action}"
        );
        assert!(last(&clean).starts_with("Nothing new or stale."), "{clean}");
        assert!(!clean.contains("Needs action"), "{clean}");
    }

    #[test]
    fn the_exit_code_is_one_only_when_something_needs_action() {
        assert_eq!(exit_code(&with_action_needed()), 1);
        assert_eq!(exit_code(&clean()), 0);
    }

    #[test]
    fn only_terminal_output_has_colours() {
        let plain = render(&with_action_needed(), Output::Plain);
        let terminal = render(&with_action_needed(), Output::Terminal);

        assert!(!plain.contains('\u{1b}'), "{plain}");
        assert!(terminal.contains("\u{1b}["), "{terminal}");
        assert!(terminal.contains("NEW"), "{terminal}");
    }
}

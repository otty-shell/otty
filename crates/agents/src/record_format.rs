//! Whether OTTY still understands the records an agent writes (spec
//! 2026-11-19, Decision 9).
//!
//! Agents change their files without notice. A reader that quietly skips
//! what it does not understand would show fewer tokens as if true, so each
//! reader counts the records of the kind it reads that it saw and the ones
//! it understood, and notes the agent's version written in the file and
//! what it could not read: the names of the fields that were missing, and
//! of the fields found where they were looked for. Never a value.
//!
//! The newest files decide: once they hold enough such records and fewer
//! than half of them were understood, the format changed. A few odd lines,
//! such as the last line of a file the agent is still writing, never do.

use std::collections::BTreeSet;

use serde_json::Value;

/// The fewest records of the kind a reader reads, in the newest files, the
/// verdict is made on.
const FEWEST_RECORDS: u64 = 5;

/// The most field names a report lists.
const MOST_FIELDS: usize = 20;

/// The longest field name a report lists: a longer key is data, such as a
/// file path used as a key, not the name of a field.
const LONGEST_FIELD_NAME: usize = 40;

/// The kind of file a reader reads, as a report names it, and the newest
/// version of the agent the reader was checked against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RecordFormat {
    file_kind: &'static str,
    tested_version: &'static str,
}

impl RecordFormat {
    /// The files of `file_kind`, such as "session file
    /// (projects/<project>/<session>.jsonl)", read up to the agent's
    /// version `tested_version`.
    pub(crate) const fn new(
        file_kind: &'static str,
        tested_version: &'static str,
    ) -> Self {
        Self {
            file_kind,
            tested_version,
        }
    }
}

/// What a reader saw in one file: the records of the kind it reads, the
/// ones it understood, the agent's version written in the file, and what
/// it could not read in the others.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RecordCounts {
    seen: u64,
    understood: u64,
    version: Option<String>,
    missing: BTreeSet<&'static str>,
    fields: BTreeSet<String>,
}

impl RecordCounts {
    /// The agent's version written in the file, when it writes one.
    pub(crate) fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    /// One record of the kind was seen and understood.
    pub(crate) fn add_understood(&mut self) {
        self.seen += 1;
        self.understood += 1;
    }

    /// One record of the kind was seen but not understood: `missing` names
    /// the field it needs that was missing or of another shape, such as
    /// `message.usage`, and `found` is what was found at `place`, the field
    /// it was looked for in (empty for the record itself). Only the names
    /// of the fields of `found` are kept, never their values.
    pub(crate) fn add_not_understood(
        &mut self,
        missing: &'static str,
        place: &str,
        found: Option<&Value>,
    ) {
        self.seen += 1;
        self.missing.insert(missing);

        let Some(Value::Object(fields)) = found else {
            return;
        };
        for name in fields.keys().filter(|name| is_field_name(name)) {
            if self.fields.len() >= MOST_FIELDS {
                break;
            }
            let field = if place.is_empty() {
                name.clone()
            } else {
                format!("{place}.{name}")
            };
            self.fields.insert(field);
        }
    }

    /// The agent's `version` was written in the file; the last one read
    /// stays.
    pub(crate) fn saw_version(&mut self, version: &str) {
        if !version.is_empty() && self.version.as_deref() != Some(version) {
            self.version = Some(version.to_string());
        }
    }

    /// Adds what `other` saw, a file older than the ones seen so far: the
    /// version of the newest file stays.
    fn add(&mut self, other: &Self) {
        self.seen += other.seen;
        self.understood += other.understood;
        if self.version.is_none() {
            self.version.clone_from(&other.version);
        }
        self.missing.extend(other.missing.iter().copied());
        for field in &other.fields {
            if self.fields.len() >= MOST_FIELDS {
                break;
            }
            self.fields.insert(field.clone());
        }
    }
}

#[cfg(test)]
impl RecordCounts {
    /// How many records of the kind the reader reads it saw.
    pub(crate) fn seen(&self) -> u64 {
        self.seen
    }

    /// How many of them it understood.
    pub(crate) fn understood(&self) -> u64 {
        self.understood
    }

    /// The fields that were missing or of another shape.
    pub(crate) fn missing(&self) -> Vec<&'static str> {
        self.missing.iter().copied().collect()
    }

    /// The names of the fields found where those were looked for.
    pub(crate) fn fields(&self) -> &BTreeSet<String> {
        &self.fields
    }
}

/// The records of an agent's files OTTY no longer understands: the agent
/// changed their format (spec 2026-11-19, Decision 9). It holds the names
/// of fields and counts, never a value of a record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatChange {
    format: RecordFormat,
    counts: RecordCounts,
}

impl FormatChange {
    /// The agent's version written in the newest files, when it writes
    /// one, such as "2.2.0".
    ///
    /// ```
    /// use otty_agents::fixtures::format_change;
    ///
    /// let change = format_change(Some("2.2.0"), 40, 0);
    ///
    /// assert_eq!(change.version(), Some("2.2.0"));
    /// ```
    pub fn version(&self) -> Option<&str> {
        self.counts.version()
    }

    /// The newest version of the agent this OTTY was checked against.
    pub fn tested_version(&self) -> &'static str {
        self.format.tested_version
    }

    /// How many records of the kind OTTY reads the newest files hold.
    pub fn seen(&self) -> u64 {
        self.counts.seen
    }

    /// How many of them OTTY understood.
    pub fn understood(&self) -> u64 {
        self.counts.understood
    }

    /// The plain text report of the change, for the user to send to the
    /// makers of OTTY: the agent named `agent`, its version, the `folder`
    /// of the profile as the user reads it, the kind of file, the counts
    /// and the names of the fields that were missing or found in their
    /// place. It is in English, the language of OTTY's issues, and holds
    /// no key, token or text of a session.
    ///
    /// ```
    /// use otty_agents::fixtures::format_change;
    ///
    /// let report =
    ///     format_change(Some("2.2.0"), 40, 0).report("Claude Code", "~/.claude");
    ///
    /// assert!(report.contains("Version: 2.2.0"));
    /// ```
    pub fn report(&self, agent: &str, folder: &str) -> String {
        let counts = &self.counts;
        let version = counts.version().unwrap_or("not written in the files");
        let missing: Vec<&str> = counts.missing.iter().copied().collect();
        let fields: Vec<&str> =
            counts.fields.iter().map(String::as_str).collect();

        let mut lines = vec![
            format!("OTTY could not read the files of {agent}."),
            String::new(),
            format!("Agent: {agent}"),
            format!("Version: {version}"),
            format!("Tested up to: {}", self.format.tested_version),
            format!("Folder: {folder}"),
            format!("File kind: {}", self.format.file_kind),
            format!(
                "Records: {} seen, {} understood",
                counts.seen, counts.understood
            ),
            format!("Missing or changed: {}", missing.join(", ")),
        ];
        if !fields.is_empty() {
            lines.push(format!("Fields found there: {}", fields.join(", ")));
        }

        lines.join("\n")
    }
}

/// The change of the format the newest files show, judged by the
/// `newest_first` counts of the files read with `format`: none until the
/// newest files hold [`FEWEST_RECORDS`] records of the kind, and none when
/// at least half of them were understood.
pub(crate) fn format_change<'a>(
    format: RecordFormat,
    newest_first: impl IntoIterator<Item = &'a RecordCounts>,
) -> Option<FormatChange> {
    let mut newest = RecordCounts::default();
    for counts in newest_first {
        newest.add(counts);
        if newest.seen >= FEWEST_RECORDS {
            break;
        }
    }

    let is_changed = newest.seen >= FEWEST_RECORDS
        && newest.understood.saturating_mul(2) < newest.seen;
    is_changed.then_some(FormatChange {
        format,
        counts: newest,
    })
}

/// Whether `name`, a key of a record, is the name of a field and not data:
/// short, and only ASCII letters, digits, `_` and `-`.
fn is_field_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= LONGEST_FIELD_NAME
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;

    const FORMAT: RecordFormat =
        RecordFormat::new("session file (projects/*/*.jsonl)", "2.1.328");

    fn counts(understood: u64, not_understood: u64) -> RecordCounts {
        let mut counts = RecordCounts::default();
        for _ in 0..understood {
            counts.add_understood();
        }
        for _ in 0..not_understood {
            counts.add_not_understood("message.usage", "message", None);
        }
        counts
    }

    #[test]
    fn records_that_are_not_understood_change_the_format() {
        let mut file = counts(0, 40);
        file.saw_version("2.2.0");

        let change = format_change(FORMAT, [&file]);

        let change = change.expect("no record was understood");
        assert_eq!(change.version(), Some("2.2.0"));
        assert_eq!(change.tested_version(), "2.1.328");
        assert_eq!((change.seen(), change.understood()), (40, 0));
    }

    #[test]
    fn far_fewer_understood_records_change_the_format() {
        assert!(format_change(FORMAT, [&counts(4, 36)]).is_some());
        assert!(format_change(FORMAT, [&counts(19, 21)]).is_some());
    }

    #[test]
    fn a_few_odd_lines_do_not_change_the_format() {
        let one_odd = counts(39, 1);
        let half = counts(20, 20);
        let too_few = counts(0, 4);

        assert_eq!(format_change(FORMAT, [&one_odd]), None);
        assert_eq!(format_change(FORMAT, [&half]), None);
        assert_eq!(format_change(FORMAT, [&too_few]), None);
        assert_eq!(format_change(FORMAT, []), None);
    }

    /// The newest files decide: old files read well do not hide new
    /// records OTTY does not understand, and new files read well do not
    /// keep an old odd file a change.
    #[test]
    fn the_newest_files_decide() {
        let old = counts(1000, 0);
        let new = counts(0, 6);

        let after_update = format_change(FORMAT, [&new, &old]);
        let after_fix = format_change(FORMAT, [&old, &new]);

        assert!(after_update.is_some());
        assert_eq!(format_change(FORMAT, [&counts(0, 2), &old]), None);
        assert_eq!(after_fix, None);
    }

    /// Several small new files together make enough records.
    #[test]
    fn small_newest_files_are_judged_together() {
        let small = counts(0, 2);
        let old = counts(1000, 0);

        let change = format_change(FORMAT, [&small, &small, &small, &old]);

        let change = change.expect("six new records, none understood");
        assert_eq!((change.seen(), change.understood()), (6, 0));
    }

    /// Spec 2026-11-19, Decision 9: the report names the agent, its
    /// version, the folder, the kind of file and what was not understood,
    /// and holds no value of a record.
    #[test]
    fn the_report_holds_names_and_counts_but_no_value_of_a_record() {
        let record: Value = serde_json::from_str(
            r#"{"id":"msg_secret","model":"claude-opus-9","role":"assistant","content":[{"type":"text","text":"the prompt sk-ant-123"}],"tokenUsage":{"in":5},"/home/user/secret":1}"#,
        )
        .expect("a made-up record");
        let mut file = RecordCounts::default();
        for _ in 0..6 {
            file.add_not_understood("message.usage", "message", Some(&record));
        }
        file.add_understood();
        file.saw_version("2.2.0");

        let change = format_change(FORMAT, [&file]).expect("a change");
        let report = change.report("Claude Code", "~/.claude-work");

        for line in [
            "Agent: Claude Code",
            "Version: 2.2.0",
            "Tested up to: 2.1.328",
            "Folder: ~/.claude-work",
            "File kind: session file (projects/*/*.jsonl)",
            "Records: 7 seen, 1 understood",
            "Missing or changed: message.usage",
            "Fields found there: message.content, message.id, message.model, \
             message.role, message.tokenUsage",
        ] {
            assert!(report.contains(line), "{line} in {report}");
        }
        for value in [
            "msg_secret",
            "claude-opus-9",
            "sk-ant",
            "prompt",
            "secret",
            "5",
        ] {
            assert!(!report.contains(value), "{value} in {report}");
        }
    }

    #[test]
    fn a_report_says_when_the_files_name_no_version() {
        let change = format_change(FORMAT, [&counts(0, 5)]).expect("a change");

        let report = change.report("Codex", "~/.codex");

        assert!(report.contains("Version: not written in the files"));
    }

    #[test]
    fn only_short_plain_keys_are_field_names() {
        assert!(is_field_name("cache_read_input_tokens"));
        assert!(is_field_name("tokenUsage"));
        assert!(!is_field_name(""));
        assert!(!is_field_name("/home/user/project/file.rs"));
        assert!(!is_field_name("a key with spaces"));
        assert!(!is_field_name(&"x".repeat(41)));
    }
}

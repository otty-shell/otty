//! What a part saw in one file, for OTTY's "format changed" verdict: the
//! records of the kind it reads, the ones it understood, and the names
//! (never the values) of the fields it missed and found.

use crate::agents::{RecordCounts, TokenCount};

/// The most field names the counts keep.
const MOST_FIELDS: usize = 20;

/// The longest field name the counts keep: a longer key is data, such as
/// a file path used as a key, not the name of a field.
const LONGEST_FIELD_NAME: usize = 40;

impl RecordCounts {
    /// One record of the kind was seen and understood.
    pub fn add_understood(&mut self) {
        self.seen += 1;
        self.understood += 1;
    }

    /// One record of the kind was seen but not understood: `missing` names
    /// the field it needs that was missing or of another shape, such as
    /// `message.usage`, and `found` are the names of the fields found at
    /// `place`, where it was looked for (empty for the record itself).
    /// Only names are kept, never values; a name that looks like data is
    /// left out.
    ///
    /// ```
    /// use otty_plugin_sdk::agents::RecordCounts;
    ///
    /// let mut counts = RecordCounts::default();
    /// counts.add_understood();
    /// counts.add_not_understood("message.usage", "message", ["id", "model"]);
    ///
    /// assert_eq!((counts.seen, counts.understood), (2, 1));
    /// assert_eq!(counts.missing, ["message.usage"]);
    /// assert_eq!(counts.fields, ["message.id", "message.model"]);
    /// ```
    pub fn add_not_understood<'name>(
        &mut self,
        missing: &str,
        place: &str,
        found: impl IntoIterator<Item = &'name str>,
    ) {
        self.seen += 1;
        if !self.missing.iter().any(|known| known == missing) {
            self.missing.push(missing.to_string());
        }

        for name in found.into_iter().filter(|name| is_field_name(name)) {
            if self.fields.len() >= MOST_FIELDS {
                break;
            }
            let field = if place.is_empty() {
                name.to_string()
            } else {
                format!("{place}.{name}")
            };
            if !self.fields.contains(&field) {
                self.fields.push(field);
            }
        }
    }

    /// One record was seen that is not even a record of the file's kind,
    /// such as a line that is not JSON.
    pub fn add_broken(&mut self) {
        self.add_not_understood("record", "", []);
    }

    /// The agent's `version` was written in the file; the last one read
    /// stays.
    pub fn saw_version(&mut self, version: &str) {
        if !version.is_empty() {
            self.version = Some(version.to_string());
        }
    }
}

impl Default for RecordCounts {
    /// Nothing seen yet.
    fn default() -> Self {
        Self {
            seen: 0,
            understood: 0,
            version: None,
            missing: Vec::new(),
            fields: Vec::new(),
        }
    }
}

impl Default for TokenCount {
    /// No tokens.
    fn default() -> Self {
        Self {
            input: 0,
            cached: 0,
            cache_written: 0,
            output: 0,
            reasoning: 0,
        }
    }
}

/// Whether `name` looks like the name of a field, not like data.
fn is_field_name(name: &str) -> bool {
    !name.is_empty() && name.chars().count() <= LONGEST_FIELD_NAME
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn understood_records_count_as_seen_too() {
        let mut counts = RecordCounts::default();

        counts.add_understood();
        counts.add_understood();

        assert_eq!((counts.seen, counts.understood), (2, 2));
        assert!(counts.missing.is_empty());
    }

    #[test]
    fn a_missing_field_is_named_once() {
        let mut counts = RecordCounts::default();

        counts.add_not_understood("message.usage", "message", ["id"]);
        counts.add_not_understood("message.usage", "message", ["id"]);

        assert_eq!(counts.seen, 2);
        assert_eq!(counts.understood, 0);
        assert_eq!(counts.missing, ["message.usage"]);
        assert_eq!(counts.fields, ["message.id"]);
    }

    #[test]
    fn names_that_look_like_data_are_left_out() {
        let mut counts = RecordCounts::default();
        let path = "/home/user/projects/a-very-long-folder-name/session.jsonl";

        counts.add_not_understood("usage", "", ["", path, "model"]);

        assert_eq!(counts.fields, ["model"]);
    }

    #[test]
    fn at_most_twenty_field_names_are_kept() {
        let mut counts = RecordCounts::default();
        let names: Vec<String> =
            (0..30).map(|number| format!("field{number}")).collect();

        counts.add_not_understood(
            "usage",
            "",
            names.iter().map(String::as_str),
        );

        assert_eq!(counts.fields.len(), MOST_FIELDS);
    }

    #[test]
    fn a_broken_record_is_seen_and_not_understood() {
        let mut counts = RecordCounts::default();

        counts.add_broken();

        assert_eq!((counts.seen, counts.understood), (1, 0));
        assert_eq!(counts.missing, ["record"]);
    }

    #[test]
    fn the_last_version_read_stays() {
        let mut counts = RecordCounts::default();

        counts.saw_version("1.0.0");
        counts.saw_version("");
        counts.saw_version("1.1.0");

        assert_eq!(counts.version.as_deref(), Some("1.1.0"));
    }
}

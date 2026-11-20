//! The answers of a profile, read from its database: each answer with its
//! provider, model, tokens, the cost OpenCode wrote and the folder of its
//! session.
//!
//! The database is the profile's one spend place, together with its
//! write-ahead log, `opencode.db-wal`: OpenCode writes new rows to the log
//! first and copies them into the database only now and then, so the
//! database file alone may not change for a long time. When either file
//! changes, the whole database is read; each answer carries the id of its
//! message, so OTTY counts it once.

use std::collections::BTreeMap;

use otty_plugin_sdk::{
    Answers, ReadError, RecordCounts, SpendItem, SpendKind, SpendPlace, clock,
};

use crate::database::{Database, SessionRow};
use crate::glob::escaped;
use crate::message::{Record, read_record};
use crate::sessions::newest_version;

/// What follows the name of the database in the name of its write-ahead
/// log.
const LOG_SUFFIX: &str = "-wal";

/// How many days back the answers are read: OTTY counts the tokens of the
/// last 30 days, or of the current month on its 31st day; three days more
/// cover any time zone.
const READ_DAYS: i64 = 33;

/// Milliseconds in a day.
const DAY_MS: i64 = 24 * 3600 * 1000;

/// How many of the newest answers tell the format OpenCode writes now: old
/// answers in an older format neither hide a change nor make one.
const NEWEST_RECORDS: usize = 20;

/// A record of the kind OTTY reads, as it counts for the format: when it
/// was written and, for one not understood, what was missing.
struct Seen {
    at_ms: i64,
    not_understood: Option<(&'static str, &'static str, Vec<String>)>,
}

/// Where the answers of the profile whose database is `database` are: the
/// database and its write-ahead log.
pub(crate) fn spend_places(database: &str) -> Vec<SpendPlace> {
    let database = escaped(database);

    vec![
        SpendPlace {
            glob: database.clone(),
            kind: SpendKind::Database,
        },
        SpendPlace {
            glob: format!("{database}{LOG_SUFFIX}"),
            kind: SpendKind::Database,
        },
    ]
}

/// The answers of the last days in the database of `item`, the database
/// itself or its write-ahead log. No database gives no answers.
pub(crate) fn read_answers(item: &SpendItem) -> Result<Answers, ReadError> {
    let path = item.path.strip_suffix(LOG_SUFFIX).unwrap_or(&item.path);
    let since_ms = clock::now_ms().saturating_sub(READ_DAYS * DAY_MS);

    let Some(database) = Database::open(path).map_err(ReadError::Host)? else {
        return Ok(Answers {
            answers: Vec::new(),
            records: RecordCounts::default(),
        });
    };
    let sessions = database.sessions().map_err(ReadError::Host)?;
    let folders = folders_of(&sessions);

    let mut answers = Vec::new();
    let mut seen = Vec::new();
    database
        .for_each_message(since_ms, |row| match read_record(&row) {
            Record::Answer(mut answer) => {
                answer.project =
                    folders.get(row.session_id.as_str()).map(|f| f.to_string());
                seen.push(Seen {
                    at_ms: row.at_ms,
                    not_understood: None,
                });
                answers.push(*answer);
            },
            Record::NotUnderstood {
                missing,
                place,
                found,
            } => seen.push(Seen {
                at_ms: row.at_ms,
                not_understood: Some((missing, place, found)),
            }),
            _ => {},
        })
        .map_err(ReadError::Host)?;

    let mut records = newest_counts(seen);
    if let Some(version) = newest_version(&sessions) {
        records.saw_version(&version);
    }

    Ok(Answers { answers, records })
}

/// The folder of each session of `sessions` that names one, by its id. A
/// `BTreeMap`, not a `HashMap`: the seed of a `HashMap` would make the
/// `.wasm` ask for `wasi:random`, which OTTY does not give.
fn folders_of(sessions: &[SessionRow]) -> BTreeMap<&str, &str> {
    sessions
        .iter()
        .map(|session| (session.id.as_str(), session.directory.trim()))
        .filter(|(_, folder)| !folder.is_empty())
        .collect()
}

/// The counts of the [`NEWEST_RECORDS`] newest records of `seen`.
fn newest_counts(mut seen: Vec<Seen>) -> RecordCounts {
    seen.sort_by_key(|record| std::cmp::Reverse(record.at_ms));

    let mut counts = RecordCounts::default();
    for record in seen.into_iter().take(NEWEST_RECORDS) {
        match record.not_understood {
            None => counts.add_understood(),
            Some((missing, place, found)) => counts.add_not_understood(
                missing,
                place,
                found.iter().map(String::as_str),
            ),
        }
    }

    counts
}

#[cfg(test)]
mod tests {
    use otty_plugin_sdk::ProviderRef;

    use super::*;
    use crate::testing::{
        DAY, DataFolder, MadeUpDatabase, NOW, first_shape_answer,
        first_shape_question, millis, second_shape_answer,
    };

    /// A made-up data folder with `ses_1` in `/home/user/otty` and its
    /// helper session `ses_2`.
    fn data_folder() -> (DataFolder, MadeUpDatabase) {
        let data = DataFolder::new();
        let database = data.first_shape();
        database.session("ses_1", None, "/home/user/otty", "1.4.0", (NOW, NOW));
        database.session(
            "ses_2",
            Some("ses_1"),
            "/home/user/otty",
            "1.4.0",
            (NOW, NOW),
        );

        (data, database)
    }

    fn openai(input: u64, cost: f64) -> String {
        first_shape_answer("openai", "gpt-5.5", (input, 2, 1, 100, 0), cost)
    }

    fn read(data: &DataFolder, name: &str) -> Answers {
        let item = SpendItem {
            path: data.file(name),
            kind: SpendKind::Database,
        };

        data.run(|| read_answers(&item))
            .expect("the database is read")
    }

    #[test]
    fn the_spend_places_are_the_database_and_its_log() {
        let places = spend_places("/data/[x]/opencode.db");

        let globs: Vec<&str> =
            places.iter().map(|place| place.glob.as_str()).collect();
        assert_eq!(
            globs,
            ["/data/\\[x\\]/opencode.db", "/data/\\[x\\]/opencode.db-wal"]
        );
        assert!(places.iter().all(|place| place.kind == SpendKind::Database));
    }

    #[test]
    fn answers_have_their_provider_model_folder_and_the_cost_opencode_wrote() {
        let (data, mut database) = data_folder();
        database.message("msg_1", "ses_1", "", NOW, &openai(10, 0.25));
        database.message("msg_2", "ses_2", "", NOW - 60, &openai(20, 0.5));
        database.message(
            "msg_3",
            "ses_1",
            "",
            NOW - 120,
            &first_shape_question(),
        );
        let local =
            first_shape_answer("test", "qwen-local", (7, 1, 0, 0, 0), 0.0);
        database.message("msg_4", "ses_1", "", NOW - DAY, &local);
        database.message("msg_5", "ses_gone", "", NOW, &openai(1, 0.0));

        let read = read(&data, "opencode.db");

        let ids: Vec<&str> = read
            .answers
            .iter()
            .filter_map(|answer| answer.identity.as_deref())
            .collect();
        assert_eq!(ids, ["msg_1", "msg_2", "msg_4", "msg_5"]);
        let first = &read.answers[0];
        assert_eq!(first.at_ms, millis(NOW));
        assert_eq!(first.model.as_deref(), Some("gpt-5.5"));
        assert_eq!(first.provider, Some(ProviderRef::Id("openai".to_string())));
        assert_eq!(first.login, None, "OpenCode does not write the login");
        assert_eq!(first.project.as_deref(), Some("/home/user/otty"));
        assert_eq!(first.cost_microdollars, Some(250_000));
        assert_eq!(read.answers[1].project.as_deref(), Some("/home/user/otty"));
        assert_eq!(read.answers[2].provider, None, "a local model");
        assert_eq!(read.answers[3].project, None, "a session that is gone");
        assert_eq!((read.records.seen, read.records.understood), (4, 4));
        assert_eq!(read.records.version.as_deref(), Some("1.4.0"));
    }

    /// A change in the log is a change of the database: the log is never
    /// read as a database of its own.
    #[test]
    fn the_log_reads_the_database_it_belongs_to() {
        let (data, mut database) = data_folder();
        database.message("msg_1", "ses_1", "", NOW, &openai(10, 0.25));

        let read = read(&data, "opencode.db-wal");

        assert_eq!(read.answers.len(), 1);
    }

    #[test]
    fn answers_of_both_shapes_are_read() {
        let data = DataFolder::new();
        let mut database = data.second_shape();
        database.session("ses_1", None, "/home/user/site", "2.3.2", (NOW, NOW));
        let answer =
            second_shape_answer("anthropic", "claude-x", (1, 1, 0, 0, 0), 0.1);
        database.message("msg_1", "ses_1", "assistant", NOW, &answer);
        database.message("msg_2", "ses_1", "user", NOW, "{}");

        let read = read(&data, "opencode.db");

        assert_eq!(read.answers.len(), 1);
        assert_eq!(read.answers[0].project.as_deref(), Some("/home/user/site"));
        assert_eq!(read.records.version.as_deref(), Some("2.3.2"));
    }

    #[test]
    fn answers_older_than_the_counted_days_are_left_out() {
        let (data, mut database) = data_folder();
        database.message(
            "msg_1",
            "ses_1",
            "",
            NOW - 40 * DAY,
            &openai(10, 0.25),
        );
        database.message(
            "msg_2",
            "ses_1",
            "",
            NOW - 30 * DAY,
            &openai(10, 0.25),
        );

        let read = read(&data, "opencode.db");

        assert_eq!(read.answers.len(), 1);
        assert_eq!(read.answers[0].identity.as_deref(), Some("msg_2"));
    }

    #[test]
    fn no_database_has_no_answers() {
        let data = DataFolder::new();

        let read = read(&data, "opencode.db");

        assert!(read.answers.is_empty());
        assert_eq!(read.records.seen, 0);
    }

    /// The newest answers tell the format: renamed tokens in most of them
    /// are counted as not understood, with the names of the fields found,
    /// however many older answers were understood.
    #[test]
    fn renamed_tokens_in_the_newest_answers_are_not_understood() {
        let (data, mut database) = data_folder();
        let renamed = r#"{"role":"assistant","modelID":"m","providerID":"openai",
            "usage":{"in":1},"finish":"stop"}"#;
        for number in 0..12 {
            database.message(
                &format!("msg_{number}"),
                "ses_1",
                "",
                NOW - number,
                renamed,
            );
        }
        for number in 0..30 {
            database.message(
                &format!("msg_old_{number}"),
                "ses_1",
                "",
                NOW - DAY - number,
                &openai(10, 0.25),
            );
        }

        let read = read(&data, "opencode.db");

        assert_eq!(read.records.seen, NEWEST_RECORDS as u64);
        assert_eq!(read.records.understood, NEWEST_RECORDS as u64 - 12);
        assert_eq!(read.records.missing, ["tokens"]);
        assert!(read.records.fields.contains(&"usage".to_string()));
        assert_eq!(read.answers.len(), 30);
    }
}

//! The tokens and the money a profile of OpenCode or Kilo spent, counted
//! from the answers in its database (spec 2026-11-19, Decision 4): each
//! answer under the provider and the model it names, with the cost the
//! agent wrote.

use std::cmp::Reverse;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use super::database::{Database, MessageRow, SessionRow};
use super::location::DatabaseLocation;
use super::message::{Record, read_record};
use super::provider_ids::provider_of;
use crate::error::SourceError;
use crate::local_time::LocalNow;
use crate::profile::Profile;
use crate::record_format::{RecordCounts, format_change};
use crate::spend::{Spend, SpendCounter};
use crate::spend_files::{CountedAnswer, FileSpend};
use crate::spend_tally::{first_counted_day, tally};

/// The answers of one session through one provider, and how well its
/// messages were understood, while they are gathered.
#[derive(Default)]
struct SessionAnswers {
    newest_ms: i64,
    answers: Vec<CountedAnswer>,
    records: RecordCounts,
}

/// Counts the tokens of a profile from its database.
pub(super) struct DatabaseSpend {
    location: Arc<DatabaseLocation>,
}

impl DatabaseSpend {
    /// A counter that finds the database of a profile with `location`.
    pub(super) fn new(location: Arc<DatabaseLocation>) -> Self {
        Self { location }
    }
}

impl SpendCounter for DatabaseSpend {
    /// No database means nothing was spent yet; one that can not be read
    /// is "can not read". When the newest answers are not understood, the
    /// agent changed their format.
    fn spend(
        &self,
        profile: &Profile,
        now: &LocalNow,
    ) -> Result<Spend, SourceError> {
        let since = now.start_of(first_counted_day(now));
        let path = self.location.database_of(profile);

        let Some(database) = Database::open(&path)? else {
            return Ok(tally(std::iter::empty(), now, None));
        };
        let sessions = database.sessions()?;
        let messages = database.messages(millis_of(since))?;

        let spends = answers_by_session(&sessions, &messages);
        let format = self.location.fork().format();
        if let Some(change) =
            format_change(format, spends.iter().map(FileSpend::records))
        {
            return Err(SourceError::FormatChanged(Box::new(change)));
        }

        Ok(tally(spends.iter(), now, None))
    }
}

/// The answers among `messages`, one [`FileSpend`] for each session and
/// provider, the newest first: each with the folder of its session in
/// `sessions`, the provider the answers name, and how well the messages
/// were understood, with the version of the agent that wrote the session.
/// A message that is no answer is left out.
pub(super) fn answers_by_session(
    sessions: &[SessionRow],
    messages: &[MessageRow],
) -> Vec<FileSpend> {
    let mut gathered: HashMap<(&str, Option<String>), SessionAnswers> =
        HashMap::new();

    for message in messages {
        let record = read_record(message);
        let provider = match &record {
            Record::Answer { provider, .. } => provider.clone(),
            Record::NotUnderstood { .. } => None,
            _ => continue,
        };

        let key = (message.session_id.as_str(), provider);
        let kept = gathered.entry(key).or_default();
        kept.newest_ms = kept.newest_ms.max(message.at_ms);
        match record {
            Record::Answer { answer, .. } => {
                kept.records.add_understood();
                kept.answers.push(answer);
            },
            Record::NotUnderstood {
                missing,
                place,
                found,
            } => {
                kept.records
                    .add_not_understood(missing, place, found.as_ref())
            },
            _ => {},
        }
    }

    let mut spends: Vec<(i64, FileSpend)> = gathered
        .into_iter()
        .map(|((session_id, provider), kept)| {
            let session = sessions.iter().find(|row| row.id == session_id);
            (
                kept.newest_ms,
                file_spend(session, provider.as_deref(), kept),
            )
        })
        .collect();

    spends.sort_by_key(|(newest_ms, _)| Reverse(*newest_ms));
    spends.into_iter().map(|(_, spend)| spend).collect()
}

/// `time` in milliseconds since 1970; 0 for a time before it.
pub(super) fn millis_of(time: SystemTime) -> i64 {
    let millis = time
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_millis());

    i64::try_from(millis).unwrap_or(i64::MAX)
}

/// The spend of the answers `kept` of `session` through the provider
/// `provider`.
fn file_spend(
    session: Option<&SessionRow>,
    provider: Option<&str>,
    mut kept: SessionAnswers,
) -> FileSpend {
    let project = session
        .map(|session| session.directory.trim())
        .filter(|directory| !directory.is_empty())
        .map(PathBuf::from);
    if let Some(version) =
        session.and_then(|session| session.version.as_deref())
    {
        kept.records.saw_version(version);
    }

    FileSpend::new(project, kept.answers)
        .with_provider(provider.and_then(provider_of))
        .with_records(kept.records)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::agent::AgentId;
    use crate::dollars::Dollars;
    use crate::open_code::fork::KILO;
    use crate::open_code::testing::{
        MadeUpDatabase, first_shape_answer, first_shape_question,
    };
    use crate::provider::ProviderRef;
    use crate::testing::{NOW, TempDir, utc_now};
    use crate::token_count::TokenCount;

    const DAY: u64 = 24 * 3600;

    /// A made-up Kilo data folder with its database and a counter for it.
    struct DataDir {
        temp: TempDir,
        database: MadeUpDatabase,
        counter: DatabaseSpend,
    }

    impl DataDir {
        fn path(&self) -> PathBuf {
            self.temp.path().join("kilo")
        }

        fn new() -> Self {
            let temp = TempDir::new();
            let database =
                MadeUpDatabase::first_shape(&temp.path().join("kilo/kilo.db"));
            database.session(
                "ses_1",
                None,
                "/home/user/otty",
                "7.8.27",
                (NOW, NOW),
            );
            database.session(
                "ses_2",
                Some("ses_1"),
                "/home/user/otty",
                "7.8.27",
                (NOW, NOW),
            );
            let counter =
                DatabaseSpend::new(Arc::new(DatabaseLocation::new(&KILO)));

            Self {
                temp,
                database,
                counter,
            }
        }

        fn answer(&mut self, id: &str, session: &str, at: u64, data: &str) {
            self.database.message(id, session, "", at, data);
        }

        fn spend(&self) -> Result<Spend, SourceError> {
            let profile =
                Profile::added(AgentId::new("kilo"), "Kilo", self.path());

            self.counter.spend(&profile, &utc_now())
        }
    }

    fn openai(input: u64, cost: f64) -> String {
        first_shape_answer("openai", "gpt-5.5", (input, 2, 1, 100, 0), cost)
    }

    #[test]
    fn answers_count_by_provider_and_model_with_the_cost_the_agent_wrote() {
        let mut data = DataDir::new();
        data.answer("msg_1", "ses_1", NOW, &openai(10, 0.25));
        data.answer("msg_2", "ses_2", NOW - 60, &openai(20, 0.5));
        data.answer("msg_3", "ses_1", NOW - 120, &first_shape_question());
        let local =
            first_shape_answer("test", "qwen-local", (7, 1, 0, 0, 0), 0.0);
        data.answer("msg_4", "ses_1", NOW - DAY, &local);

        let spend = data.spend().expect("the database is read");

        let today = &spend.last_30_days_by_day()[29];
        assert_eq!(today.models().len(), 1);
        let gpt = &today.models()[0];
        assert_eq!(gpt.model(), Some("gpt-5.5"));
        assert_eq!(gpt.provider(), Some(&ProviderRef::of_id("openai")));
        assert_eq!(gpt.account(), None, "the agent does not write the account");
        assert_eq!(gpt.tokens(), TokenCount::new(30, 200, 6));
        assert_eq!(gpt.requests(), 2);
        assert_eq!(gpt.cost(), Some(Dollars::from_micro_dollars(750_000)));
        let yesterday = &spend.last_30_days_by_day()[28];
        assert_eq!(yesterday.models()[0].provider(), None, "a local model");
        assert_eq!(yesterday.models()[0].cost(), Some(Dollars::default()));
        assert_eq!(spend.projects()[0].folder(), Path::new("/home/user/otty"));
    }

    #[test]
    fn answers_older_than_the_counted_days_are_left_out() {
        let mut data = DataDir::new();
        data.answer("msg_1", "ses_1", NOW - 40 * DAY, &openai(10, 0.25));

        let spend = data.spend().expect("the database is read");

        assert!(spend.last_30_days().is_zero());
        assert_eq!(spend.last_30_days_by_day().len(), 30);
    }

    #[test]
    fn no_database_has_spent_nothing() {
        let temp = TempDir::new();
        let counter =
            DatabaseSpend::new(Arc::new(DatabaseLocation::new(&KILO)));
        let profile = Profile::added(
            AgentId::new("kilo"),
            "Kilo",
            temp.path().join("kilo"),
        );

        let spend = counter
            .spend(&profile, &utc_now())
            .expect("nothing to read");

        assert!(spend.today().is_zero());
        assert_eq!(spend.last_30_days_by_day().len(), 30);
    }

    #[test]
    fn renamed_tokens_in_the_newest_answers_are_a_changed_format() {
        let mut data = DataDir::new();
        let renamed = r#"{"role":"assistant","modelID":"m","providerID":"openai",
            "usage":{"in":1},"finish":"stop"}"#;
        for number in 0..6 {
            data.answer(
                &format!("msg_{number}"),
                "ses_1",
                NOW - number,
                renamed,
            );
        }
        data.answer("msg_old", "ses_1", NOW - DAY, &openai(10, 0.25));

        let error = data.spend().expect_err("the format changed");

        let SourceError::FormatChanged(change) = error else {
            panic!("a changed format: {error:?}");
        };
        assert_eq!(change.version(), Some("7.8.27"));
        assert_eq!(change.tested_version(), "7.8.27");
        let report = change.report("Kilo", "~/.local/share/kilo");
        assert!(report.contains("usage"), "{report}");
        assert!(report.contains("kilo.db"), "{report}");
    }

    #[test]
    fn a_few_odd_answers_do_not_change_the_format() {
        let mut data = DataDir::new();
        data.answer(
            "msg_odd",
            "ses_1",
            NOW,
            r#"{"role":"assistant","finish":"stop"}"#,
        );
        for number in 1..6 {
            data.answer(
                &format!("msg_{number}"),
                "ses_1",
                NOW - number,
                &openai(1, 0.0),
            );
        }

        let spend = data.spend().expect("most answers are understood");

        assert_eq!(spend.today().input(), 5);
    }
}

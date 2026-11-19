//! The statistics of a session and of a project folder of OpenCode or
//! Kilo, read from the database over the whole history. A session counts
//! the tokens of the helper agents it started, as the profile's count
//! does.

use std::path::Path;
use std::sync::Arc;

use super::database::{Database, MessageRow, SessionRow};
use super::location::DatabaseLocation;
use super::message::{Record, read_record};
use super::spend::answers_by_session;
use crate::error::{SourceError, Unavailable};
use crate::local_time::LocalNow;
use crate::profile::Profile;
use crate::session::SessionId;
use crate::spend_tally::{tally, whole_history};
use crate::stats::{ProjectStats, SessionStats, StatsSource};
use crate::timestamp::time_from_millis;

/// What the database of a profile holds over the whole history.
struct History {
    sessions: Vec<SessionRow>,
    messages: Vec<MessageRow>,
}

/// Reads the statistics from the database.
pub(super) struct DatabaseStats {
    location: Arc<DatabaseLocation>,
}

impl DatabaseStats {
    /// A source that finds the database of a profile with `location`.
    pub(super) fn new(location: Arc<DatabaseLocation>) -> Self {
        Self { location }
    }

    /// The database of `profile` with its sessions and every message.
    fn read(&self, profile: &Profile) -> Result<Option<History>, SourceError> {
        let path = self.location.database_of(profile);

        let Some(database) = Database::open(&path)? else {
            return Ok(None);
        };

        Ok(Some(History {
            sessions: database.sessions()?,
            messages: database.messages(0)?,
        }))
    }
}

impl StatsSource for DatabaseStats {
    /// A session the database does not have is "not available".
    fn session_stats(
        &self,
        profile: &Profile,
        session: &SessionId,
        now: &LocalNow,
    ) -> Result<SessionStats, SourceError> {
        let missing = || {
            SourceError::NotAvailable(Unavailable::NoSessionFile(
                session.clone(),
            ))
        };

        let History { sessions, messages } =
            self.read(profile)?.ok_or_else(missing)?;
        let Some(row) = sessions.iter().find(|row| row.id == session.as_str())
        else {
            return Err(missing());
        };

        let family = family_of(&sessions, &row.id);
        let own: Vec<MessageRow> = messages
            .into_iter()
            .filter(|message| family.contains(&message.session_id.as_str()))
            .collect();
        let questions = own
            .iter()
            .filter(|message| message.session_id == row.id)
            .filter(|message| read_record(message) == Record::Question)
            .count();
        let spends = answers_by_session(&sessions, &own);
        let history = whole_history(spends.iter(), None);

        let stats = SessionStats::new(
            time_from_millis(row.created_ms as f64),
            time_from_millis(row.updated_ms as f64),
            u64::try_from(questions).unwrap_or(u64::MAX),
            history.models().to_vec(),
        );
        Ok(stats.with_spend(tally(spends.iter(), now, None)))
    }

    /// Every session in `folder`, with the sessions of their helper
    /// agents.
    fn project_stats(
        &self,
        profile: &Profile,
        folder: &Path,
    ) -> Result<ProjectStats, SourceError> {
        let Some(History { sessions, messages }) = self.read(profile)? else {
            return Ok(ProjectStats::default());
        };

        let in_folder: Vec<&str> = sessions
            .iter()
            .filter(|row| Path::new(row.directory.trim()) == folder)
            .map(|row| row.id.as_str())
            .collect();
        let own: Vec<MessageRow> = messages
            .into_iter()
            .filter(|message| in_folder.contains(&message.session_id.as_str()))
            .collect();

        let spends = answers_by_session(&sessions, &own);
        Ok(whole_history(spends.iter(), None))
    }
}

/// The ids of the session `id` and of every session it started, and so on
/// down, among `sessions`.
fn family_of<'a>(sessions: &'a [SessionRow], id: &'a str) -> Vec<&'a str> {
    let mut family = vec![id];
    let mut index = 0;

    while let Some(parent) = family.get(index).copied() {
        let children = sessions
            .iter()
            .filter(|row| row.parent_id.as_deref() == Some(parent))
            .map(|row| row.id.as_str())
            .filter(|child| !family.contains(child));
        family.extend(children.collect::<Vec<_>>());
        index += 1;
    }

    family
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::agent::AgentId;
    use crate::dollars::Dollars;
    use crate::open_code::fork::KILO;
    use crate::open_code::testing::{
        MadeUpDatabase, first_shape_answer, first_shape_question,
    };
    use crate::testing::{NOW, TempDir, at, utc_now};

    /// A made-up Kilo profile: `ses_1` in `/home/user/otty`, with the
    /// helper session `ses_2`, and `ses_3` in `/home/user/site`.
    struct Fixture {
        _temp: TempDir,
        profile: Profile,
        stats: DatabaseStats,
    }

    impl Fixture {
        fn new() -> Self {
            let temp = TempDir::new();
            let data_dir = temp.path().join("kilo");
            let mut made_up =
                MadeUpDatabase::first_shape(&data_dir.join("kilo.db"));
            made_up.session(
                "ses_1",
                None,
                "/home/user/otty",
                "7.8.27",
                (NOW - 600, NOW),
            );
            made_up.session(
                "ses_2",
                Some("ses_1"),
                "/home/user/otty",
                "7.8.27",
                (NOW, NOW),
            );
            made_up.session(
                "ses_3",
                None,
                "/home/user/site",
                "7.8.27",
                (NOW, NOW),
            );
            let answer = |input| {
                first_shape_answer(
                    "openai",
                    "gpt-5.5",
                    (input, 1, 0, 0, 0),
                    0.5,
                )
            };
            made_up.message(
                "q_1",
                "ses_1",
                "",
                NOW - 600,
                &first_shape_question(),
            );
            made_up.message(
                "q_2",
                "ses_1",
                "",
                NOW - 300,
                &first_shape_question(),
            );
            made_up.message("a_1", "ses_1", "", NOW - 590, &answer(10));
            made_up.message(
                "q_3",
                "ses_2",
                "",
                NOW - 100,
                &first_shape_question(),
            );
            made_up.message("a_2", "ses_2", "", NOW - 90, &answer(20));
            made_up.message("a_3", "ses_3", "", NOW - 80, &answer(40));

            Self {
                _temp: temp,
                profile: Profile::added(AgentId::new("kilo"), "Kilo", data_dir),
                stats: DatabaseStats::new(Arc::new(DatabaseLocation::new(
                    &KILO,
                ))),
            }
        }
    }

    #[test]
    fn a_session_counts_its_questions_and_the_tokens_of_its_helpers() {
        let fixture = Fixture::new();

        let stats = fixture
            .stats
            .session_stats(
                &fixture.profile,
                &SessionId::new("ses_1"),
                &utc_now(),
            )
            .expect("the session is there");

        assert_eq!(stats.started_at(), Some(at(NOW - 600)));
        assert_eq!(stats.last_active_at(), Some(at(NOW)));
        assert_eq!(
            stats.messages(),
            2,
            "the helper's question is not the user's"
        );
        assert_eq!(stats.answers(), 2);
        assert_eq!(stats.tokens().input(), 30);
        assert_eq!(
            stats.models()[0].cost(),
            Some(Dollars::from_micro_dollars(1_000_000))
        );
        assert_eq!(stats.spend().today().input(), 30);
    }

    #[test]
    fn a_session_the_database_does_not_have_is_not_available() {
        let fixture = Fixture::new();

        let stats = fixture.stats.session_stats(
            &fixture.profile,
            &SessionId::new("ses_9"),
            &utc_now(),
        );

        assert!(matches!(
            stats,
            Err(SourceError::NotAvailable(Unavailable::NoSessionFile(_)))
        ));
    }

    #[test]
    fn a_project_counts_every_session_in_its_folder() {
        let fixture = Fixture::new();

        let otty = fixture
            .stats
            .project_stats(&fixture.profile, &PathBuf::from("/home/user/otty"))
            .expect("the database is read");
        let site = fixture
            .stats
            .project_stats(&fixture.profile, &PathBuf::from("/home/user/site"))
            .expect("the database is read");

        assert_eq!(otty.tokens().input(), 30);
        assert_eq!(otty.first_at(), Some(at(NOW - 590)));
        assert_eq!(site.tokens().input(), 40);
    }
}

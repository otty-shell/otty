//! The sessions of a profile of OpenCode or Kilo, listed from its
//! database. The sessions of helper agents, which another session
//! started, are not listed: their tokens count with the others.

use std::path::PathBuf;
use std::sync::Arc;

use super::database::{Database, SessionRow};
use super::location::DatabaseLocation;
use crate::error::SourceError;
use crate::profile::Profile;
use crate::session::{AgentSession, SessionId, SessionSource};
use crate::source_id::SourceId;
use crate::timestamp::time_from_millis;

/// Lists the sessions from the database.
pub(super) struct DatabaseSessions {
    location: Arc<DatabaseLocation>,
}

impl DatabaseSessions {
    /// A source that finds the database of a profile with `location`.
    pub(super) fn new(location: Arc<DatabaseLocation>) -> Self {
        Self { location }
    }
}

impl SessionSource for DatabaseSessions {
    fn id(&self) -> SourceId {
        SourceId::AgentDatabase
    }

    /// No database means no sessions yet.
    fn sessions(
        &self,
        profile: &Profile,
    ) -> Result<Vec<AgentSession>, SourceError> {
        let path = self.location.database_of(profile);

        let Some(database) = Database::open(&path)? else {
            return Ok(Vec::new());
        };
        let rows = database.sessions()?;

        Ok(rows.iter().filter_map(listed_session).collect())
    }
}

/// The newest version of the agent the sessions `rows` were written by:
/// the one of the session that changed last and names one.
pub(super) fn newest_version(rows: &[SessionRow]) -> Option<String> {
    rows.iter()
        .filter(|row| row.version.as_deref().is_some_and(|v| !v.is_empty()))
        .max_by_key(|row| row.updated_ms)
        .and_then(|row| row.version.clone())
}

/// The session of `row` as the list shows it; `None` for the session of
/// a helper agent and for one with no folder.
fn listed_session(row: &SessionRow) -> Option<AgentSession> {
    if row.parent_id.is_some() {
        return None;
    }
    let directory = row.directory.trim();
    if directory.is_empty() {
        return None;
    }

    let changed_at = time_from_millis(row.updated_ms as f64)?;

    Some(AgentSession::new(
        SessionId::new(row.id.as_str()),
        PathBuf::from(directory),
        row.title.as_deref().unwrap_or_default(),
        changed_at,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::AgentId;
    use crate::open_code::fork::KILO;
    use crate::open_code::testing::{MadeUpDatabase, millis};
    use crate::testing::{NOW, TempDir, at};

    fn sessions_in(
        data_dir: PathBuf,
    ) -> Result<Vec<AgentSession>, SourceError> {
        let source =
            DatabaseSessions::new(Arc::new(DatabaseLocation::new(&KILO)));
        let profile = Profile::added(AgentId::new("kilo"), "Kilo", data_dir);

        source.sessions(&profile)
    }

    #[test]
    fn sessions_are_listed_without_the_ones_of_helper_agents() {
        let temp = TempDir::new();
        let data_dir = temp.path().join("kilo");
        let made_up = MadeUpDatabase::first_shape(&data_dir.join("kilo.db"));
        made_up.session(
            "ses_1",
            None,
            "/home/user/otty",
            "7.8.27",
            (NOW - 60, NOW),
        );
        made_up.session(
            "ses_2",
            Some("ses_1"),
            "/home/user/otty",
            "7.8.27",
            (NOW, NOW),
        );
        made_up.session("ses_3", None, "", "7.8.27", (NOW, NOW));

        let sessions = sessions_in(data_dir).expect("the database is read");

        assert_eq!(
            sessions,
            [AgentSession::new(
                SessionId::new("ses_1"),
                PathBuf::from("/home/user/otty"),
                "Session ses_1",
                at(NOW),
            )]
        );
    }

    #[test]
    fn no_database_has_no_sessions() {
        let temp = TempDir::new();

        let sessions = sessions_in(temp.path().join("kilo"));

        assert_eq!(sessions, Ok(Vec::new()));
    }

    #[test]
    fn the_newest_version_is_the_one_of_the_session_that_changed_last() {
        let row = |id: &str, version: &str, updated: u64| SessionRow {
            id: id.to_string(),
            parent_id: None,
            directory: String::from("/home/user/otty"),
            title: None,
            version: Some(version.to_string()),
            created_ms: millis(updated),
            updated_ms: millis(updated),
        };

        let rows = [
            row("a", "7.5.9", NOW),
            row("b", "7.8.27", NOW + 1),
            row("c", "", NOW + 2),
        ];

        assert_eq!(newest_version(&rows).as_deref(), Some("7.8.27"));
        assert_eq!(newest_version(&[]), None);
    }
}

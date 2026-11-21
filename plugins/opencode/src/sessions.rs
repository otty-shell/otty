//! The sessions of a profile, listed from its database, and the version
//! of OpenCode that wrote them. The sessions of helper agents, which
//! another session started, are not listed: their tokens count with the
//! others.

use otty_plugin_sdk::agents::{ReadError, Session};

use crate::database::{Database, SessionRow};

/// The sessions in the database at `path`, the one that changed last
/// first. No database means no sessions yet.
pub(crate) fn sessions(path: &str) -> Result<Vec<Session>, ReadError> {
    let Some(database) = Database::open(path).map_err(ReadError::Host)? else {
        return Ok(Vec::new());
    };
    let rows = database.sessions().map_err(ReadError::Host)?;

    let mut listed: Vec<Session> =
        rows.iter().filter_map(listed_session).collect();
    listed.sort_by_key(|session| std::cmp::Reverse(session.changed_ms));

    Ok(listed)
}

/// The newest version of OpenCode the sessions `rows` were written by:
/// the one of the session that changed last and names one.
pub(crate) fn newest_version(rows: &[SessionRow]) -> Option<String> {
    rows.iter()
        .filter(|row| row.version.as_deref().is_some_and(|v| !v.is_empty()))
        .max_by_key(|row| row.updated_ms)
        .and_then(|row| row.version.clone())
}

/// The session of `row` as the list shows it; `None` for the session of
/// a helper agent and for one with no folder.
fn listed_session(row: &SessionRow) -> Option<Session> {
    if row.parent_id.is_some() {
        return None;
    }
    let folder = row.directory.trim();
    if folder.is_empty() {
        return None;
    }

    Some(Session {
        id: row.id.clone(),
        folder: folder.to_string(),
        title: row.title.clone().unwrap_or_default(),
        changed_ms: row.updated_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{DataFolder, NOW, millis};

    #[test]
    fn sessions_are_listed_newest_first_without_the_ones_of_helper_agents() {
        let data = DataFolder::new();
        let database = data.first_shape();
        database.session(
            "ses_1",
            None,
            "/home/user/otty",
            "1.4.0",
            (NOW - 60, NOW - 30),
        );
        database.session(
            "ses_2",
            Some("ses_1"),
            "/home/user/otty",
            "1.4.0",
            (NOW, NOW),
        );
        database.session("ses_3", None, "", "1.4.0", (NOW, NOW));
        database.session("ses_4", None, "/home/user/site", "1.4.0", (NOW, NOW));
        let path = data.file("opencode.db");

        let found = data.run(|| sessions(&path)).expect("the database is read");

        let session = |id: &str, folder: &str, changed: u64| Session {
            id: id.to_string(),
            folder: folder.to_string(),
            title: format!("Session {id}"),
            changed_ms: millis(changed),
        };
        assert_eq!(
            found,
            [
                session("ses_4", "/home/user/site", NOW),
                session("ses_1", "/home/user/otty", NOW - 30),
            ]
        );
    }

    #[test]
    fn no_database_has_no_sessions() {
        let data = DataFolder::new();
        let path = data.file("opencode.db");

        let found = data.run(|| sessions(&path));

        assert_eq!(found, Ok(Vec::new()));
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
            row("a", "1.4.0", NOW),
            row("b", "2.3.2", NOW + 1),
            row("c", "", NOW + 2),
        ];

        assert_eq!(newest_version(&rows).as_deref(), Some("2.3.2"));
        assert_eq!(newest_version(&[]), None);
    }
}

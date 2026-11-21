//! The sessions of a profile, read from its database, and the version
//! of OpenCode that wrote them. The sessions of helper agents, which
//! another session started, are not listed: their tokens count with the
//! others.

use otty_plugin_sdk::agents::{PlaceItem, ReadError, Session};

use crate::database::{Database, SessionRow, database_path};

/// The sessions in the database of `item`, the database itself or its
/// write-ahead log: the whole session table, since OpenCode keeps few
/// sessions and reading them all takes milliseconds. No database means
/// no sessions yet.
pub(crate) fn read_sessions(
    item: &PlaceItem,
) -> Result<Vec<Session>, ReadError> {
    let path = database_path(item);
    let Some(database) = Database::open(path).map_err(ReadError::Host)? else {
        return Ok(Vec::new());
    };
    let rows = database.sessions().map_err(ReadError::Host)?;

    Ok(rows.iter().filter_map(listed_session).collect())
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
/// a helper agent. One with no folder has none: OTTY leaves it out.
fn listed_session(row: &SessionRow) -> Option<Session> {
    if row.parent_id.is_some() {
        return None;
    }
    let folder = row.directory.trim();
    let title = row.title.as_deref().map(str::trim);

    Some(Session {
        id: row.id.clone(),
        folder: (!folder.is_empty()).then(|| folder.to_string()),
        title: title.filter(|title| !title.is_empty()).map(str::to_string),
        changed_ms: row.updated_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{DataFolder, NOW, millis};

    #[test]
    fn sessions_are_read_without_the_ones_of_helper_agents() {
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
        let item = data.item("opencode.db");

        let mut found = data
            .run(|| read_sessions(&item))
            .expect("the database is read");

        found.sort_by(|one, other| one.id.cmp(&other.id));
        assert_eq!(
            found,
            [
                Session {
                    id: "ses_1".to_string(),
                    folder: Some("/home/user/otty".to_string()),
                    title: Some("Session ses_1".to_string()),
                    changed_ms: millis(NOW - 30),
                },
                Session {
                    id: "ses_3".to_string(),
                    folder: None,
                    title: Some("Session ses_3".to_string()),
                    changed_ms: millis(NOW),
                },
            ]
        );
    }

    /// The write-ahead log is read as its database.
    #[test]
    fn the_log_gives_the_sessions_of_its_database() {
        let data = DataFolder::new();
        let database = data.first_shape();
        database.session("ses_1", None, "/home/user/otty", "1.4.0", (NOW, NOW));
        let item = data.item("opencode.db-wal");

        let found = data
            .run(|| read_sessions(&item))
            .expect("the database is read");

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, "ses_1");
    }

    #[test]
    fn no_database_has_no_sessions() {
        let data = DataFolder::new();
        let item = data.item("opencode.db");

        let found = data.run(|| read_sessions(&item));

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

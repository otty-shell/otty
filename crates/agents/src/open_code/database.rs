//! The SQLite database of OpenCode or Kilo, opened read-only.
//!
//! OTTY never writes to it, never migrates it and never holds a lock that
//! would stop the agent: the agent keeps it in WAL mode, so a reader sees
//! the last committed rows while the agent goes on writing. The database
//! has two shapes, and both are read when there: the first (Kilo 7,
//! OpenCode 1) with the tables `session` and `message`, the second
//! (OpenCode 2) with `session_v2` and `session_message`.

use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{Connection, OpenFlags, Row, params};

use crate::error::SourceError;

/// How long a read waits while the agent holds the database, such as
/// while it checkpoints its journal.
const BUSY_WAIT: Duration = Duration::from_secs(2);

/// The tables of the sessions, in both shapes.
const SESSION_TABLES: [&str; 2] = ["session", "session_v2"];

/// A session as the database keeps it: a plain row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SessionRow {
    /// The id of the session, such as `ses_…`.
    pub(super) id: String,
    /// The session that started this one, for the session of a helper
    /// agent.
    pub(super) parent_id: Option<String>,
    /// The folder the session ran in.
    pub(super) directory: String,
    /// The title of the session.
    pub(super) title: Option<String>,
    /// The version of the agent that wrote the session, such as "7.5.9".
    pub(super) version: Option<String>,
    /// When the session started, in milliseconds since 1970.
    pub(super) created_ms: i64,
    /// When the session changed last, in milliseconds since 1970.
    pub(super) updated_ms: i64,
}

/// A message as the database keeps it: a plain row. Its text is not in
/// it: the agent keeps the text in other rows, which OTTY never reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct MessageRow {
    /// The id of the message, the same in both shapes.
    pub(super) id: String,
    /// The session of the message.
    pub(super) session_id: String,
    /// The kind of message, such as `assistant`, in the second shape; the
    /// first shape keeps it in the data, as `role`.
    pub(super) kind: Option<String>,
    /// When the message was written, in milliseconds since 1970.
    pub(super) at_ms: i64,
    /// The JSON of the message: its model, tokens and cost.
    pub(super) data: String,
}

/// A login the agent keeps in its `credential` table: a plain row, whose
/// value holds a key or a token. It is read only to tell the account, and
/// dropped right after.
pub(super) struct CredentialRow {
    /// The provider of the login, such as `openai`.
    pub(super) integration_id: String,
    /// The JSON of the login.
    pub(super) value: String,
}

/// The database of one profile, open for reading only.
pub(super) struct Database {
    connection: Connection,
    path: PathBuf,
}

impl Database {
    /// Opens the database at `path` for reading only; `None` when there is
    /// no file, as before the agent's first run.
    pub(super) fn open(path: &Path) -> Result<Option<Self>, SourceError> {
        if !path.is_file() {
            return Ok(None);
        }

        let flags =
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
        let connection = Connection::open_with_flags(path, flags)
            .map_err(|error| unreadable(path, &error))?;
        connection
            .busy_timeout(BUSY_WAIT)
            .and_then(|()| connection.pragma_update(None, "query_only", true))
            .map_err(|error| unreadable(path, &error))?;

        Ok(Some(Self {
            connection,
            path: path.to_path_buf(),
        }))
    }

    /// Every session of both shapes; a session in both is listed once,
    /// as its newest row.
    pub(super) fn sessions(&self) -> Result<Vec<SessionRow>, SourceError> {
        let mut sessions: Vec<SessionRow> = Vec::new();

        for table in SESSION_TABLES {
            if !self.has_table(table)? {
                continue;
            }
            let sql = format!(
                "SELECT id, parent_id, directory, title, version,
                 time_created, time_updated FROM {table}"
            );
            for row in self.rows(&sql, [], session_row)? {
                match sessions.iter_mut().find(|kept| kept.id == row.id) {
                    Some(kept) if kept.updated_ms < row.updated_ms => {
                        *kept = row;
                    },
                    Some(_) => {},
                    None => sessions.push(row),
                }
            }
        }

        Ok(sessions)
    }

    /// The messages of the user and the answers of the models of both
    /// shapes written at `since_ms` or later, in milliseconds since 1970.
    /// A message in both shapes is in both: its id tells them apart.
    pub(super) fn messages(
        &self,
        since_ms: i64,
    ) -> Result<Vec<MessageRow>, SourceError> {
        let mut messages = Vec::new();

        if self.has_table("message")? {
            messages.extend(self.rows(
                "SELECT id, session_id, NULL, time_created, data
                 FROM message WHERE time_created >= ?1",
                params![since_ms],
                message_row,
            )?);
        }
        if self.has_table("session_message")? {
            messages.extend(self.rows(
                "SELECT id, session_id, type, time_created, data
                 FROM session_message
                 WHERE time_created >= ?1 AND type IN ('assistant', 'user')",
                params![since_ms],
                message_row,
            )?);
        }

        Ok(messages)
    }

    /// The logins of the `credential` table; none in a database that has
    /// no such table yet.
    pub(super) fn credentials(
        &self,
    ) -> Result<Vec<CredentialRow>, SourceError> {
        if !self.has_table("credential")? {
            return Ok(Vec::new());
        }

        self.rows(
            "SELECT integration_id, value FROM credential
             WHERE integration_id IS NOT NULL",
            [],
            |row| {
                Ok(CredentialRow {
                    integration_id: row.get(0)?,
                    value: row.get(1)?,
                })
            },
        )
    }

    /// Whether the database has the table `name`.
    fn has_table(&self, name: &str) -> Result<bool, SourceError> {
        let found = self.rows(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
            params![name],
            |_| Ok(()),
        )?;

        Ok(!found.is_empty())
    }

    /// The rows `sql` selects with `values`, each read with `read`.
    fn rows<T>(
        &self,
        sql: &str,
        values: impl rusqlite::Params,
        read: impl FnMut(&Row<'_>) -> rusqlite::Result<T>,
    ) -> Result<Vec<T>, SourceError> {
        let read_all = || -> rusqlite::Result<Vec<T>> {
            let mut statement = self.connection.prepare(sql)?;
            let rows = statement.query_map(values, read)?;
            rows.collect()
        };

        read_all().map_err(|error| unreadable(&self.path, &error))
    }
}

/// The session of `row`.
fn session_row(row: &Row<'_>) -> rusqlite::Result<SessionRow> {
    Ok(SessionRow {
        id: row.get(0)?,
        parent_id: row.get(1)?,
        directory: row.get(2)?,
        title: row.get(3)?,
        version: row.get(4)?,
        created_ms: row.get(5)?,
        updated_ms: row.get(6)?,
    })
}

/// The message of `row`.
fn message_row(row: &Row<'_>) -> rusqlite::Result<MessageRow> {
    Ok(MessageRow {
        id: row.get(0)?,
        session_id: row.get(1)?,
        kind: row.get(2)?,
        at_ms: row.get(3)?,
        data: row.get(4)?,
    })
}

/// The failure to read the database at `path`, with SQLite's reason.
fn unreadable(path: &Path, error: &rusqlite::Error) -> SourceError {
    SourceError::Unreadable {
        path: path.to_path_buf(),
        detail: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_code::testing::{
        MadeUpDatabase, first_shape_answer, first_shape_question, millis,
        second_shape_answer,
    };
    use crate::testing::{NOW, TempDir};

    const TOKENS: (u64, u64, u64, u64, u64) = (10, 2, 1, 300, 5);

    fn open(path: &Path) -> Database {
        Database::open(path)
            .expect("the database opens")
            .expect("the database is there")
    }

    #[test]
    fn no_file_is_no_database() {
        let temp = TempDir::new();

        let database = Database::open(&temp.path().join("kilo.db"));

        assert!(matches!(database, Ok(None)));
    }

    #[test]
    fn sessions_and_messages_of_the_first_shape_are_read() {
        let temp = TempDir::new();
        let mut made_up =
            MadeUpDatabase::first_shape(&temp.path().join("kilo.db"));
        made_up.session(
            "ses_1",
            None,
            "/home/user/otty",
            "7.8.27",
            (NOW, NOW + 5),
        );
        made_up.session(
            "ses_2",
            Some("ses_1"),
            "/home/user/otty",
            "7.8.27",
            (NOW, NOW),
        );
        let answer = first_shape_answer("openai", "gpt-5.5", TOKENS, 0.5);
        made_up.message("msg_1", "ses_1", "", NOW, &first_shape_question());
        made_up.message("msg_2", "ses_1", "", NOW + 1, &answer);
        made_up.message("msg_0", "ses_1", "", NOW - 3600, &answer);

        let database = open(made_up.path());
        let sessions = database.sessions().expect("sessions are read");
        let messages =
            database.messages(millis(NOW)).expect("messages are read");

        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].id, "ses_1");
        assert_eq!(sessions[0].parent_id, None);
        assert_eq!(sessions[0].directory, "/home/user/otty");
        assert_eq!(sessions[0].version.as_deref(), Some("7.8.27"));
        assert_eq!(sessions[0].updated_ms, millis(NOW + 5));
        assert_eq!(sessions[1].parent_id.as_deref(), Some("ses_1"));
        let ids: Vec<&str> =
            messages.iter().map(|row| row.id.as_str()).collect();
        assert_eq!(ids, ["msg_1", "msg_2"], "an older message is left out");
        assert_eq!(messages[1].kind, None);
        assert_eq!(messages[1].data, answer);
    }

    #[test]
    fn sessions_and_messages_of_the_second_shape_are_read() {
        let temp = TempDir::new();
        let path = temp.path().join("opencode.db");
        let mut made_up = MadeUpDatabase::second_shape(&path);
        made_up.session("ses_1", None, "/home/user/site", "2.3.2", (NOW, NOW));
        let answer = second_shape_answer("anthropic", "claude-x", TOKENS, 0.1);
        made_up.message("msg_1", "ses_1", "assistant", NOW, &answer);
        made_up.message("msg_2", "ses_1", "compaction", NOW, "{}");

        let database = open(&path);
        let sessions = database.sessions().expect("sessions are read");
        let messages = database.messages(0).expect("messages are read");

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].version.as_deref(), Some("2.3.2"));
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].kind.as_deref(), Some("assistant"));
        assert_eq!(messages[0].at_ms, millis(NOW));
    }

    #[test]
    fn the_logins_of_the_credential_table_are_read() {
        let temp = TempDir::new();
        let made_up = MadeUpDatabase::first_shape(&temp.path().join("kilo.db"));
        made_up.credential("cred_1", "deepseek", r#"{"type":"key","key":"k"}"#);

        let credentials =
            open(made_up.path()).credentials().expect("logins are read");

        assert_eq!(credentials.len(), 1);
        assert_eq!(credentials[0].integration_id, "deepseek");
    }

    #[test]
    fn the_database_is_never_written() {
        let temp = TempDir::new();
        let made_up = MadeUpDatabase::first_shape(&temp.path().join("kilo.db"));

        let database = open(made_up.path());
        let written = database.connection.execute(
            "INSERT INTO session (id, project_id, directory, title, version,
             time_created, time_updated) VALUES ('x', 'p', '/', 't', 'v', 0, 0)",
            [],
        );

        assert!(written.is_err(), "the database is opened read-only");
        assert!(database.sessions().expect("sessions are read").is_empty());
    }

    /// The agent writes while OTTY reads: OTTY sees what the agent
    /// committed and is not stopped by what it is still writing.
    #[test]
    fn the_database_is_read_while_the_agent_writes_to_it() {
        let temp = TempDir::new();
        let made_up = MadeUpDatabase::first_shape(&temp.path().join("kilo.db"));
        made_up.session("ses_1", None, "/home/user/otty", "7.8.27", (NOW, NOW));
        let database = open(made_up.path());

        made_up
            .connection()
            .execute_batch(
                "BEGIN IMMEDIATE;
                 INSERT INTO session (id, project_id, directory, title,
                 version, time_created, time_updated)
                 VALUES ('ses_2', 'p', '/', 't', 'v', 0, 0);",
            )
            .expect("the agent writes");
        let sessions = database.sessions();

        let sessions = sessions.expect("the read is not stopped");
        assert_eq!(sessions.len(), 1, "only what the agent committed");
    }
}

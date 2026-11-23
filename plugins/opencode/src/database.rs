//! The SQLite database of OpenCode, read through the host, which opens it
//! read-only: the plugin never writes to it, never migrates it and never
//! holds a lock that would stop OpenCode. OpenCode keeps it in WAL mode,
//! so a read sees the last committed rows while OpenCode goes on writing.
//!
//! The database has two shapes, and both are read when there: the first
//! (OpenCode 1) with the tables `session` and `message`, the second
//! (OpenCode 2) with `session_v2` and `session_message`. The logins of
//! the `credential` table are never read, and its `value` column is a key
//! column the SQL never sees as text. The text of a conversation is kept in other
//! tables, which the plugin never reads.

use otty_plugin_sdk::agents::{Place, PlaceItem, PlaceKind};
use otty_plugin_sdk::sqlite::{self, Param, Value};
use otty_plugin_sdk::{HostError, files};

use crate::glob::escaped;

/// What follows the name of the database in the name of its write-ahead
/// log.
const LOG_SUFFIX: &str = "-wal";

/// The tables of the sessions, in both shapes.
const SESSION_TABLES: [&str; 2] = ["session", "session_v2"];

/// How many messages one query asks for: the host gives at most 50,000
/// rows a query, and the plugin's memory is small.
const PAGE_ROWS: i64 = 2000;

/// A session as the database keeps it: a plain row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionRow {
    /// The id of the session, such as `ses_…`.
    pub(crate) id: String,
    /// The session that started this one, for the session of a helper
    /// agent.
    pub(crate) parent_id: Option<String>,
    /// The folder the session ran in.
    pub(crate) directory: String,
    /// The title of the session.
    pub(crate) title: Option<String>,
    /// The version of OpenCode that wrote the session, such as "2.3.2".
    pub(crate) version: Option<String>,
    /// When the session started, in milliseconds since 1970.
    pub(crate) created_ms: i64,
    /// When the session changed last, in milliseconds since 1970.
    pub(crate) updated_ms: i64,
}

/// A message as the database keeps it: a plain row. Its text is not in
/// it: OpenCode keeps the text in other rows, which are never read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MessageRow {
    /// The id of the message, the same in both shapes.
    pub(crate) id: String,
    /// The session of the message.
    pub(crate) session_id: String,
    /// The kind of message, such as `assistant`, in the second shape; the
    /// first shape keeps it in the data, as `role`.
    pub(crate) kind: Option<String>,
    /// When the message was written, in milliseconds since 1970.
    pub(crate) at_ms: i64,
    /// The JSON of the message: its model, tokens and cost.
    pub(crate) data: String,
}

/// The database of one profile, with the tables it has.
#[derive(Debug)]
pub(crate) struct Database {
    path: String,
    tables: Vec<String>,
}

impl Database {
    /// The database at `path`; `None` when there is no file, as before
    /// OpenCode's first run.
    pub(crate) fn open(path: &str) -> Result<Option<Self>, HostError> {
        let found = files::list(&escaped(path))?;
        if found.is_empty() {
            return Ok(None);
        }

        let rows = sqlite::query(
            path,
            "SELECT name FROM sqlite_master WHERE type = 'table'",
            &[],
        )?;
        let tables = rows.iter().filter_map(|row| text_at(row, 0)).collect();

        Ok(Some(Self {
            path: path.to_string(),
            tables,
        }))
    }

    /// Every session of both shapes; a session in both is listed once, as
    /// its newest row.
    pub(crate) fn sessions(&self) -> Result<Vec<SessionRow>, HostError> {
        let mut sessions: Vec<SessionRow> = Vec::new();

        for table in SESSION_TABLES {
            if !self.has_table(table) {
                continue;
            }
            let sql = format!(
                "SELECT id, parent_id, directory, title, version,
                 time_created, time_updated FROM {table}"
            );
            let rows = sqlite::query(&self.path, &sql, &[])?;

            for row in rows.iter().filter_map(|row| session_row(row)) {
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

    /// Gives `take` each message of the user and each answer of a model
    /// of both shapes written at `since_ms` or later, in milliseconds
    /// since 1970, a page of rows at a time. A message in both shapes is
    /// given twice: its id tells them apart.
    pub(crate) fn for_each_message(
        &self,
        since_ms: i64,
        mut take: impl FnMut(MessageRow),
    ) -> Result<(), HostError> {
        if self.has_table("message") {
            self.pages(
                "SELECT rowid, id, session_id, NULL, time_created, data
                 FROM message
                 WHERE time_created >= ?1 AND rowid > ?2
                 ORDER BY rowid LIMIT ?3",
                since_ms,
                &mut take,
            )?;
        }
        if self.has_table("session_message") {
            self.pages(
                "SELECT rowid, id, session_id, type, time_created, data
                 FROM session_message
                 WHERE time_created >= ?1 AND rowid > ?2
                   AND type IN ('assistant', 'user')
                 ORDER BY rowid LIMIT ?3",
                since_ms,
                &mut take,
            )?;
        }

        Ok(())
    }

    /// Whether the database has the table `name`.
    fn has_table(&self, name: &str) -> bool {
        self.tables.iter().any(|table| table == name)
    }

    /// Gives `take` the messages `sql` selects, page by page: `sql` takes
    /// the first time, the last rowid read and the size of a page, and
    /// gives the rowid first.
    fn pages(
        &self,
        sql: &str,
        since_ms: i64,
        take: &mut impl FnMut(MessageRow),
    ) -> Result<(), HostError> {
        let mut after = 0;

        loop {
            let params = [
                Param::Integer(since_ms),
                Param::Integer(after),
                Param::Integer(PAGE_ROWS),
            ];
            let rows = sqlite::query(&self.path, sql, &params)?;
            let is_last = (rows.len() as i64) < PAGE_ROWS;

            for row in rows {
                if let Some(row_id) = integer_at(&row, 0) {
                    after = after.max(row_id);
                }
                if let Some(message) = message_row(&row) {
                    take(message);
                }
            }

            if is_last {
                return Ok(());
            }
        }
    }
}

/// Where the records of the database at `database` are, its sessions and
/// its answers: the database and its write-ahead log, `opencode.db-wal`.
/// OpenCode writes new rows to the log first and copies them into the
/// database only now and then, so the database file alone may not change
/// for a long time. When either file changes, OTTY asks for the whole
/// database again.
pub(crate) fn database_places(database: &str) -> Vec<Place> {
    let database = escaped(database);

    vec![
        Place {
            glob: database.clone(),
            kind: PlaceKind::Database,
        },
        Place {
            glob: format!("{database}{LOG_SUFFIX}"),
            kind: PlaceKind::Database,
        },
    ]
}

/// The path of the database `item` is a file of: the database itself, or
/// the one of a write-ahead log.
pub(crate) fn database_path(item: &PlaceItem) -> &str {
    let path = item.file.path.as_str();

    path.strip_suffix(LOG_SUFFIX).unwrap_or(path)
}

/// The session of `row`; `None` for a row with no id or no times.
fn session_row(row: &[Value]) -> Option<SessionRow> {
    Some(SessionRow {
        id: text_at(row, 0)?,
        parent_id: text_at(row, 1),
        directory: text_at(row, 2).unwrap_or_default(),
        title: text_at(row, 3),
        version: text_at(row, 4),
        created_ms: integer_at(row, 5)?,
        updated_ms: integer_at(row, 6)?,
    })
}

/// The message of `row`, after its rowid; `None` for a row that misses
/// its id, session, time or data.
fn message_row(row: &[Value]) -> Option<MessageRow> {
    Some(MessageRow {
        id: text_at(row, 1)?,
        session_id: text_at(row, 2)?,
        kind: text_at(row, 3),
        at_ms: integer_at(row, 4)?,
        data: text_at(row, 5)?,
    })
}

/// The text at `index` of `row`.
fn text_at(row: &[Value], index: usize) -> Option<String> {
    match row.get(index)? {
        Value::Text(text) => Some(text.clone()),
        _ => None,
    }
}

/// The whole number at `index` of `row`; a real number is rounded down.
fn integer_at(row: &[Value], index: usize) -> Option<i64> {
    match row.get(index)? {
        Value::Integer(number) => Some(*number),
        Value::Real(number) if number.is_finite() => Some(*number as i64),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{
        DataFolder, NOW, first_shape_answer, first_shape_question, millis,
        second_shape_answer,
    };

    const TOKENS: (u64, u64, u64, u64, u64) = (10, 2, 1, 300, 5);

    fn item(path: &str) -> PlaceItem {
        PlaceItem {
            file: files::FileInfo {
                path: path.to_string(),
                size: 0,
                changed_ms: 0,
            },
            kind: PlaceKind::Database,
        }
    }

    #[test]
    fn the_places_are_the_database_and_its_log() {
        let places = database_places("/data/[x]/opencode.db");

        let globs: Vec<&str> =
            places.iter().map(|place| place.glob.as_str()).collect();
        assert_eq!(
            globs,
            ["/data/\\[x\\]/opencode.db", "/data/\\[x\\]/opencode.db-wal"]
        );
        assert!(places.iter().all(|place| place.kind == PlaceKind::Database));
    }

    #[test]
    fn the_log_is_read_as_its_database() {
        assert_eq!(
            database_path(&item("/data/opencode.db-wal")),
            "/data/opencode.db"
        );
        assert_eq!(
            database_path(&item("/data/opencode.db")),
            "/data/opencode.db"
        );
    }

    fn open(data: &DataFolder) -> Database {
        let path = data.file("opencode.db");

        data.run(|| Database::open(&path))
            .expect("the database opens")
            .expect("the database is there")
    }

    fn messages(
        data: &DataFolder,
        database: &Database,
        since_ms: i64,
    ) -> Vec<MessageRow> {
        let mut messages = Vec::new();

        data.run(|| {
            database.for_each_message(since_ms, |row| messages.push(row))
        })
        .expect("messages are read");

        messages
    }

    #[test]
    fn no_file_is_no_database() {
        let data = DataFolder::new();
        let path = data.file("opencode.db");

        let database = data.run(|| Database::open(&path));

        assert!(matches!(database, Ok(None)));
    }

    #[test]
    fn sessions_and_messages_of_the_first_shape_are_read() {
        let data = DataFolder::new();
        let mut made_up = data.first_shape();
        made_up.session(
            "ses_1",
            None,
            "/home/user/otty",
            "1.4.0",
            (NOW, NOW + 5),
        );
        made_up.session(
            "ses_2",
            Some("ses_1"),
            "/home/user/otty",
            "1.4.0",
            (NOW, NOW),
        );
        let answer = first_shape_answer("openai", "gpt-5.5", TOKENS, 0.5);
        made_up.message("msg_1", "ses_1", "", NOW, &first_shape_question());
        made_up.message("msg_2", "ses_1", "", NOW + 1, &answer);
        made_up.message("msg_0", "ses_1", "", NOW - 3600, &answer);

        let database = open(&data);
        let sessions =
            data.run(|| database.sessions()).expect("sessions are read");
        let messages = messages(&data, &database, millis(NOW));

        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].id, "ses_1");
        assert_eq!(sessions[0].parent_id, None);
        assert_eq!(sessions[0].directory, "/home/user/otty");
        assert_eq!(sessions[0].version.as_deref(), Some("1.4.0"));
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
        let data = DataFolder::new();
        let mut made_up = data.second_shape();
        made_up.session("ses_1", None, "/home/user/site", "2.3.2", (NOW, NOW));
        let answer = second_shape_answer("anthropic", "claude-x", TOKENS, 0.1);
        made_up.message("msg_1", "ses_1", "assistant", NOW, &answer);
        made_up.message("msg_2", "ses_1", "compaction", NOW, "{}");

        let database = open(&data);
        let sessions =
            data.run(|| database.sessions()).expect("sessions are read");
        let messages = messages(&data, &database, 0);

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].version.as_deref(), Some("2.3.2"));
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].kind.as_deref(), Some("assistant"));
        assert_eq!(messages[0].at_ms, millis(NOW));
    }

    /// More messages than one page are all read, each once.
    #[test]
    fn messages_are_read_page_by_page() {
        let data = DataFolder::new();
        let mut made_up = data.first_shape();
        let count = PAGE_ROWS as u64 * 2 + 7;
        made_up.connection().execute_batch("BEGIN").expect("begin");
        for number in 0..count {
            made_up.message(&format!("msg_{number}"), "ses_1", "", NOW, "{}");
        }
        made_up
            .connection()
            .execute_batch("COMMIT")
            .expect("commit");

        let database = open(&data);
        let mut ids: Vec<String> = messages(&data, &database, 0)
            .into_iter()
            .map(|row| row.id)
            .collect();

        ids.sort();
        ids.dedup();
        assert_eq!(ids.len() as u64, count);
    }

    /// The SQL of the plugin never sees a key: naming the key column
    /// anywhere but as a result column is refused, and so is the column
    /// without the rowid of its table.
    #[test]
    fn a_key_column_is_never_read_as_text() {
        let data = DataFolder::new();
        let made_up = data.first_shape();
        made_up.credential("cred_1", "deepseek", r#"{"type":"key","key":"k"}"#);
        let path = data.file("opencode.db");

        let filtered = data.run(|| {
            sqlite::query(
                &path,
                "SELECT rowid FROM credential WHERE value LIKE '%sk-%'",
                &[],
            )
        });
        let without_rowid = data
            .run(|| sqlite::query(&path, "SELECT value FROM credential", &[]));

        assert!(
            matches!(filtered, Err(HostError::NotAllowed(_))),
            "{filtered:?}"
        );
        assert!(without_rowid.is_err());
    }

    #[test]
    fn the_database_is_never_written() {
        let data = DataFolder::new();
        let _made_up = data.first_shape();
        let path = data.file("opencode.db");

        let written = data.run(|| {
            sqlite::query(
                &path,
                "INSERT INTO session (id, project_id, directory, title,
                 version, time_created, time_updated)
                 VALUES ('x', 'p', '/', 't', 'v', 0, 0)",
                &[],
            )
        });

        let database = open(&data);
        assert!(written.is_err(), "the database is opened read-only");
        assert!(data.run(|| database.sessions()).expect("read").is_empty());
    }

    /// OpenCode writes while the plugin reads: the plugin sees what
    /// OpenCode committed and is not stopped by what it is still writing.
    #[test]
    fn the_database_is_read_while_opencode_writes_to_it() {
        let data = DataFolder::new();
        let made_up = data.first_shape();
        made_up.session("ses_1", None, "/home/user/otty", "1.4.0", (NOW, NOW));
        made_up
            .connection()
            .execute_batch(
                "BEGIN IMMEDIATE;
                 INSERT INTO session (id, project_id, directory, title,
                 version, time_created, time_updated)
                 VALUES ('ses_2', 'p', '/', 't', 'v', 0, 0);",
            )
            .expect("OpenCode writes");

        let database = open(&data);
        let sessions = data.run(|| database.sessions());

        let sessions = sessions.expect("the read is not stopped");
        assert_eq!(sessions.len(), 1, "only what OpenCode committed");
    }
}

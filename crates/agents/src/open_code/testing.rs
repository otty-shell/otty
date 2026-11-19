//! Made-up databases in the shapes Kilo and OpenCode write, for the tests
//! of this module. Only the tests write to them; OTTY itself never does.

use std::path::{Path, PathBuf};

use rusqlite::{Connection, params};

/// The tables of the first shape (Kilo 7, OpenCode 1): `session` and
/// `message`, with the `credential` table of the logins.
const FIRST_SHAPE: &str = "
    CREATE TABLE session (
        id TEXT PRIMARY KEY,
        project_id TEXT NOT NULL,
        parent_id TEXT,
        directory TEXT NOT NULL,
        title TEXT NOT NULL,
        version TEXT NOT NULL,
        time_created INTEGER NOT NULL,
        time_updated INTEGER NOT NULL
    );
    CREATE TABLE message (
        id TEXT PRIMARY KEY,
        session_id TEXT NOT NULL,
        time_created INTEGER NOT NULL,
        time_updated INTEGER NOT NULL,
        data TEXT NOT NULL
    );
    CREATE TABLE credential (
        id TEXT PRIMARY KEY,
        integration_id TEXT,
        label TEXT NOT NULL,
        value TEXT NOT NULL
    );
";

/// The tables of the second shape (OpenCode 2): `session_v2` and
/// `session_message`, whose kind of message is a column.
const SECOND_SHAPE: &str = "
    CREATE TABLE session_v2 (
        id TEXT PRIMARY KEY,
        project_id TEXT NOT NULL,
        parent_id TEXT,
        directory TEXT NOT NULL,
        title TEXT,
        version TEXT NOT NULL,
        time_created INTEGER NOT NULL,
        time_updated INTEGER NOT NULL
    );
    CREATE TABLE session_message (
        id TEXT PRIMARY KEY,
        session_id TEXT NOT NULL,
        type TEXT NOT NULL,
        seq INTEGER NOT NULL,
        time_created INTEGER NOT NULL,
        time_updated INTEGER NOT NULL,
        data TEXT NOT NULL
    );
    CREATE TABLE credential (
        id TEXT PRIMARY KEY,
        integration_id TEXT,
        label TEXT NOT NULL,
        value TEXT NOT NULL
    );
";

/// A made-up database of Kilo or OpenCode, in WAL mode as the agents keep
/// theirs, with an open connection that writes to it.
pub(super) struct MadeUpDatabase {
    path: PathBuf,
    connection: Connection,
    second_shape: bool,
    next_seq: i64,
}

impl MadeUpDatabase {
    /// The file of the database.
    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    /// The connection that writes to it, as the agent's would.
    pub(super) fn connection(&self) -> &Connection {
        &self.connection
    }

    /// A database of the first shape at `path`.
    pub(super) fn first_shape(path: &Path) -> Self {
        Self::create(path, FIRST_SHAPE, false)
    }

    /// A database of the second shape at `path`.
    pub(super) fn second_shape(path: &Path) -> Self {
        Self::create(path, SECOND_SHAPE, true)
    }

    /// Adds the session `id` in `directory`, a child of `parent` when one
    /// is given, written by the agent's `version`, created `created`
    /// seconds after the epoch and changed last at `updated`.
    pub(super) fn session(
        &self,
        id: &str,
        parent: Option<&str>,
        directory: &str,
        version: &str,
        (created, updated): (u64, u64),
    ) {
        let table = if self.second_shape {
            "session_v2"
        } else {
            "session"
        };

        self.connection
            .execute(
                &format!(
                    "INSERT INTO {table} (id, project_id, parent_id, directory,
                     title, version, time_created, time_updated)
                     VALUES (?1, 'prj_made_up', ?2, ?3, ?4, ?5, ?6, ?7)"
                ),
                params![
                    id,
                    parent,
                    directory,
                    format!("Session {id}"),
                    version,
                    millis(created),
                    millis(updated)
                ],
            )
            .expect("add a session");
    }

    /// Adds the message `id` of `session`, written `at` seconds after the
    /// epoch, with the
    /// JSON `data`. In the second shape, `kind` is its `type` column; in
    /// the first, the kind is in `data` and `kind` is not used.
    pub(super) fn message(
        &mut self,
        id: &str,
        session: &str,
        kind: &str,
        at: u64,
        data: &str,
    ) {
        if self.second_shape {
            self.next_seq += 1;
            self.connection
                .execute(
                    "INSERT INTO session_message (id, session_id, type, seq,
                     time_created, time_updated, data)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?5, ?6)",
                    params![id, session, kind, self.next_seq, millis(at), data],
                )
                .expect("add a message");
            return;
        }

        self.connection
            .execute(
                "INSERT INTO message (id, session_id, time_created,
                 time_updated, data) VALUES (?1, ?2, ?3, ?3, ?4)",
                params![id, session, millis(at), data],
            )
            .expect("add a message");
    }

    /// Adds a login of the provider `integration` with the JSON `value`,
    /// as the agent keeps it in its `credential` table.
    pub(super) fn credential(&self, id: &str, integration: &str, value: &str) {
        self.connection
            .execute(
                "INSERT INTO credential (id, integration_id, label, value)
                 VALUES (?1, ?2, 'Imported', ?3)",
                params![id, integration, value],
            )
            .expect("add a credential");
    }

    fn create(path: &Path, tables: &str, second_shape: bool) -> Self {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create the data folder");
        }

        let connection = Connection::open(path).expect("create the database");
        connection
            .pragma_update(None, "journal_mode", "wal")
            .expect("use the WAL journal");
        connection.execute_batch(tables).expect("create the tables");

        Self {
            path: path.to_path_buf(),
            connection,
            second_shape,
            next_seq: 0,
        }
    }
}

/// The JSON of an answer in the first shape: the model `model` of the
/// provider `provider`, its tokens (`input`, `output`, `reasoning`, cache
/// `read` and `write`) and its `cost` in dollars.
pub(super) fn first_shape_answer(
    provider: &str,
    model: &str,
    (input, output, reasoning, read, write): (u64, u64, u64, u64, u64),
    cost: f64,
) -> String {
    format!(
        r#"{{"role":"assistant","parentID":"msg_made_up","modelID":"{model}",
        "providerID":"{provider}","mode":"code","agent":"code",
        "path":{{"cwd":"/home/user/otty","root":"/home/user/otty"}},
        "cost":{cost},"tokens":{{"total":0,"input":{input},"output":{output},
        "reasoning":{reasoning},"cache":{{"read":{read},"write":{write}}}}},
        "time":{{"created":1,"completed":2}},"finish":"stop"}}"#
    )
}

/// The JSON of an answer in the second shape, with the same parts as
/// [`first_shape_answer`]: the model and its provider are in `model`.
pub(super) fn second_shape_answer(
    provider: &str,
    model: &str,
    (input, output, reasoning, read, write): (u64, u64, u64, u64, u64),
    cost: f64,
) -> String {
    format!(
        r#"{{"agent":"build","model":{{"id":"{model}","providerID":"{provider}"}},
        "content":[],"finish":"stop","cost":{cost},
        "tokens":{{"input":{input},"output":{output},"reasoning":{reasoning},
        "cache":{{"read":{read},"write":{write}}}}},
        "time":{{"created":1,"completed":2}}}}"#
    )
}

/// The JSON of a message of the user in the first shape.
pub(super) fn first_shape_question() -> String {
    String::from(
        r#"{"role":"user","time":{"created":1},"agent":"code",
        "model":{"providerID":"openai","modelID":"gpt-5.5"}}"#,
    )
}

/// `seconds` after the epoch in milliseconds, as the agents write times.
pub(super) fn millis(seconds: u64) -> i64 {
    i64::try_from(seconds * 1000).expect("a time of the tests")
}

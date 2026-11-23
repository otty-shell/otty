//! Test-only values shared by the tests of this crate: the test host over
//! `fixtures/`, which stands for the home folder, and made-up OpenCode
//! data folders with their databases in the shapes OpenCode writes. Only
//! the tests write to these databases; the plugin itself never does.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use otty_plugin_sdk::agents::{PlaceItem, PlaceKind, Profile};
use otty_plugin_sdk::files::FileInfo;
use otty_plugin_sdk::test_host::TestHost;
use rusqlite::{Connection, params};

/// The agent part the tests call.
pub(crate) const AGENT: &str = "opencode";

/// 2026-11-16T10:00:00Z in seconds since the Unix epoch: the clock of the
/// tests, and the time of the newest made-up answers.
pub(crate) const NOW: u64 = 1_794_823_200;

/// One day in seconds.
pub(crate) const DAY: u64 = 24 * 3600;

/// The tables of the first shape (OpenCode 1): `session` and `message`,
/// with the `credential` table of the logins.
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

/// How many made-up data folders the tests of this run made, so each one
/// gets its own.
static MADE: AtomicUsize = AtomicUsize::new(0);

/// A made-up data folder of OpenCode, `<temporary folder>/opencode`, with
/// a host whose `XDG_DATA_HOME` points at the folder above it, as when the
/// user moved OpenCode's data. It is removed when dropped.
pub(crate) struct DataFolder {
    root: PathBuf,
    host: TestHost,
}

impl DataFolder {
    /// The data folder itself.
    pub(crate) fn path(&self) -> PathBuf {
        self.root.join("opencode")
    }

    /// The path of `name` in the data folder, as text.
    pub(crate) fn file(&self, name: &str) -> String {
        self.path().join(name).display().to_string()
    }

    /// The file `name` of the data folder, as OTTY asks the part to read
    /// a file of a database place. The part does not use its size and
    /// time, so they are left at zero.
    pub(crate) fn item(&self, name: &str) -> PlaceItem {
        PlaceItem {
            file: FileInfo {
                path: self.file(name),
                size: 0,
                changed_ms: 0,
            },
            kind: PlaceKind::Database,
        }
    }

    /// The default profile of the data folder.
    pub(crate) fn profile(&self) -> Profile {
        Profile {
            agent: AGENT.to_string(),
            folder: self.path().display().to_string(),
            is_default: true,
        }
    }

    /// A new empty data folder, with the clock of the host at [`NOW`].
    pub(crate) fn new() -> Self {
        let number = MADE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "otty-plugin-opencode-{}-{number}",
            std::process::id()
        ));
        std::fs::create_dir_all(root.join("opencode"))
            .expect("create the data folder");

        let host = host()
            .with_var("XDG_DATA_HOME", &root.display().to_string())
            .with_now_ms(millis(NOW));

        Self { root, host }
    }

    /// A database of the first shape, `opencode.db`, in the data folder.
    pub(crate) fn first_shape(&self) -> MadeUpDatabase {
        MadeUpDatabase::create(&self.path().join("opencode.db"), FIRST_SHAPE)
    }

    /// A database of the second shape, `opencode.db`, in the data folder.
    pub(crate) fn second_shape(&self) -> MadeUpDatabase {
        MadeUpDatabase::create(&self.path().join("opencode.db"), SECOND_SHAPE)
    }

    /// Runs `run` as the part `opencode` of the host.
    pub(crate) fn run<R>(&self, run: impl FnOnce() -> R) -> R {
        as_part(&self.host, run)
    }
}

impl Drop for DataFolder {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// A made-up OpenCode database, in WAL mode as OpenCode keeps its own,
/// with an open connection that writes to it.
pub(crate) struct MadeUpDatabase {
    connection: Connection,
    second_shape: bool,
    next_seq: i64,
}

impl MadeUpDatabase {
    /// The connection that writes to it, as OpenCode's would.
    pub(crate) fn connection(&self) -> &Connection {
        &self.connection
    }

    /// Adds the session `id` in `directory`, a child of `parent` when one
    /// is given, written by OpenCode's `version`, created `created`
    /// seconds after the epoch and changed last at `updated`.
    pub(crate) fn session(
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
    /// epoch, with the JSON `data`. In the second shape, `kind` is its
    /// `type` column; in the first, the kind is in `data` and `kind` is
    /// not used.
    pub(crate) fn message(
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
    /// as OpenCode keeps it in its `credential` table, and gives its
    /// rowid.
    pub(crate) fn credential(
        &self,
        id: &str,
        integration: &str,
        value: &str,
    ) -> i64 {
        self.connection
            .execute(
                "INSERT INTO credential (id, integration_id, label, value)
                 VALUES (?1, ?2, 'Imported', ?3)",
                params![id, integration, value],
            )
            .expect("add a credential");

        self.connection.last_insert_rowid()
    }

    fn create(path: &Path, tables: &str) -> Self {
        let connection = Connection::open(path).expect("create the database");
        connection
            .pragma_update(None, "journal_mode", "wal")
            .expect("use the WAL journal");
        connection
            .pragma_update(None, "synchronous", "off")
            .expect("write the made-up rows fast");
        connection.execute_batch(tables).expect("create the tables");

        Self {
            connection,
            second_shape: tables == SECOND_SHAPE,
            next_seq: 0,
        }
    }
}

/// The host over `fixtures/`.
pub(crate) fn host() -> TestHost {
    TestHost::new(env!("CARGO_MANIFEST_DIR"))
}

/// The absolute path of `relative` in the home folder of `host`.
pub(crate) fn path(host: &TestHost, relative: &str) -> String {
    host.home().join(relative).display().to_string()
}

/// Runs `run` as the part `opencode` of `host`.
pub(crate) fn as_part<R>(host: &TestHost, run: impl FnOnce() -> R) -> R {
    host.as_agent(AGENT, run)
}

/// The JSON of an answer in the first shape: the model `model` of the
/// provider `provider`, its tokens (`input`, `output`, `reasoning`, cache
/// `read` and `write`) and its `cost` in dollars.
pub(crate) fn first_shape_answer(
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
pub(crate) fn second_shape_answer(
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
pub(crate) fn first_shape_question() -> String {
    String::from(
        r#"{"role":"user","time":{"created":1},"agent":"code",
        "model":{"providerID":"openai","modelID":"gpt-5.5"}}"#,
    )
}

/// `seconds` after the epoch in milliseconds, as OpenCode writes times.
pub(crate) fn millis(seconds: u64) -> i64 {
    i64::try_from(seconds * 1000).expect("a time of the tests")
}

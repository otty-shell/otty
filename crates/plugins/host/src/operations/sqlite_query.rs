//! `sqlite.query`: one read-only statement on a SQLite database the part
//! declares, opened read-only while the agent may be writing to it.
//!
//! Four guards keep it read-only and short: the database is opened with
//! `SQLITE_OPEN_READ_ONLY` and `PRAGMA query_only`; a statement that is
//! not read-only is refused; an authorizer refuses `ATTACH`; the query is
//! interrupted after [`QUERY_TIMEOUT`].
//!
//! Key columns the part declares never reach the plugin's SQL. The
//! statement is first only prepared, to learn where its result columns
//! come from and where it names a key column; then it runs with every key
//! column read as `NULL`, and the host fills in the result columns that
//! are key columns themselves (see `sqlite_keys`).

use std::path::Path;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::Duration;

use otty_plugin_manifest::{Environment, KeyColumn, Permissions};
use rusqlite::hooks::{AuthAction, AuthContext, Authorization};
use rusqlite::types::{Value as SqlValue, ValueRef};
use rusqlite::{Connection, ErrorCode, OpenFlags};

use crate::operations::real_path::reachable_file;
use crate::operations::sqlite_keys::{
    Cell, ColumnOrigin, fill_keys, key_results,
};
use crate::operations::v0_1::otty::host::sqlite::{Param, Value};
use crate::operations::v0_1::otty::host::types::HostError;

/// The most rows one query gives (spec "Host operations").
pub(super) const MOST_ROWS: usize = 50_000;

/// How long one query may run (spec "Host operations").
pub(super) const QUERY_TIMEOUT: Duration = Duration::from_secs(10);

/// What the authorizer saw, shared with the closure SQLite calls.
type Seen = Arc<Mutex<Sightings>>;

/// What the authorizer saw while a statement was prepared.
#[derive(Default)]
struct Sightings {
    /// What it refused, for the error the plugin gets.
    refused: Option<String>,
    /// How many times the statement names each key column, in the order
    /// of the part's key columns.
    key_reads: Vec<usize>,
}

/// How the authorizer answers a read of a key column.
#[derive(Clone, Copy)]
enum KeyReads {
    /// Allowed and counted: the statement is only prepared, never run.
    Counted,
    /// Read as `NULL`, so the plugin's SQL never sees a key.
    AsNull,
}

/// Why reading the rows stopped.
enum QueryError {
    NotReadOnly,
    TooManyRows,
    Sqlite(rusqlite::Error),
}

impl From<rusqlite::Error> for QueryError {
    fn from(error: rusqlite::Error) -> Self {
        QueryError::Sqlite(error)
    }
}

/// The rows of one read-only statement `sql` on the database `database`,
/// with `params` bound to its `?`s; at most [`MOST_ROWS`], stopped after
/// `timeout`. A result column that is a key column the part declares
/// comes back as the place of its key, or as JSON with the places of its
/// key fields.
pub(super) fn query(
    permissions: &Permissions,
    env: &Environment,
    database: &str,
    sql: &str,
    params: &[Param],
    timeout: Duration,
) -> Result<Vec<Vec<Cell>>, HostError> {
    let path = Path::new(database);
    let real = reachable_file(permissions, env, path, Permissions::may_query)?;
    let key_columns = permissions.key_columns(path);
    let seen = Seen::default();
    let refused =
        |error: QueryError| host_error(&error, &seen, database, sql, timeout);

    let mut connection = open(&real)?;
    guard(&mut connection, &key_columns, KeyReads::Counted, &seen)?;
    let origins = result_origins(&connection, sql).map_err(refused)?;
    let key_reads = std::mem::take(&mut lock(&seen).key_reads);
    let keys =
        key_results(&connection, database, &key_columns, &origins, &key_reads)?;

    guard(&mut connection, &key_columns, KeyReads::AsNull, &seen)?;
    let stop = interrupt_after(&connection, timeout);
    let rows = read_rows(&connection, sql, params);
    // With no key columns the authorizer lets the host read the keys of
    // the result itself.
    guard(&mut connection, &[], KeyReads::AsNull, &seen)?;
    let cells = rows
        .and_then(|rows| Ok(fill_keys(&connection, database, &keys, rows)?));
    drop(stop);

    cells.map_err(refused)
}

/// The database at `path`, read-only, with `query_only` on.
fn open(path: &Path) -> Result<Connection, HostError> {
    let flags =
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let connection = Connection::open_with_flags(path, flags)
        .map_err(|error| failed(&error))?;

    connection
        .pragma_update(None, "query_only", true)
        .map_err(|error| failed(&error))?;

    Ok(connection)
}

/// Sets the authorizer of `connection`: it refuses `ATTACH` and answers a
/// read of one of `key_columns` as `key_reads` says, noting both in
/// `seen`.
fn guard(
    connection: &mut Connection,
    key_columns: &[KeyColumn],
    key_reads: KeyReads,
    seen: &Seen,
) -> Result<(), HostError> {
    let key_columns: Vec<(String, String)> = key_columns
        .iter()
        .map(|key| (key.table().to_string(), key.column().to_string()))
        .collect();
    lock(seen).key_reads = vec![0; key_columns.len()];
    let seen = Arc::clone(seen);

    let hook = move |context: AuthContext<'_>| match context.action {
        AuthAction::Attach { .. } => {
            lock(&seen).refused = Some("ATTACH".to_string());

            Authorization::Deny
        },
        AuthAction::Read {
            table_name,
            column_name,
        } => {
            let key = key_columns.iter().position(|(table, column)| {
                table.eq_ignore_ascii_case(table_name)
                    && column.eq_ignore_ascii_case(column_name)
            });

            match (key, key_reads) {
                (None, _) => Authorization::Allow,
                (Some(key), KeyReads::Counted) => {
                    lock(&seen).key_reads[key] += 1;

                    Authorization::Allow
                },
                (Some(_), KeyReads::AsNull) => Authorization::Ignore,
            }
        },
        _ => Authorization::Allow,
    };

    connection
        .authorizer(Some(hook))
        .map_err(|error| failed(&error))
}

/// Where each result column of `sql` comes from, when it is a column of a
/// table; the statement is prepared, never run.
fn result_origins(
    connection: &Connection,
    sql: &str,
) -> Result<Vec<Option<ColumnOrigin>>, QueryError> {
    let statement = connection.prepare(sql)?;
    if !statement.readonly() {
        return Err(QueryError::NotReadOnly);
    }

    let origins = statement
        .columns_with_metadata()
        .map(|column| match (column.table_name(), column.origin_name()) {
            (Some(table), Some(origin)) => {
                Some(ColumnOrigin::new(table, origin))
            },
            _ => None,
        })
        .collect();

    Ok(origins)
}

/// A guard that interrupts the query on `connection` once `timeout`
/// passed; dropping it first lets the query be.
fn interrupt_after(
    connection: &Connection,
    timeout: Duration,
) -> mpsc::Sender<()> {
    let handle = connection.get_interrupt_handle();
    let (stop, stopped) = mpsc::channel::<()>();

    let spawned = thread::Builder::new()
        .name("otty-plugin-sqlite-timeout".to_string())
        .spawn(move || {
            if stopped.recv_timeout(timeout) == Err(RecvTimeoutError::Timeout) {
                handle.interrupt();
            }
        });
    if spawned.is_err() {
        log::warn!("the timeout of a plugin's SQLite query could not start");
    }

    stop
}

/// The rows of `sql`, refused when it is not read-only.
fn read_rows(
    connection: &Connection,
    sql: &str,
    params: &[Param],
) -> Result<Vec<Vec<Value>>, QueryError> {
    let mut statement = connection.prepare(sql)?;
    if !statement.readonly() {
        return Err(QueryError::NotReadOnly);
    }

    let columns = statement.column_count();
    let params = params.iter().map(sql_value);
    let mut rows = statement.query(rusqlite::params_from_iter(params))?;

    let mut found = Vec::new();
    while let Some(row) = rows.next()? {
        if found.len() >= MOST_ROWS {
            return Err(QueryError::TooManyRows);
        }

        let values = (0..columns)
            .map(|column| row.get_ref(column).map(value_of))
            .collect::<Result<Vec<_>, _>>()?;
        found.push(values);
    }

    Ok(found)
}

/// `error` in the words of a host error.
fn host_error(
    error: &QueryError,
    seen: &Seen,
    database: &str,
    sql: &str,
    timeout: Duration,
) -> HostError {
    let refused = lock(seen).refused.take();

    match (error, refused) {
        (QueryError::NotReadOnly, _) => {
            HostError::NotAllowed(format!("not read-only: {sql}"))
        },
        (QueryError::TooManyRows, _) => {
            HostError::TooBig(format!("{sql} gives more than {MOST_ROWS} rows"))
        },
        (QueryError::Sqlite(_), Some(what)) => {
            HostError::NotAllowed(format!("{what} of {database}"))
        },
        (QueryError::Sqlite(error), None)
            if error.sqlite_error_code()
                == Some(ErrorCode::OperationInterrupted) =>
        {
            HostError::TimedOut(format!(
                "a query of {database} ran past {} s",
                timeout.as_secs_f32()
            ))
        },
        (QueryError::Sqlite(error), None) => failed(error),
    }
}

fn lock(seen: &Seen) -> std::sync::MutexGuard<'_, Sightings> {
    seen.lock().unwrap_or_else(PoisonError::into_inner)
}

fn sql_value(param: &Param) -> SqlValue {
    match param {
        Param::Null => SqlValue::Null,
        Param::Integer(number) => SqlValue::Integer(*number),
        Param::Real(number) => SqlValue::Real(*number),
        Param::Text(text) => SqlValue::Text(text.clone()),
        Param::Blob(bytes) => SqlValue::Blob(bytes.clone()),
    }
}

fn value_of(value: ValueRef<'_>) -> Value {
    match value {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(number) => Value::Integer(number),
        ValueRef::Real(number) => Value::Real(number),
        ValueRef::Text(text) => {
            Value::Text(String::from_utf8_lossy(text).into_owned())
        },
        ValueRef::Blob(bytes) => Value::Blob(bytes.to_vec()),
    }
}

fn failed(error: &rusqlite::Error) -> HostError {
    HostError::Failed(error.to_string())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::operations::real_path::not_allowed;
    use crate::operations::v0_1::otty::host::keys::{KeyPlace, SqliteField};
    use crate::testing::{TempDir, debug_err, files_permissions};

    /// A made-up key; no output may hold it.
    const KEY: &str = "ex-made-up-key";

    struct Home {
        dir: TempDir,
        env: Environment,
    }

    impl Home {
        fn path(&self, relative: &str) -> PathBuf {
            self.dir.path().join(relative)
        }

        fn database(&self) -> String {
            self.path(".alpha/agent.db").display().to_string()
        }

        /// A home folder with `.alpha/agent.db`: a `session` table, a
        /// `credential` table whose `value` column the part declares as a
        /// key, and a `login` table, with no `INTEGER PRIMARY KEY`, whose
        /// JSON column `data` holds a key in its field `/key`.
        fn new() -> Self {
            let dir = TempDir::new();
            let env = Environment::new(dir.path().to_path_buf());
            let home = Self { dir, env };

            std::fs::create_dir_all(home.path(".alpha")).expect("folder");
            home.write(&format!(
                "CREATE TABLE session (id INTEGER PRIMARY KEY, title TEXT, \
                 tokens INTEGER, cost REAL, data BLOB);
                 INSERT INTO session VALUES (1, 'Fix the cart', 1200, 0.5, x'0102');
                 INSERT INTO session VALUES (2, 'Notes', NULL, NULL, NULL);
                 CREATE TABLE credential (id INTEGER PRIMARY KEY, value TEXT);
                 INSERT INTO credential VALUES (1, '{KEY}');
                 CREATE TABLE login (name TEXT, data TEXT);
                 INSERT INTO login VALUES
                     ('work', '{{\"type\":\"api\",\"key\":\"{KEY}\"}}');"
            ));

            home
        }

        /// Runs `sql` on the database, as the agent writing to it.
        fn write(&self, sql: &str) {
            let connection =
                Connection::open(self.path(".alpha/agent.db")).expect("open");

            connection.execute_batch(sql).expect("written");
        }

        fn query_as(
            &self,
            part: &str,
            relative: &str,
            sql: &str,
            params: &[Param],
            timeout: Duration,
        ) -> Result<Vec<Vec<Cell>>, String> {
            let permissions = files_permissions(part, &self.env);
            let database = self.path(relative).display().to_string();

            debug_err(query(
                &permissions,
                &self.env,
                &database,
                sql,
                params,
                timeout,
            ))
        }

        fn query(&self, sql: &str) -> Result<Vec<Vec<Cell>>, String> {
            self.query_as("alpha", ".alpha/agent.db", sql, &[], QUERY_TIMEOUT)
        }

        /// The place of the key of `table.column` in row `row_id`, or of
        /// its field `pointer`.
        fn place(
            &self,
            table: &str,
            column: &str,
            row_id: i64,
            pointer: Option<&str>,
        ) -> KeyPlace {
            KeyPlace::SqliteField(SqliteField {
                database: self.database(),
                table: table.to_string(),
                column: column.to_string(),
                row_id,
                pointer: pointer.map(str::to_string),
            })
        }
    }

    fn shown(rows: Result<Vec<Vec<Cell>>, String>) -> Result<String, String> {
        let shown = rows.map(|rows| format!("{rows:?}"));
        let (Ok(text) | Err(text)) = &shown;
        assert!(!text.contains(KEY), "{text}");

        shown
    }

    /// `rows` as the cells of plain values.
    fn values(rows: Vec<Vec<Value>>) -> String {
        let cells: Vec<Vec<Cell>> = rows
            .into_iter()
            .map(|row| row.into_iter().map(Cell::Value).collect())
            .collect();

        format!("{cells:?}")
    }

    fn refused_with(
        rows: Result<Vec<Vec<Cell>>, String>,
        variant: &str,
        text: &str,
    ) -> bool {
        rows.is_err_and(|error| {
            assert!(!error.contains(KEY), "{error}");

            error.starts_with(&format!("HostError::{variant}("))
                && error.contains(text)
        })
    }

    #[test]
    fn a_select_gives_its_rows_with_bound_params() {
        let home = Home::new();

        let rows = home.query_as(
            "alpha",
            ".alpha/agent.db",
            "SELECT id, title, tokens, cost, data FROM session WHERE id >= ?1 \
             ORDER BY id",
            &[Param::Integer(1)],
            QUERY_TIMEOUT,
        );

        let expected = vec![
            vec![
                Value::Integer(1),
                Value::Text("Fix the cart".to_string()),
                Value::Integer(1200),
                Value::Real(0.5),
                Value::Blob(vec![1, 2]),
            ],
            vec![
                Value::Integer(2),
                Value::Text("Notes".to_string()),
                Value::Null,
                Value::Null,
                Value::Null,
            ],
        ];
        assert_eq!(shown(rows), Ok(values(expected)));
    }

    #[test]
    fn a_write_statement_is_refused_and_changes_nothing() {
        let home = Home::new();

        let delete = home.query("DELETE FROM session");
        let create = home.query("CREATE TABLE other (id INTEGER)");

        assert!(refused_with(delete, "NotAllowed", "not read-only"));
        assert!(refused_with(create, "NotAllowed", "not read-only"));
        let left = home.query("SELECT count(*) FROM session");
        assert_eq!(shown(left), Ok(values(vec![vec![Value::Integer(2)]])));
    }

    #[test]
    fn attach_is_refused() {
        let home = Home::new();
        let other = home.path(".alpha/other.db").display().to_string();

        let attach = home.query(&format!("ATTACH DATABASE '{other}' AS other"));

        assert!(refused_with(attach, "NotAllowed", "ATTACH"));
    }

    #[test]
    fn a_key_column_comes_back_as_the_place_of_its_key() {
        let home = Home::new();
        let expected = format!(
            "{:?}",
            [[
                Cell::Value(Value::Integer(1)),
                Cell::Key(home.place("credential", "value", 1, None)),
            ]]
        );

        let named = home.query("SELECT id, value FROM credential");
        let star = home.query("SELECT * FROM credential");
        let rowid = home.query("SELECT rowid, VALUE FROM CREDENTIAL");

        assert_eq!(shown(named), Ok(expected.clone()));
        assert_eq!(shown(star), Ok(expected.clone()));
        assert_eq!(shown(rowid), Ok(expected));
    }

    #[test]
    fn a_json_key_column_comes_back_with_its_key_fields_as_places() {
        let home = Home::new();

        let rows = home.query("SELECT name, data, rowid FROM login");

        let expected = [[
            Cell::Value(Value::Text("work".to_string())),
            Cell::JsonWithKeys {
                json: r#"{"key":{"$key":0},"type":"api"}"#.to_string(),
                places: vec![home.place("login", "data", 1, Some("/key"))],
            },
            Cell::Value(Value::Integer(1)),
        ]];
        assert_eq!(shown(rows), Ok(format!("{expected:?}")));
    }

    #[test]
    fn a_key_column_needs_the_rowid_of_its_table_in_the_result() {
        let home = Home::new();

        let alone = home.query("SELECT value FROM credential");
        let other_rowid = home.query(
            "SELECT session.id, credential.value FROM session, credential",
        );

        assert!(refused_with(alone, "Failed", "rowid of credential"));
        assert!(refused_with(other_rowid, "Failed", "rowid of credential"));
    }

    #[test]
    fn a_key_column_named_outside_the_result_columns_is_refused() {
        let home = Home::new();
        let statements = [
            "SELECT id FROM credential WHERE value LIKE 'ex-%'",
            "SELECT id, value FROM credential WHERE value LIKE 'ex-%'",
            "SELECT id, upper(value) FROM credential",
            "SELECT id, length(value) FROM credential",
            "SELECT id, value AS v FROM credential WHERE v LIKE 'ex-%'",
            "SELECT id FROM credential ORDER BY value",
            "SELECT session.id FROM session JOIN credential \
             ON substr(credential.value, 1, 1) = 'e'",
            "SELECT rowid, json_extract(data, '$.type') FROM login",
        ];

        for sql in statements {
            let rows = home.query(sql);
            let printed = format!("{rows:?}");

            assert!(
                refused_with(rows, "NotAllowed", "key column"),
                "{sql}: {printed}"
            );
        }
    }

    #[test]
    fn the_sql_reads_a_key_column_as_null_where_the_host_cannot_tell() {
        let home = Home::new();

        let through_a_subquery = home.query(
            "SELECT id, value FROM (SELECT id, value FROM credential) \
             WHERE value LIKE 'ex-%'",
        );
        let through_a_cte = home.query(
            "WITH c AS (SELECT id, value FROM credential) \
             SELECT id, value FROM c WHERE value IS NOT NULL",
        );

        assert_eq!(shown(through_a_subquery), Ok("[]".to_string()));
        assert_eq!(shown(through_a_cte), Ok("[]".to_string()));
    }

    #[test]
    fn a_key_column_with_no_key_in_it_comes_back_as_null() {
        let home = Home::new();
        home.write(
            "INSERT INTO credential VALUES (2, NULL), (3, '  '), (4, x'6b6579');
             INSERT INTO login VALUES ('broken', 'not JSON'), ('empty', NULL);",
        );

        let column =
            home.query("SELECT id, value FROM credential WHERE id > 1");
        let json = home.query("SELECT rowid, data FROM login WHERE rowid > 1");

        let null = |id: i64| vec![Value::Integer(id), Value::Null];
        assert_eq!(shown(column), Ok(values(vec![null(2), null(3), null(4)])));
        assert_eq!(shown(json), Ok(values(vec![null(2), null(3)])));
    }

    #[test]
    fn a_slow_query_stops() {
        let home = Home::new();

        let rows = home.query_as(
            "alpha",
            ".alpha/agent.db",
            "WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM n) \
             SELECT count(*) FROM n",
            &[],
            Duration::from_millis(100),
        );

        assert!(refused_with(rows, "TimedOut", "ran past"));
    }

    #[test]
    fn more_rows_than_the_most_is_too_big() {
        let home = Home::new();

        let rows = home.query(&format!(
            "WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM n \
             WHERE x < {}) SELECT x FROM n",
            MOST_ROWS + 1
        ));

        assert!(refused_with(rows, "TooBig", "rows"));
    }

    #[test]
    fn a_database_outside_the_globs_or_of_another_part_is_refused() {
        let home = Home::new();
        let database = home.path(".alpha/agent.db");

        let beta = home.query_as(
            "beta",
            ".alpha/agent.db",
            "SELECT id FROM session",
            &[],
            QUERY_TIMEOUT,
        );

        assert_eq!(
            shown(beta),
            debug_err(Err(not_allowed(&database))).map(|()| String::new())
        );
    }
}

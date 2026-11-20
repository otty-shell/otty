//! `sqlite.query`: one read-only statement on a SQLite database the part
//! declares, opened read-only while the agent may be writing to it.
//!
//! Four guards keep it read-only and short: the database is opened with
//! `SQLITE_OPEN_READ_ONLY` and `PRAGMA query_only`; a statement that is
//! not read-only is refused; an authorizer refuses `ATTACH` and reading a
//! key column the part declares (`keys.open` gives those as key handles);
//! the query is interrupted after [`QUERY_TIMEOUT`].

use std::path::Path;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::Duration;

use otty_plugin_manifest::{Environment, Permissions};
use rusqlite::hooks::{AuthAction, AuthContext, Authorization};
use rusqlite::types::{Value as SqlValue, ValueRef};
use rusqlite::{Connection, ErrorCode, OpenFlags};

use crate::operations::real_path::reachable_file;
use crate::operations::v0_1::otty::host::sqlite::{Param, Value};
use crate::operations::v0_1::otty::host::types::HostError;

/// The most rows one query gives (spec "Host operations").
pub(super) const MOST_ROWS: usize = 50_000;

/// How long one query may run (spec "Host operations").
pub(super) const QUERY_TIMEOUT: Duration = Duration::from_secs(10);

/// What the authorizer refused, for the error the plugin gets.
type Refusals = Arc<Mutex<Option<String>>>;

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
/// `timeout`.
pub(super) fn query(
    permissions: &Permissions,
    env: &Environment,
    database: &str,
    sql: &str,
    params: &[Param],
    timeout: Duration,
) -> Result<Vec<Vec<Value>>, HostError> {
    let path = Path::new(database);
    let real = reachable_file(permissions, env, path, Permissions::may_query)?;
    let key_columns = permissions.key_columns(path);

    let mut connection = open(&real)?;
    let refusals = Refusals::default();
    let hook = authorizer(key_columns, Arc::clone(&refusals));
    connection
        .authorizer(Some(hook))
        .map_err(|error| failed(&error))?;

    let stop = interrupt_after(&connection, timeout);
    let rows = read_rows(&connection, sql, params);
    drop(stop);

    rows.map_err(|error| host_error(&error, &refusals, database, sql, timeout))
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

/// The authorizer: refuses `ATTACH` and reading one of `key_columns`,
/// noting what it refused in `refusals`.
fn authorizer(
    key_columns: Vec<(String, String)>,
    refusals: Refusals,
) -> impl FnMut(AuthContext<'_>) -> Authorization + Send + 'static {
    move |context| {
        let refused = match context.action {
            AuthAction::Attach { .. } => Some("ATTACH".to_string()),
            AuthAction::Read {
                table_name,
                column_name,
            } if is_key_column(&key_columns, table_name, column_name) => {
                Some(format!("the key column {table_name}.{column_name}"))
            },
            _ => None,
        };

        match refused {
            Some(what) => {
                *refusals.lock().unwrap_or_else(PoisonError::into_inner) =
                    Some(what);

                Authorization::Deny
            },
            None => Authorization::Allow,
        }
    }
}

/// Whether `table.column` is one of `key_columns`; SQL names compare with
/// no regard to case.
fn is_key_column(
    key_columns: &[(String, String)],
    table: &str,
    column: &str,
) -> bool {
    key_columns.iter().any(|(key_table, key_column)| {
        key_table.eq_ignore_ascii_case(table)
            && key_column.eq_ignore_ascii_case(column)
    })
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
    refusals: &Refusals,
    database: &str,
    sql: &str,
    timeout: Duration,
) -> HostError {
    let refused = refusals
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .take();

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
    use crate::testing::{TempDir, debug_err, files_permissions};

    struct Home {
        dir: TempDir,
        env: Environment,
    }

    impl Home {
        fn path(&self, relative: &str) -> PathBuf {
            self.dir.path().join(relative)
        }

        /// A home folder with `.alpha/agent.db`: a `session` table and a
        /// `credential` table whose `value` column the part declares as a
        /// key.
        fn new() -> Self {
            let dir = TempDir::new();
            let env = Environment::new(dir.path().to_path_buf());
            let home = Self { dir, env };

            let database = home.path(".alpha/agent.db");
            std::fs::create_dir_all(home.path(".alpha")).expect("folder");
            let connection = Connection::open(&database).expect("database");
            connection
                .execute_batch(
                    "CREATE TABLE session (id INTEGER PRIMARY KEY, title TEXT, \
                     tokens INTEGER, cost REAL, data BLOB);
                     INSERT INTO session VALUES (1, 'Fix the cart', 1200, 0.5, x'0102');
                     INSERT INTO session VALUES (2, 'Notes', NULL, NULL, NULL);
                     CREATE TABLE credential (id INTEGER PRIMARY KEY, value TEXT);
                     INSERT INTO credential VALUES (1, '{\"key\":\"sk-1\"}');",
                )
                .expect("tables");

            home
        }

        fn query_as(
            &self,
            part: &str,
            relative: &str,
            sql: &str,
            params: &[Param],
            timeout: Duration,
        ) -> Result<Vec<Vec<Value>>, String> {
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

        fn query(&self, sql: &str) -> Result<Vec<Vec<Value>>, String> {
            self.query_as("alpha", ".alpha/agent.db", sql, &[], QUERY_TIMEOUT)
        }
    }

    fn shown(rows: Result<Vec<Vec<Value>>, String>) -> Result<String, String> {
        rows.map(|rows| format!("{rows:?}"))
    }

    fn refused_with(
        rows: Result<Vec<Vec<Value>>, String>,
        variant: &str,
        text: &str,
    ) -> bool {
        rows.is_err_and(|error| {
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
        assert_eq!(shown(rows), Ok(format!("{expected:?}")));
    }

    #[test]
    fn a_write_statement_is_refused_and_changes_nothing() {
        let home = Home::new();

        let delete = home.query("DELETE FROM session");
        let create = home.query("CREATE TABLE other (id INTEGER)");

        assert!(refused_with(delete, "NotAllowed", "not read-only"));
        assert!(refused_with(create, "NotAllowed", "not read-only"));
        let left = home.query("SELECT count(*) FROM session");
        assert_eq!(shown(left), Ok(format!("{:?}", [[Value::Integer(2)]])));
    }

    #[test]
    fn attach_is_refused() {
        let home = Home::new();
        let other = home.path(".alpha/other.db").display().to_string();

        let attach = home.query(&format!("ATTACH DATABASE '{other}' AS other"));

        assert!(refused_with(attach, "NotAllowed", "ATTACH"));
    }

    #[test]
    fn a_declared_key_column_read_as_text_is_refused() {
        let home = Home::new();

        let value = home.query("SELECT value FROM credential");
        let star = home.query("SELECT * FROM credential");
        let upper = home.query("SELECT VALUE FROM CREDENTIAL");
        let id = home.query("SELECT id FROM credential");

        assert!(refused_with(value, "NotAllowed", "credential.value"));
        assert!(refused_with(star, "NotAllowed", "credential.value"));
        assert!(refused_with(upper, "NotAllowed", "value"));
        assert_eq!(shown(id), Ok(format!("{:?}", [[Value::Integer(1)]])));
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

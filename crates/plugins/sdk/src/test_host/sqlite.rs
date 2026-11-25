//! SQLite fixture files, opened read-only as OTTY opens them, queried
//! with OTTY's rules: `ATTACH` is refused, the SQL reads key columns as
//! `NULL`, and the result columns that are key columns come back as key
//! handles (see `sqlite_keys`).

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use otty_plugin_manifest::{KeyColumn, Permissions};
use rusqlite::hooks::{AuthAction, AuthContext, Authorization};
use rusqlite::types::{Value as SqlValue, ValueRef};
use rusqlite::{Connection, OpenFlags};
use serde_json::Value as Json;

use crate::sqlite::{Param, Value};
use crate::test_host::sqlite_keys::{ColumnOrigin, fill_keys, key_results};
use crate::test_host::{HostState, not_allowed};
use crate::{HostError, SqliteField};

/// The most rows one query gives (spec "Host operations").
const MOST_ROWS: usize = 50_000;

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

/// The rows of one read-only `SELECT` on `database`. A result column that
/// is a key column the part declares comes back as a key handle, or as
/// JSON with handles in place of its key fields.
pub(super) fn query(
    state: &mut HostState,
    permissions: &Permissions,
    database: &str,
    sql: &str,
    params: &[Param],
) -> Result<Vec<Vec<Value>>, HostError> {
    let path = PathBuf::from(database);
    if !permissions.may_query(&path) {
        return Err(not_allowed(&path));
    }
    let key_columns = permissions.key_columns(&path);
    let seen = Seen::default();
    let refused = |error: rusqlite::Error| match lock(&seen).refused.take() {
        Some(what) => HostError::NotAllowed(format!("{what} of {database}")),
        None => failed(error),
    };

    let mut connection = open(&path)?;
    guard(&mut connection, &key_columns, KeyReads::Counted, &seen)?;
    let Some(origins) = result_origins(&connection, sql).map_err(refused)?
    else {
        return Err(HostError::NotAllowed(format!("not read-only: {sql}")));
    };
    let key_reads = std::mem::take(&mut lock(&seen).key_reads);
    let keys =
        key_results(&connection, database, &key_columns, &origins, &key_reads)?;

    guard(&mut connection, &key_columns, KeyReads::AsNull, &seen)?;
    let rows = read_rows(&connection, sql, params)?;
    // With no key columns the authorizer lets the host read the keys of
    // the result itself.
    guard(&mut connection, &[], KeyReads::AsNull, &seen)?;

    fill_keys(state, &connection, &keys, rows).map_err(failed)
}

/// The key in the field `field` of a database.
pub(super) fn key_field(field: &SqliteField) -> Result<String, HostError> {
    let connection = open(Path::new(&field.database))?;
    let sql = format!(
        "SELECT {} FROM {} WHERE rowid = ?1",
        quoted(&field.column),
        quoted(&field.table)
    );

    let text: String = connection
        .query_row(&sql, [field.row_id], |row| row.get(0))
        .map_err(|_| {
            HostError::NotFound(format!("{} row {}", field.table, field.row_id))
        })?;

    let Some(pointer) = &field.pointer else {
        return Ok(text);
    };
    let document: Json = serde_json::from_str(&text).map_err(|_| {
        HostError::Failed(format!(
            "{}.{} is not JSON",
            field.table, field.column
        ))
    })?;

    document
        .pointer(pointer)
        .and_then(Json::as_str)
        .map(str::to_string)
        .ok_or_else(|| {
            HostError::NotFound(format!(
                "{}.{} {pointer}",
                field.table, field.column
            ))
        })
}

/// `name` as an SQL identifier in double quotes.
pub(super) fn quoted(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

pub(super) fn failed(error: rusqlite::Error) -> HostError {
    HostError::Failed(error.to_string())
}

/// Sets the authorizer of `connection`: it refuses `ATTACH` and answers a
/// read of one of `key_columns` as `key_reads` says, noting both in
/// `seen`. SQL names compare with no regard to case.
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

    connection.authorizer(Some(hook)).map_err(failed)
}

/// Where each result column of `sql` comes from, when it is a column of a
/// table; `None` when `sql` is not read-only. The statement is prepared,
/// never run.
fn result_origins(
    connection: &Connection,
    sql: &str,
) -> Result<Option<Vec<Option<ColumnOrigin>>>, rusqlite::Error> {
    let statement = connection.prepare(sql)?;
    if !statement.readonly() {
        return Ok(None);
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

    Ok(Some(origins))
}

/// The rows of `sql`, at most [`MOST_ROWS`].
fn read_rows(
    connection: &Connection,
    sql: &str,
    params: &[Param],
) -> Result<Vec<Vec<Value>>, HostError> {
    let mut statement = connection.prepare(sql).map_err(failed)?;
    let columns = statement.column_count();
    let params = params.iter().map(sql_value);
    let mut rows = statement
        .query(rusqlite::params_from_iter(params))
        .map_err(failed)?;

    let mut found = Vec::new();
    while let Some(row) = rows.next().map_err(failed)? {
        if found.len() >= MOST_ROWS {
            return Err(HostError::TooBig(format!("{sql} gives more rows")));
        }
        let values = (0..columns)
            .map(|column| row.get_ref(column).map(value_of))
            .collect::<Result<Vec<_>, _>>()
            .map_err(failed)?;

        found.push(values);
    }

    Ok(found)
}

fn lock(seen: &Seen) -> MutexGuard<'_, Sightings> {
    seen.lock().unwrap_or_else(PoisonError::into_inner)
}

fn open(path: &Path) -> Result<Connection, HostError> {
    let flags =
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let connection =
        Connection::open_with_flags(path, flags).map_err(failed)?;
    connection
        .pragma_update(None, "query_only", true)
        .map_err(failed)?;

    Ok(connection)
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

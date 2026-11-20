//! SQLite fixture files, opened read-only as OTTY opens them.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use otty_plugin_manifest::Permissions;
use rusqlite::hooks::{AuthAction, AuthContext, Authorization};
use rusqlite::types::{Value as SqlValue, ValueRef};
use rusqlite::{Connection, OpenFlags};
use serde_json::Value as Json;

use crate::sqlite::{Param, Value};
use crate::test_host::not_allowed;
use crate::{HostError, SqliteField};

/// The most rows one query gives (spec "Host operations").
const MOST_ROWS: usize = 50_000;

/// What the authorizer refused, for the error the plugin gets.
type Refusals = Arc<Mutex<Option<String>>>;

/// The rows of one read-only `SELECT` on `database`.
pub(super) fn query(
    permissions: &Permissions,
    database: &str,
    sql: &str,
    params: &[Param],
) -> Result<Vec<Vec<Value>>, HostError> {
    let path = PathBuf::from(database);
    if !permissions.may_query(&path) {
        return Err(not_allowed(&path));
    }

    let mut connection = open(&path)?;
    let refusals = Refusals::default();
    refuse_attach_and_key_columns(
        &mut connection,
        permissions.key_columns(&path),
        Arc::clone(&refusals),
    )?;

    let mut statement = connection.prepare(sql).map_err(|error| {
        let refused = refusals
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        match refused {
            Some(what) => {
                HostError::NotAllowed(format!("{what} of {database}"))
            },
            None => failed(error),
        }
    })?;
    if !statement.readonly() {
        return Err(HostError::NotAllowed(format!("not read-only: {sql}")));
    }

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

/// Makes `connection` refuse `ATTACH` and any statement that reads one of
/// `key_columns`, as the host refuses them, noting what it refused in
/// `refusals`. SQL names compare with no regard to case.
fn refuse_attach_and_key_columns(
    connection: &mut Connection,
    key_columns: Vec<(String, String)>,
    refusals: Refusals,
) -> Result<(), HostError> {
    connection
        .authorizer(Some(move |context: AuthContext<'_>| {
            let refused = match context.action {
                AuthAction::Attach { .. } => Some("ATTACH".to_string()),
                AuthAction::Read {
                    table_name,
                    column_name,
                } if is_key_column(&key_columns, table_name, column_name) => {
                    Some(format!(
                        "the key column {table_name}.{column_name} \
                         (open its key with keys::open)"
                    ))
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
        }))
        .map_err(failed)
}

/// Whether `table.column` is one of `key_columns`.
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

/// `name` as an SQL identifier in double quotes.
fn quoted(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

fn failed(error: rusqlite::Error) -> HostError {
    HostError::Failed(error.to_string())
}

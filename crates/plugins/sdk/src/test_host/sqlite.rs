//! SQLite fixture files, opened read-only as OTTY opens them.

use std::path::{Path, PathBuf};

use otty_plugin_manifest::Permissions;
use rusqlite::hooks::{AuthAction, AuthContext, Authorization};
use rusqlite::types::{Value as SqlValue, ValueRef};
use rusqlite::{Connection, ErrorCode, OpenFlags};
use serde_json::Value as Json;

use crate::sqlite::{Param, Value};
use crate::test_host::not_allowed;
use crate::{HostError, SqliteField};

/// The most rows one query gives (spec "Host operations").
const MOST_ROWS: usize = 50_000;

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
    refuse_key_columns(&mut connection, &path, permissions)?;
    let mut statement = connection.prepare(sql).map_err(|error| {
        if error.sqlite_error_code()
            == Some(ErrorCode::AuthorizationForStatementDenied)
        {
            return HostError::NotAllowed(format!(
                "a key column of {database}: open its key with keys::open"
            ));
        }

        failed(error)
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

/// Makes `connection` refuse any statement that reads a column of
/// `database` the part declares as a key, as the host refuses it.
fn refuse_key_columns(
    connection: &mut Connection,
    database: &Path,
    permissions: &Permissions,
) -> Result<(), HostError> {
    let database = database.to_path_buf();
    let permissions = permissions.clone();

    connection
        .authorizer(Some(move |context: AuthContext<'_>| {
            match context.action {
                AuthAction::Read {
                    table_name,
                    column_name,
                } if permissions.is_key_column(
                    &database,
                    table_name,
                    column_name,
                ) =>
                {
                    Authorization::Deny
                },
                _ => Authorization::Allow,
            }
        }))
        .map_err(failed)
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

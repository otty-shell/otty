//! Key columns in the result of a query, as OTTY gives them: a result
//! column that is a key column the part declares comes back as a key
//! handle, or as JSON with handles in place of its key fields.
//!
//! The query reads key columns as `NULL`, so the test host reads each key
//! itself, at the row the result's rowid column of that table names. A
//! statement that names a key column anywhere but as a result column is
//! refused, as OTTY refuses it.

use otty_plugin_manifest::KeyColumn;
use rusqlite::types::ValueRef;
use rusqlite::{Connection, OptionalExtension, Statement};
use serde_json::Value as Json;

use crate::sqlite::Value;
use crate::test_host::HostState;
use crate::test_host::files::replace_keys;
use crate::test_host::sqlite::{failed, quoted};
use crate::{HostError, JsonWithKeys};

/// The table and column a result column comes from.
#[derive(Clone)]
pub(super) struct ColumnOrigin {
    table: String,
    column: String,
}

impl ColumnOrigin {
    /// The column `column` of the table `table`, as SQLite names them.
    pub(super) fn new(table: &str, column: &str) -> Self {
        Self {
            table: table.to_string(),
            column: column.to_string(),
        }
    }

    /// Whether this is the declared key column `key`.
    fn is(&self, key: &KeyColumn) -> bool {
        self.table.eq_ignore_ascii_case(key.table())
            && self.column.eq_ignore_ascii_case(key.column())
    }
}

/// A result column that is a key column, and where its row id is.
pub(super) struct KeyResult {
    /// The result column of the key.
    index: usize,
    /// The result column of the rowid of the key's table.
    rowid: usize,
    /// The key column as the part declares it.
    declared: KeyColumn,
    /// The key's table and column as SQLite names them.
    origin: ColumnOrigin,
}

/// The result columns that are key columns of `key_columns`, from the
/// `origins` of the result columns and how many times the statement names
/// each key column (`key_reads`). A key column named anywhere but as a
/// result column is refused, and so is a key column whose table's rowid is
/// not in the result too.
pub(super) fn key_results(
    connection: &Connection,
    database: &str,
    key_columns: &[KeyColumn],
    origins: &[Option<ColumnOrigin>],
    key_reads: &[usize],
) -> Result<Vec<KeyResult>, HostError> {
    let found: Vec<(usize, &ColumnOrigin, &KeyColumn)> = origins
        .iter()
        .enumerate()
        .filter_map(|(index, origin)| {
            let origin = origin.as_ref()?;
            let declared = key_columns.iter().find(|key| origin.is(key))?;

            Some((index, origin, declared))
        })
        .collect();

    for (key, reads) in key_columns.iter().zip(key_reads) {
        let as_results = found
            .iter()
            .filter(|(_, _, declared)| *declared == key)
            .count();
        if *reads > as_results {
            return Err(HostError::NotAllowed(format!(
                "the key column {}.{} of {database} named outside the \
                 result columns",
                key.table(),
                key.column()
            )));
        }
    }

    found
        .into_iter()
        .map(|(index, origin, declared)| {
            let rowid = rowid_column(connection, origins, &origin.table)?
                .ok_or_else(|| {
                    HostError::Failed(format!(
                        "{0}.{1} comes back as a key only with the rowid of \
                         {0} in the result",
                        declared.table(),
                        declared.column()
                    ))
                })?;

            Ok(KeyResult {
                index,
                rowid,
                declared: declared.clone(),
                origin: origin.clone(),
            })
        })
        .collect()
}

/// `rows` with the result columns of `keys` filled with handles to the
/// keys the test host reads at their rows.
pub(super) fn fill_keys(
    state: &mut HostState,
    connection: &Connection,
    keys: &[KeyResult],
    rows: Vec<Vec<Value>>,
) -> Result<Vec<Vec<Value>>, rusqlite::Error> {
    let mut readers = keys
        .iter()
        .map(|key| {
            connection.prepare(&format!(
                "SELECT {} FROM {} WHERE rowid = ?1",
                quoted(&key.origin.column),
                quoted(&key.origin.table)
            ))
        })
        .collect::<Result<Vec<_>, _>>()?;

    rows.into_iter()
        .map(|mut row| {
            let row_ids: Vec<Option<i64>> = keys
                .iter()
                .map(|key| match row.get(key.rowid) {
                    Some(Value::Integer(row_id)) => Some(*row_id),
                    _ => None,
                })
                .collect();

            for ((key, reader), row_id) in
                keys.iter().zip(&mut readers).zip(row_ids)
            {
                row[key.index] = match row_id {
                    Some(row_id) => key_value(state, reader, key, row_id)?,
                    None => Value::Null,
                };
            }

            Ok(row)
        })
        .collect()
}

/// The result column of the rowid of `table`: its `rowid`, or its
/// `INTEGER PRIMARY KEY`, which SQLite names in place of the rowid.
fn rowid_column(
    connection: &Connection,
    origins: &[Option<ColumnOrigin>],
    table: &str,
) -> Result<Option<usize>, HostError> {
    let primary_key = integer_primary_key(connection, table).map_err(failed)?;

    let found = origins.iter().position(|origin| {
        origin.as_ref().is_some_and(|origin| {
            origin.table.eq_ignore_ascii_case(table)
                && (origin.column.eq_ignore_ascii_case("rowid")
                    || primary_key.as_deref().is_some_and(|primary_key| {
                        origin.column.eq_ignore_ascii_case(primary_key)
                    }))
        })
    });

    Ok(found)
}

/// The `INTEGER PRIMARY KEY` of `table`, the column that is its rowid.
fn integer_primary_key(
    connection: &Connection,
    table: &str,
) -> Result<Option<String>, rusqlite::Error> {
    let mut statement = connection
        .prepare("SELECT name, type FROM pragma_table_info(?1) WHERE pk > 0")?;
    let primary_key = statement
        .query_map([table], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    match primary_key.as_slice() {
        [(name, kind)] if kind.eq_ignore_ascii_case("INTEGER") => {
            Ok(Some(name.clone()))
        },
        _ => Ok(None),
    }
}

/// The value of the key column `key` at the row `row_id`: a key handle,
/// or JSON with handles in place of its key fields. A key column with no
/// key in it, or a JSON key column that is not JSON, gives `NULL`.
fn key_value(
    state: &mut HostState,
    reader: &mut Statement<'_>,
    key: &KeyResult,
    row_id: i64,
) -> Result<Value, rusqlite::Error> {
    let text = reader
        .query_row([row_id], |row| {
            Ok(match row.get_ref(0)? {
                ValueRef::Text(text) => {
                    Some(String::from_utf8_lossy(text).into_owned())
                },
                _ => None,
            })
        })
        .optional()?
        .flatten();
    let Some(text) = text else {
        return Ok(Value::Null);
    };

    if key.declared.fields().is_empty() {
        if text.trim().is_empty() {
            return Ok(Value::Null);
        }

        return Ok(Value::Key(state.add_key(text)));
    }

    let Ok(mut document) = serde_json::from_str::<Json>(&text) else {
        return Ok(Value::Null);
    };
    let mut keys = Vec::new();
    replace_keys(&mut document, "", key.declared.fields(), &mut |value| {
        keys.push(state.add_key(value));

        keys.len() - 1
    });

    Ok(Value::JsonWithKeys(JsonWithKeys::new(
        document.to_string(),
        keys,
    )))
}

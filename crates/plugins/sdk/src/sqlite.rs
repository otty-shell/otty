//! Read-only queries of the SQLite databases the called part declares in
//! `sqlite`.

/// A value bound to a `?` of the SQL.
pub use crate::bindings::host::otty::host::sqlite::Param;
use crate::{HostError, JsonWithKeys, Key};

/// One value of a row. A column the part declares in `keys` comes back as
/// a [`Key`]; a JSON column with key fields as [`JsonWithKeys`].
#[derive(Debug)]
pub enum Value {
    /// SQL `NULL`.
    Null,
    /// An integer.
    Integer(i64),
    /// A floating point number.
    Real(f64),
    /// Text.
    Text(String),
    /// Bytes.
    Blob(Vec<u8>),
    /// A key column.
    Key(Key),
    /// A JSON column whose key fields are replaced by handles.
    JsonWithKeys(JsonWithKeys),
}

impl Value {
    #[cfg(target_arch = "wasm32")]
    fn from_wasm(
        value: crate::bindings::host::otty::host::sqlite::Value,
    ) -> Self {
        use crate::bindings::host::otty::host::sqlite::Value as Wit;

        match value {
            Wit::Null => Value::Null,
            Wit::Integer(number) => Value::Integer(number),
            Wit::Real(number) => Value::Real(number),
            Wit::Text(text) => Value::Text(text),
            Wit::Blob(bytes) => Value::Blob(bytes),
            Wit::Key(key) => Value::Key(Key::from_wasm(key)),
            Wit::JsonWithKeys(document) => {
                Value::JsonWithKeys(JsonWithKeys::from_wasm(document))
            },
        }
    }
}

/// The rows of one read-only `SELECT` on `database`, with `params` bound
/// to its `?`s in order: at most 50,000 rows, within 10 s.
pub fn query(
    database: &str,
    sql: &str,
    params: &[Param],
) -> Result<Vec<Vec<Value>>, HostError> {
    #[cfg(target_arch = "wasm32")]
    {
        let rows = crate::bindings::host::otty::host::sqlite::query(
            database, sql, params,
        )?;

        Ok(rows
            .into_iter()
            .map(|row| row.into_iter().map(Value::from_wasm).collect())
            .collect())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        crate::test_host::query(database, sql, params)
    }
}

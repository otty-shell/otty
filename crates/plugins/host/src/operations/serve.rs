//! How the host serves the operations of `otty:host@0.1.0` to the part
//! being called. Each one checks the permissions of that part only.
//!
//! Every operation works. The bodies of files, lines, SQLite, `env`,
//! keys, HTTP, the keyring and `files.remove` live in modules next to
//! this one; `programs` and `terminal` are served in modules of their
//! own.

use std::io;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use wasmtime::component::Resource;

use crate::calls::CallState;
use crate::keys::KeyHandle;
use crate::operations;
use crate::operations::lines::read_lines;
use crate::operations::listing::list;
use crate::operations::remove;
use crate::operations::small_file::read_small_file;
use crate::operations::sqlite_keys::with_handles;
use crate::operations::sqlite_query::{QUERY_TIMEOUT, query};
use crate::operations::v0_1::otty::host::files::{Batch, FileInfo};
use crate::operations::v0_1::otty::host::http::{Request, Response};
use crate::operations::v0_1::otty::host::keys::{JsonWithKeys, KeyPlace};
use crate::operations::v0_1::otty::host::log::Level;
use crate::operations::v0_1::otty::host::sqlite::{Param, Value};
use crate::operations::v0_1::otty::host::types::HostError;
use crate::operations::v0_1::otty::host::{
    clock, env, files, http, keyring, keys, log, sqlite, types,
};
use crate::operations::variables::variable;

impl types::Host for CallState {}

impl keys::HostKey for CallState {
    fn fingerprint(
        &mut self,
        key: Resource<KeyHandle>,
    ) -> Result<String, HostError> {
        operations::keys::fingerprint(self, &key)
    }

    fn has_prefix(
        &mut self,
        key: Resource<KeyHandle>,
        text: String,
    ) -> Result<bool, HostError> {
        operations::keys::has_prefix(self, &key, &text)
    }

    fn jwt_claims(
        &mut self,
        key: Resource<KeyHandle>,
    ) -> Result<Option<String>, HostError> {
        operations::keys::jwt_claims(self, &key)
    }

    fn given_in_otty(
        &mut self,
        key: Resource<KeyHandle>,
    ) -> Result<bool, HostError> {
        operations::keys::given_in_otty(self, &key)
    }

    fn drop(&mut self, key: Resource<KeyHandle>) -> wasmtime::Result<()> {
        self.table().delete(key)?;

        Ok(())
    }
}

impl keys::Host for CallState {
    fn open(
        &mut self,
        place: KeyPlace,
    ) -> Result<Resource<KeyHandle>, HostError> {
        operations::keys::open(self, place)
    }
}

impl files::Host for CallState {
    fn list(&mut self, glob: String) -> Result<Vec<FileInfo>, HostError> {
        list(self.permissions(), self.env(), &glob)
    }

    fn read(
        &mut self,
        path: String,
        max_bytes: u32,
    ) -> Result<Vec<u8>, HostError> {
        read_small_file(self.permissions(), self.env(), &path, max_bytes)
    }

    fn read_lines(
        &mut self,
        path: String,
        offset: u64,
        max_bytes: u32,
        markers: Vec<Vec<u8>>,
    ) -> Result<Batch, HostError> {
        let mut cursor = std::mem::take(self.zstd_cursor());
        let batch = read_lines(
            self.permissions(),
            self.env(),
            &mut cursor,
            &path,
            offset,
            max_bytes,
            &markers,
        );
        *self.zstd_cursor() = cursor;

        batch
    }

    fn read_json_with_keys(
        &mut self,
        path: String,
    ) -> Result<JsonWithKeys, HostError> {
        operations::keys::read_json_with_keys(self, &path)
    }

    fn remove(&mut self, path: String) -> Result<(), HostError> {
        remove::remove(self.permissions(), Path::new(&path))
    }
}

impl sqlite::Host for CallState {
    fn query(
        &mut self,
        database: String,
        sql: String,
        params: Vec<Param>,
    ) -> Result<Vec<Vec<Value>>, HostError> {
        let rows = query(
            self.permissions(),
            self.env(),
            &database,
            &sql,
            &params,
            QUERY_TIMEOUT,
        )?;

        with_handles(self, rows)
    }
}

impl http::Host for CallState {
    fn send(&mut self, request: Request) -> Result<Response, HostError> {
        operations::http::send(self, request)
    }
}

impl keyring::Host for CallState {
    fn read(
        &mut self,
        service: String,
        user: String,
    ) -> Result<Option<JsonWithKeys>, HostError> {
        operations::keyring::read(self, &service, &user)
    }
}

impl env::Host for CallState {
    fn var(&mut self, name: String) -> Option<String> {
        variable(self.permissions(), self.env(), &name)
    }

    fn home(&mut self) -> String {
        self.env().home().display().to_string()
    }

    fn language(&mut self) -> String {
        self.outside().language()
    }
}

impl clock::Host for CallState {
    fn now_ms(&mut self) -> i64 {
        let since_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();

        i64::try_from(since_epoch.as_millis()).unwrap_or(i64::MAX)
    }
}

impl log::Host for CallState {
    fn write(&mut self, level: Level, text: String) {
        let level = match level {
            Level::Debug => "debug",
            Level::Info => "info",
            Level::Warn => "warn",
            Level::Error => "error",
        };

        self.log().push(format!("[{level}] {text}\n").as_bytes());
    }
}

/// An I/O error of `path` in the words of a host error.
pub(super) fn io_error(error: &io::Error, path: &Path) -> HostError {
    match error.kind() {
        io::ErrorKind::NotFound => {
            HostError::NotFound(path.display().to_string())
        },
        _ => HostError::Failed(format!("{}: {error}", path.display())),
    }
}

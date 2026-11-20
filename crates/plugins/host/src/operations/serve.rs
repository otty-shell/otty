//! How the host serves the operations of `otty:host@0.1.0` to the part
//! being called. Each one checks the permissions of that part only.
//!
//! Until issues 04 to 06 bring them, most operations refuse every call
//! with "not in this OTTY yet". Only the simplest ones work already:
//! `env`, `clock`, `log` and `files.read`.

use std::fs::File;
use std::io::{self, Read};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use wasmtime::component::Resource;

use crate::calls::CallState;
use crate::keys::KeyHandle;
use crate::operations::v0_1::otty::host::files::{Batch, FileInfo};
use crate::operations::v0_1::otty::host::http::{Request, Response};
use crate::operations::v0_1::otty::host::keys::{JsonWithKeys, KeyPlace};
use crate::operations::v0_1::otty::host::log::Level;
use crate::operations::v0_1::otty::host::programs::{Output, Talk};
use crate::operations::v0_1::otty::host::sqlite::{Param, Value};
use crate::operations::v0_1::otty::host::terminal::HiddenTerminal;
use crate::operations::v0_1::otty::host::types::HostError;
use crate::operations::v0_1::otty::host::{
    clock, env, files, http, keyring, keys, log, programs, sqlite, terminal,
    types,
};

/// The most bytes `files.read` gives (spec "Host operations").
const READ_CAP: u32 = 4 * 1024 * 1024;

impl types::Host for CallState {}

impl keys::HostKey for CallState {
    fn fingerprint(
        &mut self,
        _key: Resource<KeyHandle>,
    ) -> Result<String, HostError> {
        Err(not_yet("key.fingerprint"))
    }

    fn has_prefix(
        &mut self,
        _key: Resource<KeyHandle>,
        _text: String,
    ) -> Result<bool, HostError> {
        Err(not_yet("key.has-prefix"))
    }

    fn jwt_claims(
        &mut self,
        _key: Resource<KeyHandle>,
    ) -> Result<Option<String>, HostError> {
        Err(not_yet("key.jwt-claims"))
    }

    fn drop(&mut self, key: Resource<KeyHandle>) -> wasmtime::Result<()> {
        self.table().delete(key)?;

        Ok(())
    }
}

impl keys::Host for CallState {
    fn open(
        &mut self,
        _place: KeyPlace,
    ) -> Result<Resource<KeyHandle>, HostError> {
        Err(not_yet("keys.open"))
    }
}

impl files::Host for CallState {
    fn list(&mut self, _glob: String) -> Result<Vec<FileInfo>, HostError> {
        Err(not_yet("files.list"))
    }

    fn read(
        &mut self,
        path: String,
        max_bytes: u32,
    ) -> Result<Vec<u8>, HostError> {
        let path = PathBuf::from(path);
        if !self.permissions().may_read(&path) {
            return Err(HostError::NotAllowed(path.display().to_string()));
        }

        let limit = max_bytes.min(READ_CAP);
        let mut bytes = Vec::new();
        File::open(&path)
            .and_then(|file| {
                file.take(u64::from(limit) + 1).read_to_end(&mut bytes)
            })
            .map_err(|error| io_error(&error, &path))?;

        if bytes.len() > limit as usize {
            return Err(HostError::TooBig(format!(
                "{} is more than {limit} bytes",
                path.display()
            )));
        }

        Ok(bytes)
    }

    fn read_lines(
        &mut self,
        _path: String,
        _offset: u64,
        _max_bytes: u32,
        _markers: Vec<Vec<u8>>,
    ) -> Result<Batch, HostError> {
        Err(not_yet("files.read-lines"))
    }

    fn read_json_with_keys(
        &mut self,
        _path: String,
    ) -> Result<JsonWithKeys, HostError> {
        Err(not_yet("files.read-json-with-keys"))
    }

    fn remove(&mut self, _path: String) -> Result<(), HostError> {
        Err(not_yet("files.remove"))
    }
}

impl sqlite::Host for CallState {
    fn query(
        &mut self,
        _database: String,
        _sql: String,
        _params: Vec<Param>,
    ) -> Result<Vec<Vec<Value>>, HostError> {
        Err(not_yet("sqlite.query"))
    }
}

impl http::Host for CallState {
    fn send(&mut self, _request: Request) -> Result<Response, HostError> {
        Err(not_yet("http.send"))
    }
}

impl programs::HostTalk for CallState {
    fn start(
        &mut self,
        _program: String,
        _args: Vec<String>,
        _env: Vec<(String, String)>,
    ) -> Result<Resource<Talk>, HostError> {
        Err(not_yet("programs.talk"))
    }

    fn send_line(
        &mut self,
        _talk: Resource<Talk>,
        _line: String,
    ) -> Result<(), HostError> {
        Err(not_yet("programs.talk"))
    }

    fn next_line(
        &mut self,
        _talk: Resource<Talk>,
        _timeout_ms: u32,
    ) -> Result<Option<String>, HostError> {
        Err(not_yet("programs.talk"))
    }

    fn drop(&mut self, talk: Resource<Talk>) -> wasmtime::Result<()> {
        self.table().delete(talk)?;

        Ok(())
    }
}

impl programs::Host for CallState {
    fn run(
        &mut self,
        _program: String,
        _args: Vec<String>,
        _env: Vec<(String, String)>,
        _timeout_ms: u32,
    ) -> Result<Output, HostError> {
        Err(not_yet("programs.run"))
    }
}

impl terminal::HostHiddenTerminal for CallState {
    fn start(
        &mut self,
        _program: String,
        _args: Vec<String>,
        _env: Vec<(String, String)>,
        _cols: u16,
        _rows: u16,
    ) -> Result<Resource<HiddenTerminal>, HostError> {
        Err(not_yet("terminal.hidden-terminal"))
    }

    fn send_keys(
        &mut self,
        _terminal: Resource<HiddenTerminal>,
        _keys: String,
    ) -> Result<(), HostError> {
        Err(not_yet("terminal.hidden-terminal"))
    }

    fn screen(
        &mut self,
        _terminal: Resource<HiddenTerminal>,
    ) -> Result<String, HostError> {
        Err(not_yet("terminal.hidden-terminal"))
    }

    fn wait_for_change(
        &mut self,
        _terminal: Resource<HiddenTerminal>,
        _timeout_ms: u32,
    ) -> Result<bool, HostError> {
        Err(not_yet("terminal.hidden-terminal"))
    }

    fn drop(
        &mut self,
        terminal: Resource<HiddenTerminal>,
    ) -> wasmtime::Result<()> {
        self.table().delete(terminal)?;

        Ok(())
    }
}

impl terminal::Host for CallState {}

impl keyring::Host for CallState {
    fn read(
        &mut self,
        _service: String,
        _user: String,
    ) -> Result<Option<JsonWithKeys>, HostError> {
        Err(not_yet("keyring.read"))
    }
}

impl env::Host for CallState {
    fn var(&mut self, name: String) -> Option<String> {
        if !self.permissions().may_read_var(&name) {
            return None;
        }

        self.env().var(&name).map(str::to_string)
    }

    fn home(&mut self) -> String {
        self.env().home().display().to_string()
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

/// The refusal of an operation that arrives with a later issue.
fn not_yet(operation: &str) -> HostError {
    HostError::Failed(format!("{operation} is not in this OTTY yet"))
}

/// An I/O error of `path` in the words of a host error.
fn io_error(error: &io::Error, path: &std::path::Path) -> HostError {
    match error.kind() {
        io::ErrorKind::NotFound => {
            HostError::NotFound(path.display().to_string())
        },
        _ => HostError::Failed(format!("{}: {error}", path.display())),
    }
}

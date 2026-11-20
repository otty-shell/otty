//! A host for plain `cargo test` of a plugin, natively, with no
//! WebAssembly: the host operations are served from the plugin's
//! `fixtures/` folder, which stands for the home folder, and from canned
//! HTTP answers. Every call is checked against the permissions the
//! plugin's `otty-plugin.toml` gives the part being tested, as OTTY checks
//! them.
//!
//! ```no_run
//! use otty_plugin_sdk::test_host::TestHost;
//!
//! let host = TestHost::new(env!("CARGO_MANIFEST_DIR"));
//! let home = host.as_agent("example-agent", otty_plugin_sdk::env::home);
//!
//! assert!(home.ends_with("fixtures"));
//! ```
//!
//! Key columns of a SQLite database are refused in a query, as OTTY
//! refuses them: a plugin selects the `rowid` and opens the key with
//! [`keys::open`](crate::keys::open). Keyring entries are made up with
//! [`TestHost::with_keyring_entry`] and read only after
//! [`TestHost::with_keyring_allowed`], as OTTY reads them only after the
//! user's Allow.
//!
//! What it does not do as OTTY does: it decodes no zstd file and runs no
//! program and no terminal (they are refused after the permission check).

mod files;
mod http;
mod keyring;
mod keys;
mod sqlite;

use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

use otty_plugin_manifest::{Environment, Manifest, PartRef, Permissions};

use crate::files::{Batch, FileInfo};
use crate::http::{Request, Response};
use crate::log::Level;
use crate::sqlite::{Param, Value};
use crate::{HostError, JsonWithKeys, Key, KeyPlace};

thread_local! {
    /// The host and the part of the test running on this thread.
    static CURRENT: RefCell<Option<Current>> = const { RefCell::new(None) };
}

/// The host of a plugin's tests. Calls of the plugin's functions run
/// inside [`as_agent`](Self::as_agent) or
/// [`as_provider`](Self::as_provider), which name the part being tested.
#[derive(Debug)]
pub struct TestHost {
    state: Rc<RefCell<HostState>>,
}

impl TestHost {
    /// The folder that stands for the home folder: the plugin's
    /// `fixtures/`.
    pub fn home(&self) -> PathBuf {
        self.state.borrow().home.clone()
    }

    /// The requests the plugin sent, in order, with each key filled in.
    pub fn requests(&self) -> Vec<SentRequest> {
        self.state.borrow().requests.clone()
    }

    /// The files the plugin removed. The test host never removes a
    /// fixture; it only notes the path.
    pub fn removed(&self) -> Vec<PathBuf> {
        self.state.borrow().removed.clone()
    }

    /// The lines the plugin wrote to its log, such as `[warn] text`.
    pub fn log(&self) -> Vec<String> {
        self.state.borrow().log.clone()
    }

    /// The host of the plugin in `plugin_folder`, the folder of its
    /// `otty-plugin.toml`, usually `env!("CARGO_MANIFEST_DIR")`.
    ///
    /// # Panics
    ///
    /// When the manifest is missing or refused, as OTTY would refuse it.
    pub fn new(plugin_folder: impl AsRef<Path>) -> Self {
        let folder = plugin_folder.as_ref();
        let path = folder.join("otty-plugin.toml");
        let text = fs::read_to_string(&path).unwrap_or_else(|error| {
            panic!("the test host reads {}: {error}", path.display())
        });
        let manifest = Manifest::parse(&text).unwrap_or_else(|error| {
            panic!("OTTY refuses {}: {error}", path.display())
        });

        let fixtures = folder.join("fixtures");
        let home = fixtures.canonicalize().unwrap_or(fixtures);

        let state = HostState {
            manifest,
            home,
            vars: Vec::new(),
            now_ms: None,
            keyring: Vec::new(),
            keyring_allowed: false,
            keys: Vec::new(),
            answers: Vec::new(),
            requests: Vec::new(),
            removed: Vec::new(),
            log: Vec::new(),
        };

        Self {
            state: Rc::new(RefCell::new(state)),
        }
    }

    /// The same host with the variable `name` set to `value`, as the
    /// environment OTTY reads. A part reads it only when it lists it.
    pub fn with_var(self, name: &str, value: &str) -> Self {
        self.state
            .borrow_mut()
            .vars
            .push((name.to_string(), value.to_string()));

        self
    }

    /// The same host with the made-up system keyring entry of `service`
    /// and `user` holding `text`. A part reads it only when it declares it
    /// and once the keyring is allowed.
    pub fn with_keyring_entry(
        self,
        service: &str,
        user: &str,
        text: &str,
    ) -> Self {
        self.state.borrow_mut().keyring.push((
            service.to_string(),
            user.to_string(),
            text.to_string(),
        ));

        self
    }

    /// The same host with the keyring allowed, as after the user's Allow
    /// in Agents mode. Until then every keyring read is refused.
    pub fn with_keyring_allowed(self) -> Self {
        self.state.borrow_mut().keyring_allowed = true;

        self
    }

    /// The same host with its clock stopped at `now_ms`.
    pub fn with_now_ms(self, now_ms: i64) -> Self {
        self.state.borrow_mut().now_ms = Some(now_ms);

        self
    }

    /// Answers a request of `method` to exactly `url` with `status` and
    /// `body`, every time it is sent.
    pub fn answer(&self, method: &str, url: &str, status: u16, body: &[u8]) {
        self.state.borrow_mut().answers.push(CannedAnswer {
            method: method.to_string(),
            url: url.to_string(),
            status,
            body: body.to_vec(),
        });
    }

    /// A handle to the made-up key `value`, as OTTY gives a provider part.
    pub fn key(&self, value: &str) -> Key {
        self.state.borrow_mut().add_key(value.to_string())
    }

    /// Runs `run` as the agent part `id`: its host operations get only
    /// that part's permissions.
    ///
    /// # Panics
    ///
    /// When the manifest lists no agent part `id`.
    pub fn as_agent<R>(&self, id: &str, run: impl FnOnce() -> R) -> R {
        self.as_part(&PartRef::agent(id), run)
    }

    /// Runs `run` as the provider part `id`: its host operations get only
    /// that part's permissions.
    ///
    /// # Panics
    ///
    /// When the manifest lists no provider part `id`.
    pub fn as_provider<R>(&self, id: &str, run: impl FnOnce() -> R) -> R {
        self.as_part(&PartRef::provider(id), run)
    }

    fn as_part<R>(&self, part: &PartRef, run: impl FnOnce() -> R) -> R {
        let permissions = {
            let state = self.state.borrow();
            let declared = state
                .manifest
                .permissions_of(part)
                .unwrap_or_else(|| panic!("otty-plugin.toml lists no {part}"));

            Permissions::new(declared, &state.environment())
        };

        let current = Current {
            state: Rc::clone(&self.state),
            permissions,
        };
        let _restore = Restore(CURRENT.replace(Some(current)));

        run()
    }
}

/// One request the plugin sent, its key filled in.
#[derive(Debug, Clone, PartialEq)]
pub struct SentRequest {
    method: String,
    url: String,
    headers: Vec<(String, String)>,
    body: Option<Vec<u8>>,
}

impl SentRequest {
    /// The method, such as `GET`.
    pub fn method(&self) -> &str {
        &self.method
    }

    /// The URL.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// The headers as sent, keys filled in.
    pub fn headers(&self) -> &[(String, String)] {
        &self.headers
    }

    /// The value of the first header `name`, compared without case.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(header, _)| header.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    /// The body, if any.
    pub fn body(&self) -> Option<&[u8]> {
        self.body.as_deref()
    }
}

/// What the host of one test holds.
#[derive(Debug)]
struct HostState {
    manifest: Manifest,
    home: PathBuf,
    vars: Vec<(String, String)>,
    now_ms: Option<i64>,
    keyring: Vec<(String, String, String)>,
    keyring_allowed: bool,
    keys: Vec<String>,
    answers: Vec<CannedAnswer>,
    requests: Vec<SentRequest>,
    removed: Vec<PathBuf>,
    log: Vec<String>,
}

impl HostState {
    /// The made-up key of the handle `id`.
    fn key_text(&self, id: usize) -> Result<&str, HostError> {
        self.keys
            .get(id)
            .map(String::as_str)
            .ok_or_else(|| HostError::NotFound(format!("key handle {id}")))
    }

    /// The home folder and the variables, as OTTY gives them.
    fn environment(&self) -> Environment {
        self.vars
            .iter()
            .fold(Environment::new(self.home.clone()), |env, (name, value)| {
                env.with_var(name, value)
            })
    }

    /// The variable `name`, if it is set.
    fn var(&self, name: &str) -> Option<&str> {
        self.vars
            .iter()
            .find(|(set, _)| set == name)
            .map(|(_, value)| value.as_str())
    }

    /// The text of the keyring entry of `service` and `user`; refused
    /// until the keyring is allowed.
    fn keyring_entry(
        &self,
        service: &str,
        user: &str,
    ) -> Result<Option<String>, HostError> {
        if !self.keyring_allowed {
            return Err(HostError::NotAllowed(
                "the system keyring: the user has not allowed OTTY to read it"
                    .to_string(),
            ));
        }

        let text = self
            .keyring
            .iter()
            .find(|(entry_service, entry_user, _)| {
                entry_service == service && entry_user == user
            })
            .map(|(_, _, text)| text.clone());

        Ok(text)
    }

    /// A handle to `value`.
    fn add_key(&mut self, value: String) -> Key {
        self.keys.push(value);

        Key::from_test_host(self.keys.len() - 1)
    }
}

/// An answer to every request of one method to one URL.
#[derive(Debug, Clone)]
struct CannedAnswer {
    method: String,
    url: String,
    status: u16,
    body: Vec<u8>,
}

/// The host of the running test and the permissions of its part.
struct Current {
    state: Rc<RefCell<HostState>>,
    permissions: Permissions,
}

/// Puts back the part that ran before, when a part's run ends or panics.
struct Restore(Option<Current>);

impl Drop for Restore {
    fn drop(&mut self) {
        CURRENT.set(self.0.take());
    }
}

pub(crate) fn key_fingerprint(id: usize) -> Result<String, HostError> {
    with_host(|state, _| Ok(keys::fingerprint(state.key_text(id)?)))
}

pub(crate) fn key_has_prefix(id: usize, text: &str) -> Result<bool, HostError> {
    with_host(|state, _| Ok(state.key_text(id)?.starts_with(text)))
}

pub(crate) fn key_jwt_claims(id: usize) -> Result<Option<String>, HostError> {
    with_host(|state, _| Ok(keys::jwt_claims(state.key_text(id)?)))
}

pub(crate) fn open_key(place: &KeyPlace) -> Result<Key, HostError> {
    with_host(|state, permissions| keys::open(state, permissions, place))
}

pub(crate) fn list_files(glob: &str) -> Result<Vec<FileInfo>, HostError> {
    with_host(|state, permissions| files::list(&state.home, permissions, glob))
}

pub(crate) fn read_file(
    path: &str,
    max_bytes: u32,
) -> Result<Vec<u8>, HostError> {
    with_host(|_, permissions| files::read(permissions, path, max_bytes))
}

pub(crate) fn read_lines(
    path: &str,
    offset: u64,
    max_bytes: u32,
    markers: &[Vec<u8>],
) -> Result<Batch, HostError> {
    with_host(|_, permissions| {
        files::read_lines(permissions, path, offset, max_bytes, markers)
    })
}

pub(crate) fn read_json_with_keys(
    path: &str,
) -> Result<JsonWithKeys, HostError> {
    with_host(|state, permissions| {
        files::read_json_with_keys(state, permissions, path)
    })
}

pub(crate) fn remove_file(path: &str) -> Result<(), HostError> {
    with_host(|state, permissions| {
        let path = PathBuf::from(path);
        if !permissions.may_write(&path) {
            return Err(not_allowed(&path));
        }

        state.removed.push(path);

        Ok(())
    })
}

pub(crate) fn query(
    database: &str,
    sql: &str,
    params: &[Param],
) -> Result<Vec<Vec<Value>>, HostError> {
    with_host(|_, permissions| {
        sqlite::query(permissions, database, sql, params)
    })
}

pub(crate) fn send(request: &Request<'_>) -> Result<Response, HostError> {
    with_host(|state, permissions| http::send(state, permissions, request))
}

pub(crate) fn start_program(
    program: &str,
    args: &[String],
) -> Result<std::convert::Infallible, HostError> {
    with_host(|_, permissions| {
        if !permissions.may_run(program, args) {
            return Err(HostError::NotAllowed(program.to_string()));
        }

        Err(HostError::Failed(
            "the test host runs no program".to_string(),
        ))
    })
}

pub(crate) fn start_terminal(
    program: &str,
    args: &[String],
) -> Result<std::convert::Infallible, HostError> {
    with_host(|_, permissions| {
        if !permissions.may_start_terminal(program, args) {
            return Err(HostError::NotAllowed(program.to_string()));
        }

        Err(HostError::Failed(
            "the test host runs no terminal".to_string(),
        ))
    })
}

pub(crate) fn read_keyring(
    service: &str,
    user: &str,
) -> Result<Option<JsonWithKeys>, HostError> {
    with_host(|state, permissions| {
        keyring::read(state, permissions, service, user)
    })
}

pub(crate) fn var(name: &str) -> Option<String> {
    with_host(|state, permissions| {
        let value = permissions
            .may_read_var(name)
            .then(|| state.var(name))
            .flatten();

        Ok(value.map(str::to_string))
    })
    .ok()
    .flatten()
}

pub(crate) fn home() -> String {
    with_host(|state, _| Ok(state.home.display().to_string()))
        .unwrap_or_default()
}

pub(crate) fn now_ms() -> i64 {
    let stopped = with_host(|state, _| Ok(state.now_ms)).ok().flatten();

    stopped.unwrap_or_else(|| {
        let since_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();

        i64::try_from(since_epoch.as_millis()).unwrap_or(i64::MAX)
    })
}

pub(crate) fn write_log(level: Level, text: &str) {
    let level = match level {
        Level::Debug => "debug",
        Level::Info => "info",
        Level::Warn => "warn",
        Level::Error => "error",
    };

    let _ = with_host(|state, _| {
        state.log.push(format!("[{level}] {text}"));

        Ok(())
    });
}

/// Runs `operation` with the host and the permissions of the part being
/// tested; outside [`TestHost::as_agent`] and [`TestHost::as_provider`]
/// every operation fails.
fn with_host<T>(
    operation: impl FnOnce(&mut HostState, &Permissions) -> Result<T, HostError>,
) -> Result<T, HostError> {
    CURRENT.with_borrow(|current| match current {
        Some(current) => {
            operation(&mut current.state.borrow_mut(), &current.permissions)
        },
        None => Err(HostError::Failed(
            "no test host: call the plugin inside TestHost::as_agent or \
             TestHost::as_provider"
                .to_string(),
        )),
    })
}

/// The refusal of `path`, which the part does not declare.
fn not_allowed(path: &Path) -> HostError {
    HostError::NotAllowed(path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::Request;
    use crate::sqlite::{Param, Value};
    use crate::testing::MadeUpPlugin;
    use crate::{
        JsonField, KeyringEntry, SqliteField, env, files, keys, programs,
    };

    const AGENT: &str = "made-up-agent";
    const PROVIDER: &str = "made-up-cloud";

    fn text(path: &Path) -> String {
        path.display().to_string()
    }

    #[test]
    fn a_part_reads_its_files_and_another_part_of_the_plugin_does_not() {
        let plugin = MadeUpPlugin::new();
        let state = plugin.write(".made-up/state.json", b"{}");
        let host = TestHost::new(plugin.folder());

        let as_agent =
            host.as_agent(AGENT, || files::read(&text(&state), 1024));
        let as_provider =
            host.as_provider(PROVIDER, || files::read(&text(&state), 1024));

        assert_eq!(as_agent, Ok(b"{}".to_vec()));
        assert_eq!(as_provider, Err(HostError::NotAllowed(text(&state))));
    }

    #[test]
    fn a_call_outside_a_part_fails() {
        let plugin = MadeUpPlugin::new();
        let state = plugin.write(".made-up/state.json", b"{}");
        let _host = TestHost::new(plugin.folder());

        let read = files::read(&text(&state), 1024);

        assert!(matches!(read, Err(HostError::Failed(_))));
    }

    #[test]
    fn a_listing_gives_the_files_the_part_may_read() {
        let plugin = MadeUpPlugin::new();
        let first = plugin.write(".made-up/sessions/a.jsonl", b"{}\n");
        plugin.write(".made-up/sessions/b.txt", b"");
        plugin.write(".other/sessions/c.jsonl", b"");
        let host = TestHost::new(plugin.folder());
        let glob = format!("{}/*/sessions/*.jsonl", text(&host.home()));

        let found = host.as_agent(AGENT, || files::list(&glob)).expect("files");

        let paths: Vec<&str> =
            found.iter().map(|file| file.path.as_str()).collect();
        assert_eq!(paths, [text(&first)]);
        assert_eq!(found[0].size, 3);
    }

    #[test]
    fn lines_come_in_batches_of_complete_lines_with_a_marker() {
        let plugin = MadeUpPlugin::new();
        let path = plugin.write(
            ".made-up/s.jsonl",
            b"{\"type\":\"answer\",\"n\":1}\n{\"type\":\"user\"}\n{\"type\":\"answer\",\"n\":2}\n{\"type\":\"answer\",\"n\":3}",
        );
        let host = TestHost::new(plugin.folder());
        let markers = vec![b"\"answer\"".to_vec()];

        let (first, second) = host.as_agent(AGENT, || {
            let first = files::read_lines(&text(&path), 0, 40, &markers)
                .expect("a batch");
            let second = files::read_lines(
                &text(&path),
                first.next_offset,
                1024,
                &markers,
            )
            .expect("a batch");

            (first, second)
        });

        let first_lines: Vec<&[u8]> = first.lines().collect();
        assert_eq!(first_lines, [&b"{\"type\":\"answer\",\"n\":1}"[..]]);
        assert_eq!(first.skipped, 1);
        assert!(!first.at_end);
        let second_lines: Vec<&[u8]> = second.lines().collect();
        assert_eq!(
            second_lines,
            [
                &b"{\"type\":\"answer\",\"n\":2}"[..],
                &b"{\"type\":\"answer\",\"n\":3}"[..],
            ]
        );
        assert!(second.at_end);
    }

    #[test]
    fn the_lines_iterator_reads_a_file_to_its_end() {
        let plugin = MadeUpPlugin::new();
        let path = plugin.write(".made-up/s.jsonl", b"a\nb\nc\n");
        let host = TestHost::new(plugin.folder());

        let lines: Vec<Vec<u8>> = host.as_agent(AGENT, || {
            files::lines(&text(&path), &[])
                .flat_map(|batch| {
                    batch
                        .expect("a batch")
                        .lines()
                        .map(<[u8]>::to_vec)
                        .collect::<Vec<_>>()
                })
                .collect()
        });

        assert_eq!(lines, [b"a".to_vec(), b"b".to_vec(), b"c".to_vec()]);
    }

    #[test]
    fn a_key_file_comes_with_handles_in_place_of_its_keys() {
        let plugin = MadeUpPlugin::new();
        let auth = plugin.write(
            ".made-up/auth.json",
            br#"{"work": {"key": "ex-made-up-key", "name": "Work"}, "token": "x.y.z"}"#,
        );
        let host = TestHost::new(plugin.folder());

        let (json, fingerprint, plain_read) = host.as_agent(AGENT, || {
            let document =
                files::json_with_keys(&text(&auth)).expect("the file");
            let fingerprint = document.key(1).expect("a key").fingerprint();

            (
                document.json().to_string(),
                fingerprint,
                files::read(&text(&auth), 1024),
            )
        });

        assert_eq!(
            json,
            r#"{"token":{"$key":0},"work":{"key":{"$key":1},"name":"Work"}}"#
        );
        assert_eq!(
            fingerprint.as_deref(),
            Ok(
                "27c8d30ec451e808415d2307396308459e728ccef0adc5a41908fccae8a2e1ae"
            )
        );
        assert_eq!(plain_read, Err(HostError::NotAllowed(text(&auth))));
    }

    #[test]
    fn a_declared_key_place_opens_and_another_does_not() {
        let plugin = MadeUpPlugin::new();
        let auth = plugin.write(
            ".made-up/auth.json",
            br#"{"token": "eyJhbGciOiJub25lIn0.eyJlbWFpbCI6ImRldkBleGFtcGxlLmNvbSIsImV4cCI6MTkwMDAwMDAwMH0.made-up-signature", "other": "x"}"#,
        );
        let host = TestHost::new(plugin.folder());
        let place = |pointer: &str| {
            KeyPlace::JsonField(JsonField {
                file: text(&auth),
                pointer: pointer.to_string(),
            })
        };

        let (claims, prefix, other) = host.as_agent(AGENT, || {
            let token = keys::open(&place("/token")).expect("a handle");

            (
                token.jwt_claims(),
                token.has_prefix("eyJ"),
                keys::open(&place("/other")).map(|_| ()),
            )
        });
        let as_provider = host
            .as_provider(PROVIDER, || keys::open(&place("/token")).map(|_| ()));

        assert_eq!(
            claims,
            Ok(Some(
                r#"{"email":"dev@example.com","exp":1900000000}"#.to_string()
            ))
        );
        assert_eq!(prefix, Ok(true));
        assert!(matches!(other, Err(HostError::NotAllowed(_))));
        assert!(matches!(as_provider, Err(HostError::NotAllowed(_))));
    }

    #[test]
    fn a_key_variable_opens_only_as_a_handle() {
        let plugin = MadeUpPlugin::new();
        let host = TestHost::new(plugin.folder())
            .with_var("MADE_UP_KEY", "ex-made-up-key")
            .with_var("MADE_UP_HOME", "/data/made-up")
            .with_var("OTHER", "x");

        let (key_as_text, home, other, fingerprint) =
            host.as_agent(AGENT, || {
                let key =
                    keys::open(&KeyPlace::EnvVar("MADE_UP_KEY".to_string()))
                        .expect("a handle");

                (
                    env::var("MADE_UP_KEY"),
                    env::var("MADE_UP_HOME"),
                    env::var("OTHER"),
                    key.fingerprint(),
                )
            });

        assert_eq!(key_as_text, None);
        assert_eq!(home.as_deref(), Some("/data/made-up"));
        assert_eq!(other, None);
        assert_eq!(fingerprint.map(|text| text.len()), Ok(64));
    }

    #[test]
    fn a_request_goes_only_to_a_declared_host_with_its_key_filled_in() {
        let plugin = MadeUpPlugin::new();
        let host = TestHost::new(plugin.folder());
        host.answer("GET", "https://api.example.com/v1/me", 200, b"{\"id\":1}");
        let key = host.key("ex-made-up-key");

        let (answer, other_host, plain) = host.as_provider(PROVIDER, || {
            let request = Request::get("https://api.example.com/v1/me")
                .header_with_key("authorization", "Bearer ", &key);

            (
                crate::http::request(&request),
                crate::http::request(&Request::get(
                    "https://evil.example.com/",
                )),
                crate::http::request(&Request::get("http://api.example.com/")),
            )
        });
        let as_agent = host.as_agent(AGENT, || {
            crate::http::request(&Request::get("https://api.example.com/v1/me"))
        });

        assert_eq!(
            answer.map(|response| response.body),
            Ok(b"{\"id\":1}".to_vec())
        );
        assert_eq!(
            other_host,
            Err(HostError::NotAllowed("evil.example.com".to_string()))
        );
        assert!(matches!(plain, Err(HostError::NotAllowed(_))));
        assert_eq!(
            as_agent,
            Err(HostError::NotAllowed("api.example.com".to_string()))
        );
        let sent = host.requests();
        assert_eq!(sent.len(), 1);
        assert_eq!(
            sent[0].header("Authorization"),
            Some("Bearer ex-made-up-key")
        );
    }

    #[test]
    fn a_removal_inside_the_write_globs_is_noted_and_nothing_is_removed() {
        let plugin = MadeUpPlugin::new();
        let probe = plugin.write(".made-up/probe/p.jsonl", b"{}\n");
        let state = plugin.write(".made-up/state.json", b"{}");
        let host = TestHost::new(plugin.folder());

        let (removed, refused) = host.as_agent(AGENT, || {
            (files::remove(&text(&probe)), files::remove(&text(&state)))
        });

        assert_eq!(removed, Ok(()));
        assert_eq!(refused, Err(HostError::NotAllowed(text(&state))));
        assert_eq!(host.removed(), std::slice::from_ref(&probe));
        assert!(probe.exists());
    }

    #[test]
    fn a_database_is_queried_read_only() {
        let plugin = MadeUpPlugin::new();
        let database = plugin.write(".made-up/data.db", b"");
        let connection =
            rusqlite::Connection::open(&database).expect("a database");
        connection
            .execute_batch(
                "CREATE TABLE message (id INTEGER, model TEXT);
                 INSERT INTO message VALUES (1, 'large'), (2, 'small');
                 CREATE TABLE credential (value TEXT);
                 INSERT INTO credential VALUES ('{\"key\": \"ex-made-up-key\"}');",
            )
            .expect("the fixture");
        drop(connection);
        let host = TestHost::new(plugin.folder());

        let (rows, write, key) = host.as_agent(AGENT, || {
            (
                crate::sqlite::query(
                    &text(&database),
                    "SELECT id, model FROM message WHERE id > ?",
                    &[Param::Integer(1)],
                ),
                crate::sqlite::query(
                    &text(&database),
                    "DELETE FROM message",
                    &[],
                ),
                keys::open(&KeyPlace::SqliteField(SqliteField {
                    database: text(&database),
                    table: "credential".to_string(),
                    column: "value".to_string(),
                    row_id: 1,
                    pointer: Some("/key".to_string()),
                }))
                .and_then(|key| key.fingerprint()),
            )
        });

        let rows = rows.expect("the rows");
        assert_eq!(rows.len(), 1);
        assert!(
            matches!(&rows[0][..], [Value::Integer(2), Value::Text(model)] if model == "small")
        );
        assert!(matches!(write, Err(HostError::NotAllowed(_))));
        assert_eq!(
            key.as_deref(),
            Ok(
                "27c8d30ec451e808415d2307396308459e728ccef0adc5a41908fccae8a2e1ae"
            )
        );
    }

    #[test]
    fn a_key_column_is_refused_in_a_query_and_opened_by_its_row() {
        let plugin = MadeUpPlugin::new();
        let database = plugin.write(".made-up/keys.db", b"");
        let connection =
            rusqlite::Connection::open(&database).expect("a database");
        connection
            .execute_batch(
                "CREATE TABLE credential (provider TEXT, value TEXT);
                 INSERT INTO credential VALUES
                     ('made-up-cloud', '{\"key\": \"ex-made-up-key\"}');",
            )
            .expect("the fixture");
        drop(connection);
        let host = TestHost::new(plugin.folder());
        let path = text(&database);

        let (as_text, every_column, rows) = host.as_agent(AGENT, || {
            (
                crate::sqlite::query(
                    &path,
                    "SELECT value FROM credential",
                    &[],
                ),
                crate::sqlite::query(&path, "SELECT * FROM credential", &[]),
                crate::sqlite::query(
                    &path,
                    "SELECT rowid, provider FROM credential",
                    &[],
                ),
            )
        });

        assert!(matches!(as_text, Err(HostError::NotAllowed(_))));
        assert!(matches!(every_column, Err(HostError::NotAllowed(_))));
        let rows = rows.expect("the rows");
        assert!(matches!(
            &rows[0][..],
            [Value::Integer(1), Value::Text(provider)] if provider == "made-up-cloud"
        ));
    }

    #[test]
    fn a_keyring_entry_is_read_only_after_the_allow_with_keys_as_handles() {
        let plugin = MadeUpPlugin::new();
        let entry = r#"{"key": "ex-made-up-key", "name": "Work"}"#;
        let refused = TestHost::new(plugin.folder())
            .with_keyring_entry("made-up", "work", entry)
            .as_agent(AGENT, || {
                crate::keyring::read("made-up", "work").map(|_| ())
            });
        let host = TestHost::new(plugin.folder())
            .with_keyring_entry("made-up", "work", entry)
            .with_keyring_entry("made-up", "plain", "ex-made-up-key")
            .with_keyring_allowed();

        let (work, plain, missing, undeclared, opened) =
            host.as_agent(AGENT, || {
                let work = crate::keyring::read("made-up", "work")
                    .expect("an entry")
                    .expect("there");
                let plain = crate::keyring::read("made-up", "plain")
                    .expect("an entry")
                    .expect("there");
                let opened = keys::open(&KeyPlace::Keyring(KeyringEntry {
                    service: "made-up".to_string(),
                    user: "work".to_string(),
                    pointer: Some("/key".to_string()),
                }))
                .and_then(|key| key.fingerprint());

                (
                    (
                        work.json().to_string(),
                        work.key(0).expect("a key").fingerprint(),
                    ),
                    (
                        plain.json().to_string(),
                        plain.key(0).expect("a key").fingerprint(),
                    ),
                    crate::keyring::read("made-up", "home").map(|entry| {
                        entry.map(|entry| entry.json().to_string())
                    }),
                    crate::keyring::read("made-up", "other").map(|_| ()),
                    opened,
                )
            });

        let fingerprint =
            "27c8d30ec451e808415d2307396308459e728ccef0adc5a41908fccae8a2e1ae";
        assert!(matches!(refused, Err(HostError::NotAllowed(_))));
        assert_eq!(work.0, r#"{"key":{"$key":0},"name":"Work"}"#);
        assert_eq!(work.1.as_deref(), Ok(fingerprint));
        assert_eq!(plain.0, r#"{"$key":0}"#);
        assert_eq!(plain.1.as_deref(), Ok(fingerprint));
        assert_eq!(missing, Ok(None));
        assert!(matches!(undeclared, Err(HostError::NotAllowed(_))));
        assert_eq!(opened.as_deref(), Ok(fingerprint));
    }

    #[test]
    fn programs_are_checked_and_never_run() {
        let plugin = MadeUpPlugin::new();
        let host = TestHost::new(plugin.folder());
        let version = vec!["--version".to_string()];
        let other = vec!["--yolo".to_string()];

        let (declared, undeclared) = host.as_agent(AGENT, || {
            (
                programs::run("made-up", &version, &[], 1000),
                programs::run("made-up", &other, &[], 1000),
            )
        });

        assert!(matches!(declared, Err(HostError::Failed(_))));
        assert_eq!(
            undeclared,
            Err(HostError::NotAllowed("made-up".to_string()))
        );
    }

    #[test]
    fn the_clock_can_be_stopped_and_the_log_is_kept() {
        let plugin = MadeUpPlugin::new();
        let host =
            TestHost::new(plugin.folder()).with_now_ms(1_763_600_000_000);

        let now = host.as_agent(AGENT, || {
            crate::log::warn("the format changed");
            crate::clock::now_ms()
        });

        assert_eq!(now, 1_763_600_000_000);
        assert_eq!(host.log(), ["[warn] the format changed"]);
    }
}

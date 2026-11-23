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
//! Zstd files are decoded for `files::read_lines`, as OTTY decodes them.
//! A SQLite query follows OTTY's rules: `ATTACH` is refused; a key column
//! comes back as a key handle, or as JSON with handles in place of its key
//! fields, when the result also holds the rowid of its table; the SQL
//! reads a key column as `NULL`, and naming it anywhere but as a result
//! column is refused. Keyring entries are made up
//! with [`TestHost::with_keyring_entry`] and read only after
//! [`TestHost::with_keyring_allowed`], as OTTY reads them only after the
//! user's Allow.
//!
//! What it does not do as OTTY does: it runs no program and no terminal:
//! a declared program answers with the output, lines or screens the test
//! gave for it ([`TestHost::answer_run`], [`TestHost::answer_talk`],
//! [`TestHost::answer_terminal`]). Neither a program nor a query runs
//! out of time.

mod files;
mod http;
mod keyring;
mod keys;
mod programs;
mod sqlite;
mod sqlite_keys;

use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

use otty_plugin_manifest::{Environment, Manifest, PartRef, Permissions};

pub use self::programs::StartedProgram;
use crate::agents::{LoginFolder, Place, PlaceItem};
use crate::files::{Batch, FileInfo};
use crate::http::{Request, Response};
use crate::log::Level;
use crate::programs::Output;
use crate::sqlite::{Param, Value};
use crate::{HostError, JsonWithKeys, Key, KeyPlace};

/// The working folder of hidden terminals in the home folder, as OTTY
/// starts them: `agent-probe` in OTTY's cache folder.
const TERMINAL_FOLDER: &str = ".cache/otty/agent-probe";

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

    /// The programs the plugin ran, talked to or started in a hidden
    /// terminal, in order, with their arguments and variables.
    pub fn started(&self) -> Vec<StartedProgram> {
        self.state.borrow().programs.started().to_vec()
    }

    /// The lines the plugin sent to the programs it talks to and the keys
    /// it typed in its hidden terminals, in order.
    pub fn sent(&self) -> Vec<String> {
        self.state.borrow().programs.sent().to_vec()
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
        let terminal_folder = home.join(TERMINAL_FOLDER);

        let state = HostState {
            manifest,
            home,
            vars: Vec::new(),
            language: String::from("en"),
            now_ms: None,
            keyring: Vec::new(),
            keyring_allowed: false,
            terminal_folder,
            keys: Vec::new(),
            answers: Vec::new(),
            requests: Vec::new(),
            removed: Vec::new(),
            log: Vec::new(),
            programs: programs::Programs::default(),
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

    /// The same host with hidden terminals started in `folder` instead of
    /// `~/.cache/otty/agent-probe`, the folder OTTY uses.
    pub fn with_terminal_folder(self, folder: impl Into<PathBuf>) -> Self {
        self.state.borrow_mut().terminal_folder = folder.into();

        self
    }

    /// The same host with OTTY's interface in the language `tag`, such as
    /// `ru`; English until then.
    pub fn with_language(self, tag: &str) -> Self {
        self.state.borrow_mut().language = tag.to_string();

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

    /// Answers every run of `program` with exactly `args` with `output`.
    pub fn answer_run(&self, program: &str, args: &[&str], output: Output) {
        self.state
            .borrow_mut()
            .programs
            .answer_run(program, args, output);
    }

    /// Every talk with `program` and exactly `args` writes `lines`, one
    /// for each `next_line`, whatever it is sent, and then ends.
    pub fn answer_talk(&self, program: &str, args: &[&str], lines: &[&str]) {
        self.state
            .borrow_mut()
            .programs
            .answer_talk(program, args, lines);
    }

    /// Every hidden terminal of `program` with exactly `args` shows
    /// `screens`, one for each `wait_for_change`, and then stays still.
    /// Its screen is empty before the first wait.
    pub fn answer_terminal(
        &self,
        program: &str,
        args: &[&str],
        screens: &[&str],
    ) {
        self.state
            .borrow_mut()
            .programs
            .answer_terminal(program, args, screens);
    }

    /// OTTY's interface switches to the language `tag` now, as when the
    /// user picks another: the next [`env::language`](crate::env::language)
    /// of any part gives it.
    pub fn set_language(&self, tag: &str) {
        self.state.borrow_mut().language = tag.to_string();
    }

    /// A handle to the made-up key `value`, as OTTY gives a provider part
    /// a key it read from a place of an agent.
    pub fn key(&self, value: &str) -> Key {
        self.state.borrow_mut().add_key(value.to_string())
    }

    /// A handle to the made-up key `value` the user gave in OTTY for an
    /// `AskKey` slot.
    pub fn given_key(&self, value: &str) -> Key {
        self.state.borrow_mut().add_given_key(value.to_string())
    }

    /// A handle to the key at `place`, as OTTY gives a provider part the
    /// login an agent part found: the host reads the key again at each use
    /// of the handle, and a place with no key there now, such as a
    /// credentials file that is `{}`, is a handle whose every use fails
    /// with [`HostError::NotFound`] (issue 29). The provider part needs no
    /// permission for the place: OTTY reads it for the part.
    ///
    /// ```no_run
    /// use otty_plugin_sdk::test_host::TestHost;
    /// use otty_plugin_sdk::{HostError, KeyPlace};
    ///
    /// let host = TestHost::new(env!("CARGO_MANIFEST_DIR"));
    /// let key = host.key_at(&KeyPlace::EnvVar("NO_SUCH_KEY".into()));
    ///
    /// let prefix = host.as_provider("example", || key.has_prefix("sk-"));
    ///
    /// assert!(matches!(prefix, Err(HostError::NotFound(_))));
    /// ```
    pub fn key_at(&self, place: &KeyPlace) -> Key {
        self.state.borrow_mut().add_key_at(place.clone())
    }

    /// Runs `run` as the agent part `id`: its host operations get only
    /// that part's permissions.
    ///
    /// # Panics
    ///
    /// When the manifest lists no agent part `id`.
    pub fn as_agent<R>(&self, id: &str, run: impl FnOnce() -> R) -> R {
        self.as_part(&PartRef::agent(id), &[], run)
    }

    /// Runs `run` as the provider part `id`: its host operations get only
    /// that part's permissions.
    ///
    /// # Panics
    ///
    /// When the manifest lists no provider part `id`.
    pub fn as_provider<R>(&self, id: &str, run: impl FnOnce() -> R) -> R {
        self.as_part(&PartRef::provider(id), &[], run)
    }

    /// Runs `run` as the provider part `id` in a call given the login
    /// folders `folders`, as OTTY calls `accounts`, or `numbers` and
    /// `read-bill` with the folders an account was found in: inside each
    /// folder the part also reads what the `logins` of its manifest
    /// declare for the folder's agent part.
    ///
    /// # Panics
    ///
    /// When the manifest lists no provider part `id`.
    pub fn as_provider_in<R>(
        &self,
        id: &str,
        folders: &[LoginFolder],
        run: impl FnOnce() -> R,
    ) -> R {
        let folders: Vec<otty_plugin_manifest::LoginFolder> = folders
            .iter()
            .map(|LoginFolder::Profile((agent, folder))| {
                otty_plugin_manifest::LoginFolder::new(agent, folder)
            })
            .collect();

        self.as_part(&PartRef::provider(id), &folders, run)
    }

    /// The files of `places` the agent part `id` may read, each as OTTY
    /// asks the part to read it: with its size, its change time and the
    /// kind of its place, in the order of their paths and each once. A
    /// place the part may not list gives no files, as in OTTY.
    ///
    /// # Panics
    ///
    /// When the manifest lists no agent part `id`.
    pub fn place_items(&self, id: &str, places: &[Place]) -> Vec<PlaceItem> {
        let mut items: Vec<PlaceItem> = self.as_agent(id, || {
            places
                .iter()
                .flat_map(|place| {
                    let found = crate::files::list(&place.glob);
                    let kind = place.kind;

                    found
                        .unwrap_or_default()
                        .into_iter()
                        .map(move |file| PlaceItem { file, kind })
                })
                .collect()
        });

        items.sort_by(|one, other| one.file.path.cmp(&other.file.path));
        items.dedup_by(|one, other| one.file.path == other.file.path);
        items
    }

    fn as_part<R>(
        &self,
        part: &PartRef,
        folders: &[otty_plugin_manifest::LoginFolder],
        run: impl FnOnce() -> R,
    ) -> R {
        let permissions = {
            let state = self.state.borrow();
            let declared = state
                .manifest
                .permissions_of(part)
                .unwrap_or_else(|| panic!("otty-plugin.toml lists no {part}"));

            Permissions::new(declared, &state.environment())
                .in_login_folders(folders)
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
    language: String,
    now_ms: Option<i64>,
    keyring: Vec<(String, String, String)>,
    keyring_allowed: bool,
    terminal_folder: PathBuf,
    keys: Vec<HeldKey>,
    answers: Vec<CannedAnswer>,
    requests: Vec<SentRequest>,
    removed: Vec<PathBuf>,
    log: Vec<String>,
    programs: programs::Programs,
}

impl HostState {
    /// The made-up key of the handle `id`, read again from its place for
    /// a handle to a place. A blank key is no key, as OTTY reads it.
    fn key_text(&self, id: usize) -> Result<String, HostError> {
        let not_found = || HostError::NotFound(format!("key handle {id}"));
        let key = self.keys.get(id).ok_or_else(not_found)?;

        let text = match &key.source {
            HeldSource::Text(text) => text.clone(),
            HeldSource::Place(place) => keys::read_at(self, place)?,
        };

        match text.trim().is_empty() {
            true => Err(not_found()),
            false => Ok(text),
        }
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

    /// Whether the user gave the key of the handle `id` in OTTY.
    fn is_given_key(&self, id: usize) -> Result<bool, HostError> {
        self.keys
            .get(id)
            .map(|key| key.is_given)
            .ok_or_else(|| HostError::NotFound(format!("key handle {id}")))
    }

    /// A handle to `value`, read from a place of an agent.
    fn add_key(&mut self, value: String) -> Key {
        self.hold_key(value, false)
    }

    /// A handle to `value`, which the user gave in OTTY.
    fn add_given_key(&mut self, value: String) -> Key {
        self.hold_key(value, true)
    }

    /// A handle to the key at `place`, read at each use.
    fn add_key_at(&mut self, place: KeyPlace) -> Key {
        self.keys.push(HeldKey {
            source: HeldSource::Place(place),
            is_given: false,
        });

        Key::from_test_host(self.keys.len() - 1)
    }

    /// A handle to `value`, given in OTTY or not.
    fn hold_key(&mut self, text: String, is_given: bool) -> Key {
        self.keys.push(HeldKey {
            source: HeldSource::Text(text),
            is_given,
        });

        Key::from_test_host(self.keys.len() - 1)
    }
}

/// A made-up key the host holds for a handle, and where it came from.
#[derive(Debug)]
struct HeldKey {
    source: HeldSource,
    is_given: bool,
}

/// Where the host reads the key of a handle.
#[derive(Debug)]
enum HeldSource {
    /// A made-up key, held as it is.
    Text(String),
    /// The key at a place of an agent, read again at each use.
    Place(KeyPlace),
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
    with_host(|state, _| Ok(keys::fingerprint(&state.key_text(id)?)))
}

pub(crate) fn key_has_prefix(id: usize, text: &str) -> Result<bool, HostError> {
    with_host(|state, _| Ok(state.key_text(id)?.starts_with(text)))
}

pub(crate) fn key_is_given_in_otty(id: usize) -> Result<bool, HostError> {
    with_host(|state, _| state.is_given_key(id))
}

pub(crate) fn key_jwt_claims(id: usize) -> Result<Option<String>, HostError> {
    with_host(|state, _| Ok(keys::jwt_claims(&state.key_text(id)?)))
}

pub(crate) fn open_key(place: &KeyPlace) -> Result<Key, HostError> {
    with_host(|state, permissions| keys::open(state, permissions, place))
}

pub(crate) fn list_files(glob: &str) -> Result<Vec<FileInfo>, HostError> {
    with_host(|_, permissions| files::list(permissions, glob))
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
        if fs::symlink_metadata(&path).is_err() {
            return Err(HostError::NotFound(path.display().to_string()));
        }
        let is_full_folder = fs::read_dir(&path)
            .is_ok_and(|mut entries| entries.next().is_some());
        if is_full_folder {
            return Err(HostError::Failed(format!(
                "{} is a folder that is not empty",
                path.display()
            )));
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
    with_host(|state, permissions| {
        sqlite::query(state, permissions, database, sql, params)
    })
}

pub(crate) fn send(request: &Request<'_>) -> Result<Response, HostError> {
    with_host(|state, permissions| http::send(state, permissions, request))
}

pub(crate) fn run_program(
    program: &str,
    args: &[String],
    env: &[(String, String)],
) -> Result<Output, HostError> {
    with_host(|state, permissions| {
        state.programs.run(permissions, program, args, env)
    })
}

pub(crate) fn start_talk(
    program: &str,
    args: &[String],
    env: &[(String, String)],
) -> Result<usize, HostError> {
    with_host(|state, permissions| {
        state.programs.start_talk(permissions, program, args, env)
    })
}

pub(crate) fn send_line(talk: usize, line: &str) -> Result<(), HostError> {
    with_host(|state, _| state.programs.send_line(talk, line))
}

pub(crate) fn next_line(talk: usize) -> Result<Option<String>, HostError> {
    with_host(|state, _| state.programs.next_line(talk))
}

pub(crate) fn start_terminal(
    program: &str,
    args: &[String],
    env: &[(String, String)],
) -> Result<usize, HostError> {
    with_host(|state, permissions| {
        state
            .programs
            .start_terminal(permissions, program, args, env)
    })
}

pub(crate) fn terminal_folders() -> Result<Vec<String>, HostError> {
    with_host(|state, permissions| {
        if !permissions.has_terminals() {
            return Err(HostError::NotAllowed(
                "the working folder of hidden terminals".to_string(),
            ));
        }

        Ok(vec![state.terminal_folder.display().to_string()])
    })
}

pub(crate) fn send_keys(terminal: usize, keys: &str) -> Result<(), HostError> {
    with_host(|state, _| state.programs.send_keys(terminal, keys))
}

pub(crate) fn screen(terminal: usize) -> Result<String, HostError> {
    with_host(|state, _| state.programs.screen(terminal))
}

pub(crate) fn wait_for_change(terminal: usize) -> Result<bool, HostError> {
    with_host(|state, _| state.programs.wait_for_change(terminal))
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

pub(crate) fn language() -> String {
    with_host(|state, _| Ok(state.language.clone()))
        .unwrap_or_else(|_| String::from("en"))
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

    /// Issue 25: the files of the places, once each, with their kind.
    #[test]
    fn the_items_of_places_are_their_files_once_each() {
        let plugin = MadeUpPlugin::new();
        let first = plugin.write(".made-up/sessions/a.jsonl", b"{}\n");
        let second = plugin.write(".made-up/sessions/b.jsonl", b"");
        let host = TestHost::new(plugin.folder());
        let sessions = text(&host.home().join(".made-up/sessions"));
        let places = [
            Place {
                glob: format!("{sessions}/*.jsonl"),
                kind: crate::agents::PlaceKind::Files,
            },
            Place {
                glob: format!("{sessions}/a.jsonl"),
                kind: crate::agents::PlaceKind::Files,
            },
        ];

        let items = host.place_items(AGENT, &places);

        let paths: Vec<&str> =
            items.iter().map(|item| item.file.path.as_str()).collect();
        assert_eq!(paths, [text(&first), text(&second)]);
        assert_eq!(items[0].file.size, 3);
    }

    /// A variable the part lists may name a folder outside the fixtures:
    /// its files are listed, as OTTY lists them.
    #[test]
    fn a_listing_reaches_the_folder_a_listed_variable_names() {
        let plugin = MadeUpPlugin::new();
        let outside = plugin.folder().join("elsewhere");
        std::fs::create_dir_all(&outside).expect("the folder");
        std::fs::write(outside.join("a.jsonl"), b"{}\n").expect("a file");
        let outside = outside.canonicalize().expect("the folder");
        let host = TestHost::new(plugin.folder())
            .with_var("MADE_UP_HOME", &text(&outside));
        let glob = format!("{}/*.jsonl", text(&outside));

        let found = host.as_agent(AGENT, || files::list(&glob)).expect("files");

        let paths: Vec<&str> =
            found.iter().map(|file| file.path.as_str()).collect();
        assert_eq!(paths, [text(&outside.join("a.jsonl"))]);
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
    fn a_zstd_file_gives_the_same_lines_as_its_plain_copy() {
        let plugin = MadeUpPlugin::new();
        let content = b"{\"answer\":1}\n{\"user\":2}\n{\"answer\":3}\n";
        let compressed = ruzstd::encoding::compress_to_vec(
            &content[..],
            ruzstd::encoding::CompressionLevel::Fastest,
        );
        let plain = plugin.write(".made-up/plain.jsonl", content);
        let packed = plugin.write(".made-up/packed.jsonl.zst", &compressed);
        let host = TestHost::new(plugin.folder());
        let markers = vec![b"\"answer\"".to_vec()];

        let (plain, packed) = host.as_agent(AGENT, || {
            let read = |path: &Path| {
                files::read_lines(&text(path), 0, 1024, &markers)
                    .expect("a batch")
            };

            (read(&plain), read(&packed))
        });

        assert_eq!(packed.lines, plain.lines);
        assert_eq!(packed.skipped, 1);
        assert_eq!(packed.next_offset, content.len() as u64);
    }

    #[test]
    fn a_glob_with_dot_dot_or_not_absolute_is_refused() {
        let plugin = MadeUpPlugin::new();
        let host = TestHost::new(plugin.folder());
        let up = format!("{}/.made-up/../*", text(&host.home()));

        let (up_listed, relative_listed) = host
            .as_agent(AGENT, || (files::list(&up), files::list(".made-up/*")));

        assert!(matches!(up_listed, Err(HostError::NotAllowed(_))));
        assert!(matches!(relative_listed, Err(HostError::NotAllowed(_))));
    }

    /// The profile `folder` of the agent part `agent`, as OTTY gives it to
    /// a call of a provider part.
    fn profile_folder(agent: &str, folder: &Path) -> Vec<LoginFolder> {
        vec![LoginFolder::Profile((agent.to_string(), text(folder)))]
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decision 5: in a
    /// call given a profile folder of an agent part its `logins` name, a
    /// provider part reads in that folder what they declare, the login
    /// with its key as a handle; outside such a call, and in a folder of
    /// another agent part, it reads nothing there.
    #[test]
    fn a_provider_part_reads_in_the_login_folders_of_its_call() {
        let plugin = MadeUpPlugin::new();
        let auth = plugin.write("work/auth.json", br#"{"token": "x.y.z"}"#);
        let session = plugin.write("work/sessions/a.jsonl", b"{}\n");
        let folder = auth.parent().expect("the profile folder");
        let host = TestHost::new(plugin.folder());
        let read = || {
            let login = files::json_with_keys(&text(&auth))
                .map(|document| document.json().to_string());

            (login, files::read(&text(&session), 1024))
        };

        let in_call =
            host.as_provider_in(PROVIDER, &profile_folder(AGENT, folder), read);
        let outside = host.as_provider(PROVIDER, read);
        let of_another_agent = host.as_provider_in(
            PROVIDER,
            &profile_folder("other-agent", folder),
            read,
        );

        assert_eq!(
            in_call,
            (
                Ok(r#"{"token":{"$key":0}}"#.to_string()),
                Ok(b"{}\n".to_vec())
            )
        );
        for refused in [outside, of_another_agent] {
            assert_eq!(refused.0, Err(HostError::NotAllowed(text(&auth))));
            assert_eq!(refused.1, Err(HostError::NotAllowed(text(&session))));
        }
    }

    /// A program the provider part starts gets the folder variable of its
    /// logins only set to a login folder of the call.
    #[test]
    fn the_folder_variable_reaches_a_program_only_with_a_login_folder() {
        let plugin = MadeUpPlugin::new();
        let auth = plugin.write("work/auth.json", br#"{"token": "x.y.z"}"#);
        let folder = auth.parent().expect("the profile folder");
        let host = TestHost::new(plugin.folder());
        let found = profile_folder(AGENT, folder);
        host.answer_run(
            "made-up",
            &["--usage"],
            programs::Output {
                status: Some(0),
                stdout: b"12% used".to_vec(),
                stderr: Vec::new(),
            },
        );
        let run_in = |folder: &str| {
            let env = [("MADE_UP_HOME".to_string(), folder.to_string())];

            programs::run("made-up", &["--usage".to_string()], &env, 1000)
                .map(|output| output.stdout)
        };

        let in_folder =
            host.as_provider_in(PROVIDER, &found, || run_in(&text(folder)));
        let elsewhere =
            host.as_provider_in(PROVIDER, &found, || run_in("/elsewhere"));
        let outside = host.as_provider(PROVIDER, || run_in(&text(folder)));

        let refused =
            HostError::NotAllowed("variable MADE_UP_HOME".to_string());
        assert_eq!(in_folder, Ok(b"12% used".to_vec()));
        assert_eq!(elsewhere, Err(refused.clone()));
        assert_eq!(outside, Err(refused));
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

    /// OTTY reads a key without the spaces around it and finds no key in
    /// a blank field, so a plugin tells a blank key from a key by asking.
    #[test]
    fn a_blank_key_is_not_there() {
        let plugin = MadeUpPlugin::new();
        let auth = plugin.write(
            ".made-up/auth.json",
            br#"{"work": {"key": "  "}, "token": "x.y.z"}"#,
        );
        let host = TestHost::new(plugin.folder());
        let blank = KeyPlace::JsonField(JsonField {
            file: text(&auth),
            pointer: "/work/key".to_string(),
        });

        let (fingerprint, prefix, opened) = host.as_agent(AGENT, || {
            let document =
                files::json_with_keys(&text(&auth)).expect("the file");
            let key = document.key(1).expect("a handle");

            (
                key.fingerprint(),
                key.has_prefix(""),
                keys::open(&blank).map(|_| ()),
            )
        });

        assert!(matches!(fingerprint, Err(HostError::NotFound(_))));
        assert!(matches!(prefix, Err(HostError::NotFound(_))));
        assert!(matches!(opened, Err(HostError::NotFound(_))));
    }

    /// Issue 29: OTTY gives a provider part a handle to the login an agent
    /// part found and reads the key at each use, so a login whose file
    /// holds no token now is a handle every use of which is not found,
    /// and one whose token comes back is read again.
    #[test]
    fn a_handle_to_a_place_reads_the_key_at_each_use() {
        let plugin = MadeUpPlugin::new();
        let auth = plugin.write(".made-up/auth.json", b"{}");
        let host = TestHost::new(plugin.folder());
        let key = host.key_at(&KeyPlace::JsonField(JsonField {
            file: text(&auth),
            pointer: "/token".to_string(),
        }));

        let missing = host.as_provider(PROVIDER, || key.has_prefix("sk-"));
        plugin.write(".made-up/auth.json", br#"{"token": "sk-made-up"}"#);
        let back = host.as_provider(PROVIDER, || key.has_prefix("sk-"));

        assert!(matches!(missing, Err(HostError::NotFound(_))));
        assert_eq!(back, Ok(true));
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
    fn a_missing_file_is_not_found_and_not_noted() {
        let plugin = MadeUpPlugin::new();
        let missing =
            plugin.folder().join("fixtures/.made-up/probe/gone.jsonl");
        let host = TestHost::new(plugin.folder());

        let removed = host.as_agent(AGENT, || files::remove(&text(&missing)));

        assert_eq!(removed, Err(HostError::NotFound(text(&missing))));
        assert!(host.removed().is_empty());
    }

    #[test]
    fn a_folder_that_is_not_empty_is_not_removed() {
        let plugin = MadeUpPlugin::new();
        // A folder whose name matches the part's `write` glob.
        let inside = plugin.write(".made-up/probe/old.jsonl/p.jsonl", b"{}\n");
        let folder = inside.parent().expect("a folder").to_path_buf();
        let host = TestHost::new(plugin.folder());

        let removed = host.as_agent(AGENT, || files::remove(&text(&folder)));

        assert!(matches!(removed, Err(HostError::Failed(_))));
        assert!(host.removed().is_empty());
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
    fn a_key_column_comes_back_as_handles_with_the_rowid_of_its_row() {
        let plugin = MadeUpPlugin::new();
        let database = plugin.write(".made-up/keys.db", b"");
        let connection =
            rusqlite::Connection::open(&database).expect("a database");
        connection
            .execute_batch(
                "CREATE TABLE credential (provider TEXT, value TEXT);
                 INSERT INTO credential VALUES
                     ('made-up-cloud', '{\"type\": \"api\", \"key\": \"ex-made-up-key\"}');",
            )
            .expect("the fixture");
        drop(connection);
        let host = TestHost::new(plugin.folder());
        let path = text(&database);

        let (rows, fingerprint, alone, in_where, in_a_subquery) = host
            .as_agent(AGENT, || {
                let query = |sql: &str| crate::sqlite::query(&path, sql, &[]);
                let rows =
                    query("SELECT rowid, provider, value FROM credential")
                        .expect("the rows");
                let fingerprint = match &rows[0][2] {
                    Value::JsonWithKeys(document) => {
                        document.key(0).expect("a key").fingerprint()
                    },
                    other => panic!("JSON with keys, not {other:?}"),
                };

                (
                    format!("{rows:?}"),
                    fingerprint,
                    query("SELECT value FROM credential").map(|_| ()),
                    query(
                        "SELECT rowid FROM credential WHERE value LIKE 'ex-%'",
                    )
                    .map(|_| ()),
                    query(
                        "SELECT r, value FROM \
                         (SELECT rowid AS r, value FROM credential) \
                         WHERE value IS NOT NULL",
                    )
                    .map(|rows| rows.len()),
                )
            });

        assert!(!rows.contains("ex-made-up-key"), "{rows}");
        assert!(
            rows.contains(r#"{\"key\":{\"$key\":0},\"type\":\"api\"}"#),
            "{rows}"
        );
        assert!(rows.starts_with("[[Integer(1), Text(\"made-up-cloud\")"));
        assert_eq!(
            fingerprint.as_deref(),
            Ok(
                "27c8d30ec451e808415d2307396308459e728ccef0adc5a41908fccae8a2e1ae"
            )
        );
        assert!(
            matches!(&alone, Err(HostError::Failed(what)) if what.contains("rowid of credential")),
            "{alone:?}"
        );
        assert!(
            matches!(&in_where, Err(HostError::NotAllowed(what)) if what.contains("key column")),
            "{in_where:?}"
        );
        assert_eq!(in_a_subquery, Ok(0));
    }

    #[test]
    fn attach_and_a_key_column_in_other_case_are_refused_as_the_host_does() {
        let plugin = MadeUpPlugin::new();
        let database = plugin.write(".made-up/keys.db", b"");
        let other = plugin.write(".made-up/other.db", b"");
        let connection =
            rusqlite::Connection::open(&database).expect("a database");
        connection
            .execute_batch(
                "CREATE TABLE Credential (provider TEXT, Value TEXT);
                 INSERT INTO Credential VALUES ('made-up-cloud', 'x');",
            )
            .expect("the fixture");
        drop(connection);
        let host = TestHost::new(plugin.folder());
        let path = text(&database);
        let attach = format!("ATTACH DATABASE '{}' AS other", text(&other));

        let (attached, as_text) = host.as_agent(AGENT, || {
            (
                crate::sqlite::query(&path, &attach, &[]),
                crate::sqlite::query(
                    &path,
                    "SELECT rowid FROM credential WHERE VALUE = 'x'",
                    &[],
                ),
            )
        });

        assert!(
            matches!(&attached, Err(HostError::NotAllowed(what)) if what.contains("ATTACH")),
            "{attached:?}"
        );
        assert!(
            matches!(&as_text, Err(HostError::NotAllowed(what)) if what.contains("key column")),
            "{as_text:?}"
        );
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
    fn a_declared_program_gives_its_canned_output_and_is_noted() {
        let plugin = MadeUpPlugin::new();
        let host = TestHost::new(plugin.folder());
        host.answer_run(
            "made-up",
            &["--version"],
            programs::Output {
                status: Some(0),
                stdout: b"made-up 1.2.3\n".to_vec(),
                stderr: Vec::new(),
            },
        );
        let version = vec!["--version".to_string()];
        let other = vec!["--yolo".to_string()];
        let home = vec![("MADE_UP_HOME".to_string(), "/data".to_string())];

        let (declared, undeclared) = host.as_agent(AGENT, || {
            (
                programs::run("made-up", &version, &home, 1000),
                programs::run("made-up", &other, &[], 1000),
            )
        });
        let as_provider = host.as_provider(PROVIDER, || {
            programs::run("made-up", &version, &[], 1000)
        });

        assert_eq!(
            declared.map(|output| output.stdout),
            Ok(b"made-up 1.2.3\n".to_vec())
        );
        assert_eq!(
            undeclared,
            Err(HostError::NotAllowed("made-up".to_string()))
        );
        assert_eq!(
            as_provider,
            Err(HostError::NotAllowed("made-up".to_string()))
        );
        let started = host.started();
        assert_eq!(started.len(), 1);
        assert_eq!(started[0].args(), version);
        assert_eq!(started[0].env(), home);
    }

    #[test]
    fn a_variable_the_part_does_not_list_is_refused() {
        let plugin = MadeUpPlugin::new();
        let host = TestHost::new(plugin.folder());
        let version = vec!["--version".to_string()];
        let path = vec![("PATH".to_string(), "/evil".to_string())];

        let run = host.as_agent(AGENT, || {
            programs::run("made-up", &version, &path, 1000)
        });

        assert_eq!(
            run,
            Err(HostError::NotAllowed("variable PATH".to_string()))
        );
        assert!(host.started().is_empty());
    }

    #[test]
    fn a_talk_gives_its_canned_lines_and_notes_what_was_sent() {
        let plugin = MadeUpPlugin::new();
        let host = TestHost::new(plugin.folder());
        host.answer_talk(
            "made-up",
            &["app-server", "--profile", "work"],
            &["{\"id\":1}"],
        );
        let args = vec![
            "app-server".to_string(),
            "--profile".to_string(),
            "work".to_string(),
        ];

        let lines = host.as_agent(AGENT, || {
            let talk = programs::Talk::start("made-up", &args, &[])?;
            talk.send_line("{\"method\":\"hello\"}")?;

            Ok::<_, HostError>((talk.next_line(1000)?, talk.next_line(1000)?))
        });

        assert_eq!(lines, Ok((Some("{\"id\":1}".to_string()), None)));
        assert_eq!(host.sent(), ["{\"method\":\"hello\"}"]);
    }

    #[test]
    fn a_hidden_terminal_shows_its_canned_screens() {
        let plugin = MadeUpPlugin::new();
        let host = TestHost::new(plugin.folder());
        host.answer_terminal("made-up", &["--tui"], &["Loading", "Usage 29%"]);
        let tui = vec!["--tui".to_string()];

        let screens = host.as_agent(AGENT, || {
            let terminal = crate::terminal::HiddenTerminal::start(
                "made-up",
                &tui,
                &[],
                100,
                40,
            )?;
            let before = terminal.screen()?;
            terminal.send_keys("/usage\r")?;
            let mut changes = Vec::new();
            while terminal.wait_for_change(1000)? {
                changes.push(terminal.screen()?);
            }

            Ok::<_, HostError>((before, changes))
        });
        let not_in_a_terminal = host.as_agent(AGENT, || {
            crate::terminal::HiddenTerminal::start(
                "made-up",
                &["--version".to_string()],
                &[],
                100,
                40,
            )
            .map(|_| ())
        });

        assert_eq!(
            screens,
            Ok((
                String::new(),
                vec!["Loading".to_string(), "Usage 29%".to_string()]
            ))
        );
        assert_eq!(host.sent(), ["/usage\r"]);
        assert_eq!(
            not_in_a_terminal,
            Err(HostError::NotAllowed("made-up".to_string()))
        );
    }

    #[test]
    fn the_working_folder_of_hidden_terminals_is_told_to_a_part_with_one() {
        let plugin = MadeUpPlugin::new();
        let host = TestHost::new(plugin.folder());
        let moved = TestHost::new(plugin.folder())
            .with_terminal_folder("/data/cache/otty/agent-probe");

        let folders = host.as_agent(AGENT, crate::terminal::working_folders);
        let moved = moved.as_agent(AGENT, crate::terminal::working_folders);
        let provider =
            host.as_provider(PROVIDER, crate::terminal::working_folders);

        let default = host.home().join(".cache/otty/agent-probe");
        assert_eq!(folders, Ok(vec![default.display().to_string()]));
        assert_eq!(moved, Ok(vec!["/data/cache/otty/agent-probe".to_string()]));
        assert_eq!(
            provider,
            Err(HostError::NotAllowed(
                "the working folder of hidden terminals".to_string()
            ))
        );
    }

    #[test]
    fn a_declared_program_with_no_canned_answer_fails() {
        let plugin = MadeUpPlugin::new();
        let host = TestHost::new(plugin.folder());
        let version = vec!["--version".to_string()];

        let run = host
            .as_agent(AGENT, || programs::run("made-up", &version, &[], 1000));

        assert_eq!(
            run,
            Err(HostError::Failed(
                "no canned answer for made-up --version".to_string()
            ))
        );
    }

    /// Issue 20: the language is the one OTTY shows at each call, not the
    /// one of the start.
    #[test]
    fn the_language_is_the_one_of_each_call() {
        let plugin = MadeUpPlugin::new();
        let host = TestHost::new(plugin.folder());

        let before = host.as_agent(AGENT, crate::env::language);
        host.set_language("pt-BR");
        let after = host.as_agent(AGENT, crate::env::language);

        assert_eq!(before, "en");
        assert_eq!(after, "pt-BR");
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

//! The permissions of one part made concrete for this computer, as the
//! host operations check them on every call (spec
//! 2026-11-19-agent-plugins, Decision 7), and for one call with login
//! folders, what a provider part reads inside them (spec
//! 2026-11-23-accounts-apart-from-profiles, Decision 5).

use std::path::{Path, PathBuf};

use globset::{GlobBuilder, GlobMatcher, GlobSet};

use crate::environment::Environment;
use crate::path_glob::{glob_set, is_plain_absolute, pattern_set};
use crate::permissions::declared::{
    JsonPointer, KeyRule, KeyringRule, LoginRule, PartPermissions, ProgramRule,
};

/// What one part may do right now, with `~` and its variables replaced
/// by this computer's folders. A call carries the permissions of the part
/// it is for only, so another part's paths and hosts are refused like any
/// path or host nobody declared.
///
/// A call of a provider part may also carry login folders
/// ([`Self::in_login_folders`]): inside them the part reads what its
/// [`LoginRule`]s declare, and nowhere else.
#[derive(Debug, Clone)]
pub struct Permissions {
    read: GlobSet,
    write: GlobSet,
    sqlite: GlobSet,
    key_files: GlobSet,
    env: Vec<String>,
    key_vars: Vec<String>,
    run: Vec<ProgramMatcher>,
    terminal: Vec<ProgramMatcher>,
    keyring: Vec<KeyringRule>,
    keys: Vec<KeyMatcher>,
    http: Vec<String>,
    logins: Vec<LoginRule>,
    folder_read: GlobSet,
    folder_write: GlobSet,
    folder_key_files: GlobSet,
    folder_vars: Vec<(String, PathBuf)>,
}

impl Permissions {
    /// The permissions `declared` by a part, for the folders of `env`.
    pub fn new(declared: &PartPermissions, env: &Environment) -> Self {
        let mut key_files = Vec::new();
        let mut key_vars = Vec::new();
        let mut keys = Vec::new();
        for rule in declared.keys() {
            match rule {
                KeyRule::JsonFile { files, fields } => {
                    key_files.extend(files.iter().cloned());
                    keys.push(KeyMatcher::JsonFile {
                        files: glob_set(files, env),
                        fields: fields.clone(),
                    });
                },
                KeyRule::SqliteColumn {
                    databases,
                    table,
                    column,
                    fields,
                } => keys.push(KeyMatcher::SqliteColumn {
                    databases: glob_set(databases, env),
                    table: table.clone(),
                    column: column.clone(),
                    fields: fields.clone(),
                }),
                KeyRule::EnvVars { names } => {
                    key_vars.extend(names.iter().cloned());
                },
            }
        }

        Self {
            read: glob_set(declared.read(), env),
            write: glob_set(declared.write(), env),
            sqlite: glob_set(declared.sqlite(), env),
            key_files: glob_set(&key_files, env),
            env: declared.env().to_vec(),
            key_vars,
            run: declared.run().iter().map(ProgramMatcher::new).collect(),
            terminal: declared
                .terminal()
                .iter()
                .map(ProgramMatcher::new)
                .collect(),
            keyring: declared.keyring().to_vec(),
            keys,
            http: declared.http().to_vec(),
            logins: declared.logins().to_vec(),
            folder_read: GlobSet::empty(),
            folder_write: GlobSet::empty(),
            folder_key_files: GlobSet::empty(),
            folder_vars: Vec::new(),
        }
    }

    /// The same permissions for one call with the login folders
    /// `folders`: inside each folder of an agent part its [`LoginRule`]s
    /// name, the part also reads their files and their key files, keys
    /// as handles, writes and deletes their `write` files, and a program
    /// it starts may get the rule's `folder_env` set to that folder. OTTY's own login folder is read by
    /// the rule that offers "Log in with". A folder of another agent part
    /// or one that is not an absolute path with no `..` adds nothing.
    pub fn in_login_folders(&self, folders: &[LoginFolder]) -> Self {
        let mut read = Vec::new();
        let mut write = Vec::new();
        let mut key_files = Vec::new();
        let mut keys = self.keys.clone();
        let mut vars = Vec::new();
        let usable = folders
            .iter()
            .filter(|folder| is_plain_absolute(folder.folder()));
        for folder in usable {
            let path = folder.folder();
            let rules =
                self.logins.iter().filter(|rule| match folder.agent() {
                    Some(agent) => rule.agent() == agent,
                    None => rule.offers_log_in(),
                });
            for rule in rules {
                read.extend(
                    rule.read().iter().filter_map(|glob| glob.resolve(path)),
                );
                write.extend(
                    rule.write().iter().filter_map(|glob| glob.resolve(path)),
                );
                for login_keys in rule.keys() {
                    let files: Vec<String> = login_keys
                        .files()
                        .iter()
                        .filter_map(|glob| glob.resolve(path))
                        .collect();
                    keys.push(KeyMatcher::JsonFile {
                        files: pattern_set(&files),
                        fields: login_keys.fields().to_vec(),
                    });
                    key_files.extend(files);
                }
                if let Some(name) = rule.folder_env() {
                    vars.push((name.to_string(), path.to_path_buf()));
                }
            }
        }

        Self {
            keys,
            folder_read: pattern_set(&read),
            folder_write: pattern_set(&write),
            folder_key_files: pattern_set(&key_files),
            folder_vars: vars,
            ..self.clone()
        }
    }

    /// Whether the part may read the bytes of the file `path`. A key file
    /// is refused: it is read only with its keys replaced by handles.
    pub fn may_read(&self, path: &Path) -> bool {
        is_plain_absolute(path)
            && (self.read.is_match(path) || self.folder_read.is_match(path))
            && !self.is_key_file(path)
    }

    /// Whether the part may write or delete the file `path`. A key file is
    /// never written.
    pub fn may_write(&self, path: &Path) -> bool {
        is_plain_absolute(path)
            && (self.write.is_match(path) || self.folder_write.is_match(path))
            && !self.is_key_file(path)
    }

    /// Whether the part may query the SQLite database `path`, read-only.
    pub fn may_query(&self, path: &Path) -> bool {
        is_plain_absolute(path) && self.sqlite.is_match(path)
    }

    /// Whether the part may read the key file `path` as JSON with its key
    /// fields replaced by handles.
    pub fn may_read_with_keys(&self, path: &Path) -> bool {
        is_plain_absolute(path) && self.is_key_file(path)
    }

    /// The JSON fields that hold keys in the key file `file`; none for a
    /// file the part does not declare in `keys`.
    pub fn key_fields(&self, file: &Path) -> Vec<JsonPointer> {
        if !is_plain_absolute(file) {
            return Vec::new();
        }

        self.keys
            .iter()
            .filter_map(|key| match key {
                KeyMatcher::JsonFile { files, fields }
                    if files.is_match(file) =>
                {
                    Some(fields.iter().cloned())
                },
                _ => None,
            })
            .flatten()
            .collect()
    }

    /// Each key column the part declares in the SQLite database
    /// `database`. A query never reads them as text: they come back as
    /// key handles.
    pub fn key_columns(&self, database: &Path) -> Vec<KeyColumn> {
        if !is_plain_absolute(database) {
            return Vec::new();
        }

        self.keys
            .iter()
            .filter_map(|key| match key {
                KeyMatcher::SqliteColumn {
                    databases,
                    table,
                    column,
                    fields,
                } if databases.is_match(database) => Some(KeyColumn {
                    table: table.clone(),
                    column: column.clone(),
                    fields: fields.clone(),
                }),
                _ => None,
            })
            .collect()
    }

    /// Whether the part may read the variable `name` as text. A variable
    /// that holds a key is read only through a key handle.
    pub fn may_read_var(&self, name: &str) -> bool {
        self.env.iter().any(|listed| listed == name)
            && !self.key_vars.iter().any(|key_var| key_var == name)
    }

    /// Whether a program the part starts may get the variable `name` set
    /// to `value`. In a call with login folders, a `folder_env` of the
    /// part's logins may only point at one of those folders; any other
    /// variable only when the part may read it as text.
    pub fn may_pass_var(&self, name: &str, value: &str) -> bool {
        let mut folders = self
            .folder_vars
            .iter()
            .filter(|(var, _)| var == name)
            .map(|(_, folder)| folder)
            .peekable();
        if folders.peek().is_none() {
            return self.may_read_var(name);
        }

        let value = Path::new(value);

        is_plain_absolute(value) && folders.any(|folder| folder == value)
    }

    /// Whether the part may run `program` with exactly the arguments
    /// `args`.
    pub fn may_run(&self, program: &str, args: &[String]) -> bool {
        self.run.iter().any(|rule| rule.matches(program, args))
    }

    /// Whether the part may start `program` with `args` in a hidden
    /// terminal.
    pub fn may_start_terminal(&self, program: &str, args: &[String]) -> bool {
        self.terminal.iter().any(|rule| rule.matches(program, args))
    }

    /// Whether the part may start any program in a hidden terminal.
    pub fn has_terminals(&self) -> bool {
        !self.terminal.is_empty()
    }

    /// Whether the part may send an HTTPS request to `host`; hosts are
    /// compared whole, with no subdomains.
    pub fn may_call(&self, host: &str) -> bool {
        self.http
            .iter()
            .any(|listed| listed.eq_ignore_ascii_case(host))
    }

    /// Whether the part may read the keyring entry of `service` and
    /// `user`.
    pub fn may_read_keyring(&self, service: &str, user: &str) -> bool {
        self.keyring
            .iter()
            .any(|rule| rule.service() == service && rule.user() == user)
    }

    /// The JSON fields that hold keys in the keyring entry of `service`
    /// and `user`; none when the whole entry is the key or the part does
    /// not declare the entry.
    pub fn keyring_fields(
        &self,
        service: &str,
        user: &str,
    ) -> Vec<JsonPointer> {
        self.keyring
            .iter()
            .filter(|rule| rule.service() == service && rule.user() == user)
            .flat_map(|rule| rule.fields().iter().cloned())
            .collect()
    }

    /// Whether the part may open a key handle at `place`.
    pub fn may_open_key(&self, place: &KeyPlace) -> bool {
        match place {
            KeyPlace::JsonField { file, pointer } => {
                is_plain_absolute(file)
                    && self
                        .keys
                        .iter()
                        .any(|key| key.has_json_field(file, pointer))
            },
            KeyPlace::EnvVar(name) => {
                self.key_vars.iter().any(|key_var| key_var == name)
            },
            KeyPlace::Keyring {
                service,
                user,
                field,
            } => self.keyring.iter().any(|rule| {
                rule.service() == service
                    && rule.user() == user
                    && field_allowed(rule.fields(), field.as_deref())
            }),
            KeyPlace::SqliteField {
                database,
                table,
                column,
                pointer,
            } => {
                is_plain_absolute(database)
                    && self.keys.iter().any(|key| {
                        key.has_sqlite_field(
                            database,
                            table,
                            column,
                            pointer.as_deref(),
                        )
                    })
            },
        }
    }

    /// Whether `path` is a key file of the part, declared in `keys` or in
    /// a login folder of the call.
    fn is_key_file(&self, path: &Path) -> bool {
        self.key_files.is_match(path) || self.folder_key_files.is_match(path)
    }
}

/// A login folder OTTY gives a provider part for one call: the config
/// folder of a profile of an agent part of the same plugin, such as
/// `~/.codex-work` of `codex`, or OTTY's own login folder of the part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginFolder {
    agent: Option<String>,
    folder: PathBuf,
}

impl LoginFolder {
    /// The id of the agent part of the profile; `None` for OTTY's own
    /// login folder, which the entry that offers "Log in with" reads.
    pub fn agent(&self) -> Option<&str> {
        self.agent.as_deref()
    }

    /// The profile's config folder.
    pub fn folder(&self) -> &Path {
        &self.folder
    }

    /// The folder `folder` of a profile of the agent part `agent`.
    ///
    /// ```
    /// use std::path::Path;
    ///
    /// use otty_plugin_manifest::LoginFolder;
    ///
    /// let folder = LoginFolder::new("codex", "/home/user/.codex-work");
    ///
    /// assert_eq!(folder.agent(), Some("codex"));
    /// assert_eq!(folder.folder(), Path::new("/home/user/.codex-work"));
    /// ```
    pub fn new(agent: &str, folder: impl Into<PathBuf>) -> Self {
        Self {
            agent: Some(agent.to_string()),
            folder: folder.into(),
        }
    }

    /// OTTY's own login folder `folder` of the part (spec
    /// 2026-11-23-accounts-apart-from-profiles, Decision 6), such as
    /// `~/.local/share/otty/accounts/<id>`.
    ///
    /// ```
    /// use otty_plugin_manifest::LoginFolder;
    ///
    /// let folder = LoginFolder::own("/home/user/.local/share/otty/accounts/1");
    ///
    /// assert_eq!(folder.agent(), None);
    /// ```
    pub fn own(folder: impl Into<PathBuf>) -> Self {
        Self {
            agent: None,
            folder: folder.into(),
        }
    }
}

/// A place a key is read from, as a plugin asks for its handle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyPlace {
    /// A field of a JSON file.
    JsonField {
        /// The file.
        file: PathBuf,
        /// The JSON pointer of the field.
        pointer: String,
    },
    /// An environment variable.
    EnvVar(String),
    /// A system keyring entry, or one JSON field of it.
    Keyring {
        /// The keyring service.
        service: String,
        /// The keyring user.
        user: String,
        /// The JSON pointer of the field, or `None` for the whole entry.
        field: Option<String>,
    },
    /// A column of a SQLite table, or one JSON field of it.
    SqliteField {
        /// The database file.
        database: PathBuf,
        /// The table.
        table: String,
        /// The column.
        column: String,
        /// The JSON pointer of the field, or `None` for the whole column.
        pointer: Option<String>,
    },
}

/// A column of a SQLite table that a part declares as holding keys.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyColumn {
    table: String,
    column: String,
    fields: Vec<JsonPointer>,
}

impl KeyColumn {
    /// The table, as the manifest names it.
    pub fn table(&self) -> &str {
        &self.table
    }

    /// The column, as the manifest names it.
    pub fn column(&self) -> &str {
        &self.column
    }

    /// The JSON fields of the column that hold keys; empty when the whole
    /// column is the key.
    pub fn fields(&self) -> &[JsonPointer] {
        &self.fields
    }
}

/// A program rule with its argument patterns compiled.
#[derive(Debug, Clone)]
struct ProgramMatcher {
    program: String,
    /// `None` when a pattern does not compile: the rule then matches
    /// nothing.
    args: Option<Vec<GlobMatcher>>,
}

impl ProgramMatcher {
    fn new(rule: &ProgramRule) -> Self {
        let args = rule
            .args()
            .iter()
            .map(|arg| {
                GlobBuilder::new(arg)
                    .build()
                    .map(|glob| glob.compile_matcher())
            })
            .collect::<Result<Vec<_>, _>>()
            .ok();

        Self {
            program: rule.program().to_string(),
            args,
        }
    }

    fn matches(&self, program: &str, args: &[String]) -> bool {
        let Some(patterns) = &self.args else {
            return false;
        };

        self.program == program
            && patterns.len() == args.len()
            && patterns
                .iter()
                .zip(args)
                .all(|(pattern, arg)| pattern.is_match(arg))
    }
}

/// A key place of files or databases, with its globs compiled.
#[derive(Debug, Clone)]
enum KeyMatcher {
    JsonFile {
        files: GlobSet,
        fields: Vec<JsonPointer>,
    },
    SqliteColumn {
        databases: GlobSet,
        table: String,
        column: String,
        fields: Vec<JsonPointer>,
    },
}

impl KeyMatcher {
    fn has_json_field(&self, file: &Path, pointer: &str) -> bool {
        match self {
            KeyMatcher::JsonFile { files, fields } => {
                files.is_match(file)
                    && fields.iter().any(|field| field.matches(pointer))
            },
            _ => false,
        }
    }

    fn has_sqlite_field(
        &self,
        database: &Path,
        table: &str,
        column: &str,
        pointer: Option<&str>,
    ) -> bool {
        match self {
            KeyMatcher::SqliteColumn {
                databases,
                table: key_table,
                column: key_column,
                fields,
            } => {
                databases.is_match(database)
                    && key_table == table
                    && key_column == column
                    && field_allowed(fields, pointer)
            },
            _ => false,
        }
    }
}

/// Whether `field` is one of `fields`; with no fields, the whole value is
/// the key and only `None` is allowed.
fn field_allowed(fields: &[JsonPointer], field: Option<&str>) -> bool {
    match field {
        Some(field) => fields.iter().any(|allowed| allowed.matches(field)),
        None => fields.is_empty(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::Manifest;
    use crate::part::PartRef;
    use crate::testing::{
        OPENAI_MANIFEST, OPENCODE_MANIFEST, edited, with_codex_logins,
    };

    fn env() -> Environment {
        Environment::new(PathBuf::from("/home/user"))
            .with_var("CODEX_HOME", "/data/codex")
            .with_var("XDG_DATA_HOME", "/data/xdg")
    }

    fn permissions_of(text: &str, part: &PartRef) -> Permissions {
        let manifest = Manifest::parse(text).expect("valid");
        let declared = manifest.permissions_of(part).expect("listed part");

        Permissions::new(declared, &env())
    }

    fn args(args: &[&str]) -> Vec<String> {
        args.iter().map(|arg| arg.to_string()).collect()
    }

    /// The permissions of the provider part of [`with_codex_logins`],
    /// outside a call with login folders.
    fn codex_logins() -> Permissions {
        permissions_of(&with_codex_logins(), &PartRef::provider("openai"))
    }

    fn json_field(file: &str, pointer: &str) -> KeyPlace {
        KeyPlace::JsonField {
            file: PathBuf::from(file),
            pointer: pointer.to_string(),
        }
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, "What OTTY stores":
    /// in a call with a login folder, wherever it is, the part reads its
    /// declared files there, the key files only with keys as handles.
    #[test]
    fn in_a_login_folder_the_part_reads_what_its_logins_declare() {
        let called = codex_logins()
            .in_login_folders(&[LoginFolder::new("codex", "/work/codex-a")]);
        let auth = Path::new("/work/codex-a/auth.json");

        let fields: Vec<String> = called
            .key_fields(auth)
            .iter()
            .map(|field| field.as_str().to_string())
            .collect();

        assert!(called.may_read_with_keys(auth));
        assert!(!called.may_read(auth));
        assert!(!called.may_write(auth));
        assert_eq!(fields, ["/OPENAI_API_KEY", "/tokens/access_token"]);
        assert!(called.may_open_key(&json_field(
            "/work/codex-a/auth.json",
            "/tokens/access_token"
        )));
        assert!(!called.may_open_key(&json_field(
            "/work/codex-a/auth.json",
            "/tokens/refresh_token"
        )));
        assert!(
            called.may_read(Path::new("/work/codex-a/sessions/2026/a.jsonl"))
        );
        assert!(called.may_call("chatgpt.com"));
    }

    /// A file of the folder the part does not declare, a file of another
    /// folder and a folder of another agent part are refused.
    #[test]
    fn a_login_folder_gives_only_its_declared_files() {
        let called = codex_logins().in_login_folders(&[
            LoginFolder::new("codex", "/work/codex-a"),
            LoginFolder::new("claude-code", "/work/claude"),
            LoginFolder::new("codex", "work/relative"),
        ]);

        assert!(!called.may_read(Path::new("/work/codex-a/config.toml")));
        assert!(!called.may_read_with_keys(Path::new("/work/codex-a/a.json")));
        assert!(
            !called.may_read_with_keys(Path::new("/work/codex-b/auth.json"))
        );
        assert!(!called.may_read(Path::new("/work/codex-b/sessions/a.jsonl")));
        assert!(
            !called.may_read_with_keys(Path::new("/work/claude/auth.json"))
        );
        assert!(!called.may_read(Path::new("/work/codex-a/../x/auth.json")));
        assert!(
            !called.may_read_with_keys(Path::new("work/relative/auth.json"))
        );
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decision 6: OTTY's
    /// own login folder is read as a folder of the agent part whose entry
    /// offers "Log in with", and the entry's variable may point there.
    #[test]
    fn an_own_login_folder_is_read_as_the_entry_that_offers_log_in() {
        let offering = edited(
            &with_codex_logins(),
            r#"folder_env = "CODEX_HOME""#,
            "folder_env = \"CODEX_HOME\"\nlog_in = true",
        );
        let base = permissions_of(&offering, &PartRef::provider("openai"));
        let own = "/data/otty/accounts/6f0c1e2a";

        let called = base.in_login_folders(&[LoginFolder::own(own)]);
        let not_offered =
            codex_logins().in_login_folders(&[LoginFolder::own(own)]);

        let auth = Path::new("/data/otty/accounts/6f0c1e2a/auth.json");
        let session = "/data/otty/accounts/6f0c1e2a/sessions/a.jsonl";
        assert!(called.may_read_with_keys(auth));
        assert!(called.may_read(Path::new(session)));
        assert!(called.may_pass_var("CODEX_HOME", own));
        assert!(!not_offered.may_read_with_keys(auth));
        assert!(!not_offered.may_pass_var("CODEX_HOME", own));
    }

    /// Issue 09 of spec 2026-11-23-accounts-apart-from-profiles: the
    /// `write` globs of a `logins` entry let the part write and delete
    /// inside each login folder of the call only, an own login folder
    /// too, so a probe cleans up the files it left there. A key file is
    /// never written, and outside the call nothing is.
    #[test]
    fn a_login_folder_is_written_only_where_its_logins_declare() {
        let writing = edited(
            &with_codex_logins(),
            r#"read = ["sessions/**"]"#,
            "read = [\"sessions/**\"]\nlog_in = true\n\
             write = [\"probe/*.jsonl\", \"auth.json\"]",
        );
        let base = permissions_of(&writing, &PartRef::provider("openai"));
        let own = "/data/otty/accounts/6f0c1e2a";

        let called = base.in_login_folders(&[
            LoginFolder::new("codex", "/work/codex-a"),
            LoginFolder::own(own),
        ]);

        assert!(called.may_write(Path::new("/work/codex-a/probe/a.jsonl")));
        assert!(called.may_write(Path::new(&format!("{own}/probe/b.jsonl"))));
        assert!(!called.may_write(Path::new("/work/codex-a/probe/a.txt")));
        assert!(!called.may_write(Path::new("/work/codex-b/probe/a.jsonl")));
        assert!(!called.may_write(Path::new("/work/codex-a/auth.json")));
        assert!(!base.may_write(Path::new("/work/codex-a/probe/a.jsonl")));
    }

    /// Nothing of the login rules applies outside a call with folders.
    #[test]
    fn with_no_login_folder_the_logins_give_nothing() {
        let base = codex_logins();
        let auth = Path::new("/work/codex-a/auth.json");

        assert!(!base.may_read_with_keys(auth));
        assert!(base.key_fields(auth).is_empty());
        assert!(!base.may_open_key(&json_field(
            "/work/codex-a/auth.json",
            "/tokens/access_token"
        )));
        assert!(!base.may_read(Path::new("/work/codex-a/sessions/a.jsonl")));
        assert!(!base.in_login_folders(&[]).may_read_with_keys(auth));
    }

    /// A program the part starts gets `folder_env` only pointing at a
    /// login folder of the call; the part never reads it as text.
    #[test]
    fn the_folder_variable_points_only_at_a_login_folder() {
        let base = codex_logins();
        let called = base
            .in_login_folders(&[LoginFolder::new("codex", "/work/codex-a")]);

        assert!(called.may_pass_var("CODEX_HOME", "/work/codex-a"));
        assert!(called.may_pass_var("CODEX_HOME", "/work/codex-a/"));
        assert!(!called.may_pass_var("CODEX_HOME", "/work/codex-b"));
        assert!(!called.may_pass_var("CODEX_HOME", "/work/codex-a/../x"));
        assert!(!called.may_pass_var("CODEX_HOME", "codex-a"));
        assert!(!called.may_read_var("CODEX_HOME"));
        assert!(!base.may_pass_var("CODEX_HOME", "/work/codex-a"));
    }

    /// A variable the part lists in `env` reaches its programs with any
    /// value, as before.
    #[test]
    fn a_listed_variable_reaches_a_program_with_any_value() {
        let codex = permissions_of(OPENAI_MANIFEST, &PartRef::agent("codex"));

        assert!(codex.may_pass_var("CODEX_HOME", "/data/codex"));
        assert!(!codex.may_pass_var("HOME", "/home/user"));
    }

    #[test]
    fn an_agent_part_reads_its_globs_only() {
        let codex = permissions_of(OPENAI_MANIFEST, &PartRef::agent("codex"));

        assert!(
            codex.may_read(Path::new("/home/user/.codex/sessions/a.jsonl"))
        );
        assert!(codex.may_read(Path::new("/data/codex/history.jsonl")));
        assert!(!codex.may_read(Path::new("/home/user/.ssh/id_ed25519")));
        assert!(!codex.may_read(Path::new("/home/user/.codex/../.ssh/key")));
        assert!(!codex.may_read(Path::new("relative/.codex/a.jsonl")));
        assert!(!codex.may_write(Path::new("/home/user/.codex/a.jsonl")));
    }

    #[test]
    fn a_key_file_is_read_only_with_keys_replaced() {
        let codex = permissions_of(OPENAI_MANIFEST, &PartRef::agent("codex"));
        let auth = Path::new("/home/user/.codex/auth.json");

        assert!(!codex.may_read(auth));
        assert!(codex.may_read_with_keys(auth));
        assert!(codex.may_open_key(&KeyPlace::JsonField {
            file: auth.to_path_buf(),
            pointer: "/tokens/access_token".to_string(),
        }));
        assert!(!codex.may_open_key(&KeyPlace::JsonField {
            file: auth.to_path_buf(),
            pointer: "/tokens/other".to_string(),
        }));
    }

    #[test]
    fn a_key_file_names_its_key_fields() {
        let codex = permissions_of(OPENAI_MANIFEST, &PartRef::agent("codex"));
        let openai =
            permissions_of(OPENAI_MANIFEST, &PartRef::provider("openai"));
        let auth = Path::new("/home/user/.codex/auth.json");

        let fields: Vec<String> = codex
            .key_fields(auth)
            .iter()
            .map(|field| field.as_str().to_string())
            .collect();

        assert_eq!(fields, ["/OPENAI_API_KEY", "/tokens/access_token"]);
        assert!(codex.key_fields(Path::new("/home/user/a.json")).is_empty());
        assert!(openai.key_fields(auth).is_empty());
    }

    #[test]
    fn a_path_of_another_part_of_the_same_plugin_is_refused() {
        let openai =
            permissions_of(OPENAI_MANIFEST, &PartRef::provider("openai"));
        let auth = PathBuf::from("/home/user/.codex/auth.json");

        assert!(!openai.may_read(Path::new("/home/user/.codex/a.jsonl")));
        assert!(!openai.may_read_with_keys(&auth));
        assert!(!openai.may_open_key(&KeyPlace::JsonField {
            file: auth,
            pointer: "/tokens/access_token".to_string(),
        }));
        assert!(!openai.may_run("codex", &args(&["--version"])));
        assert!(!openai.may_read_var("CODEX_HOME"));
    }

    #[test]
    fn a_provider_part_calls_its_hosts_only() {
        let openai =
            permissions_of(OPENAI_MANIFEST, &PartRef::provider("openai"));
        let codex = permissions_of(OPENAI_MANIFEST, &PartRef::agent("codex"));

        assert!(openai.may_call("chatgpt.com"));
        assert!(openai.may_call("API.openai.com"));
        assert!(!openai.may_call("evil.chatgpt.com"));
        assert!(!openai.may_call("openai.com"));
        assert!(!codex.may_call("chatgpt.com"));
    }

    #[test]
    fn a_program_runs_only_with_its_declared_arguments() {
        let codex = permissions_of(OPENAI_MANIFEST, &PartRef::agent("codex"));

        assert!(codex.may_run("codex", &args(&["--version"])));
        assert!(codex.may_run(
            "codex",
            &args(&["-s", "read-only", "-a", "never", "app-server",])
        ));
        assert!(!codex.may_run("codex", &args(&["--version", "--yolo"])));
        assert!(!codex.may_run("codex", &args(&["exec", "rm -rf ~"])));
        assert!(!codex.may_run("sh", &args(&["--version"])));
        assert!(!codex.may_start_terminal("codex", &args(&["--version"])));
        assert!(!codex.has_terminals());
    }

    #[test]
    fn a_part_with_a_terminal_rule_has_terminals() {
        let text = edited(
            OPENAI_MANIFEST,
            "[[agents.agent.permissions.run]]",
            "[[agents.agent.permissions.terminal]]\nprogram = \"codex\"\nargs = [\"tui\"]\n\n[[agents.agent.permissions.run]]",
        );
        let codex = permissions_of(&text, &PartRef::agent("codex"));

        assert!(codex.has_terminals());
        assert!(codex.may_start_terminal("codex", &args(&["tui"])));
    }

    #[test]
    fn a_database_names_its_key_columns() {
        let opencode =
            permissions_of(OPENCODE_MANIFEST, &PartRef::agent("opencode"));
        let codex = permissions_of(OPENAI_MANIFEST, &PartRef::agent("codex"));
        let database = Path::new("/data/xdg/opencode/opencode.db");

        let columns = opencode.key_columns(database);

        assert_eq!(columns.len(), 1);
        assert_eq!(columns[0].table(), "credential");
        assert_eq!(columns[0].column(), "value");
        let fields: Vec<&str> = columns[0]
            .fields()
            .iter()
            .map(JsonPointer::as_str)
            .collect();
        assert_eq!(fields, ["/key", "/access", "/refresh"]);
        assert!(
            opencode
                .key_columns(Path::new("/data/xdg/other/x.db"))
                .is_empty()
        );
        assert!(codex.key_columns(database).is_empty());
    }

    #[test]
    fn variables_are_read_only_when_listed() {
        let codex = permissions_of(OPENAI_MANIFEST, &PartRef::agent("codex"));

        assert!(codex.may_read_var("CODEX_HOME"));
        assert!(!codex.may_read_var("HOME"));
    }

    #[test]
    fn a_keyring_entry_names_its_key_fields() {
        let manifest = edited(
            OPENCODE_MANIFEST,
            "[[agents.agent.permissions.keys]]",
            "[[agents.agent.permissions.keyring]]\n\
             service = \"made-up\"\n\
             user = \"work\"\n\
             fields = [\"/key\", \"/*/token\"]\n\n\
             [[agents.agent.permissions.keyring]]\n\
             service = \"made-up\"\n\
             user = \"plain\"\n\n\
             [[agents.agent.permissions.keys]]",
        );
        let opencode = permissions_of(&manifest, &PartRef::agent("opencode"));

        let fields = |user: &str| -> Vec<String> {
            opencode
                .keyring_fields("made-up", user)
                .iter()
                .map(|field| field.as_str().to_string())
                .collect()
        };

        assert_eq!(fields("work"), ["/key", "/*/token"]);
        assert!(fields("plain").is_empty());
        assert!(fields("other").is_empty());
    }

    #[test]
    fn a_database_and_its_key_column_follow_the_declared_globs() {
        let opencode =
            permissions_of(OPENCODE_MANIFEST, &PartRef::agent("opencode"));
        let database = PathBuf::from("/data/xdg/opencode/opencode.db");

        assert!(opencode.may_query(&database));
        assert!(!opencode.may_query(Path::new("/data/xdg/other/x.db")));
        assert!(opencode.may_open_key(&KeyPlace::SqliteField {
            database: database.clone(),
            table: "credential".to_string(),
            column: "value".to_string(),
            pointer: Some("/access".to_string()),
        }));
        assert!(!opencode.may_open_key(&KeyPlace::SqliteField {
            database,
            table: "credential".to_string(),
            column: "value".to_string(),
            pointer: None,
        }));
    }
}

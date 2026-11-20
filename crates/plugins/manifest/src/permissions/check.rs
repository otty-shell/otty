//! The permissions of one part made concrete for this computer, as the
//! host operations check them on every call (spec
//! 2026-11-19-agent-plugins, Decision 7).

use std::path::{Path, PathBuf};

use globset::{GlobBuilder, GlobMatcher, GlobSet};

use crate::environment::Environment;
use crate::path_glob::{glob_set, is_plain_absolute};
use crate::permissions::declared::{
    JsonPointer, KeyRule, KeyringRule, PartPermissions, ProgramRule,
};

/// What one part may do right now, with `~` and its variables replaced
/// by this computer's folders. A call carries the permissions of the part
/// it is for only, so another part's paths and hosts are refused like any
/// path or host nobody declared.
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
        }
    }

    /// Whether the part may read the bytes of the file `path`. A key file
    /// is refused: it is read only with its keys replaced by handles.
    pub fn may_read(&self, path: &Path) -> bool {
        is_plain_absolute(path)
            && self.read.is_match(path)
            && !self.key_files.is_match(path)
    }

    /// Whether the part may write or delete the file `path`. A key file is
    /// never written.
    pub fn may_write(&self, path: &Path) -> bool {
        is_plain_absolute(path)
            && self.write.is_match(path)
            && !self.key_files.is_match(path)
    }

    /// Whether the part may query the SQLite database `path`, read-only.
    pub fn may_query(&self, path: &Path) -> bool {
        is_plain_absolute(path) && self.sqlite.is_match(path)
    }

    /// Whether the part may read the key file `path` as JSON with its key
    /// fields replaced by handles.
    pub fn may_read_with_keys(&self, path: &Path) -> bool {
        is_plain_absolute(path) && self.key_files.is_match(path)
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

    /// Whether the part may read the variable `name` as text. A variable
    /// that holds a key is read only through a key handle.
    pub fn may_read_var(&self, name: &str) -> bool {
        self.env.iter().any(|listed| listed == name)
            && !self.key_vars.iter().any(|key_var| key_var == name)
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

    /// Whether `column` of `table` in the database `database` holds keys,
    /// as a whole or in its JSON fields: such a column is read only as
    /// key handles, never as text.
    pub fn is_key_column(
        &self,
        database: &Path,
        table: &str,
        column: &str,
    ) -> bool {
        is_plain_absolute(database)
            && self.keys.iter().any(|key| match key {
                KeyMatcher::SqliteColumn {
                    databases,
                    table: key_table,
                    column: key_column,
                    ..
                } => {
                    databases.is_match(database)
                        && key_table == table
                        && key_column == column
                },
                _ => false,
            })
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
    use crate::testing::{OPENAI_MANIFEST, OPENCODE_MANIFEST, edited};

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
            "[[agent.permissions.keys]]",
            "[[agent.permissions.keyring]]\n\
             service = \"made-up\"\n\
             user = \"work\"\n\
             fields = [\"/key\", \"/*/token\"]\n\n\
             [[agent.permissions.keyring]]\n\
             service = \"made-up\"\n\
             user = \"plain\"\n\n\
             [[agent.permissions.keys]]",
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
    fn a_key_column_is_known_with_or_without_key_fields() {
        let opencode =
            permissions_of(OPENCODE_MANIFEST, &PartRef::agent("opencode"));
        let database = Path::new("/data/xdg/opencode/opencode.db");

        assert!(opencode.is_key_column(database, "credential", "value"));
        assert!(!opencode.is_key_column(database, "credential", "id"));
        assert!(!opencode.is_key_column(database, "message", "value"));
        assert!(!opencode.is_key_column(
            Path::new("/data/xdg/other/x.db"),
            "credential",
            "value"
        ));
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

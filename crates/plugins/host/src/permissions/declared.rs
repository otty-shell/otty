//! The permissions one part declares in the manifest, checked but not yet
//! made concrete for this computer.

use std::fmt;

use globset::Glob;

use crate::manifest::ManifestError;
use crate::manifest::raw::{
    RawAgentPermissions, RawKeyPlace, RawKeyring, RawProgram,
    RawProviderPermissions,
};
use crate::part::PartRef;
use crate::path_glob::{PathGlob, is_variable_name};

/// What one part of a plugin declares it may do (spec
/// 2026-11-19-agent-plugins, "Permissions"). Only the part itself gets
/// these while one of its functions runs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PartPermissions {
    read: Vec<PathGlob>,
    write: Vec<PathGlob>,
    sqlite: Vec<PathGlob>,
    env: Vec<String>,
    run: Vec<ProgramRule>,
    terminal: Vec<ProgramRule>,
    keyring: Vec<KeyringRule>,
    keys: Vec<KeyRule>,
    http: Vec<String>,
}

impl PartPermissions {
    /// The files the part reads.
    pub fn read(&self) -> &[PathGlob] {
        &self.read
    }

    /// The files the part writes or deletes.
    pub fn write(&self) -> &[PathGlob] {
        &self.write
    }

    /// The SQLite databases the part queries, read-only.
    pub fn sqlite(&self) -> &[PathGlob] {
        &self.sqlite
    }

    /// The environment variables the part reads as text and starts its
    /// globs with.
    pub fn env(&self) -> &[String] {
        &self.env
    }

    /// The programs the part runs, with their arguments.
    pub fn run(&self) -> &[ProgramRule] {
        &self.run
    }

    /// The programs the part starts in a hidden terminal.
    pub fn terminal(&self) -> &[ProgramRule] {
        &self.terminal
    }

    /// The system keyring entries the part reads, keys as handles.
    pub fn keyring(&self) -> &[KeyringRule] {
        &self.keyring
    }

    /// Where the part's keys are: readable only as key handles.
    pub fn keys(&self) -> &[KeyRule] {
        &self.keys
    }

    /// The hosts the part sends HTTPS requests to.
    pub fn http(&self) -> &[String] {
        &self.http
    }

    /// The checked permissions of the agent part `part`.
    pub(crate) fn of_agent(
        raw: &RawAgentPermissions,
        part: &PartRef,
    ) -> Result<Self, ManifestError> {
        for name in &raw.env {
            check_variable(name, part)?;
        }

        let globs = |texts: &[String]| path_globs(texts, &raw.env, part);
        let keys = raw
            .keys
            .iter()
            .map(|place| KeyRule::of_raw(place, &raw.env, part))
            .collect::<Result<_, _>>()?;

        Ok(Self {
            read: globs(&raw.read)?,
            write: globs(&raw.write)?,
            sqlite: globs(&raw.sqlite)?,
            env: raw.env.clone(),
            run: program_rules(&raw.run, part)?,
            terminal: program_rules(&raw.terminal, part)?,
            keyring: keyring_rules(&raw.keyring, part)?,
            keys,
            http: Vec::new(),
        })
    }

    /// The checked permissions of the provider part `part`.
    pub(crate) fn of_provider(
        raw: &RawProviderPermissions,
        part: &PartRef,
    ) -> Result<Self, ManifestError> {
        for host in &raw.http {
            check_host(host, part)?;
        }

        Ok(Self {
            http: raw.http.clone(),
            ..Self::default()
        })
    }
}

/// A program a part may run, and the arguments it may pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramRule {
    program: String,
    args: Vec<String>,
}

impl ProgramRule {
    /// The program, looked up on `PATH`.
    pub fn program(&self) -> &str {
        &self.program
    }

    /// One pattern per argument: the text itself, where `*` stands for
    /// any text and `?` for one character.
    pub fn args(&self) -> &[String] {
        &self.args
    }

    fn of_raw(raw: &RawProgram, part: &PartRef) -> Result<Self, ManifestError> {
        let bad_part = |reason: String| ManifestError::BadPart {
            part: part.clone(),
            reason,
        };

        if raw.program.trim().is_empty() {
            return Err(bad_part("runs a program with no name".to_string()));
        }
        for arg in &raw.args {
            if Glob::new(arg).is_err() {
                return Err(bad_part(format!(
                    "runs `{}` with the argument pattern `{arg}`, which is \
                     not valid",
                    raw.program
                )));
            }
        }

        Ok(Self {
            program: raw.program.clone(),
            args: raw.args.clone(),
        })
    }
}

impl fmt::Display for ProgramRule {
    /// Writes the command line, such as `codex --version`.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.program)?;
        for arg in &self.args {
            write!(formatter, " {arg}")?;
        }

        Ok(())
    }
}

/// A system keyring entry a part may read, with the JSON fields of the
/// entry that hold keys; no fields means the whole entry is a key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyringRule {
    service: String,
    user: String,
    fields: Vec<JsonPointer>,
}

impl KeyringRule {
    /// The keyring service.
    pub fn service(&self) -> &str {
        &self.service
    }

    /// The keyring user.
    pub fn user(&self) -> &str {
        &self.user
    }

    /// The JSON fields of the entry that hold keys.
    pub fn fields(&self) -> &[JsonPointer] {
        &self.fields
    }
}

/// Where a part's keys are. These places are readable only with the keys
/// replaced by key handles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyRule {
    /// JSON fields of files, such as `/tokens/access_token` of
    /// `~/.codex*/auth.json`.
    JsonFile {
        /// The files.
        files: Vec<PathGlob>,
        /// The fields that hold keys.
        fields: Vec<JsonPointer>,
    },
    /// A column of a SQLite table; with fields, the column holds JSON and
    /// those fields hold keys, else the column itself is the key.
    SqliteColumn {
        /// The databases.
        databases: Vec<PathGlob>,
        /// The table.
        table: String,
        /// The column.
        column: String,
        /// The JSON fields of the column that hold keys.
        fields: Vec<JsonPointer>,
    },
    /// Environment variables that hold keys.
    EnvVars {
        /// The variables.
        names: Vec<String>,
    },
}

impl KeyRule {
    fn of_raw(
        raw: &RawKeyPlace,
        listed_vars: &[String],
        part: &PartRef,
    ) -> Result<Self, ManifestError> {
        let bad_part = |reason: &str| ManifestError::BadPart {
            part: part.clone(),
            reason: reason.to_string(),
        };
        let fields =
            || json_pointers(raw.fields.as_deref().unwrap_or(&[]), part);

        match (&raw.files, &raw.sqlite, &raw.env) {
            (Some(files), None, None) => {
                if raw.table.is_some() || raw.column.is_some() {
                    return Err(bad_part(
                        "gives a table or a column to a key place in files",
                    ));
                }
                let fields = fields()?;
                if fields.is_empty() {
                    return Err(bad_part(
                        "lists key files with no `fields` holding the keys",
                    ));
                }

                Ok(KeyRule::JsonFile {
                    files: path_globs(files, listed_vars, part)?,
                    fields,
                })
            },
            (None, Some(databases), None) => {
                let (Some(table), Some(column)) = (&raw.table, &raw.column)
                else {
                    return Err(bad_part(
                        "lists key databases with no `table` and `column`",
                    ));
                };

                Ok(KeyRule::SqliteColumn {
                    databases: path_globs(databases, listed_vars, part)?,
                    table: table.clone(),
                    column: column.clone(),
                    fields: fields()?,
                })
            },
            (None, None, Some(names)) => {
                if raw.fields.is_some()
                    || raw.table.is_some()
                    || raw.column.is_some()
                {
                    return Err(bad_part(
                        "gives fields, a table or a column to key variables",
                    ));
                }
                for name in names {
                    check_variable(name, part)?;
                }

                Ok(KeyRule::EnvVars {
                    names: names.clone(),
                })
            },
            _ => Err(bad_part(
                "has a key place that is not exactly one of `files`, \
                 `sqlite` or `env`",
            )),
        }
    }
}

/// A JSON pointer such as `/tokens/access_token`, where a `*` part stands
/// for any one key, as in `/*/key`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonPointer {
    text: String,
}

impl JsonPointer {
    /// The pointer as written.
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// Whether the concrete pointer `pointer` is one this pattern names.
    pub(crate) fn matches(&self, pointer: &str) -> bool {
        let wanted: Vec<&str> = self.text.split('/').collect();
        let given: Vec<&str> = pointer.split('/').collect();

        wanted.len() == given.len()
            && wanted
                .iter()
                .zip(&given)
                .all(|(wanted, given)| *wanted == "*" || wanted == given)
    }
}

impl fmt::Display for JsonPointer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.text)
    }
}

fn path_globs(
    texts: &[String],
    listed_vars: &[String],
    part: &PartRef,
) -> Result<Vec<PathGlob>, ManifestError> {
    texts
        .iter()
        .map(|text| {
            PathGlob::parse(text, listed_vars).map_err(|problem| {
                ManifestError::BadGlob {
                    part: part.clone(),
                    glob: text.clone(),
                    problem,
                }
            })
        })
        .collect()
}

fn program_rules(
    raw: &[RawProgram],
    part: &PartRef,
) -> Result<Vec<ProgramRule>, ManifestError> {
    raw.iter()
        .map(|program| ProgramRule::of_raw(program, part))
        .collect()
}

fn keyring_rules(
    raw: &[RawKeyring],
    part: &PartRef,
) -> Result<Vec<KeyringRule>, ManifestError> {
    raw.iter()
        .map(|entry| {
            if entry.service.is_empty() || entry.user.is_empty() {
                return Err(ManifestError::BadPart {
                    part: part.clone(),
                    reason: "reads a keyring entry with no service or user"
                        .to_string(),
                });
            }

            Ok(KeyringRule {
                service: entry.service.clone(),
                user: entry.user.clone(),
                fields: json_pointers(&entry.fields, part)?,
            })
        })
        .collect()
}

fn json_pointers(
    texts: &[String],
    part: &PartRef,
) -> Result<Vec<JsonPointer>, ManifestError> {
    texts
        .iter()
        .map(|text| {
            if !text.starts_with('/') {
                return Err(ManifestError::BadPart {
                    part: part.clone(),
                    reason: format!(
                        "names the key field `{text}`, which is not a JSON \
                         pointer such as /tokens/access_token"
                    ),
                });
            }

            Ok(JsonPointer { text: text.clone() })
        })
        .collect()
}

fn check_variable(name: &str, part: &PartRef) -> Result<(), ManifestError> {
    if is_variable_name(name) {
        return Ok(());
    }

    Err(ManifestError::BadPart {
        part: part.clone(),
        reason: format!("names `{name}`, which is not a variable name"),
    })
}

/// Checks that `host` is a lowercase host name such as `api.openai.com`.
pub(crate) fn check_host(
    host: &str,
    part: &PartRef,
) -> Result<(), ManifestError> {
    let label_ok = |label: &str| {
        !label.is_empty()
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label.bytes().all(|byte| {
                byte.is_ascii_lowercase()
                    || byte.is_ascii_digit()
                    || byte == b'-'
            })
    };

    if host.split('.').all(label_ok) {
        return Ok(());
    }

    Err(ManifestError::BadHost {
        part: part.clone(),
        host: host.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pointer(text: &str) -> JsonPointer {
        JsonPointer {
            text: text.to_string(),
        }
    }

    #[test]
    fn a_star_in_a_pointer_stands_for_any_one_key() {
        assert!(pointer("/*/key").matches("/openai/key"));
        assert!(!pointer("/*/key").matches("/openai/x/key"));
        assert!(!pointer("/*/key").matches("/openai/access"));
        assert!(pointer("/tokens/id_token").matches("/tokens/id_token"));
    }

    #[test]
    fn a_program_rule_reads_as_its_command_line() {
        let rule = ProgramRule {
            program: "codex".to_string(),
            args: vec!["--version".to_string()],
        };

        assert_eq!(rule.to_string(), "codex --version");
    }

    #[test]
    fn hosts_are_lowercase_names() {
        let part = PartRef::provider("openai");

        assert!(check_host("api.openai.com", &part).is_ok());
        for host in ["", "API.openai.com", "https://openai.com", "a..b", "-a.b"]
        {
            assert!(check_host(host, &part).is_err(), "{host}");
        }
    }
}

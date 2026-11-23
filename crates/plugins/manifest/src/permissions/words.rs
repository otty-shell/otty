//! The permissions of a part in plain words, line by line, as the install
//! dialog and the plugin's page show them (spec 2026-11-19-agent-plugins,
//! "Permissions": "How it is shown").

use std::fmt;

use crate::permissions::declared::{KeyRule, LoginRule, PartPermissions};

/// One permission of a part. Its text is the plain English line; the UI
/// may also build its own words from the values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionLine {
    /// Reads files matching a glob.
    ReadsFiles(String),
    /// Writes and deletes files matching a glob.
    WritesFiles(String),
    /// Queries SQLite databases matching a glob, read-only.
    QueriesDatabases(String),
    /// Reads an environment variable as text.
    ReadsVariable(String),
    /// Runs a program with arguments, given as its command line.
    RunsProgram(String),
    /// Starts a program in a hidden terminal, given as its command line.
    StartsHiddenTerminal(String),
    /// Reads an entry of the system keyring, keys as handles.
    ReadsKeyring {
        /// The keyring service.
        service: String,
        /// The keyring user.
        user: String,
    },
    /// Reads logins in files matching a glob, as handles.
    ReadsLoginsInFiles(String),
    /// Reads logins in a column of SQLite databases matching a glob, as
    /// handles.
    ReadsLoginsInDatabase {
        /// The databases.
        databases: String,
        /// The table.
        table: String,
        /// The column.
        column: String,
    },
    /// Reads a key in an environment variable, as a handle.
    ReadsKeyInVariable(String),
    /// Sends HTTPS requests to a host.
    SendsRequests(String),
    /// Reads files matching a glob inside each profile folder of an agent
    /// part of the plugin.
    ReadsInProfiles {
        /// The name of the agent part, such as `Codex`.
        agent: String,
        /// The glob inside the profile folder.
        glob: String,
    },
    /// Reads logins in files matching a glob inside each profile folder of
    /// an agent part of the plugin, as handles.
    ReadsLoginsInProfiles {
        /// The name of the agent part, such as `Codex`.
        agent: String,
        /// The glob inside the profile folder.
        glob: String,
    },
    /// Gets the logins and keys of the provider's accounts, as handles.
    GetsProviderKeys(String),
}

impl fmt::Display for PermissionLine {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PermissionLine::ReadsFiles(glob) => {
                write!(formatter, "Reads files in `{glob}`")
            },
            PermissionLine::WritesFiles(glob) => {
                write!(formatter, "Writes and deletes files in `{glob}`")
            },
            PermissionLine::QueriesDatabases(glob) => {
                write!(formatter, "Reads the SQLite databases `{glob}`")
            },
            PermissionLine::ReadsVariable(name) => {
                write!(formatter, "Reads the environment variable `{name}`")
            },
            PermissionLine::RunsProgram(command) => {
                write!(formatter, "Runs `{command}`")
            },
            PermissionLine::StartsHiddenTerminal(command) => {
                write!(formatter, "Runs `{command}` in a hidden terminal")
            },
            PermissionLine::ReadsKeyring { service, user } => write!(
                formatter,
                "Reads the keyring entry `{service}` of `{user}` once you \
                 allow the keyring (OTTY gives its keys only as handles)"
            ),
            PermissionLine::ReadsLoginsInFiles(glob) => write!(
                formatter,
                "Reads your login in `{glob}` (OTTY gives it only as a handle)"
            ),
            PermissionLine::ReadsLoginsInDatabase {
                databases,
                table,
                column,
            } => write!(
                formatter,
                "Reads your login in the column `{column}` of `{table}` in \
                 `{databases}` (OTTY gives it only as a handle)"
            ),
            PermissionLine::ReadsKeyInVariable(name) => write!(
                formatter,
                "Reads your key in the variable `{name}` (OTTY gives it only \
                 as a handle)"
            ),
            PermissionLine::SendsRequests(host) => {
                write!(formatter, "Sends requests to `{host}`")
            },
            PermissionLine::ReadsInProfiles { agent, glob } => write!(
                formatter,
                "Reads files in `{glob}` in each {agent} profile"
            ),
            PermissionLine::ReadsLoginsInProfiles { agent, glob } => write!(
                formatter,
                "Reads your login in `{glob}` in each {agent} profile (OTTY \
                 gives it only as a handle)"
            ),
            PermissionLine::GetsProviderKeys(provider) => write!(
                formatter,
                "Gets the logins and keys of {provider} accounts found in \
                 your agents, as handles"
            ),
        }
    }
}

impl PartPermissions {
    /// The permissions in plain words, one line each, in the order of the
    /// manifest's fields.
    pub fn lines(&self) -> Vec<PermissionLine> {
        let mut lines = Vec::new();

        for glob in self.read() {
            lines.push(PermissionLine::ReadsFiles(glob.to_string()));
        }
        for glob in self.write() {
            lines.push(PermissionLine::WritesFiles(glob.to_string()));
        }
        for glob in self.sqlite() {
            lines.push(PermissionLine::QueriesDatabases(glob.to_string()));
        }
        for name in self.env() {
            lines.push(PermissionLine::ReadsVariable(name.clone()));
        }
        for rule in self.run() {
            lines.push(PermissionLine::RunsProgram(rule.to_string()));
        }
        for rule in self.terminal() {
            lines.push(PermissionLine::StartsHiddenTerminal(rule.to_string()));
        }
        for rule in self.keyring() {
            lines.push(PermissionLine::ReadsKeyring {
                service: rule.service().to_string(),
                user: rule.user().to_string(),
            });
        }
        for rule in self.keys() {
            lines.extend(key_lines(rule));
        }
        for host in self.http() {
            lines.push(PermissionLine::SendsRequests(host.clone()));
        }
        for rule in self.logins() {
            lines.extend(login_lines(rule));
        }

        lines
    }
}

fn login_lines(rule: &LoginRule) -> Vec<PermissionLine> {
    let agent = rule.agent_name();
    let logins = rule
        .keys()
        .iter()
        .flat_map(|keys| keys.files())
        .map(|glob| PermissionLine::ReadsLoginsInProfiles {
            agent: agent.to_string(),
            glob: glob.to_string(),
        });
    let read = rule
        .read()
        .iter()
        .map(|glob| PermissionLine::ReadsInProfiles {
            agent: agent.to_string(),
            glob: glob.to_string(),
        });

    logins.chain(read).collect()
}

fn key_lines(rule: &KeyRule) -> Vec<PermissionLine> {
    match rule {
        KeyRule::JsonFile { files, .. } => files
            .iter()
            .map(|glob| PermissionLine::ReadsLoginsInFiles(glob.to_string()))
            .collect(),
        KeyRule::SqliteColumn {
            databases,
            table,
            column,
            ..
        } => databases
            .iter()
            .map(|glob| PermissionLine::ReadsLoginsInDatabase {
                databases: glob.to_string(),
                table: table.clone(),
                column: column.clone(),
            })
            .collect(),
        KeyRule::EnvVars { names } => names
            .iter()
            .map(|name| PermissionLine::ReadsKeyInVariable(name.clone()))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use crate::manifest::Manifest;
    use crate::testing::{OPENAI_MANIFEST, with_codex_logins};

    fn texts(lines: Vec<super::PermissionLine>) -> Vec<String> {
        lines.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn an_agent_part_lists_its_permissions_in_plain_words() {
        let manifest = Manifest::parse(OPENAI_MANIFEST).expect("valid");

        assert_eq!(
            texts(manifest.agents()[0].permission_lines()),
            [
                "Reads files in `~/.codex*/**`",
                "Reads files in `{CODEX_HOME}/**`",
                "Reads the environment variable `CODEX_HOME`",
                "Runs `codex --version`",
                "Runs `codex -s read-only -a never app-server`",
                "Reads your login in `~/.codex*/auth.json` (OTTY gives it only \
             as a handle)",
                "Reads your login in `{CODEX_HOME}/auth.json` (OTTY gives it \
             only as a handle)",
            ]
        );
    }

    #[test]
    fn a_provider_part_lists_its_hosts_and_the_keys_it_gets() {
        let manifest = Manifest::parse(OPENAI_MANIFEST).expect("valid");

        assert_eq!(
            texts(manifest.providers()[0].permission_lines()),
            [
                "Sends requests to `chatgpt.com`",
                "Sends requests to `api.openai.com`",
                "Gets the logins and keys of OpenAI accounts found in your \
             agents, as handles",
            ]
        );
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decision 5: the
    /// files a provider part reads inside each profile folder of an agent
    /// part of its plugin, the logins as handles.
    #[test]
    fn a_provider_part_lists_what_it_reads_in_profiles() {
        let manifest = Manifest::parse(&with_codex_logins()).expect("valid");

        assert_eq!(
            texts(manifest.providers()[0].permission_lines()),
            [
                "Sends requests to `chatgpt.com`",
                "Sends requests to `api.openai.com`",
                "Reads your login in `auth.json` in each Codex profile (OTTY \
                 gives it only as a handle)",
                "Reads files in `sessions/**` in each Codex profile",
                "Gets the logins and keys of OpenAI accounts found in your \
                 agents, as handles",
            ]
        );
    }
}

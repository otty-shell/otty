//! Test-only values shared by the tests of this crate: the manifests of
//! the spec's plugins.

/// The manifest of the spec's OpenAI plugin: one agent part and one
/// provider part.
pub(crate) const OPENAI_MANIFEST: &str = r#"
[plugin]
id = "openai"
name = "OpenAI"
version = "0.4.0"
description = "Codex sessions, tokens and limits; ChatGPT and OpenAI account numbers."
repository = "https://github.com/otty-shell/otty-plugin-openai"
license = "Apache-2.0"
authors = ["OTTY team"]
wasm = "openai.wasm"

[[agents.agent]]
id = "codex"
name = "Codex"
icon = "codex.svg"
program = "codex"
tested_version = "0.169.0"

[agents.agent.permissions]
read = ["~/.codex*/**", "{CODEX_HOME}/**"]
env = ["CODEX_HOME"]

[[agents.agent.permissions.run]]
program = "codex"
args = ["--version"]

[[agents.agent.permissions.run]]
program = "codex"
args = ["-s", "read-only", "-a", "never", "app-server"]

[[agents.agent.permissions.keys]]
files = ["~/.codex*/auth.json", "{CODEX_HOME}/auth.json"]
fields = ["/OPENAI_API_KEY", "/tokens/access_token"]

[[agents.provider]]
id = "openai"
name = "OpenAI"
icon = "openai.svg"
hosts = ["openai.com", "chatgpt.com"]
usage_page = "https://platform.openai.com/usage"

[agents.provider.permissions]
http = ["chatgpt.com", "api.openai.com"]
"#;

/// The manifest of the spec's OpenCode plugin: one agent part with a
/// database and a key column.
pub(crate) const OPENCODE_MANIFEST: &str = r#"
[plugin]
id = "opencode"
name = "OpenCode"
version = "0.2.0"
wasm = "opencode.wasm"

[[agents.agent]]
id = "opencode"
name = "OpenCode"
icon = "opencode.svg"
program = "opencode"

[agents.agent.permissions]
read = ["~/.local/share/opencode/**", "{XDG_DATA_HOME}/opencode/**"]
sqlite = ["~/.local/share/opencode/*.db", "{XDG_DATA_HOME}/opencode/*.db"]
env = ["XDG_DATA_HOME"]

[[agents.agent.permissions.keys]]
sqlite = ["~/.local/share/opencode/*.db", "{XDG_DATA_HOME}/opencode/*.db"]
table = "credential"
column = "value"
fields = ["/key", "/access", "/refresh"]
"#;

/// A `logins` entry of the provider part of [`OPENAI_MANIFEST`]: inside
/// each Codex profile folder it reads `sessions/**`, and the keys of
/// `auth.json` as handles, and points `codex` at the folder with
/// `CODEX_HOME`.
pub(crate) const CODEX_LOGINS: &str = r#"
[[agents.provider.logins]]
agent = "codex"
folder_env = "CODEX_HOME"
read = ["sessions/**"]

[[agents.provider.logins.keys]]
files = ["auth.json"]
fields = ["/OPENAI_API_KEY", "/tokens/access_token"]
"#;

/// [`OPENAI_MANIFEST`] whose provider part reads the logins of Codex
/// profiles, [`CODEX_LOGINS`].
pub(crate) fn with_codex_logins() -> String {
    format!("{OPENAI_MANIFEST}{CODEX_LOGINS}")
}

/// `text` with the first `from` replaced by `to`; `from` must be there.
pub(crate) fn edited(text: &str, from: &str, to: &str) -> String {
    assert!(text.contains(from), "the text holds {from:?}");

    text.replacen(from, to, 1)
}

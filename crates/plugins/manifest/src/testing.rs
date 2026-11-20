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

[api]
agent = "0.1"
provider = "0.1"

[[agent]]
id = "codex"
name = "Codex"
icon = "codex.svg"
program = "codex"
tested_version = "0.169.0"

[agent.permissions]
read = ["~/.codex*/**", "{CODEX_HOME}/**"]
env = ["CODEX_HOME"]

[[agent.permissions.run]]
program = "codex"
args = ["--version"]

[[agent.permissions.run]]
program = "codex"
args = ["-s", "read-only", "-a", "never", "app-server"]

[[agent.permissions.keys]]
files = ["~/.codex*/auth.json", "{CODEX_HOME}/auth.json"]
fields = ["/OPENAI_API_KEY", "/tokens/access_token"]

[[provider]]
id = "openai"
name = "OpenAI"
icon = "openai.svg"
hosts = ["openai.com", "chatgpt.com"]
usage_page = "https://platform.openai.com/usage"
add_by_hand = "admin-key"

[provider.permissions]
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

[api]
agent = "0.1"

[[agent]]
id = "opencode"
name = "OpenCode"
icon = "opencode.svg"
program = "opencode"

[agent.permissions]
read = ["~/.local/share/opencode/**", "{XDG_DATA_HOME}/opencode/**"]
sqlite = ["~/.local/share/opencode/*.db", "{XDG_DATA_HOME}/opencode/*.db"]
env = ["XDG_DATA_HOME"]

[[agent.permissions.keys]]
sqlite = ["~/.local/share/opencode/*.db", "{XDG_DATA_HOME}/opencode/*.db"]
table = "credential"
column = "value"
fields = ["/key", "/access", "/refresh"]
"#;

/// `text` with the first `from` replaced by `to`; `from` must be there.
pub(crate) fn edited(text: &str, from: &str, to: &str) -> String {
    assert!(text.contains(from), "the text holds {from:?}");

    text.replacen(from, to, 1)
}

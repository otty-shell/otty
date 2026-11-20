//! The plugins built into the OTTY binary: copies of the official plugins
//! from `assets/plugins/<id>/`, each with its `SHA256SUMS` (spec
//! 2026-11-19-agent-plugins, Decision 10, "Embedded plugins").

use std::borrow::Cow;
use std::collections::BTreeMap;

use crate::error::InstallError;
use crate::package::{MANIFEST_FILE, sha256_hex};

/// The name of the file that lists the SHA-256 of every other file of a
/// built-in copy, as `sha256sum` writes it.
pub const SUMS_FILE: &str = "SHA256SUMS";

/// One plugin built into OTTY: its files and their hashes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddedPlugin {
    id: &'static str,
    sums: Cow<'static, str>,
    files: BTreeMap<String, Cow<'static, [u8]>>,
    releases: Option<&'static str>,
}

impl EmbeddedPlugin {
    /// The plugin id, the name of its folder in `assets/plugins/`.
    pub fn id(&self) -> &str {
        self.id
    }

    /// The repository whose releases update the plugin once it has one;
    /// `None` while it lives in OTTY's repository and a newer OTTY brings
    /// its updates.
    pub fn releases(&self) -> Option<&str> {
        self.releases
    }

    /// The plugin `id` with the `SHA256SUMS` text `sums` and no files yet.
    pub fn new(id: &'static str, sums: impl Into<Cow<'static, str>>) -> Self {
        Self {
            id,
            sums: sums.into(),
            files: BTreeMap::new(),
            releases: None,
        }
    }

    /// The same plugin with the file `name`.
    pub fn with_file(
        mut self,
        name: &str,
        bytes: impl Into<Cow<'static, [u8]>>,
    ) -> Self {
        self.files.insert(name.to_string(), bytes.into());

        self
    }

    /// The same plugin, updated from the releases of `repository` from now
    /// on.
    pub fn with_releases(mut self, repository: &'static str) -> Self {
        self.releases = Some(repository);

        self
    }

    /// The files, checked against `SHA256SUMS`: every file is listed with
    /// its hash and every listed file is there.
    pub(crate) fn checked_files(
        &self,
    ) -> Result<BTreeMap<String, Vec<u8>>, InstallError> {
        let damaged = || InstallError::DamagedBuiltIn(self.id.to_string());

        let mut listed = BTreeMap::new();
        for line in self.sums.lines().filter(|line| !line.trim().is_empty()) {
            let Some((hash, name)) = line.split_once(char::is_whitespace)
            else {
                return Err(damaged());
            };
            let name = name.trim_start().trim_start_matches('*');
            listed.insert(name.to_string(), hash.to_ascii_lowercase());
        }

        if listed.len() != self.files.len() {
            return Err(damaged());
        }
        let mut files = BTreeMap::new();
        for (name, bytes) in &self.files {
            if listed.get(name) != Some(&sha256_hex(bytes)) {
                return Err(damaged());
            }
            files.insert(name.clone(), bytes.to_vec());
        }

        Ok(files)
    }
}

/// The plugins built into this OTTY, installed on first start.
///
/// All five official plugins are built in (spec Decision 10). Each copy in
/// `assets/plugins/<id>/` is made by `plugins/embed.sh <folder>` from
/// `plugins/<folder>/`, and listed here as
/// `EmbeddedPlugin::new(id, include_str!(".../SHA256SUMS"))` with one
/// `with_file(name, include_bytes!(...))` per file of the copy.
pub fn embedded_plugins() -> Vec<EmbeddedPlugin> {
    vec![anthropic(), openai(), opencode(), deepseek(), openrouter()]
}

/// The Anthropic plugin: the agent part Claude Code and the provider part
/// Anthropic.
fn anthropic() -> EmbeddedPlugin {
    EmbeddedPlugin::new(
        "anthropic",
        include_str!("../../../../assets/plugins/anthropic/SHA256SUMS"),
    )
    .with_file(
        MANIFEST_FILE,
        include_bytes!("../../../../assets/plugins/anthropic/otty-plugin.toml")
            .as_slice(),
    )
    .with_file(
        "anthropic.wasm",
        include_bytes!("../../../../assets/plugins/anthropic/anthropic.wasm")
            .as_slice(),
    )
    .with_file(
        "anthropic.svg",
        include_bytes!("../../../../assets/plugins/anthropic/anthropic.svg")
            .as_slice(),
    )
    .with_file(
        "claude-code.svg",
        include_bytes!("../../../../assets/plugins/anthropic/claude-code.svg")
            .as_slice(),
    )
}

/// The OpenAI plugin: the agent part Codex and the provider part OpenAI.
fn openai() -> EmbeddedPlugin {
    EmbeddedPlugin::new(
        "openai",
        include_str!("../../../../assets/plugins/openai/SHA256SUMS"),
    )
    .with_file(
        MANIFEST_FILE,
        include_bytes!("../../../../assets/plugins/openai/otty-plugin.toml")
            .as_slice(),
    )
    .with_file(
        "openai.wasm",
        include_bytes!("../../../../assets/plugins/openai/openai.wasm")
            .as_slice(),
    )
    .with_file(
        "codex.svg",
        include_bytes!("../../../../assets/plugins/openai/codex.svg")
            .as_slice(),
    )
    .with_file(
        "openai.svg",
        include_bytes!("../../../../assets/plugins/openai/openai.svg")
            .as_slice(),
    )
}

/// The OpenCode plugin: the agent part OpenCode.
fn opencode() -> EmbeddedPlugin {
    EmbeddedPlugin::new(
        "opencode",
        include_str!("../../../../assets/plugins/opencode/SHA256SUMS"),
    )
    .with_file(
        MANIFEST_FILE,
        include_bytes!("../../../../assets/plugins/opencode/otty-plugin.toml")
            .as_slice(),
    )
    .with_file(
        "opencode.wasm",
        include_bytes!("../../../../assets/plugins/opencode/opencode.wasm")
            .as_slice(),
    )
    .with_file(
        "opencode.svg",
        include_bytes!("../../../../assets/plugins/opencode/opencode.svg")
            .as_slice(),
    )
}

/// The DeepSeek plugin: the provider part DeepSeek.
fn deepseek() -> EmbeddedPlugin {
    EmbeddedPlugin::new(
        "deepseek",
        include_str!("../../../../assets/plugins/deepseek/SHA256SUMS"),
    )
    .with_file(
        MANIFEST_FILE,
        include_bytes!("../../../../assets/plugins/deepseek/otty-plugin.toml")
            .as_slice(),
    )
    .with_file(
        "deepseek.wasm",
        include_bytes!("../../../../assets/plugins/deepseek/deepseek.wasm")
            .as_slice(),
    )
    .with_file(
        "deepseek.svg",
        include_bytes!("../../../../assets/plugins/deepseek/deepseek.svg")
            .as_slice(),
    )
}

/// The OpenRouter plugin: the provider part OpenRouter.
fn openrouter() -> EmbeddedPlugin {
    EmbeddedPlugin::new(
        "openrouter",
        include_str!("../../../../assets/plugins/openrouter/SHA256SUMS"),
    )
    .with_file(
        MANIFEST_FILE,
        include_bytes!(
            "../../../../assets/plugins/openrouter/otty-plugin.toml"
        )
        .as_slice(),
    )
    .with_file(
        "openrouter.wasm",
        include_bytes!("../../../../assets/plugins/openrouter/openrouter.wasm")
            .as_slice(),
    )
    .with_file(
        "openrouter.svg",
        include_bytes!("../../../../assets/plugins/openrouter/openrouter.svg")
            .as_slice(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::embedded_example;

    #[test]
    fn the_test_copy_of_the_template_matches_its_hashes() {
        let files = embedded_example().checked_files().expect("whole");

        assert_eq!(
            files.keys().map(String::as_str).collect::<Vec<_>>(),
            [
                "example-agent.svg",
                "example-cloud.svg",
                "example.wasm",
                "otty-plugin.toml"
            ]
        );
    }

    /// Every copy built into OTTY is whole and holds its manifest, so a
    /// copy left out of date by `plugins/embed.sh` fails here.
    #[test]
    fn every_built_in_copy_matches_its_hashes() {
        let plugins = embedded_plugins();

        let ids: Vec<&str> = plugins.iter().map(EmbeddedPlugin::id).collect();
        assert_eq!(
            ids,
            ["anthropic", "openai", "opencode", "deepseek", "openrouter"]
        );
        for plugin in &plugins {
            let files = plugin.checked_files().expect("whole");
            assert!(files.contains_key(MANIFEST_FILE), "{}", plugin.id());
            assert!(
                files.contains_key(&format!("{}.wasm", plugin.id())),
                "{}",
                plugin.id()
            );
        }
    }

    #[test]
    fn a_changed_file_is_damage() {
        let damaged =
            embedded_example().with_file("example.wasm", b"other".as_slice());

        assert_eq!(
            damaged.checked_files(),
            Err(InstallError::DamagedBuiltIn("example".to_string()))
        );
    }

    #[test]
    fn a_file_missing_from_the_sums_is_damage() {
        let extra =
            embedded_example().with_file("extra.svg", b"<svg/>".as_slice());

        assert_eq!(
            extra.checked_files(),
            Err(InstallError::DamagedBuiltIn("example".to_string()))
        );
    }
}

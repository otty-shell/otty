//! Why an install, an update or a change of a plugin did not happen, in
//! plain words for Plugins mode.

use otty_plugin_manifest::{ManifestError, PartRef};

use crate::icon::IconProblem;

/// Why an install, an update, a reload or a change was refused. The text
/// of each value is a plain reasin Plugins mode can show as it is.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InstallError {
    /// The pasted text is not a GitHub repository link.
    #[error(
        "`{0}` is not a GitHub repository link such as \
         https://github.com/owner/repository"
    )]
    BadUrl(String),
    /// GitHub refuses more requests without a login for now.
    #[error(
        "GitHub refuses more requests from this computer for now. Try again \
         in an hour."
    )]
    RateLimited,
    /// The repository is private or does not exist.
    #[error("The repository {0} is private or does not exist")]
    RepositoryNotFound(String),
    /// GitHub could not be reached.
    #[error("OTTY could not reach GitHub: {0}")]
    Network(String),
    /// GitHub answered something OTTY does not understand.
    #[error("GitHub answered something unexpected: {0}")]
    UnexpectedAnswer(String),
    /// No release has a semver tag and a manifest.
    #[error("This repository has no release OTTY can install")]
    NoRelease,
    /// The `.wasm` exports a contract version this OTTY does not know:
    /// that of every release, or that of a folder.
    #[error("{0}")]
    NeedsNewerOtty(String),
    /// The manifest is refused.
    #[error("{0}")]
    Manifest(#[from] ManifestError),
    /// The manifest's version is not the release tag.
    #[error(
        "The release {tag} holds the plugin version {version}; they must be \
         the same"
    )]
    VersionIsNotTag {
        /// The release tag.
        tag: String,
        /// The manifest's version.
        version: String,
    },
    /// A file the manifest names is not in the release or the folder.
    #[error("The plugin has no file `{0}`")]
    MissingFile(String),
    /// A file could not be downloaded.
    #[error("The file `{name}` could not be downloaded: {reason}")]
    Download {
        /// The file name.
        name: String,
        /// Why, in plain words.
        reason: String,
    },
    /// A file is larger than OTTY allows.
    #[error("The file `{name}` is larger than {limit} bytes")]
    TooLarge {
        /// The file name.
        name: String,
        /// The limit in bytes.
        limit: u64,
    },
    /// An icon is refused.
    #[error("The icon `{name}` {problem}")]
    BadIcon {
        /// The icon's file name.
        name: String,
        /// Why it is refused.
        problem: IconProblem,
    },
    /// Another plugin with this id is installed from elsewhere.
    #[error("A plugin `{id}` is already installed from {origin}")]
    IdTaken {
        /// The plugin id.
        id: String,
        /// Where the installed one came from, in words.
        origin: String,
    },
    /// The plugin is already installed from the same place.
    #[error("The plugin `{0}` is already installed")]
    AlreadyInstalled(String),
    /// A release tag OTTY installed before now serves other files.
    #[error(
        "The release {tag} of {name} changed after it was installed. OTTY \
         keeps the installed copy."
    )]
    ReleaseChanged {
        /// The release tag.
        tag: String,
        /// The plugin's name.
        name: String,
    },
    /// A newer copy has another plugin id.
    #[error("The new copy is the plugin `{found}`, not `{expected}`")]
    OtherPlugin {
        /// The id of the installed plugin.
        expected: String,
        /// The id the new copy has.
        found: String,
    },
    /// The `.wasm` does not match the manifest or does not compile.
    #[error("{0}")]
    Compile(String),
    /// A file or folder could not be read or written.
    #[error("{0}")]
    Disk(String),
    /// No installed plugin has this id.
    #[error("No plugin `{0}` is installed")]
    NotInstalled(String),
    /// The plugin has no such part.
    #[error("The plugin `{plugin}` has no {part}")]
    NoSuchPart {
        /// The plugin id.
        plugin: String,
        /// The part.
        part: PartRef,
    },
    /// Reload is only for plugins installed from a folder.
    #[error("The plugin `{0}` was not installed from a folder")]
    NotLocal(String),
}

impl InstallError {
    /// A disk error, with what OTTY was doing.
    pub(crate) fn disk(doing: &str, error: &std::io::Error) -> Self {
        Self::Disk(format!("OTTY could not {doing}: {error}"))
    }
}

//! Why a manifest is refused, in plain words for the install dialog.

use crate::part::{PartKind, PartRef};
use crate::path_glob::GlobProblem;

/// Why `otty-plugin.toml` is refused. The text of each value is a plain
/// reasin Plugins mode can show as it is.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ManifestError {
    /// The file is not TOML, or not the shape of a manifest.
    #[error("The manifest can not be read: {0}")]
    Shape(String),
    /// The plugin id is not lowercase `a-z0-9-`.
    #[error(
        "`{0}` is not a valid plugin id: use lowercase letters, digits and \
         dashes"
    )]
    BadPluginId(String),
    /// A part id is not lowercase `a-z0-9-`.
    #[error(
        "`{id}` is not a valid id of a {kind} part: use lowercase letters, \
         digits and dashes"
    )]
    BadPartId {
        /// The kind of the part.
        kind: PartKind,
        /// The id as written.
        id: String,
    },
    /// The plugin version is not semver.
    #[error("The plugin version `{0}` is not a semver version such as 1.2.0")]
    BadVersion(String),
    /// The plugin or a part has an empty name.
    #[error("The {0} has no name")]
    MissingName(String),
    /// `[plugin] wasm` is not a plain `.wasm` file name.
    #[error("`{0}` is not a plain file name ending in `.wasm`")]
    BadWasmName(String),
    /// A part names no icon.
    #[error("The {0} has no icon")]
    MissingIcon(PartRef),
    /// A part's icon is not a plain `.svg` file name.
    #[error(
        "The icon `{icon}` of the {part} is not a plain file name ending in `.svg`"
    )]
    BadIconName {
        /// The part.
        part: PartRef,
        /// The icon as written.
        icon: String,
    },
    /// The manifest lists no part.
    #[error("The plugin lists no agent part and no provider part")]
    NoParts,
    /// Two parts of one kind share an id.
    #[error("Two {kind} parts have the id `{id}`")]
    DuplicatePart {
        /// The kind of the parts.
        kind: PartKind,
        /// The shared id.
        id: String,
    },
    /// A permission glob is refused.
    #[error("The glob `{glob}` of the {part} {problem}")]
    BadGlob {
        /// The part.
        part: PartRef,
        /// The glob as written.
        glob: String,
        /// Why it is refused.
        problem: GlobProblem,
    },
    /// A provider host is not a lowercase host name.
    #[error(
        "`{host}` of the {part} is not a host name such as api.example.com"
    )]
    BadHost {
        /// The part.
        part: PartRef,
        /// The host as written.
        host: String,
    },
    /// Another field of a part is refused.
    #[error("The {part} {reason}")]
    BadPart {
        /// The part.
        part: PartRef,
        /// Why, in words that follow the part's name.
        reason: String,
    },
}

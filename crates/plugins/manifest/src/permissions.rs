//! The permissions of each part of a plugin: what the manifest declares,
//! the checks the host operations make on every call, and the plain words
//! the install dialog shows.

mod check;
mod declared;
mod words;

pub use crate::permissions::check::{KeyColumn, KeyPlace, Permissions};
pub(crate) use crate::permissions::declared::check_host;
pub use crate::permissions::declared::{
    JsonPointer, KeyRule, KeyringRule, PartPermissions, ProgramRule,
};
pub use crate::permissions::words::PermissionLine;

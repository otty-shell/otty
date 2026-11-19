//! The profile dialog (spec Decisions 1 and 11): add a profile with a path
//! of a config folder the user types or a new folder OTTY creates, or edit
//! the name and the config folder of a profile, and for an API key profile
//! the admin key of the organization it pays through. The agent's program
//! is found by
//! its name; there is no program path to set. The typed config folder
//! suggests folders as the New session dialog does (spec Decision 17); a
//! folder that is not there is no error, as a new profile may name one.

mod admin_key_fields;
mod event;
mod folder;
mod model;
mod reducer;
mod state;
mod view;

pub use event::{AdminKeyChoice, FolderChoice, ProfileDialogIntent};
pub(crate) use model::ProfileDialogViewModel;
pub(crate) use reducer::{open_add, open_edit, reduce};
pub(crate) use state::ProfileDialog;
#[cfg(feature = "test-ids")]
pub(crate) use view::CONFIG_FOLDER_ID;
pub(crate) use view::{ProfileDialogProps, view};

//! The profile dialog (spec Decisions 1 and 11): add a profile with a path
//! of a config folder the user types or a new folder OTTY creates, or edit
//! the name and the config folder of a profile, and for an API key profile
//! its admin key and its monthly budget. The agent's program is found by
//! its name; there is no program path to set.

mod event;
mod folder;
mod model;
mod money_fields;
mod reducer;
mod state;
mod view;

pub use event::{AdminKeyChoice, FolderChoice, ProfileDialogIntent};
pub(crate) use folder::{FolderProblem, full_path};
pub(crate) use model::ProfileDialogViewModel;
pub(crate) use reducer::{open_add, open_edit, reduce};
pub(crate) use state::ProfileDialog;
pub(crate) use view::{ProfileDialogProps, view};

//! The profile dialog (spec Decision 1): add a profile with a config folder
//! OTTY found, a path the user types or a new folder OTTY creates, or edit
//! the name, the config folder and the program path of a profile, and for
//! an API key profile its admin key and its monthly budget.

mod event;
mod folder;
mod model;
mod money_fields;
mod reducer;
mod state;
mod view;

pub use event::{AdminKeyChoice, FolderChoice, ProfileDialogIntent};
pub(crate) use model::ProfileDialogViewModel;
pub(crate) use reducer::{open_add, open_edit, reduce, take_candidates};
pub(crate) use state::ProfileDialog;
pub(crate) use view::{ProfileDialogProps, view};

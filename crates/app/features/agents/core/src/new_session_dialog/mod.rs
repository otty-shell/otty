//! The New session dialog (spec Decision 15): a new agent session in any
//! folder, not only in a listed project. The folder field looks the typed
//! folder up and suggests the folders it may stand for; the profile is one
//! of those that can start a session. Start opens the agent tab in the
//! folder exactly as New session of a project screen does. The folder is
//! not stored: the project shows once the agent wrote its first session
//! and the sessions are read again.

mod event;
mod model;
mod reducer;
mod state;
mod view;

pub use event::NewSessionDialogIntent;
pub(crate) use model::NewSessionDialogViewModel;
pub(crate) use reducer::{open, reduce};
pub(crate) use state::NewSessionDialog;
pub(crate) use view::{NewSessionDialogProps, view};

//! The create dialogs (spec Decision 11): Pull image, Create volume and
//! Create network.

pub(crate) mod event;
pub(crate) mod types;
pub(crate) mod view;

mod model;
mod reducer;
mod state;

pub(crate) use model::FormDialogViewModel;
pub(crate) use reducer::reduce;
pub(crate) use state::CreateState;

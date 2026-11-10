//! The actions on the rows of a list (spec Decision 11): the buttons of the
//! inspector, the row menu, Prune, the confirmation of a destructive action
//! and the engine calls that run.

pub(crate) mod ctx;
pub(crate) mod event;
pub(crate) mod types;
pub(crate) mod view;

mod model;
mod offers;
mod reducer;
mod state;

pub(crate) use ctx::ActionsCtx;
#[cfg(test)]
pub(crate) use model::ActionButton;
pub(crate) use model::{
    ActionsViewModel, ConfirmDialogViewModel, PruneViewModel, RowMenuViewModel,
};
pub(crate) use offers::container_actions;
pub(crate) use reducer::reduce;
pub(crate) use state::ActionsState;

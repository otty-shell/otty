//! The Run container form in the main area and the image panel beside it
//! (spec Decision 18).

pub(crate) mod ctx;
pub(crate) mod event;
pub(crate) mod types;
pub(crate) mod view;

mod checks;
mod model;
mod reducer;
mod state;

pub(crate) use ctx::{RunCtx, RunEngine};
pub(crate) use model::{RunFormViewModel, RunPanelViewModel};
pub(crate) use reducer::reduce;
pub(crate) use state::RunState;

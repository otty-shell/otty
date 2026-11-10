//! The inspector in the right panel (spec Decision 12): the tabs of the
//! selected row, its followed logs, its inspect tree and the links in them.

pub(crate) mod ctx;
pub(crate) mod event;
pub(crate) mod types;
pub(crate) mod view;

mod inspect_links;
mod inspect_tree;
mod model;
mod reducer;
mod state;

pub(crate) use ctx::InspectorCtx;
pub(crate) use model::InspectorTabsViewModel;
#[cfg(test)]
pub(crate) use model::TabContent;
pub(crate) use reducer::reduce;
pub(crate) use state::InspectorState;
pub use state::{LOG_TAIL, MAX_LOG_LINES};

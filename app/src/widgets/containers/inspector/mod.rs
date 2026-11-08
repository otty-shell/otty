pub(crate) mod event;
pub(crate) mod types;
pub(crate) mod view;

mod inspect_links;
mod inspect_tree;
mod model;
mod reducer;
mod state;
mod subscription;

pub(super) use model::InspectorTabsViewModel;
#[cfg(test)]
pub(super) use model::TabContent;
pub(super) use reducer::{InspectorCtx, InspectorOutcome, reduce};
pub(super) use state::InspectorState;
pub(super) use subscription::followed_logs;

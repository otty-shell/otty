pub(crate) mod event;
pub(crate) mod types;
pub(crate) mod view;

mod inspect_links;
mod inspect_tree;
mod model;
mod reducer;
mod state;
mod subscription;

pub(in super::super) use model::InspectorTabsViewModel;
#[cfg(test)]
pub(in super::super) use model::TabContent;
pub(in super::super) use reducer::{InspectorCtx, InspectorOutcome, reduce};
pub(in super::super) use state::InspectorState;
pub(in super::super) use subscription::followed_logs;

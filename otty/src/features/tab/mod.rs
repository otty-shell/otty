mod event;
mod model;
mod state;

pub(crate) use event::{TabDeps, TabEvent, tab_reducer};
pub(crate) use model::{TabContent, TabItem, TabOpenRequest};
pub(crate) use state::TabState;

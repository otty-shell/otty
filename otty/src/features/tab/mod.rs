pub(crate) mod event;
pub(crate) mod model;

pub(crate) use event::{TabEvent, tab_reducer};
pub(crate) use model::{TabContent, TabItem, TabOpenRequest};

use crate::types::{InspectorTab, OpenList};

/// What the inspector reads from the rest of the feature.
#[derive(Debug)]
pub(crate) struct InspectorCtx<'a> {
    /// Whether the inspector is on screen: Containers mode is active, the
    /// right panel is shown and the Run form, which shows the image panel in
    /// its place, is closed.
    pub(crate) is_on_screen: bool,
    /// The open tab; the parent keeps it.
    pub(crate) tab: InspectorTab,
    /// Goes up whenever a list of any engine may have changed; the inspect
    /// tree is built again when it moves, because its links point into the
    /// lists.
    pub(crate) lists_version: u64,
    /// The list the main area shows; `None` without engines.
    pub(crate) open: Option<OpenList<'a>>,
}

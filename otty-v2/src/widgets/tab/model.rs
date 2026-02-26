/// Discriminant identifying the content kind of a workspace tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TabContent {
    Terminal,
    Settings,
    QuickLaunchWizard,
    QuickLaunchError,
}

/// Metadata for a single tab entry.
pub(crate) struct TabItem {
    id: u64,
    title: String,
    content: TabContent,
}

impl TabItem {
    /// Create tab metadata with immutable identity and content kind.
    pub(crate) fn new(id: u64, title: String, content: TabContent) -> Self {
        Self { id, title, content }
    }

    /// Return tab identifier.
    pub(crate) fn id(&self) -> u64 {
        self.id
    }

    /// Return tab title shown in the tab bar.
    pub(crate) fn title(&self) -> &str {
        &self.title
    }

    /// Return content discriminator used by feature owners.
    pub(crate) fn content(&self) -> TabContent {
        self.content
    }

    /// Update tab title through tab reducer domain APIs.
    pub(crate) fn set_title(&mut self, title: String) {
        self.title = title;
    }
}

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

    /// Return content discriminator.
    pub(crate) fn content(&self) -> TabContent {
        self.content
    }

    /// Update tab title.
    pub(crate) fn set_title(&mut self, title: String) {
        self.title = title;
    }
}

/// View model for the tabs widget.
#[derive(Debug, Clone)]
pub(crate) struct TabsViewModel {
    pub(crate) tabs: Vec<(u64, String)>,
    pub(crate) active_tab_id: Option<u64>,
    pub(crate) has_tabs: bool,
}

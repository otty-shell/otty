/// View model for the tabs widget.
#[derive(Debug, Clone)]
pub(crate) struct TabsViewModel {
    pub(crate) tabs: Vec<(u64, String)>,
    pub(crate) active_tab_id: Option<u64>,
    pub(crate) has_tabs: bool,
}

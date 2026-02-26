/// Commands processed by the tabs widget reducer.
#[derive(Debug, Clone)]
pub(crate) enum TabsCommand {
    Activate { tab_id: u64 },
    Close { tab_id: u64 },
    SetTitle { tab_id: u64, title: String },
    OpenTerminalTab { terminal_id: u64, title: String },
    OpenSettingsTab,
    OpenWizardTab { title: String },
    OpenErrorTab { title: String },
}

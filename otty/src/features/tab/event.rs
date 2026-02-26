use otty_ui_term::settings::Settings;

use crate::features::quick_launch;

/// Events emitted by tab UI and tab orchestration flows.
#[derive(Debug, Clone)]
pub(crate) enum TabEvent {
    Activate {
        tab_id: u64,
    },
    CloseRequested {
        tab_id: u64,
    },
    SetTitle {
        tab_id: u64,
        title: String,
    },
    OpenTerminalTab {
        terminal_id: u64,
    },
    OpenSettingsTab,
    OpenCommandTerminalTab {
        title: String,
        terminal_id: u64,
        settings: Box<Settings>,
    },
    OpenQuickLaunchCommandTerminalTab {
        title: String,
        terminal_id: u64,
        settings: Box<Settings>,
        command: Box<quick_launch::QuickLaunch>,
    },
    OpenQuickLaunchWizardCreateTab {
        parent_path: quick_launch::NodePath,
    },
    OpenQuickLaunchWizardEditTab {
        path: quick_launch::NodePath,
        command: Box<quick_launch::QuickLaunch>,
    },
    OpenQuickLaunchErrorTab {
        title: String,
        message: String,
    },
}

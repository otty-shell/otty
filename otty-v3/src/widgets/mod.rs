pub(crate) mod chrome;
pub(crate) mod explorer;
pub(crate) mod quick_launch;
pub(crate) mod settings;
pub(crate) mod sidebar;
pub(crate) mod tabs;
pub(crate) mod terminal_workspace;

pub(crate) struct Widgets {
    pub(crate) sidebar: sidebar::SidebarWidget,
    pub(crate) chrome: chrome::ChromeWidget,
    pub(crate) tabs: tabs::TabsWidget,
    pub(crate) quick_launch: quick_launch::QuickLaunchWidget,
    pub(crate) terminal_workspace: terminal_workspace::TerminalWorkspaceWidget,
    pub(crate) explorer: explorer::ExplorerWidget,
    pub(crate) settings: settings::SettingsWidget,
}

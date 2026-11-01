pub(crate) mod chrome;
pub(crate) mod explorer;
// The root view and the shortcuts start using the widget in tickets 05 and
// 06 of `specs/2026-11-01-modes-and-docker`; remove this then.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "the root view does not show modes yet")
)]
pub(crate) mod modes;
pub(crate) mod palette;
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
    pub(crate) palette: palette::PaletteWidget,
    pub(crate) modes: modes::ModesWidget,
}

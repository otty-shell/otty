use otty_ui_kit::components::target_dot::TargetKind;

/// One open tab as the Sessions section lists it: its title and the machine
/// it runs on. Other places that list the open tabs, such as the command
/// palette, read the tabs through it.
///
/// # Example
///
/// ```
/// use otty_app_slots::Component;
/// use otty_app_tabs::{Tabs, TabsCtx, TabsIntent};
/// use otty_ui_kit::components::target_dot::TargetKind;
///
/// let mut tabs = Tabs::new();
/// let _ =
///     tabs.reduce(TabsIntent::OpenTerminalTab, TabsCtx { shell_name: "zsh" });
///
/// let sessions = tabs.sessions();
///
/// assert_eq!(sessions.len(), 1);
/// assert_eq!(sessions[0].title(), "zsh");
/// assert_eq!(sessions[0].machine_target(), TargetKind::Local);
/// assert!(sessions[0].is_active());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenSession<'a> {
    tab_id: u64,
    title: &'a str,
    machine_name: &'a str,
    machine_target: TargetKind,
    is_active: bool,
}

impl<'a> OpenSession<'a> {
    /// Return the id of the tab.
    pub fn tab_id(&self) -> u64 {
        self.tab_id
    }

    /// Return the title of the tab.
    pub fn title(&self) -> &'a str {
        self.title
    }

    /// Return the name of the machine the tab runs on: this computer's host
    /// name or the SSH host.
    pub fn machine_name(&self) -> &'a str {
        self.machine_name
    }

    /// Return the kind of the machine the tab runs on.
    pub fn machine_target(&self) -> TargetKind {
        self.machine_target
    }

    /// Return whether the tab is the active one.
    pub fn is_active(&self) -> bool {
        self.is_active
    }

    /// An open tab with its machine.
    pub(crate) fn new(
        tab_id: u64,
        title: &'a str,
        machine_name: &'a str,
        machine_target: TargetKind,
        is_active: bool,
    ) -> Self {
        Self {
            tab_id,
            title,
            machine_name,
            machine_target,
            is_active,
        }
    }
}

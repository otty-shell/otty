//! The tabs of the OTTY app: the open sessions, a shell, a command or the
//! launch editor each, and the Sessions section of the left panel that lists
//! them grouped by the machine they run on.
//!
//! The tabs keep only what a tab is: its title, its content kind, its
//! machine and which tab is active. What runs in a tab belongs to the
//! terminal and quick launch features: opening, activating and closing a tab
//! return a [`TabsOutcome`] that carries what they need, and the binary
//! hands it on. The host name that names this computer arrives as
//! [`TabsIntent::HostNameRead`], so the section draws only from the tabs'
//! own state.
//!
//! # Example
//!
//! ```
//! use otty_app_slots::Feature;
//! use otty_app_tabs::{Tabs, TabsCtx, TabsIntent, TabsOutcome};
//!
//! let ctx = || TabsCtx { shell_name: "zsh" };
//! let mut tabs = Tabs::new();
//!
//! let opened = tabs.reduce(TabsIntent::OpenTerminalTab, ctx());
//! assert!(matches!(
//!     opened.as_slice(),
//!     [
//!         TabsOutcome::TerminalTabOpened { tab_id: 0, .. },
//!         TabsOutcome::ScrollSessionsToEnd,
//!     ]
//! ));
//! assert_eq!(tabs.active_tab_title(), Some("zsh"));
//!
//! let closed = tabs.reduce(TabsIntent::CloseActive, ctx());
//! assert!(matches!(
//!     closed.as_slice(),
//!     [TabsOutcome::Closed {
//!         tab_id: 0,
//!         new_active_id: None,
//!     }]
//! ));
//! assert_eq!(tabs.len(), 0);
//! ```

mod ctx;
mod event;
mod model;
mod reducer;
mod session;
mod state;
mod types;
mod view;

pub use ctx::TabsCtx;
pub use event::{TabsIntent, TabsOutcome};
use model::TabsViewModel;
use otty_app_domain::tab::TabMachine;
use otty_app_slots::{Feature, LeftPanel, LeftPanelSection, SectionState};
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::target_dot::TargetKind;
pub use session::OpenSession;
use state::TabsState;
pub use types::TabContent;
use types::{machine_name, machine_target};
use view::session_list;
pub use view::session_list::SESSION_LIST_SCROLL_ID;

/// The machine the status bar names while no tab is open.
static THIS_COMPUTER: TabMachine = TabMachine::ThisComputer;

/// The tabs feature: the open tabs in the order they opened, the active one
/// and this computer's host name.
///
/// # Example
///
/// ```
/// use otty_app_tabs::Tabs;
///
/// let tabs = Tabs::new();
///
/// assert_eq!(tabs.len(), 0);
/// assert_eq!(tabs.active_tab_id(), None);
/// ```
pub struct Tabs {
    state: TabsState,
}

impl Tabs {
    /// Return the id of the active tab, or `None` while no tab is open.
    pub fn active_tab_id(&self) -> Option<u64> {
        self.state.active_tab_id()
    }

    /// Return what the active tab shows, or `None` while no tab is open.
    pub fn active_tab_content(&self) -> Option<TabContent> {
        self.state.active_tab().map(|tab| tab.content())
    }

    /// Return the title of the active tab, shown in the title bar.
    pub fn active_tab_title(&self) -> Option<&str> {
        self.state.active_tab().map(|tab| tab.title())
    }

    /// Return the number of open tabs.
    pub fn len(&self) -> usize {
        self.state.len()
    }

    /// Return whether no tab is open.
    pub fn is_empty(&self) -> bool {
        self.state.is_empty()
    }

    /// Return the name of the machine the active tab runs on: this
    /// computer's host name, "This computer" before it is read, or the SSH
    /// host. While no tab is open it names this computer.
    ///
    /// # Example
    ///
    /// ```
    /// use otty_app_slots::Feature;
    /// use otty_app_tabs::{Tabs, TabsCtx, TabsIntent};
    ///
    /// let mut tabs = Tabs::new();
    /// let _ = tabs.reduce(
    ///     TabsIntent::HostNameRead(Some(String::from("build-box"))),
    ///     TabsCtx { shell_name: "zsh" },
    /// );
    ///
    /// assert_eq!(tabs.active_machine_name(), "build-box");
    /// ```
    pub fn active_machine_name(&self) -> &str {
        machine_name(self.active_machine(), self.state.host_name())
    }

    /// Return the kind of the machine the active tab runs on; this computer
    /// while no tab is open.
    pub fn active_machine_target(&self) -> TargetKind {
        machine_target(self.active_machine())
    }

    /// Return the open tabs grouped by machine, in the order the Sessions
    /// section lists them: groups in the order their first tab opened, tabs
    /// in the order they opened.
    pub fn sessions(&self) -> Vec<OpenSession<'_>> {
        let vm = TabsViewModel::from_state(&self.state);

        vm.groups
            .iter()
            .flat_map(|group| {
                group.sessions.iter().map(|session| {
                    OpenSession::new(
                        session.tab_id,
                        session.title,
                        group.name,
                        group.target,
                        session.is_active,
                    )
                })
            })
            .collect()
    }

    /// Create the tabs with no tab open.
    pub fn new() -> Self {
        Self {
            state: TabsState::default(),
        }
    }

    fn active_machine(&self) -> &TabMachine {
        match self.state.active_tab() {
            Some(tab) => tab.machine(),
            None => &THIS_COMPUTER,
        }
    }
}

impl Default for Tabs {
    fn default() -> Self {
        Self::new()
    }
}

impl Feature for Tabs {
    type Intent = TabsIntent;
    type Outcome = TabsOutcome;
    type Ctx<'a> = TabsCtx<'a>;

    fn reduce(
        &mut self,
        intent: TabsIntent,
        ctx: TabsCtx<'_>,
    ) -> Vec<TabsOutcome> {
        reducer::reduce(&mut self.state, intent, &ctx)
    }
}

impl LeftPanel for Tabs {
    /// The Sessions section: its header with the New session button, and
    /// the open tabs grouped by machine, or a hint while none is open.
    fn left_panel<'a>(
        &'a self,
        theme: ThemeProps<'a>,
        section: SectionState,
    ) -> LeftPanelSection<'a, TabsIntent> {
        let vm = TabsViewModel::from_state(&self.state);
        let content_height = session_list::content_height(&vm);

        let content = session_list::view(session_list::SessionListProps {
            vm,
            theme,
            section,
        });

        LeftPanelSection::new(content, content_height)
    }
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use otty_app_domain::exit_behavior::ExitBehavior;
    use otty_app_i18n::{self as i18n, Key};
    use otty_app_slots::SectionMessage;
    use otty_app_theme::theme::ThemeManager;
    use otty_ui_term::settings::Settings;

    use super::*;

    fn ctx() -> TabsCtx<'static> {
        TabsCtx { shell_name: "zsh" }
    }

    fn open_ssh_tab(tabs: &mut Tabs, title: &str, host: &str) {
        let _ = tabs.reduce(
            TabsIntent::OpenCommandTab {
                title: String::from(title),
                settings: Box::new(Settings::default()),
                exit_behavior: ExitBehavior::Close,
                machine: TabMachine::Ssh(String::from(host)),
            },
            ctx(),
        );
    }

    #[test]
    fn the_active_tab_is_read_through_the_getters() {
        let mut tabs = Tabs::new();
        let empty = (tabs.active_tab_id(), tabs.active_tab_content());

        let _ = tabs.reduce(TabsIntent::OpenTerminalTab, ctx());

        assert_eq!(empty, (None, None));
        assert!(!tabs.is_empty());
        assert_eq!(tabs.active_tab_id(), Some(0));
        assert_eq!(tabs.active_tab_content(), Some(TabContent::Terminal));
        assert_eq!(tabs.active_tab_title(), Some("zsh"));
    }

    /// The status bar names the machine of the active tab, and this
    /// computer while no tab is open.
    #[test]
    fn the_active_machine_is_the_machine_of_the_active_tab() {
        let mut tabs = Tabs::new();
        let before = (
            tabs.active_machine_name().to_string(),
            tabs.active_machine_target(),
        );

        open_ssh_tab(&mut tabs, "htop", "laptop");

        assert_eq!(
            before,
            (String::from(i18n::t(Key::ThisComputer)), TargetKind::Local)
        );
        assert_eq!(tabs.active_machine_name(), "laptop");
        assert_eq!(tabs.active_machine_target(), TargetKind::Ssh);
    }

    #[test]
    fn sessions_keep_the_machine_groups_of_the_sessions_section() {
        let mut tabs = Tabs::new();
        let _ = tabs.reduce(
            TabsIntent::HostNameRead(Some(String::from("fedora"))),
            ctx(),
        );
        let _ = tabs.reduce(TabsIntent::OpenTerminalTab, ctx());
        open_ssh_tab(&mut tabs, "htop", "laptop");
        let _ = tabs.reduce(TabsIntent::OpenTerminalTab, ctx());

        let sessions: Vec<(u64, &str, &str, bool)> = tabs
            .sessions()
            .iter()
            .map(|session| {
                (
                    session.tab_id(),
                    session.title(),
                    session.machine_name(),
                    session.is_active(),
                )
            })
            .collect();

        assert_eq!(
            sessions,
            vec![
                (0, "zsh", "fedora", false),
                (2, "zsh", "fedora", true),
                (1, "htop", "laptop", false),
            ]
        );
    }

    /// The section draws from the tabs' own state: the host name read
    /// earlier names this computer's group, and the frame's section state
    /// hides the rows of a closed section.
    #[test]
    fn the_left_panel_is_the_sessions_section_of_the_tabs() {
        let themes = ThemeManager::new();
        let theme = ThemeProps::new(themes.current());
        let mut tabs = Tabs::new();
        let _ = tabs.reduce(
            TabsIntent::HostNameRead(Some(String::from("build-box"))),
            ctx(),
        );
        let _ = tabs.reduce(TabsIntent::OpenTerminalTab, ctx());

        let open = tabs.left_panel(theme, SectionState::new(true, false));
        let height = open.content_height();
        let mut ui = simulator(open.into_content());

        assert!(height > 0.0);
        assert!(ui.find("build-box").is_ok());
        ui.click("zsh").expect("the row should show");
        let pressed: Vec<SectionMessage<TabsIntent>> =
            ui.into_messages().collect();
        assert!(matches!(
            pressed.as_slice(),
            [SectionMessage::Feature(TabsIntent::ActivateTab {
                tab_id: 0
            })]
        ));

        let closed = tabs.left_panel(theme, SectionState::new(false, false));
        let mut ui = simulator(closed.into_content());
        assert!(ui.find("zsh").is_err());
    }
}

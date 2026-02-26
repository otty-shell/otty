use iced::Task;
use otty_ui_term::settings::Settings;

use crate::app::Event;
use crate::features::explorer::ExplorerEvent;
use crate::features::quick_launch::{self, QuickLaunchEvent};
use crate::features::quick_launch_wizard::QuickLaunchWizardEvent;
use crate::features::settings;
use crate::features::terminal::{
    ShellSession, TerminalEvent, TerminalFeature, TerminalKind,
    terminal_settings_for_session,
};
use crate::state::State;

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

/// Activate a tab by identifier, focusing the terminal and syncing explorer.
pub(crate) fn activate_tab(state: &mut State, tab_id: u64) -> Task<Event> {
    if !state.tab.contains(tab_id) {
        return Task::none();
    }

    state.tab.activate(Some(tab_id));

    Task::batch(vec![
        Task::done(Event::Terminal(TerminalEvent::FocusActive)),
        Task::done(Event::Terminal(TerminalEvent::SyncSelection { tab_id })),
        Task::done(Event::Explorer(ExplorerEvent::SyncFromActiveTerminal)),
    ])
}

/// Close a tab and activate the most-recently-used neighbour.
pub(crate) fn close_tab(state: &mut State, tab_id: u64) -> Task<Event> {
    if !state.tab.contains(tab_id) {
        return Task::none();
    }

    let next_active = if state.tab.active_tab_id() == Some(tab_id) {
        state
            .tab
            .previous_tab_id(tab_id)
            .or_else(|| state.tab.last_tab_id())
    } else {
        state.tab.active_tab_id()
    };

    state.tab.remove(tab_id);

    if state.tab.is_empty() {
        state.tab.activate(None);
    } else if state.tab.active_tab_id() == Some(tab_id) {
        state.tab.activate(next_active);
    }

    let mut tasks = vec![
        Task::done(Event::Terminal(TerminalEvent::TabClosed { tab_id })),
        Task::done(Event::QuickLaunchWizard {
            tab_id,
            event: QuickLaunchWizardEvent::TabClosed,
        }),
        Task::done(Event::QuickLaunch(QuickLaunchEvent::TabClosed { tab_id })),
    ];

    if !state.tab.is_empty() {
        tasks.push(Task::done(Event::Terminal(TerminalEvent::FocusActive)));

        if let Some(active_id) = state.tab.active_tab_id() {
            tasks.push(Task::done(Event::Terminal(
                TerminalEvent::SyncSelection { tab_id: active_id },
            )));
        }
    }

    tasks.push(Task::done(Event::Explorer(
        ExplorerEvent::SyncFromActiveTerminal,
    )));

    Task::batch(tasks)
}

/// Open a new shell terminal tab.
pub(crate) fn open_terminal_tab(
    state: &mut State,
    terminal: &mut TerminalFeature,
    shell_session: &ShellSession,
    terminal_settings: &Settings,
) -> Task<Event> {
    let tab_id = state.allocate_tab_id();
    let terminal_id = terminal.allocate_terminal_id();
    let name = shell_session.name().to_string();

    state.tab.insert(
        tab_id,
        TabItem::new(tab_id, name.clone(), TabContent::Terminal),
    );
    state.tab.activate(Some(tab_id));

    let settings = terminal_settings_for_session(
        terminal_settings,
        shell_session.session().clone(),
    );

    Task::batch(vec![Task::done(Event::Terminal(TerminalEvent::OpenTab {
        tab_id,
        terminal_id,
        default_title: name,
        settings: Box::new(settings),
        kind: TerminalKind::Shell,
        sync_explorer: true,
        error_tab: None,
    }))])
}

/// Open a new settings tab, reloading settings state.
pub(crate) fn open_settings_tab(state: &mut State) -> Task<Event> {
    let tab_id = state.allocate_tab_id();

    state.tab.insert(
        tab_id,
        TabItem::new(tab_id, String::from("Settings"), TabContent::Settings),
    );
    state.tab.activate(Some(tab_id));

    Task::batch(vec![
        Task::done(Event::Settings(settings::SettingsEvent::Reload)),
        Task::done(Event::Explorer(ExplorerEvent::SyncFromActiveTerminal)),
    ])
}

/// Open a new terminal tab running an arbitrary command.
pub(crate) fn open_command_terminal_tab(
    state: &mut State,
    terminal: &mut TerminalFeature,
    title: String,
    tab_settings: Settings,
) -> Task<Event> {
    let tab_id = state.allocate_tab_id();
    let terminal_id = terminal.allocate_terminal_id();

    state.tab.insert(
        tab_id,
        TabItem::new(tab_id, title.clone(), TabContent::Terminal),
    );
    state.tab.activate(Some(tab_id));

    Task::done(Event::Terminal(TerminalEvent::OpenTab {
        tab_id,
        terminal_id,
        default_title: title,
        settings: Box::new(tab_settings),
        kind: TerminalKind::Command,
        sync_explorer: false,
        error_tab: None,
    }))
}

/// Open a new terminal tab launching a quick-launch command with error recovery.
pub(crate) fn open_quick_launch_command_terminal_tab(
    state: &mut State,
    terminal: &mut TerminalFeature,
    title: String,
    tab_settings: Settings,
    command: Box<quick_launch::QuickLaunch>,
) -> Task<Event> {
    let tab_id = state.allocate_tab_id();
    let terminal_id = terminal.allocate_terminal_id();
    let command_title = command.title.clone();
    let init_error_message = quick_launch::quick_launch_error_message(
        &command,
        &"terminal tab initialization failed",
    );

    state.tab.insert(
        tab_id,
        TabItem::new(tab_id, title.clone(), TabContent::Terminal),
    );
    state.tab.activate(Some(tab_id));

    Task::batch(vec![
        Task::done(Event::Terminal(TerminalEvent::OpenTab {
            tab_id,
            terminal_id,
            default_title: title,
            settings: Box::new(tab_settings),
            kind: TerminalKind::Command,
            sync_explorer: false,
            error_tab: Some((
                format!("Failed to launch \"{command_title}\""),
                init_error_message,
            )),
        })),
        Task::done(Event::Terminal(TerminalEvent::FocusActive)),
    ])
}

/// Open a wizard tab for creating a new quick-launch entry.
pub(crate) fn open_quick_launch_wizard_create_tab(
    state: &mut State,
    parent_path: quick_launch::NodePath,
) -> Task<Event> {
    let tab_id = state.allocate_tab_id();

    state.tab.insert(
        tab_id,
        TabItem::new(
            tab_id,
            String::from("Create launch"),
            TabContent::QuickLaunchWizard,
        ),
    );
    state.tab.activate(Some(tab_id));

    Task::done(Event::QuickLaunchWizard {
        tab_id,
        event: QuickLaunchWizardEvent::InitializeCreate { parent_path },
    })
}

/// Open a wizard tab for editing an existing quick-launch entry.
pub(crate) fn open_quick_launch_wizard_edit_tab(
    state: &mut State,
    path: quick_launch::NodePath,
    command: quick_launch::QuickLaunch,
) -> Task<Event> {
    let tab_id = state.allocate_tab_id();
    let title = format!("Edit {}", command.title);

    state.tab.insert(
        tab_id,
        TabItem::new(tab_id, title, TabContent::QuickLaunchWizard),
    );
    state.tab.activate(Some(tab_id));

    Task::done(Event::QuickLaunchWizard {
        tab_id,
        event: QuickLaunchWizardEvent::InitializeEdit {
            path,
            command: Box::new(command),
        },
    })
}

/// Open an error tab reporting a quick-launch failure.
pub(crate) fn open_quick_launch_error_tab(
    state: &mut State,
    title: String,
    message: String,
) -> Task<Event> {
    let tab_id = state.allocate_tab_id();

    state.tab.insert(
        tab_id,
        TabItem::new(tab_id, title.clone(), TabContent::QuickLaunchError),
    );
    state.tab.activate(Some(tab_id));

    Task::done(Event::QuickLaunch(QuickLaunchEvent::OpenErrorTab {
        tab_id,
        title,
        message,
    }))
}

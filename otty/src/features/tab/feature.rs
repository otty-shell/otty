use iced::Task;
use otty_ui_term::settings::Settings;

use super::event::TabEvent;
use super::model::{TabContent, TabItem};
use super::state::TabState;
use crate::app::Event as AppEvent;
use crate::features::explorer::ExplorerEvent;
use crate::features::quick_launch::{self, QuickLaunchEvent};
use crate::features::quick_launch_wizard::QuickLaunchWizardEvent;
use crate::features::settings;
use crate::features::terminal::{
    ShellSession, TerminalEvent, TerminalKind, terminal_settings_for_session,
};

/// Runtime dependencies required by tab reducer flows.
pub(crate) struct TabCtx<'a> {
    /// Shell session used when opening default terminal tabs.
    pub(crate) shell_session: &'a ShellSession,
    /// Terminal settings baseline used for spawned tabs.
    pub(crate) terminal_settings: &'a Settings,
}

/// Tab feature root that owns tab metadata and lifecycle orchestration.
#[derive(Default)]
pub(crate) struct TabFeature {
    state: TabState,
}

impl TabFeature {
    /// Construct tab feature with empty tab state.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Return number of tabs.
    pub(crate) fn len(&self) -> usize {
        self.state.len()
    }

    /// Return active tab identifier.
    pub(crate) fn active_tab_id(&self) -> Option<u64> {
        self.state.active_tab_id()
    }

    /// Return active tab item if present.
    pub(crate) fn active_tab(&self) -> Option<&TabItem> {
        self.state.active_tab()
    }

    /// Return active tab title.
    pub(crate) fn active_tab_title(&self) -> Option<&str> {
        self.state.active_tab().map(|tab| tab.title())
    }

    /// Return tab summaries for rendering tab bar.
    pub(crate) fn tab_summaries(&self) -> Vec<(u64, &str)> {
        self.state
            .tab_items()
            .iter()
            .map(|(id, item)| (*id, item.title()))
            .collect()
    }
}

impl TabFeature {
    /// Reduce a tab event into state updates and routed app tasks.
    pub(crate) fn reduce(
        &mut self,
        event: TabEvent,
        ctx: &TabCtx<'_>,
    ) -> Task<AppEvent> {
        match event {
            TabEvent::Activate { tab_id } => self.activate_tab(tab_id),
            TabEvent::CloseRequested { tab_id } => self.close_tab(tab_id),
            TabEvent::SetTitle { tab_id, title } => {
                self.state.set_title(tab_id, title);
                Task::none()
            },
            TabEvent::OpenTerminalTab { terminal_id } => {
                self.open_terminal_tab(terminal_id, ctx)
            },
            TabEvent::OpenSettingsTab => self.open_settings_tab(),
            TabEvent::OpenCommandTerminalTab {
                title,
                terminal_id,
                settings,
            } => self.open_command_terminal_tab(title, terminal_id, *settings),
            TabEvent::OpenQuickLaunchCommandTerminalTab {
                title,
                terminal_id,
                settings,
                command,
            } => self.open_quick_launch_command_terminal_tab(
                title,
                terminal_id,
                *settings,
                command,
            ),
            TabEvent::OpenQuickLaunchWizardCreateTab { parent_path } => {
                self.open_quick_launch_wizard_create_tab(parent_path)
            },
            TabEvent::OpenQuickLaunchWizardEditTab { path, command } => {
                self.open_quick_launch_wizard_edit_tab(path, *command)
            },
            TabEvent::OpenQuickLaunchErrorTab { title, message } => {
                self.open_quick_launch_error_tab(title, message)
            },
        }
    }
}

impl TabFeature {
    fn allocate_tab_id(&mut self) -> u64 {
        self.state.allocate_tab_id()
    }

    fn activate_tab(&mut self, tab_id: u64) -> Task<AppEvent> {
        if !self.state.contains(tab_id) {
            return Task::none();
        }

        self.state.activate(Some(tab_id));

        Task::batch(vec![
            Task::done(AppEvent::Terminal(TerminalEvent::FocusActive)),
            Task::done(AppEvent::Terminal(TerminalEvent::SyncSelection {
                tab_id,
            })),
            Task::done(AppEvent::Explorer(
                ExplorerEvent::SyncFromActiveTerminal,
            )),
        ])
    }

    fn close_tab(&mut self, tab_id: u64) -> Task<AppEvent> {
        if !self.state.contains(tab_id) {
            return Task::none();
        }

        let next_active = if self.state.active_tab_id() == Some(tab_id) {
            self.state
                .previous_tab_id(tab_id)
                .or_else(|| self.state.last_tab_id())
        } else {
            self.state.active_tab_id()
        };

        self.state.remove(tab_id);

        if self.state.is_empty() {
            self.state.activate(None);
        } else if self.state.active_tab_id() == Some(tab_id) {
            self.state.activate(next_active);
        }

        let mut tasks = vec![
            Task::done(AppEvent::Terminal(TerminalEvent::TabClosed { tab_id })),
            Task::done(AppEvent::QuickLaunchWizard {
                tab_id,
                event: QuickLaunchWizardEvent::TabClosed,
            }),
            Task::done(AppEvent::QuickLaunch(QuickLaunchEvent::TabClosed {
                tab_id,
            })),
        ];

        if !self.state.is_empty() {
            tasks.push(Task::done(AppEvent::Terminal(
                TerminalEvent::FocusActive,
            )));

            if let Some(active_id) = self.state.active_tab_id() {
                tasks.push(Task::done(AppEvent::Terminal(
                    TerminalEvent::SyncSelection { tab_id: active_id },
                )));
            }
        }

        tasks.push(Task::done(AppEvent::Explorer(
            ExplorerEvent::SyncFromActiveTerminal,
        )));

        Task::batch(tasks)
    }

    fn open_terminal_tab(
        &mut self,
        terminal_id: u64,
        ctx: &TabCtx<'_>,
    ) -> Task<AppEvent> {
        let tab_id = self.allocate_tab_id();
        let name = ctx.shell_session.name().to_string();

        self.state.insert(
            tab_id,
            TabItem::new(tab_id, name.clone(), TabContent::Terminal),
        );
        self.state.activate(Some(tab_id));

        let settings = terminal_settings_for_session(
            ctx.terminal_settings,
            ctx.shell_session.session().clone(),
        );

        Task::done(AppEvent::Terminal(TerminalEvent::OpenTab {
            tab_id,
            terminal_id,
            default_title: name,
            settings: Box::new(settings),
            kind: TerminalKind::Shell,
            sync_explorer: true,
            error_tab: None,
        }))
    }

    fn open_settings_tab(&mut self) -> Task<AppEvent> {
        let tab_id = self.allocate_tab_id();

        self.state.insert(
            tab_id,
            TabItem::new(
                tab_id,
                String::from("Settings"),
                TabContent::Settings,
            ),
        );
        self.state.activate(Some(tab_id));

        Task::batch(vec![
            Task::done(AppEvent::Settings(settings::SettingsEvent::Reload)),
            Task::done(AppEvent::Explorer(
                ExplorerEvent::SyncFromActiveTerminal,
            )),
        ])
    }

    fn open_command_terminal_tab(
        &mut self,
        title: String,
        terminal_id: u64,
        tab_settings: Settings,
    ) -> Task<AppEvent> {
        let tab_id = self.allocate_tab_id();

        self.state.insert(
            tab_id,
            TabItem::new(tab_id, title.clone(), TabContent::Terminal),
        );
        self.state.activate(Some(tab_id));

        Task::done(AppEvent::Terminal(TerminalEvent::OpenTab {
            tab_id,
            terminal_id,
            default_title: title,
            settings: Box::new(tab_settings),
            kind: TerminalKind::Command,
            sync_explorer: false,
            error_tab: None,
        }))
    }

    fn open_quick_launch_command_terminal_tab(
        &mut self,
        title: String,
        terminal_id: u64,
        tab_settings: Settings,
        command: Box<quick_launch::QuickLaunch>,
    ) -> Task<AppEvent> {
        let tab_id = self.allocate_tab_id();
        let command_title = command.title.clone();
        let init_error_message = quick_launch::quick_launch_error_message(
            &command,
            &"terminal tab initialization failed",
        );

        self.state.insert(
            tab_id,
            TabItem::new(tab_id, title.clone(), TabContent::Terminal),
        );
        self.state.activate(Some(tab_id));

        Task::batch(vec![
            Task::done(AppEvent::Terminal(TerminalEvent::OpenTab {
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
            Task::done(AppEvent::Terminal(TerminalEvent::FocusActive)),
        ])
    }

    fn open_quick_launch_wizard_create_tab(
        &mut self,
        parent_path: quick_launch::NodePath,
    ) -> Task<AppEvent> {
        let tab_id = self.allocate_tab_id();

        self.state.insert(
            tab_id,
            TabItem::new(
                tab_id,
                String::from("Create launch"),
                TabContent::QuickLaunchWizard,
            ),
        );
        self.state.activate(Some(tab_id));

        Task::done(AppEvent::QuickLaunchWizard {
            tab_id,
            event: QuickLaunchWizardEvent::InitializeCreate { parent_path },
        })
    }

    fn open_quick_launch_wizard_edit_tab(
        &mut self,
        path: quick_launch::NodePath,
        command: quick_launch::QuickLaunch,
    ) -> Task<AppEvent> {
        let tab_id = self.allocate_tab_id();
        let title = format!("Edit {}", command.title);

        self.state.insert(
            tab_id,
            TabItem::new(tab_id, title, TabContent::QuickLaunchWizard),
        );
        self.state.activate(Some(tab_id));

        Task::done(AppEvent::QuickLaunchWizard {
            tab_id,
            event: QuickLaunchWizardEvent::InitializeEdit {
                path,
                command: Box::new(command),
            },
        })
    }

    fn open_quick_launch_error_tab(
        &mut self,
        title: String,
        message: String,
    ) -> Task<AppEvent> {
        let tab_id = self.allocate_tab_id();

        self.state.insert(
            tab_id,
            TabItem::new(tab_id, title.clone(), TabContent::QuickLaunchError),
        );
        self.state.activate(Some(tab_id));

        Task::done(AppEvent::QuickLaunch(QuickLaunchEvent::OpenErrorTab {
            tab_id,
            title,
            message,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::terminal::fallback_shell_session_with_shell;

    fn ctx<'a>(settings: &'a Settings, shell: &'a ShellSession) -> TabCtx<'a> {
        TabCtx {
            shell_session: shell,
            terminal_settings: settings,
        }
    }

    #[test]
    fn given_open_settings_when_reduced_then_settings_tab_becomes_active() {
        let mut feature = TabFeature::new();
        let settings = Settings::default();
        let shell = fallback_shell_session_with_shell("/bin/sh");

        let _ =
            feature.reduce(TabEvent::OpenSettingsTab, &ctx(&settings, &shell));

        let active = feature.active_tab().expect("active tab should exist");
        assert_eq!(active.content(), TabContent::Settings);
        assert_eq!(active.title(), "Settings");
    }

    #[test]
    fn given_opened_tab_when_set_title_then_summary_reflects_new_title() {
        let mut feature = TabFeature::new();
        let settings = Settings::default();
        let shell = fallback_shell_session_with_shell("/bin/sh");

        let _ = feature.reduce(
            TabEvent::OpenCommandTerminalTab {
                title: String::from("Initial"),
                terminal_id: 1,
                settings: Box::new(Settings::default()),
            },
            &ctx(&settings, &shell),
        );

        let tab_id =
            feature.active_tab().expect("active tab should exist").id();
        let _ = feature.reduce(
            TabEvent::SetTitle {
                tab_id,
                title: String::from("Renamed"),
            },
            &ctx(&settings, &shell),
        );

        let summaries = feature.tab_summaries();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].1, "Renamed");
    }

    #[test]
    fn given_two_tabs_when_active_closed_then_previous_tab_becomes_active() {
        let mut feature = TabFeature::new();
        let settings = Settings::default();
        let shell = fallback_shell_session_with_shell("/bin/sh");

        let _ = feature.reduce(
            TabEvent::OpenCommandTerminalTab {
                title: String::from("First"),
                terminal_id: 1,
                settings: Box::new(Settings::default()),
            },
            &ctx(&settings, &shell),
        );
        let first_id =
            feature.active_tab().expect("first tab should exist").id();

        let _ = feature.reduce(
            TabEvent::OpenCommandTerminalTab {
                title: String::from("Second"),
                terminal_id: 2,
                settings: Box::new(Settings::default()),
            },
            &ctx(&settings, &shell),
        );
        let second_id =
            feature.active_tab().expect("second tab should exist").id();

        let _ = feature.reduce(
            TabEvent::CloseRequested { tab_id: second_id },
            &ctx(&settings, &shell),
        );

        assert_eq!(feature.active_tab_id(), Some(first_id));
    }
}

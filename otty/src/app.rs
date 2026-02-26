use std::time::Duration;

use iced::widget::operation::snap_to_end;
use iced::widget::{Space, column, container, mouse_area, pane_grid, row};
use iced::window::Direction;
use iced::{Element, Length, Size, Subscription, Task, Theme, window};
use otty_ui_term::settings::{
    BackendSettings, FontSettings, Settings, ThemeSettings,
};

use crate::effects::close_window;
use crate::features::explorer::{ExplorerCtx, ExplorerEvent};
use crate::features::quick_launch_wizard::{
    QuickLaunchWizardCtx, QuickLaunchWizardEvent,
};
use crate::features::terminal::{
    ShellSession, TerminalCtx, TerminalEvent,
    fallback_shell_session_with_shell, setup_shell_session_with_shell,
    shell_cwd_for_active_tab,
};
use crate::features::{Features, quick_launch, settings};
use crate::fonts::FontsConfig;
use crate::guards::{MenuGuard, context_menu_guard, inline_edit_guard};
use crate::state::{
    SIDEBAR_MENU_WIDTH, SidebarItem, SidebarPane, State,
    max_sidebar_workspace_ratio,
};
use crate::tab;
use crate::theme::{AppTheme, ThemeManager, ThemeProps};
use crate::ui::components::{resize_grips, sidebar_workspace_panel};
use crate::ui::widgets::{
    action_bar, quick_launches_context_menu, sidebar_menu, sidebar_workspace,
    sidebar_workspace_add_menu, tab_bar, tab_content,
    terminal_pane_context_menu,
};

pub(crate) const MIN_WINDOW_WIDTH: f32 = 800.0;
pub(crate) const MIN_WINDOW_HEIGHT: f32 = 600.0;
const HEADER_SEPARATOR_HEIGHT: f32 = 1.0;
const SIDEBAR_SEPARATOR_WIDTH: f32 = 0.3;
const SEPARATOR_ALPHA: f32 = 0.3;

/// App-wide events that drive the root update loop.
#[derive(Clone)]
pub(crate) enum Event {
    IcedReady,
    ActionBar(action_bar::ActionBarEvent),
    Sidebar(sidebar_menu::SidebarMenuEvent),
    SidebarWorkspace(sidebar_workspace::SidebarWorkspaceEvent),
    Explorer(ExplorerEvent),
    QuickLaunch(quick_launch::QuickLaunchEvent),
    ActivateTab {
        tab_id: u64,
    },
    CloseTabRequested {
        tab_id: u64,
    },
    SetTabTitle {
        tab_id: u64,
        title: String,
    },
    OpenCommandTerminalTab {
        title: String,
        settings: Box<Settings>,
    },
    OpenQuickLaunchCommandTerminalTab {
        title: String,
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
    Terminal(TerminalEvent),
    QuickLaunchWizard {
        tab_id: u64,
        event: QuickLaunchWizardEvent,
    },
    Settings(settings::SettingsEvent),
    SettingsApplied(settings::SettingsData),
    Keyboard(iced::keyboard::Event),
    Window(window::Event),
    ResizeWindow(Direction),
}

pub(crate) struct App {
    window_size: Size,
    theme_manager: ThemeManager,
    fonts: FontsConfig,
    terminal_settings: Settings,
    shell_session: ShellSession,
    state: State,
    features: Features,
    is_fullscreen: bool,
}

impl App {
    pub(crate) fn new() -> (Self, Task<Event>) {
        let settings_state = settings::load_initial_settings_state();
        let mut theme_manager = ThemeManager::new();
        let settings_palette = settings_state.draft().to_color_palette();
        theme_manager.set_custom_palette(settings_palette);
        let current_theme = theme_manager.current();
        let fonts = FontsConfig::default();

        let terminal_settings = terminal_settings(current_theme, &fonts);
        let shell_path = settings_state.draft().terminal_shell().to_string();
        let shell_session = match setup_shell_session_with_shell(&shell_path) {
            Ok(session) => session,
            Err(err) => {
                log::warn!("shell integration setup failed: {err}");
                fallback_shell_session_with_shell(&shell_path)
            },
        };

        let window_size = Size {
            width: MIN_WINDOW_WIDTH,
            height: MIN_WINDOW_HEIGHT,
        };
        let screen_size = Self::screen_size_from_window(window_size);
        let state = State::new(window_size, screen_size);
        let features = Features::new(settings_state);

        let app = App {
            window_size,
            theme_manager,
            fonts,
            terminal_settings,
            shell_session,
            state,
            features,
            is_fullscreen: false,
        };

        (app, Task::done(()).map(|_: ()| Event::IcedReady))
    }

    pub(crate) fn title(&self) -> String {
        String::from("OTTY")
    }

    pub(crate) fn theme(&self) -> Theme {
        self.theme_manager.iced_theme()
    }

    pub(crate) fn subscription(&self) -> Subscription<Event> {
        let mut subscriptions = Vec::new();
        for (&tab_id, terminal) in self.features.terminal().tabs() {
            for entry in terminal.terminals().values() {
                let sub = entry.terminal.subscription().with(tab_id).map(
                    |(_tab_id, event)| {
                        Event::Terminal(TerminalEvent::Widget(event))
                    },
                );
                subscriptions.push(sub);
            }
        }

        let terminal_subs = Subscription::batch(subscriptions);
        let win_subs =
            window::events().map(|(_id, event)| Event::Window(event));
        let key_subs = iced::keyboard::listen().map(Event::Keyboard);

        let mut subs = vec![terminal_subs, win_subs, key_subs];
        if self.features.quick_launch().has_active_launches()
            || self.features.quick_launch().is_dirty()
            || self.features.quick_launch().is_persist_in_flight()
        {
            subs.push(
                iced::time::every(Duration::from_millis(
                    quick_launch::QUICK_LAUNCHES_TICK_MS,
                ))
                .map(|_| {
                    Event::QuickLaunch(quick_launch::QuickLaunchEvent::Tick)
                }),
            );
        }

        Subscription::batch(subs)
    }

    pub(crate) fn update(&mut self, event: Event) -> Task<Event> {
        let mut pre_dispatch_tasks = Vec::new();
        if self.features.quick_launch().inline_edit().is_some()
            && inline_edit_guard(&event)
        {
            let ctx =
                quick_launch_ctx(&self.terminal_settings, &self.state.sidebar);
            pre_dispatch_tasks.push(self.features.quick_launch_mut().reduce(
                quick_launch::QuickLaunchEvent::CancelInlineEdit,
                &ctx,
            ));
        }

        if self.any_context_menu_open() {
            match context_menu_guard(&event) {
                MenuGuard::Allow => {},
                MenuGuard::Ignore => return Task::none(),
                MenuGuard::Dismiss => return self.close_all_context_menus(),
            }
        }

        let tabs_before = self.state.tab.len();
        let dispatch_task = self.dispatch_event(event);
        let task = if pre_dispatch_tasks.is_empty() {
            dispatch_task
        } else {
            pre_dispatch_tasks.push(dispatch_task);
            Task::batch(pre_dispatch_tasks)
        };

        if self.state.tab.len() > tabs_before {
            Task::batch(vec![task, snap_to_end(tab_bar::TAB_BAR_SCROLL_ID)])
        } else {
            task
        }
    }

    fn dispatch_event(&mut self, event: Event) -> Task<Event> {
        use Event::*;

        match event {
            IcedReady => self.open_terminal_tab(),
            ActionBar(event) => self.handle_action_bar(event),
            Sidebar(event) => self.handle_sidebar(event),
            SidebarWorkspace(event) => self.handle_sidebar_workspace(event),
            QuickLaunch(event) => {
                let ctx = quick_launch_ctx(
                    &self.terminal_settings,
                    &self.state.sidebar,
                );
                self.features.quick_launch_mut().reduce(event, &ctx)
            },
            ActivateTab { tab_id } => self.activate_tab(tab_id),
            CloseTabRequested { tab_id } => self.close_tab(tab_id),
            SetTabTitle { tab_id, title } => {
                self.state.tab.set_title(tab_id, title);
                Task::none()
            },
            OpenCommandTerminalTab { title, settings } => {
                self.open_command_terminal_tab(title, *settings)
            },
            OpenQuickLaunchCommandTerminalTab {
                title,
                settings,
                command,
            } => self.open_quick_launch_command_terminal_tab(
                title, *settings, command,
            ),
            OpenQuickLaunchWizardCreateTab { parent_path } => {
                self.open_quick_launch_wizard_create_tab(parent_path)
            },
            OpenQuickLaunchWizardEditTab { path, command } => {
                self.open_quick_launch_wizard_edit_tab(path, *command)
            },
            OpenQuickLaunchErrorTab { title, message } => {
                self.open_quick_launch_error_tab(title, message)
            },
            Explorer(event) => {
                let active_tab_id = self.state.active_tab_id();
                let editor_command =
                    self.features.settings().terminal_editor().to_string();
                let active_shell_cwd = shell_cwd_for_active_tab(
                    active_tab_id,
                    self.features.terminal(),
                );
                self.features.explorer_mut().reduce(
                    event,
                    &ExplorerCtx {
                        active_shell_cwd,
                        terminal_settings: &self.terminal_settings,
                        editor_command: &editor_command,
                    },
                )
            },
            Terminal(event) => {
                if let TerminalEvent::PaneGridCursorMoved { position, .. } =
                    &event
                {
                    self.state.sidebar.update_cursor(*position);
                }
                let ctx = self.make_terminal_ctx();
                let sync_task = self.terminal_sync_followup(&event);
                let terminal_task =
                    self.features.terminal_mut().reduce(event, &ctx);
                Task::batch(vec![terminal_task, sync_task])
            },
            QuickLaunchWizard { tab_id, event } => self
                .features
                .quick_launch_wizard_mut()
                .reduce(event, &QuickLaunchWizardCtx { tab_id }),
            Settings(event) => self.features.settings_mut().reduce(event, &()),
            SettingsApplied(settings) => self.apply_settings(&settings),
            Keyboard(event) => self.handle_keyboard(event),
            Window(window::Event::Resized(size)) => {
                self.window_size = size;
                self.state.window_size = size;
                self.state
                    .set_screen_size(Self::screen_size_from_window(size));
                self.sync_terminal_grid_sizes();
                Task::none()
            },
            Window(_) => Task::none(),
            ResizeWindow(dir) => window::latest()
                .and_then(move |id| window::drag_resize(id, dir)),
        }
    }

    fn handle_keyboard(&mut self, event: iced::keyboard::Event) -> Task<Event> {
        if let iced::keyboard::Event::KeyPressed { key, .. } = event {
            let ctx =
                quick_launch_ctx(&self.terminal_settings, &self.state.sidebar);
            if matches!(
                key,
                iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape)
            ) && self.features.quick_launch().inline_edit().is_some()
            {
                return self.features.quick_launch_mut().reduce(
                    quick_launch::QuickLaunchEvent::CancelInlineEdit,
                    &ctx,
                );
            }

            if matches!(
                key,
                iced::keyboard::Key::Named(iced::keyboard::key::Named::Delete)
            ) && self.features.quick_launch().inline_edit().is_none()
            {
                return self.features.quick_launch_mut().reduce(
                    quick_launch::QuickLaunchEvent::DeleteSelected,
                    &ctx,
                );
            }
        }

        Task::none()
    }

    fn terminal_sync_followup(&self, event: &TerminalEvent) -> Task<Event> {
        let should_sync = matches!(
            event,
            TerminalEvent::PaneClicked { .. }
                | TerminalEvent::SplitPane { .. }
                | TerminalEvent::ClosePane { .. }
                | TerminalEvent::Widget(
                    otty_ui_term::Event::ContentSync { .. }
                )
        );

        if should_sync {
            Task::done(Event::Explorer(ExplorerEvent::SyncFromActiveTerminal))
        } else {
            Task::none()
        }
    }

    fn apply_settings(
        &mut self,
        settings: &settings::SettingsData,
    ) -> Task<Event> {
        let palette = settings.to_color_palette();
        self.theme_manager.set_custom_palette(palette);
        let current_theme = self.theme_manager.current();
        self.terminal_settings = terminal_settings(current_theme, &self.fonts);
        let terminal_palette = current_theme.terminal_palette().clone();

        match setup_shell_session_with_shell(settings.terminal_shell()) {
            Ok(session) => self.shell_session = session,
            Err(err) => {
                log::warn!("shell integration setup failed: {err}");
                self.shell_session = fallback_shell_session_with_shell(
                    settings.terminal_shell(),
                );
            },
        }

        let ctx = self.make_terminal_ctx();
        self.features.terminal_mut().reduce(
            TerminalEvent::ApplyTheme {
                palette: Box::new(terminal_palette),
            },
            &ctx,
        )
    }

    pub(crate) fn view(&self) -> Element<'_, Event, Theme, iced::Renderer> {
        let theme = self.theme_manager.current();
        let theme_props: ThemeProps<'_> = ThemeProps::new(theme);

        let tab_summaries = self.state.tab_summaries();
        let active_tab_id = self.state.active_tab_id().unwrap_or(0);

        let header = self.view_header(theme_props);

        let content_row: Element<'_, Event, Theme, iced::Renderer> = if self
            .state
            .sidebar
            .is_hidden()
        {
            self.view_content_only(theme_props, &tab_summaries, active_tab_id)
        } else {
            self.view_sidebar_layout(theme_props, &tab_summaries, active_tab_id)
        };

        let content_row =
            mouse_area(content_row).on_move(|position| {
                Event::SidebarWorkspace(
                    sidebar_workspace::SidebarWorkspaceEvent::WorkspaceCursorMoved {
                        position,
                    },
                )
            });

        let mut content_layers: Vec<Element<'_, Event, Theme, iced::Renderer>> =
            vec![content_row.into()];

        if let Some(overlay) = self.view_context_menu_overlay(theme_props) {
            content_layers.push(overlay);
        }

        let content_stack = iced::widget::Stack::with_children(content_layers)
            .width(Length::Fill)
            .height(Length::Fill);

        let resize_grips_layer = if self.any_context_menu_open() {
            container(Space::new())
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        } else {
            resize_grips::view()
        };

        let root_layers: Vec<Element<'_, Event, Theme, iced::Renderer>> = vec![
            column![header, content_stack]
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
            resize_grips_layer,
        ];

        iced::widget::Stack::with_children(root_layers)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// Render the action bar and its bottom separator line.
    fn view_header<'a>(
        &'a self,
        theme_props: ThemeProps<'a>,
    ) -> Element<'a, Event, Theme, iced::Renderer> {
        let header = action_bar::view(action_bar::ActionBarProps {
            title: self.state.active_tab_title().unwrap_or("OTTY"),
            theme: theme_props,
            fonts: &self.fonts,
        })
        .map(Event::ActionBar);

        let palette = theme_props.theme.iced_palette();
        let separator = container(Space::new())
            .width(Length::Fill)
            .height(Length::Fixed(HEADER_SEPARATOR_HEIGHT))
            .style(move |_| {
                let mut background = palette.dim_white;
                background.a = SEPARATOR_ALPHA;
                iced::widget::container::Style {
                    background: Some(background.into()),
                    ..Default::default()
                }
            });

        column![header, separator]
            .width(Length::Fill)
            .height(Length::Shrink)
            .into()
    }

    /// Render the tab bar + content area when the sidebar is hidden.
    fn view_content_only<'a>(
        &'a self,
        theme_props: ThemeProps<'a>,
        tab_summaries: &[(u64, &'a str)],
        active_tab_id: u64,
    ) -> Element<'a, Event, Theme, iced::Renderer> {
        let tab_bar = tab_bar::view(tab_bar::TabBarProps {
            tabs: tab_summaries.to_vec(),
            active_tab_id,
            theme: theme_props,
        })
        .map(|e| match e {
            tab_bar::TabBarEvent::ActivateTab { tab_id } => {
                Event::ActivateTab { tab_id }
            },
            tab_bar::TabBarEvent::CloseTab { tab_id } => {
                Event::CloseTabRequested { tab_id }
            },
        });

        let content = tab_content::view(tab_content::TabContentProps {
            active_tab: self.state.active_tab(),
            terminal: self.features.terminal().state(),
            quick_launch_wizard: self.features.quick_launch_wizard().state(),
            quick_launches: self.features.quick_launch().state(),
            settings: self.features.settings().state(),
            theme: theme_props,
        })
        .map(map_tab_content_event);

        container(
            column![tab_bar, content]
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    /// Render the sidebar menu + workspace split + content area.
    fn view_sidebar_layout<'a>(
        &'a self,
        theme_props: ThemeProps<'a>,
        tab_summaries: &[(u64, &'a str)],
        active_tab_id: u64,
    ) -> Element<'a, Event, Theme, iced::Renderer> {
        let sidebar_menu = sidebar_menu::view(sidebar_menu::SidebarMenuProps {
            active_item: match self.state.sidebar.active_item() {
                SidebarItem::Terminal => {
                    sidebar_menu::SidebarMenuItem::Terminal
                },
                SidebarItem::Explorer => {
                    sidebar_menu::SidebarMenuItem::Explorer
                },
            },
            workspace_open: self.state.sidebar.is_workspace_open(),
            menu_width: SIDEBAR_MENU_WIDTH,
            theme: theme_props,
        })
        .map(Event::Sidebar);

        let palette = theme_props.theme.iced_palette();
        let sidebar_separator = container(Space::new())
            .width(Length::Fixed(SIDEBAR_SEPARATOR_WIDTH))
            .height(Length::Fill)
            .style(move |_| {
                let mut background = palette.dim_white;
                background.a = SEPARATOR_ALPHA;
                iced::widget::container::Style {
                    background: Some(background.into()),
                    ..Default::default()
                }
            });

        let state_ref = &self.state;
        let explorer_feature = self.features.explorer();
        let terminal_state = self.features.terminal().state();
        let quick_launches_state = self.features.quick_launch().state();
        let wizard_state = self.features.quick_launch_wizard().state();
        let settings_state = self.features.settings().state();
        let workspace_open = self.state.sidebar.is_workspace_open();

        let sidebar_split = pane_grid::PaneGrid::new(
            self.state.sidebar.panes(),
            move |_, pane, _| match pane {
                SidebarPane::Workspace => {
                    let workspace_content = sidebar_workspace::view(
                        sidebar_workspace::SidebarWorkspaceProps {
                            active_item: match state_ref.sidebar.active_item() {
                                SidebarItem::Terminal => {
                                    sidebar_workspace::SidebarWorkspaceItem::Terminal
                                },
                                SidebarItem::Explorer => {
                                    sidebar_workspace::SidebarWorkspaceItem::Explorer
                                },
                            },
                            quick_launches: quick_launches_state,
                            explorer: explorer_feature,
                            theme: theme_props,
                        },
                    )
                    .map(Event::SidebarWorkspace);
                    let workspace = sidebar_workspace_panel::view(
                        sidebar_workspace_panel::SidebarWorkspacePanelProps {
                            content: workspace_content,
                            visible: workspace_open,
                            theme: theme_props,
                        },
                    );
                    pane_grid::Content::new(workspace)
                },
                SidebarPane::Content => {
                    let tab_bar = tab_bar::view(tab_bar::TabBarProps {
                        tabs: tab_summaries.to_vec(),
                        active_tab_id,
                        theme: theme_props,
                    })
                    .map(|e| match e {
                        tab_bar::TabBarEvent::ActivateTab { tab_id } => {
                            Event::ActivateTab { tab_id }
                        },
                        tab_bar::TabBarEvent::CloseTab { tab_id } => {
                            Event::CloseTabRequested { tab_id }
                        },
                    });

                    let content = tab_content::view(tab_content::TabContentProps {
                        active_tab: state_ref.active_tab(),
                        terminal: terminal_state,
                        quick_launch_wizard: wizard_state,
                        quick_launches: quick_launches_state,
                        settings: settings_state,
                        theme: theme_props,
                    })
                    .map(map_tab_content_event);

                    pane_grid::Content::new(
                        container(
                            column![tab_bar, content]
                                .width(Length::Fill)
                                .height(Length::Fill),
                        )
                        .width(Length::Fill)
                        .height(Length::Fill),
                    )
                },
            },
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(0)
        .min_size(0)
        .on_resize(10.0, |event| {
            Event::Sidebar(sidebar_menu::SidebarMenuEvent::Resized(event))
        });

        row![sidebar_menu, sidebar_separator, sidebar_split]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// Render the context menu overlay layer, if any menu is open.
    fn view_context_menu_overlay<'a>(
        &'a self,
        theme_props: ThemeProps<'a>,
    ) -> Option<Element<'a, Event, Theme, iced::Renderer>> {
        self.context_menu_layer(theme_props, self.state.screen_size)
    }

    fn handle_action_bar(
        &mut self,
        event: action_bar::ActionBarEvent,
    ) -> Task<Event> {
        use action_bar::ActionBarEvent::*;

        match event {
            ToggleFullScreen => self.toggle_full_screen(),
            MinimizeWindow => {
                window::latest().and_then(|id| window::minimize(id, true))
            },
            CloseWindow => close_window(),
            ToggleSidebarVisibility => {
                self.state.sidebar.toggle_visibility();
                self.sync_terminal_grid_sizes();
                Task::none()
            },
            StartWindowDrag => window::latest().and_then(window::drag),
        }
    }

    fn handle_sidebar(
        &mut self,
        event: sidebar_menu::SidebarMenuEvent,
    ) -> Task<Event> {
        match event {
            sidebar_menu::SidebarMenuEvent::SelectItem(item) => {
                let canonical = match item {
                    sidebar_menu::SidebarMenuItem::Terminal => {
                        SidebarItem::Terminal
                    },
                    sidebar_menu::SidebarMenuItem::Explorer => {
                        SidebarItem::Explorer
                    },
                };
                self.state.sidebar.set_active_item(canonical);
                self.ensure_sidebar_workspace_open();
                Task::none()
            },
            sidebar_menu::SidebarMenuEvent::OpenSettings => {
                self.open_settings_tab()
            },
            sidebar_menu::SidebarMenuEvent::ToggleWorkspace => {
                self.toggle_sidebar_workspace();
                Task::none()
            },
            sidebar_menu::SidebarMenuEvent::Resized(event) => {
                self.handle_sidebar_resize(event);
                Task::none()
            },
        }
    }

    fn handle_sidebar_workspace(
        &mut self,
        event: sidebar_workspace::SidebarWorkspaceEvent,
    ) -> Task<Event> {
        match event {
            sidebar_workspace::SidebarWorkspaceEvent::TerminalAddMenuOpen => {
                self.state.sidebar.open_add_menu();
                Task::none()
            },
            sidebar_workspace::SidebarWorkspaceEvent::TerminalAddMenuDismiss => {
                self.state.sidebar.dismiss_add_menu();
                Task::none()
            },
            sidebar_workspace::SidebarWorkspaceEvent::TerminalAddMenuAction(action) => {
                self.state.sidebar.dismiss_add_menu();
                match action {
                    sidebar_workspace::SidebarWorkspaceAddMenuAction::CreateTab => {
                        self.open_terminal_tab()
                    },
                    sidebar_workspace::SidebarWorkspaceAddMenuAction::CreateCommand => {
                        let ctx = quick_launch_ctx(
                            &self.terminal_settings,
                            &self.state.sidebar,
                        );
                        self.features.quick_launch_mut().reduce(
                            quick_launch::QuickLaunchEvent::HeaderCreateCommand,
                            &ctx,
                        )
                    },
                    sidebar_workspace::SidebarWorkspaceAddMenuAction::CreateFolder => {
                        let ctx = quick_launch_ctx(
                            &self.terminal_settings,
                            &self.state.sidebar,
                        );
                        self.features.quick_launch_mut().reduce(
                            quick_launch::QuickLaunchEvent::HeaderCreateFolder,
                            &ctx,
                        )
                    },
                }
            },
            sidebar_workspace::SidebarWorkspaceEvent::WorkspaceCursorMoved { position } => {
                self.state.sidebar.update_cursor(position);
                Task::none()
            },
            sidebar_workspace::SidebarWorkspaceEvent::QuickLaunch(event) => {
                Task::done(Event::QuickLaunch(event))
            },
            sidebar_workspace::SidebarWorkspaceEvent::Explorer(event) => {
                Task::done(Event::Explorer(event))
            },
        }
    }

    fn ensure_sidebar_workspace_open(&mut self) {
        if self
            .state
            .sidebar
            .ensure_workspace_open(max_sidebar_workspace_ratio())
        {
            self.sync_terminal_grid_sizes();
        }
    }

    fn toggle_sidebar_workspace(&mut self) {
        self.state
            .sidebar
            .toggle_workspace(max_sidebar_workspace_ratio());
        self.sync_terminal_grid_sizes();
    }

    fn handle_sidebar_resize(&mut self, event: pane_grid::ResizeEvent) {
        self.state.sidebar.mark_resizing();
        let ctx =
            quick_launch_ctx(&self.terminal_settings, &self.state.sidebar);
        let _task = self.features.quick_launch_mut().reduce(
            quick_launch::QuickLaunchEvent::ResetInteractionState,
            &ctx,
        );
        if self
            .state
            .sidebar
            .apply_resize(event, max_sidebar_workspace_ratio())
        {
            self.sync_terminal_grid_sizes();
        }
    }

    fn toggle_full_screen(&mut self) -> Task<Event> {
        self.is_fullscreen = !self.is_fullscreen;

        let mode = if self.is_fullscreen {
            window::Mode::Fullscreen
        } else {
            window::Mode::Windowed
        };

        window::latest().and_then(move |id| window::set_mode(id, mode))
    }

    /// Build a terminal context snapshot from the current app state.
    fn make_terminal_ctx(&self) -> TerminalCtx {
        TerminalCtx {
            active_tab_id: self.state.active_tab_id(),
            pane_grid_size: self.state.pane_grid_size(),
            screen_size: self.state.screen_size,
            sidebar_cursor: self.state.sidebar.cursor(),
        }
    }

    /// Propagate the current pane grid size to the terminal feature.
    fn sync_terminal_grid_sizes(&mut self) {
        let size = self.state.pane_grid_size();
        self.features.terminal_mut().set_grid_size(size);
    }

    fn screen_size_from_window(window_size: Size) -> Size {
        let action_bar_height = action_bar::ACTION_BAR_HEIGHT;
        let height =
            (window_size.height - action_bar_height - SIDEBAR_SEPARATOR_WIDTH)
                .max(0.0);
        Size::new(window_size.width, height)
    }

    fn any_context_menu_open(&self) -> bool {
        if self.state.sidebar.add_menu().is_some()
            || self.features.quick_launch().context_menu().is_some()
        {
            return true;
        }

        self.features.terminal().has_any_context_menu()
    }

    fn close_all_context_menus(&mut self) -> Task<Event> {
        self.state.sidebar.dismiss_add_menu();
        let ctx =
            quick_launch_ctx(&self.terminal_settings, &self.state.sidebar);
        let quick_launch_task = self
            .features
            .quick_launch_mut()
            .reduce(quick_launch::QuickLaunchEvent::ContextMenuDismiss, &ctx);
        let ctx = self.make_terminal_ctx();
        let terminal_task = self
            .features
            .terminal_mut()
            .reduce(TerminalEvent::CloseAllContextMenus, &ctx);
        Task::batch(vec![quick_launch_task, terminal_task])
    }

    fn context_menu_layer<'a>(
        &'a self,
        theme: ThemeProps<'a>,
        area_size: Size,
    ) -> Option<Element<'a, Event, Theme, iced::Renderer>> {
        if let Some(menu) = self.state.sidebar.add_menu() {
            return Some(
                sidebar_workspace_add_menu::view(
                    sidebar_workspace_add_menu::SidebarWorkspaceAddMenuProps {
                        cursor: menu.cursor,
                        theme,
                        area_size,
                    },
                )
                .map(Event::SidebarWorkspace),
            );
        }

        if let Some(menu) = self.features.quick_launch().context_menu() {
            return Some(
                quick_launches_context_menu::view(
                    quick_launches_context_menu::QuickLaunchesContextMenuProps {
                        menu,
                        theme,
                        area_size,
                        launching: self.features.quick_launch().launching(),
                    },
                )
                .map(|event| {
                    Event::SidebarWorkspace(
                        sidebar_workspace::SidebarWorkspaceEvent::QuickLaunch(
                            event,
                        ),
                    )
                }),
            );
        }

        for (&tab_id, terminal) in self.features.terminal().tabs() {
            if let Some(menu) = terminal.context_menu() {
                let has_block_selection = terminal.selected_block_terminal()
                    == Some(menu.terminal_id());
                return Some(
                    terminal_pane_context_menu::view(
                        terminal_pane_context_menu::TerminalPaneContextMenuProps {
                            tab_id,
                            pane: menu.pane(),
                            cursor: menu.cursor(),
                            grid_size: menu.grid_size(),
                            terminal_id: menu.terminal_id(),
                            focus_target: menu.focus_target().clone(),
                            has_block_selection,
                            theme,
                        },
                    )
                    .map(Event::Terminal),
                );
            }
        }

        None
    }

    fn activate_tab(&mut self, tab_id: u64) -> Task<Event> {
        tab::activate_tab(&mut self.state, tab_id)
    }

    fn close_tab(&mut self, tab_id: u64) -> Task<Event> {
        tab::close_tab(&mut self.state, tab_id)
    }

    fn open_terminal_tab(&mut self) -> Task<Event> {
        tab::open_terminal_tab(
            &mut self.state,
            self.features.terminal_mut(),
            &self.shell_session,
            &self.terminal_settings,
        )
    }

    fn open_settings_tab(&mut self) -> Task<Event> {
        tab::open_settings_tab(&mut self.state)
    }

    fn open_command_terminal_tab(
        &mut self,
        title: String,
        tab_settings: Settings,
    ) -> Task<Event> {
        tab::open_command_terminal_tab(
            &mut self.state,
            self.features.terminal_mut(),
            title,
            tab_settings,
        )
    }

    fn open_quick_launch_command_terminal_tab(
        &mut self,
        title: String,
        tab_settings: Settings,
        command: Box<quick_launch::QuickLaunch>,
    ) -> Task<Event> {
        tab::open_quick_launch_command_terminal_tab(
            &mut self.state,
            self.features.terminal_mut(),
            title,
            tab_settings,
            command,
        )
    }

    fn open_quick_launch_wizard_create_tab(
        &mut self,
        parent_path: quick_launch::NodePath,
    ) -> Task<Event> {
        tab::open_quick_launch_wizard_create_tab(&mut self.state, parent_path)
    }

    fn open_quick_launch_wizard_edit_tab(
        &mut self,
        path: quick_launch::NodePath,
        command: quick_launch::QuickLaunch,
    ) -> Task<Event> {
        tab::open_quick_launch_wizard_edit_tab(&mut self.state, path, command)
    }

    fn open_quick_launch_error_tab(
        &mut self,
        title: String,
        message: String,
    ) -> Task<Event> {
        tab::open_quick_launch_error_tab(&mut self.state, title, message)
    }
}

/// Build a [`quick_launch::QuickLaunchCtx`] from individual app state fields.
///
/// Kept as a free function so Rust can split the field borrows: callers can
/// take `&mut self.features` immediately after calling this with
/// `&self.terminal_settings` and `&self.state.sidebar`.
fn quick_launch_ctx<'a>(
    terminal_settings: &'a otty_ui_term::settings::Settings,
    sidebar: &crate::state::SidebarState,
) -> quick_launch::QuickLaunchCtx<'a> {
    quick_launch::QuickLaunchCtx {
        terminal_settings,
        sidebar_cursor: sidebar.cursor(),
        sidebar_is_resizing: sidebar.is_resizing(),
    }
}

fn map_tab_content_event(event: tab_content::TabContentEvent) -> Event {
    match event {
        tab_content::TabContentEvent::Terminal(event) => Event::Terminal(event),
        tab_content::TabContentEvent::Settings(event) => Event::Settings(event),
        tab_content::TabContentEvent::QuickLaunchWizard { tab_id, event } => {
            Event::QuickLaunchWizard { tab_id, event }
        },
        tab_content::TabContentEvent::QuickLaunchError(event) => match event {},
    }
}

fn terminal_settings(theme: &AppTheme, fonts: &FontsConfig) -> Settings {
    let font_settings = FontSettings {
        size: fonts.terminal.size,
        font_type: fonts.terminal.font_type,
        ..FontSettings::default()
    };
    let theme_settings =
        ThemeSettings::new(Box::new(theme.terminal_palette().clone()));

    Settings {
        font: font_settings,
        theme: theme_settings,
        backend: BackendSettings::default(),
    }
}

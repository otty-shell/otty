use iced::mouse;
use iced::widget::operation::snap_to_end;
use iced::widget::{
    Space, column, container, mouse_area, pane_grid, row, stack, text,
};
use iced::window::Direction;
use iced::{Element, Length, Size, Subscription, Task, Theme, window};
use otty_ui_term::settings::{
    BackendSettings, FontSettings, Settings, ThemeSettings,
};
use std::time::Duration;

use crate::effects::close_window;
use crate::features::explorer;
use crate::features::quick_launches::{
    self as quick_launches, QuickLaunchEditorEvent, quick_launch_editor_reducer,
};
use crate::features::settings;
use crate::features::tab::{TabContent, TabEvent, TabOpenRequest, tab_reducer};
use crate::features::terminal::{
    ShellSession, TerminalEvent, fallback_shell_session_with_shell,
    setup_shell_session_with_shell, terminal_reducer,
};
use crate::fonts::FontsConfig;
use crate::state::{
    SIDEBAR_COLLAPSE_WORKSPACE_RATIO, SIDEBAR_DEFAULT_WORKSPACE_RATIO,
    SIDEBAR_MIN_TAB_CONTENT_RATIO, SidebarItem, SidebarPane, State,
};
use crate::theme::{AppTheme, ThemeManager, ThemeProps};
use crate::ui::widgets::action_bar;
use crate::ui::widgets::sidebar;
use crate::ui::widgets::sidebar_workspace;
use crate::ui::widgets::tab_bar;
use crate::ui::widgets::tab_content;

pub(crate) const MIN_WINDOW_WIDTH: f32 = 800.0;
pub(crate) const MIN_WINDOW_HEIGHT: f32 = 600.0;
const RESIZE_EDGE_MOUSE_AREA_THICKNESS: f32 = 6.0;
const RESIZE_CORNER_MOUSE_AREA_THICKNESS: f32 = 12.0;
const HEADER_SEPARATOR_HEIGHT: f32 = 1.0;
const SIDEBAR_SEPARATOR_WIDTH: f32 = 0.3;
const SEPARATOR_ALPHA: f32 = 0.3;

/// App-wide events that drive the root update loop.
#[derive(Debug, Clone)]
pub(crate) enum Event {
    IcedReady,
    ActionBar(action_bar::Event),
    Sidebar(sidebar::Event),
    SidebarWorkspace(sidebar_workspace::Event),
    Tab(TabEvent),
    Terminal(TerminalEvent),
    QuickLaunchEditor {
        tab_id: u64,
        event: QuickLaunchEditorEvent,
    },
    Settings(settings::SettingsEvent),
    SettingsApplied(settings::SettingsData),
    QuickLaunchSetupCompleted(Box<quick_launches::QuickLaunchSetupOutcome>),
    QuickLaunchTick,
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
    is_fullscreen: bool,
}

impl App {
    pub(crate) fn new() -> (Self, Task<Event>) {
        let settings_state = settings::load_settings_state();
        let mut theme_manager = ThemeManager::new();
        let settings_palette = settings_state.draft().to_color_palette();
        theme_manager.set_custom_palette(settings_palette);
        let current_theme = theme_manager.current();
        let fonts = FontsConfig::default();

        let terminal_settings = terminal_settings(current_theme, &fonts);
        let shell_session = match setup_shell_session_with_shell(
            settings_state.draft().terminal_shell(),
        ) {
            Ok(session) => session,
            Err(err) => {
                log::warn!("shell integration setup failed: {err}");
                fallback_shell_session_with_shell(
                    settings_state.draft().terminal_shell(),
                )
            },
        };

        let window_size = Size {
            width: MIN_WINDOW_WIDTH,
            height: MIN_WINDOW_HEIGHT,
        };
        let screen_size = Self::screen_size_from_window(window_size);
        let state = State::new(window_size, screen_size, settings_state);

        let app = App {
            window_size,
            theme_manager,
            fonts,
            terminal_settings,
            shell_session,
            state,
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
        for (&tab_id, tab) in &self.state.tab_items {
            if let TabContent::Terminal(terminal) = &tab.content {
                for entry in terminal.terminals().values() {
                    let sub = entry.terminal.subscription().with(tab_id).map(
                        |(_tab_id, event)| {
                            Event::Terminal(TerminalEvent::Widget(event))
                        },
                    );
                    subscriptions.push(sub);
                }
            }
        }

        let terminal_subs = Subscription::batch(subscriptions);
        let win_subs =
            window::events().map(|(_id, event)| Event::Window(event));
        let key_subs = iced::keyboard::listen().map(Event::Keyboard);

        let mut subs = vec![terminal_subs, win_subs, key_subs];
        if !self.state.quick_launches.launching.is_empty() {
            subs.push(
                iced::time::every(Duration::from_millis(
                    quick_launches::QUICK_LAUNCHES_TICK_MS,
                ))
                .map(|_| Event::QuickLaunchTick),
            );
        }

        Subscription::batch(subs)
    }

    pub(crate) fn update(&mut self, event: Event) -> Task<Event> {
        if self.state.quick_launches.inline_edit.is_some()
            && should_cancel_inline_edit(&event)
        {
            self.state.quick_launches.inline_edit = None;
        }

        if self.any_context_menu_open() {
            match context_menu_guard(&event) {
                MenuGuard::Allow => {},
                MenuGuard::Ignore => return Task::none(),
                MenuGuard::Dismiss => return self.close_all_context_menus(),
            }
        }

        let tabs_before = self.state.tab_items.len();
        let task = self.dispatch_event(event);

        if self.state.tab_items.len() > tabs_before {
            Task::batch(vec![task, snap_to_end(tab_bar::TAB_BAR_SCROLL_ID)])
        } else {
            task
        }
    }

    fn dispatch_event(&mut self, event: Event) -> Task<Event> {
        use Event::*;

        match event {
            IcedReady => tab_reducer(
                &mut self.state,
                &self.terminal_settings,
                &self.shell_session,
                TabEvent::NewTab {
                    request: TabOpenRequest::Terminal,
                },
            ),
            ActionBar(event) => self.handle_action_bar(event),
            Sidebar(event) => self.handle_sidebar(event),
            SidebarWorkspace(event) => self.handle_sidebar_workspace(event),
            Tab(event) => tab_reducer(
                &mut self.state,
                &self.terminal_settings,
                &self.shell_session,
                event,
            ),
            QuickLaunchSetupCompleted(result) => {
                quick_launches::quick_launches_reducer(
                    &mut self.state,
                    &self.terminal_settings,
                    quick_launches::QuickLaunchEvent::SetupCompleted(*result),
                )
            },
            QuickLaunchTick => quick_launches::quick_launches_reducer(
                &mut self.state,
                &self.terminal_settings,
                quick_launches::QuickLaunchEvent::Tick,
            ),
            Terminal(event) => terminal_reducer(&mut self.state, event),
            QuickLaunchEditor { tab_id, event } => {
                quick_launch_editor_reducer(&mut self.state, tab_id, event)
            },
            Settings(event) => {
                settings::settings_reducer(&mut self.state, event)
            },
            SettingsApplied(settings) => {
                self.apply_settings(&settings);
                Task::none()
            },
            Keyboard(event) => self.handle_keyboard(event),
            Window(window::Event::Resized(size)) => {
                self.window_size = size;
                self.state.window_size = size;
                self.state
                    .set_screen_size(Self::screen_size_from_window(size));
                Task::none()
            },
            Window(_) => Task::none(),
            ResizeWindow(dir) => window::latest()
                .and_then(move |id| window::drag_resize(id, dir)),
        }
    }

    fn handle_keyboard(&mut self, event: iced::keyboard::Event) -> Task<Event> {
        if let iced::keyboard::Event::KeyPressed { key, .. } = event {
            if matches!(
                key,
                iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape)
            ) && self.state.quick_launches.inline_edit.is_some()
            {
                self.state.quick_launches.inline_edit = None;
                return Task::none();
            }

            if matches!(
                key,
                iced::keyboard::Key::Named(iced::keyboard::key::Named::Delete)
            ) && self.state.quick_launches.inline_edit.is_none()
            {
                return quick_launches::quick_launches_reducer(
                    &mut self.state,
                    &self.terminal_settings,
                    quick_launches::QuickLaunchEvent::DeleteSelected,
                );
            }
        }

        Task::none()
    }

    fn apply_settings(&mut self, settings: &settings::SettingsData) {
        let palette = settings.to_color_palette();
        self.theme_manager.set_custom_palette(palette);
        let current_theme = self.theme_manager.current();
        self.terminal_settings = terminal_settings(current_theme, &self.fonts);
        let terminal_palette = current_theme.terminal_palette();
        for tab in self.state.tab_items.values_mut() {
            if let TabContent::Terminal(terminal) = &mut tab.content {
                terminal.apply_theme(terminal_palette.clone());
            }
        }

        match setup_shell_session_with_shell(settings.terminal_shell()) {
            Ok(session) => self.shell_session = session,
            Err(err) => {
                log::warn!("shell integration setup failed: {err}");
                self.shell_session = fallback_shell_session_with_shell(
                    settings.terminal_shell(),
                );
            },
        }
    }

    pub(crate) fn view(&self) -> Element<'_, Event, Theme, iced::Renderer> {
        let theme = self.theme_manager.current();
        let theme_props: ThemeProps<'_> = ThemeProps::new(theme);

        let header_title = self.state.active_tab_title().unwrap_or("OTTY");

        let header = action_bar::view(action_bar::Props {
            title: header_title,
            theme: theme_props,
            fonts: &self.fonts,
        })
        .map(Event::ActionBar);

        let tab_summaries = self.state.tab_summaries();
        let active_tab_id = self.state.active_tab_id.unwrap_or(0);

        let palette = theme_props.theme.iced_palette();

        let header_separator = container(Space::new())
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

        let content_row: Element<'_, Event, Theme, iced::Renderer> = if self
            .state
            .sidebar
            .hidden
        {
            let tab_bar = tab_bar::view(tab_bar::Props {
                tabs: tab_summaries.clone(),
                active_tab_id,
                theme: theme_props,
            })
            .map(Event::Tab);

            let content = tab_content::view(&self.state, theme_props);
            let content_column = column![tab_bar, content]
                .width(Length::Fill)
                .height(Length::Fill);
            container(content_column)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        } else {
            let sidebar_menu = sidebar::menu_view(sidebar::MenuProps {
                active_item: self.state.sidebar.active_item,
                workspace_open: self.state.sidebar.workspace_open,
                theme: theme_props,
            })
            .map(Event::Sidebar);

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

            let sidebar_state = &self.state;
            let sidebar_open = self.state.sidebar.workspace_open;

            let sidebar_split = pane_grid::PaneGrid::new(
                &self.state.sidebar.panes,
                move |_, pane, _| match pane {
                    SidebarPane::Workspace => {
                        let workspace_content =
                            sidebar_workspace::view(sidebar_state, theme_props)
                                .map(Event::SidebarWorkspace);
                        let workspace =
                            sidebar::workspace_view(sidebar::WorkspaceProps {
                                content: workspace_content,
                                visible: sidebar_open,
                                theme: theme_props,
                            });
                        pane_grid::Content::new(workspace)
                    },
                    SidebarPane::Content => {
                        let tab_bar = tab_bar::view(tab_bar::Props {
                            tabs: tab_summaries.clone(),
                            active_tab_id,
                            theme: theme_props,
                        })
                        .map(Event::Tab);

                        let content =
                            tab_content::view(sidebar_state, theme_props);

                        let content_column = column![tab_bar, content]
                            .width(Length::Fill)
                            .height(Length::Fill);

                        let content_container = container(content_column)
                            .width(Length::Fill)
                            .height(Length::Fill);

                        pane_grid::Content::new(content_container)
                    },
                },
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .spacing(0)
            .min_size(0)
            .on_resize(10.0, |event| {
                Event::Sidebar(sidebar::Event::Resized(event))
            });

            row![sidebar_menu, sidebar_separator, sidebar_split]
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        };

        let content_row = mouse_area(content_row).on_move(|position| {
            Event::SidebarWorkspace(
                sidebar_workspace::Event::WorkspaceCursorMoved { position },
            )
        });

        let mut content_layers: Vec<Element<'_, Event, Theme, iced::Renderer>> =
            vec![content_row.into()];

        if let Some(menu_layer) =
            self.context_menu_layer(theme_props, self.state.screen_size)
        {
            content_layers.push(menu_layer);
        }

        let content_stack = iced::widget::Stack::with_children(content_layers)
            .width(Length::Fill)
            .height(Length::Fill);

        let resize_grips = if self.any_context_menu_open() {
            container(Space::new())
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        } else {
            build_resize_grips()
        };

        let root_layers: Vec<Element<'_, Event, Theme, iced::Renderer>> = vec![
            column![header, header_separator, content_stack]
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
            resize_grips,
        ];

        iced::widget::Stack::with_children(root_layers)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn handle_action_bar(&mut self, event: action_bar::Event) -> Task<Event> {
        use action_bar::Event::*;

        match event {
            ToggleFullScreen => self.toggle_full_screen(),
            MinimizeWindow => {
                window::latest().and_then(|id| window::minimize(id, true))
            },
            CloseWindow => close_window(),
            ToggleSidebarVisibility => {
                self.state.sidebar.hidden = !self.state.sidebar.hidden;
                self.state.sync_tab_grid_sizes();
                Task::none()
            },
            StartWindowDrag => window::latest().and_then(window::drag),
        }
    }

    fn handle_sidebar(&mut self, event: sidebar::Event) -> Task<Event> {
        match event {
            sidebar::Event::SelectItem(item) => {
                self.state.sidebar.active_item = item;
                if matches!(item, SidebarItem::Terminal | SidebarItem::Explorer)
                {
                    self.ensure_sidebar_workspace_open();
                }
                Task::none()
            },
            sidebar::Event::OpenSettings => tab_reducer(
                &mut self.state,
                &self.terminal_settings,
                &self.shell_session,
                TabEvent::NewTab {
                    request: TabOpenRequest::Settings,
                },
            ),
            sidebar::Event::ToggleWorkspace => {
                self.toggle_sidebar_workspace();
                Task::none()
            },
            sidebar::Event::Resized(event) => {
                self.handle_sidebar_resize(event);
                Task::none()
            },
        }
    }

    fn handle_sidebar_workspace(
        &mut self,
        event: sidebar_workspace::Event,
    ) -> Task<Event> {
        match event {
            sidebar_workspace::Event::TerminalAddMenuOpen => {
                self.state.sidebar.add_menu =
                    Some(crate::state::SidebarAddMenuState {
                        cursor: self.state.sidebar.cursor,
                    });
                Task::none()
            },
            sidebar_workspace::Event::TerminalAddMenuDismiss => {
                self.state.sidebar.add_menu = None;
                Task::none()
            },
            sidebar_workspace::Event::TerminalAddMenuAction(action) => {
                self.state.sidebar.add_menu = None;
                match action {
                    sidebar_workspace::AddMenuAction::CreateTab => tab_reducer(
                        &mut self.state,
                        &self.terminal_settings,
                        &self.shell_session,
                        TabEvent::NewTab {
                            request: TabOpenRequest::Terminal,
                        },
                    ),
                    sidebar_workspace::AddMenuAction::CreateCommand => {
                        quick_launches::quick_launches_reducer(
                            &mut self.state,
                            &self.terminal_settings,
                            quick_launches::QuickLaunchEvent::HeaderCreateCommand,
                        )
                    },
                    sidebar_workspace::AddMenuAction::CreateFolder => {
                        quick_launches::quick_launches_reducer(
                            &mut self.state,
                            &self.terminal_settings,
                            quick_launches::QuickLaunchEvent::HeaderCreateFolder,
                        )
                    },
                }
            },
            sidebar_workspace::Event::WorkspaceCursorMoved { position } => {
                self.state.sidebar.cursor = position;
                Task::none()
            },
            sidebar_workspace::Event::QuickLaunch(event) => {
                quick_launches::quick_launches_reducer(
                    &mut self.state,
                    &self.terminal_settings,
                    event,
                )
            },
            sidebar_workspace::Event::Explorer(event) => {
                explorer::event::explorer_reducer(
                    &mut self.state,
                    &self.terminal_settings,
                    event,
                )
            },
        }
    }

    fn ensure_sidebar_workspace_open(&mut self) {
        if self.state.sidebar.workspace_open {
            return;
        }

        let ratio = self
            .state
            .sidebar
            .workspace_ratio
            .max(SIDEBAR_DEFAULT_WORKSPACE_RATIO)
            .min(max_sidebar_workspace_ratio());

        self.state.sidebar.workspace_open = true;
        self.state.sidebar.workspace_ratio = ratio;
        self.state
            .sidebar
            .panes
            .resize(self.state.sidebar.split, ratio);
        self.state.sync_tab_grid_sizes();
    }

    fn toggle_sidebar_workspace(&mut self) {
        if self.state.sidebar.workspace_open {
            self.state.sidebar.workspace_open = false;
            self.state
                .sidebar
                .panes
                .resize(self.state.sidebar.split, 0.0);
        } else {
            let ratio = self
                .state
                .sidebar
                .workspace_ratio
                .max(SIDEBAR_DEFAULT_WORKSPACE_RATIO)
                .min(max_sidebar_workspace_ratio());
            self.state.sidebar.workspace_open = true;
            self.state.sidebar.workspace_ratio = ratio;
            self.state
                .sidebar
                .panes
                .resize(self.state.sidebar.split, ratio);
        }

        self.state.sync_tab_grid_sizes();
    }

    fn handle_sidebar_resize(&mut self, event: pane_grid::ResizeEvent) {
        self.state.sidebar.mark_resizing();
        self.state.quick_launches.hovered = None;
        self.state.quick_launches.pressed = None;
        self.state.quick_launches.drag = None;
        self.state.quick_launches.drop_target = None;
        let max_ratio = max_sidebar_workspace_ratio();

        if !self.state.sidebar.workspace_open {
            if event.ratio <= SIDEBAR_COLLAPSE_WORKSPACE_RATIO {
                self.state
                    .sidebar
                    .panes
                    .resize(self.state.sidebar.split, 0.0);
                return;
            }

            let ratio = SIDEBAR_COLLAPSE_WORKSPACE_RATIO.min(max_ratio);
            self.state.sidebar.workspace_open = true;
            self.state.sidebar.workspace_ratio = ratio;
            self.state
                .sidebar
                .panes
                .resize(self.state.sidebar.split, ratio);
            self.state.sync_tab_grid_sizes();
            return;
        }

        if event.ratio <= SIDEBAR_COLLAPSE_WORKSPACE_RATIO {
            self.state.sidebar.workspace_open = false;
            self.state
                .sidebar
                .panes
                .resize(self.state.sidebar.split, 0.0);
            self.state.sync_tab_grid_sizes();
            return;
        }

        let ratio = event.ratio.min(max_ratio);
        self.state.sidebar.workspace_ratio = ratio;
        self.state
            .sidebar
            .panes
            .resize(self.state.sidebar.split, ratio);
        self.state.sync_tab_grid_sizes();
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

    fn screen_size_from_window(window_size: Size) -> Size {
        let action_bar_height = action_bar::ACTION_BAR_HEIGHT;
        let height =
            (window_size.height - action_bar_height - SIDEBAR_SEPARATOR_WIDTH)
                .max(0.0);
        Size::new(window_size.width, height)
    }

    fn any_context_menu_open(&self) -> bool {
        if self.state.sidebar.add_menu.is_some()
            || self.state.quick_launches.context_menu.is_some()
        {
            return true;
        }

        self.state.tab_items.values().any(|tab| {
            matches!(
                &tab.content,
                TabContent::Terminal(terminal)
                    if terminal.context_menu().is_some()
            )
        })
    }

    fn close_all_context_menus(&mut self) -> Task<Event> {
        let mut tasks = Vec::new();

        self.state.sidebar.add_menu = None;
        self.state.quick_launches.context_menu = None;

        for tab in self.state.tab_items.values_mut() {
            if let TabContent::Terminal(terminal) = &mut tab.content {
                if terminal.context_menu().is_some() {
                    tasks.push(terminal.close_context_menu());
                }
            }
        }

        Task::batch(tasks)
    }

    fn context_menu_layer<'a>(
        &'a self,
        theme: ThemeProps<'a>,
        area_size: Size,
    ) -> Option<Element<'a, Event, Theme, iced::Renderer>> {
        if let Some(menu) = self.state.sidebar.add_menu.as_ref() {
            return Some(
                sidebar_workspace::add_menu::view(
                    sidebar_workspace::add_menu::Props {
                        menu,
                        theme,
                        area_size,
                    },
                )
                .map(Event::SidebarWorkspace),
            );
        }

        if let Some(menu) = self.state.quick_launches.context_menu.as_ref() {
            return Some(
                crate::ui::widgets::quick_launches::context_menu::view(
                    crate::ui::widgets::quick_launches::context_menu::Props {
                        menu,
                        theme,
                        area_size,
                        launching: &self.state.quick_launches.launching,
                    },
                )
                .map(|event| {
                    Event::SidebarWorkspace(
                        sidebar_workspace::Event::QuickLaunch(event),
                    )
                }),
            );
        }

        for tab in self.state.tab_items.values() {
            if let TabContent::Terminal(terminal) = &tab.content {
                if let Some(menu) = terminal.context_menu() {
                    let has_block_selection = terminal
                        .selected_block_terminal()
                        == Some(menu.terminal_id());
                    let tab_id = tab.id;
                    return Some(
                        crate::ui::widgets::terminal::pane_context_menu::view(
                            crate::ui::widgets::terminal::pane_context_menu::Props {
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
        }

        None
    }
}

#[derive(Debug, Clone, Copy)]
enum MenuGuard {
    Allow,
    Ignore,
    Dismiss,
}

fn context_menu_guard(event: &Event) -> MenuGuard {
    use MenuGuard::*;

    match event {
        Event::SidebarWorkspace(
            sidebar_workspace::Event::TerminalAddMenuAction(_)
            | sidebar_workspace::Event::TerminalAddMenuDismiss,
        )
        | Event::SidebarWorkspace(sidebar_workspace::Event::QuickLaunch(
            quick_launches::QuickLaunchEvent::ContextMenuAction(_)
            | quick_launches::QuickLaunchEvent::ContextMenuDismiss,
        ))
        | Event::QuickLaunchSetupCompleted(_)
        | Event::QuickLaunchTick => Allow,
        Event::Terminal(event) => match event {
            TerminalEvent::CloseContextMenu { .. }
            | TerminalEvent::CopySelection { .. }
            | TerminalEvent::PasteIntoPrompt { .. }
            | TerminalEvent::CopySelectedBlockContent { .. }
            | TerminalEvent::CopySelectedBlockPrompt { .. }
            | TerminalEvent::CopySelectedBlockCommand { .. }
            | TerminalEvent::SplitPane { .. }
            | TerminalEvent::ClosePane { .. } => Allow,
            TerminalEvent::Widget(_) => Allow,
            TerminalEvent::PaneGridCursorMoved { .. } => Allow,
            _ => Dismiss,
        },
        Event::SidebarWorkspace(
            sidebar_workspace::Event::WorkspaceCursorMoved { .. },
        )
        | Event::SidebarWorkspace(sidebar_workspace::Event::QuickLaunch(
            quick_launches::QuickLaunchEvent::CursorMoved { .. },
        )) => Allow,
        Event::SidebarWorkspace(sidebar_workspace::Event::QuickLaunch(
            quick_launches::QuickLaunchEvent::NodeHovered { .. },
        )) => Ignore,
        Event::ActionBar(_) => Allow,
        Event::Window(_) | Event::ResizeWindow(_) => Allow,
        Event::Keyboard(_) => Ignore,
        _ => Dismiss,
    }
}

fn should_cancel_inline_edit(event: &Event) -> bool {
    use quick_launches::QuickLaunchEvent;

    match event {
        Event::SidebarWorkspace(sidebar_workspace::Event::QuickLaunch(
            quick_launches_event,
        )) => !matches!(
            quick_launches_event,
            QuickLaunchEvent::InlineEditChanged(_)
                | QuickLaunchEvent::InlineEditSubmit
                | QuickLaunchEvent::CursorMoved { .. }
                | QuickLaunchEvent::NodeHovered { .. }
        ),
        Event::QuickLaunchTick | Event::QuickLaunchSetupCompleted(_) => false,
        Event::SidebarWorkspace(
            sidebar_workspace::Event::WorkspaceCursorMoved { .. },
        ) => false,
        Event::Terminal(event) => !matches!(
            event,
            TerminalEvent::Widget(_)
                | TerminalEvent::PaneGridCursorMoved { .. }
        ),
        Event::Keyboard(_) | Event::Window(_) => false,
        _ => true,
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

fn build_resize_grips() -> Element<'static, Event, Theme, iced::Renderer> {
    let n_grip = mouse_area(
        container(text(""))
            .width(Length::Fill)
            .height(Length::Fixed(RESIZE_EDGE_MOUSE_AREA_THICKNESS)),
    )
    .on_press(Event::ResizeWindow(Direction::North))
    .interaction(mouse::Interaction::ResizingVertically);

    let s_grip = mouse_area(
        container(text(""))
            .width(Length::Fill)
            .height(Length::Fixed(RESIZE_EDGE_MOUSE_AREA_THICKNESS)),
    )
    .on_press(Event::ResizeWindow(Direction::South))
    .interaction(mouse::Interaction::ResizingVertically);

    let e_grip = mouse_area(
        container(text(""))
            .width(Length::Fixed(RESIZE_EDGE_MOUSE_AREA_THICKNESS))
            .height(Length::Fill),
    )
    .on_press(Event::ResizeWindow(Direction::East))
    .interaction(mouse::Interaction::ResizingHorizontally);

    let w_grip = mouse_area(
        container(text(""))
            .width(Length::Fixed(RESIZE_EDGE_MOUSE_AREA_THICKNESS))
            .height(Length::Fill),
    )
    .on_press(Event::ResizeWindow(Direction::West))
    .interaction(mouse::Interaction::ResizingHorizontally);

    let nw_grip = mouse_area(
        container(text(""))
            .width(Length::Fixed(RESIZE_CORNER_MOUSE_AREA_THICKNESS))
            .height(Length::Fixed(RESIZE_CORNER_MOUSE_AREA_THICKNESS)),
    )
    .on_press(Event::ResizeWindow(Direction::NorthWest))
    .interaction(mouse::Interaction::ResizingDiagonallyDown);

    let ne_grip = mouse_area(
        container(text(""))
            .width(Length::Fixed(RESIZE_CORNER_MOUSE_AREA_THICKNESS))
            .height(Length::Fixed(RESIZE_CORNER_MOUSE_AREA_THICKNESS)),
    )
    .on_press(Event::ResizeWindow(Direction::NorthEast))
    .interaction(mouse::Interaction::ResizingDiagonallyUp);

    let sw_grip = mouse_area(
        container(text(""))
            .width(Length::Fixed(RESIZE_CORNER_MOUSE_AREA_THICKNESS))
            .height(Length::Fixed(RESIZE_CORNER_MOUSE_AREA_THICKNESS)),
    )
    .on_press(Event::ResizeWindow(Direction::SouthWest))
    .interaction(mouse::Interaction::ResizingDiagonallyUp);

    let se_grip = mouse_area(
        container(text(""))
            .width(Length::Fixed(RESIZE_CORNER_MOUSE_AREA_THICKNESS))
            .height(Length::Fixed(RESIZE_CORNER_MOUSE_AREA_THICKNESS)),
    )
    .on_press(Event::ResizeWindow(Direction::SouthEast))
    .interaction(mouse::Interaction::ResizingDiagonallyDown);

    stack!(
        container(n_grip)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_y(iced::alignment::Vertical::Top),
        container(s_grip)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_y(iced::alignment::Vertical::Bottom),
        container(e_grip)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Right),
        container(w_grip)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Left),
        container(nw_grip)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Left)
            .align_y(iced::alignment::Vertical::Top),
        container(ne_grip)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Right)
            .align_y(iced::alignment::Vertical::Top),
        container(sw_grip)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Left)
            .align_y(iced::alignment::Vertical::Bottom),
        container(se_grip)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Right)
            .align_y(iced::alignment::Vertical::Bottom),
    )
    .into()
}

fn max_sidebar_workspace_ratio() -> f32 {
    (1.0 - SIDEBAR_MIN_TAB_CONTENT_RATIO).max(0.0)
}

use iced::mouse;
use iced::widget::{column, container, mouse_area, row, stack, text};
use iced::window::Direction;
use iced::{Element, Length, Size, Subscription, Task, Theme, window};

use crate::app::config::AppConfig;
use crate::app::fonts::FontsConfig;
use crate::app::theme::{ThemeManager, ThemeProps};
use crate::screens::terminal::{
    TerminalScreen, TerminalScreenAction, TerminalScreenEvent,
    TerminalScreenUpdate,
};
use crate::services::ServiceRegistry;
use crate::widgets::action_bar::{
    ActionBar, ActionBarEvent, ActionBarMetrics, ActionBarProps,
};

pub(crate) const MIN_WINDOW_WIDTH: f32 = 800.0;
pub(crate) const MIN_WINDOW_HEIGHT: f32 = 600.0;
const RESIZE_EDGE_MOUSE_AREA_THICKNESS: f32 = 6.0;
const RESIZE_CORNER_MOUSE_AREA_THICKNESS: f32 = 12.0;

/// Represents the currently active high-level view.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActiveView {
    #[default]
    Terminal,
}

/// App-wide events that drive the root update loop.
#[derive(Debug, Clone)]
pub(crate) enum AppEvent {
    IcedReady,
    ActionBar(ActionBarEvent),
    TerminalScreen(TerminalScreenEvent),
    Window(window::Event),
    ResizeWindow(Direction),
}

/// Commands executed at the app boundary.
#[derive(Debug, Clone)]
pub(crate) enum AppCommand {
    CloseWindow,
    ToggleFullScreen,
    MinimizeWindow,
    StartWindowDrag,
}

pub(crate) struct App {
    window_size: Size,
    active_view: ActiveView,
    theme_manager: ThemeManager,
    fonts: FontsConfig,
    _services: ServiceRegistry,
    _config: AppConfig,
    terminal_screen: TerminalScreen,
    is_fullscreen: bool,
}

impl App {
    pub(crate) fn new() -> (Self, Task<AppEvent>) {
        let mut services = ServiceRegistry::new();
        let shell_session = services
            .shell_mut()
            .setup_session()
            .expect("failed to setup shell session");

        let theme_manager = ThemeManager::new();
        let current_theme = theme_manager.current();
        let fonts = FontsConfig::default();

        let config = AppConfig::new(shell_session, current_theme, &fonts);

        let window_size = Size {
            width: MIN_WINDOW_WIDTH,
            height: MIN_WINDOW_HEIGHT,
        };
        let screen_size = Self::screen_size_from_window(window_size);
        let terminal_screen = TerminalScreen::new(config.clone(), screen_size);

        let app = App {
            window_size,
            active_view: ActiveView::default(),
            theme_manager,
            fonts,
            _services: services,
            _config: config,
            terminal_screen,
            is_fullscreen: false,
        };

        (app, Task::done(()).map(|_: ()| AppEvent::IcedReady))
    }

    pub(crate) fn title(&self) -> String {
        String::from("OTTY")
    }

    pub(crate) fn theme(&self) -> Theme {
        self.theme_manager.iced_theme()
    }

    pub(crate) fn subscription(&self) -> Subscription<AppEvent> {
        let screen_subs = self
            .terminal_screen
            .subscription()
            .map(AppEvent::TerminalScreen);
        let win_subs =
            window::events().map(|(_id, event)| AppEvent::Window(event));

        Subscription::batch(vec![screen_subs, win_subs])
    }

    pub(crate) fn update(&mut self, event: AppEvent) -> Task<AppEvent> {
        match event {
            AppEvent::IcedReady => {
                let update =
                    self.terminal_screen.update(TerminalScreenEvent::NewTab);
                self.handle_screen_update(update)
            },
            AppEvent::ActionBar(event) => self.handle_action_bar(event),
            AppEvent::TerminalScreen(event) => {
                let update = self.terminal_screen.update(event);
                self.handle_screen_update(update)
            },
            AppEvent::Window(window::Event::Resized(size)) => {
                self.window_size = size;
                self.terminal_screen
                    .set_screen_size(Self::screen_size_from_window(size));
                Task::none()
            },
            AppEvent::Window(_) => Task::none(),
            AppEvent::ResizeWindow(dir) => window::latest()
                .and_then(move |id| window::drag_resize(id, dir)),
        }
    }

    pub(crate) fn view(&self) -> Element<'_, AppEvent, Theme, iced::Renderer> {
        let theme = self.theme_manager.current();
        let theme_props = ThemeProps::new(theme);
        let mut metrics = ActionBarMetrics::default();
        metrics.title_font_size = self.fonts.ui.size * 0.9;

        let header_title =
            self.terminal_screen.active_tab_title().unwrap_or("OTTY");

        let header = ActionBar::new(ActionBarProps {
            title: header_title,
            theme: theme_props,
            metrics,
        })
        .view()
        .map(AppEvent::ActionBar);

        let main_content: Element<AppEvent, Theme, iced::Renderer> =
            match self.active_view {
                ActiveView::Terminal => self
                    .terminal_screen
                    .view(theme_props)
                    .map(AppEvent::TerminalScreen),
            };

        let content_row =
            row![main_content].width(Length::Fill).height(Length::Fill);

        let resize_grips = build_resize_grips();

        stack!(
            column![header, content_row]
                .width(Length::Fill)
                .height(Length::Fill),
            resize_grips
        )
        .into()
    }

    fn handle_action_bar(&mut self, event: ActionBarEvent) -> Task<AppEvent> {
        match event {
            ActionBarEvent::NewTab => {
                let update =
                    self.terminal_screen.update(TerminalScreenEvent::NewTab);
                self.handle_screen_update(update)
            },
            ActionBarEvent::ToggleFullScreen => {
                self.apply_command(AppCommand::ToggleFullScreen)
            },
            ActionBarEvent::ToggleTray => {
                self.apply_command(AppCommand::MinimizeWindow)
            },
            ActionBarEvent::CloseWindow => {
                self.apply_command(AppCommand::CloseWindow)
            },
            ActionBarEvent::StartWindowDrag => {
                self.apply_command(AppCommand::StartWindowDrag)
            },
        }
    }

    fn handle_screen_update(
        &mut self,
        update: TerminalScreenUpdate,
    ) -> Task<AppEvent> {
        let mut tasks = vec![update.task.map(AppEvent::TerminalScreen)];

        match update.action {
            TerminalScreenAction::ActivateView => {
                self.active_view = ActiveView::Terminal;
            },
            TerminalScreenAction::CloseWindow => {
                tasks.push(self.apply_command(AppCommand::CloseWindow));
            },
            TerminalScreenAction::None => {},
        }

        Task::batch(tasks)
    }

    fn apply_command(&mut self, command: AppCommand) -> Task<AppEvent> {
        match command {
            AppCommand::CloseWindow => window::latest().and_then(window::close),
            AppCommand::ToggleFullScreen => self.toggle_full_screen(),
            AppCommand::MinimizeWindow => {
                window::latest().and_then(|id| window::minimize(id, true))
            },
            AppCommand::StartWindowDrag => {
                window::latest().and_then(window::drag)
            },
        }
    }

    fn toggle_full_screen(&mut self) -> Task<AppEvent> {
        self.is_fullscreen = !self.is_fullscreen;

        let mode = if self.is_fullscreen {
            window::Mode::Fullscreen
        } else {
            window::Mode::Windowed
        };

        window::latest().and_then(move |id| window::set_mode(id, mode))
    }

    fn screen_size_from_window(window_size: Size) -> Size {
        let action_bar_height = ActionBarMetrics::default().height;
        let height = (window_size.height - action_bar_height).max(0.0);
        Size::new(window_size.width, height)
    }
}

fn build_resize_grips() -> Element<'static, AppEvent, Theme, iced::Renderer> {
    let n_grip = mouse_area(
        container(text(""))
            .width(Length::Fill)
            .height(Length::Fixed(RESIZE_EDGE_MOUSE_AREA_THICKNESS)),
    )
    .on_press(AppEvent::ResizeWindow(Direction::North))
    .interaction(mouse::Interaction::ResizingVertically);

    let s_grip = mouse_area(
        container(text(""))
            .width(Length::Fill)
            .height(Length::Fixed(RESIZE_EDGE_MOUSE_AREA_THICKNESS)),
    )
    .on_press(AppEvent::ResizeWindow(Direction::South))
    .interaction(mouse::Interaction::ResizingVertically);

    let e_grip = mouse_area(
        container(text(""))
            .width(Length::Fixed(RESIZE_EDGE_MOUSE_AREA_THICKNESS))
            .height(Length::Fill),
    )
    .on_press(AppEvent::ResizeWindow(Direction::East))
    .interaction(mouse::Interaction::ResizingHorizontally);

    let w_grip = mouse_area(
        container(text(""))
            .width(Length::Fixed(RESIZE_EDGE_MOUSE_AREA_THICKNESS))
            .height(Length::Fill),
    )
    .on_press(AppEvent::ResizeWindow(Direction::West))
    .interaction(mouse::Interaction::ResizingHorizontally);

    let nw_grip = mouse_area(
        container(text(""))
            .width(Length::Fixed(RESIZE_CORNER_MOUSE_AREA_THICKNESS))
            .height(Length::Fixed(RESIZE_CORNER_MOUSE_AREA_THICKNESS)),
    )
    .on_press(AppEvent::ResizeWindow(Direction::NorthWest))
    .interaction(mouse::Interaction::ResizingDiagonallyDown);

    let ne_grip = mouse_area(
        container(text(""))
            .width(Length::Fixed(RESIZE_CORNER_MOUSE_AREA_THICKNESS))
            .height(Length::Fixed(RESIZE_CORNER_MOUSE_AREA_THICKNESS)),
    )
    .on_press(AppEvent::ResizeWindow(Direction::NorthEast))
    .interaction(mouse::Interaction::ResizingDiagonallyUp);

    let sw_grip = mouse_area(
        container(text(""))
            .width(Length::Fixed(RESIZE_CORNER_MOUSE_AREA_THICKNESS))
            .height(Length::Fixed(RESIZE_CORNER_MOUSE_AREA_THICKNESS)),
    )
    .on_press(AppEvent::ResizeWindow(Direction::SouthWest))
    .interaction(mouse::Interaction::ResizingDiagonallyUp);

    let se_grip = mouse_area(
        container(text(""))
            .width(Length::Fixed(RESIZE_CORNER_MOUSE_AREA_THICKNESS))
            .height(Length::Fixed(RESIZE_CORNER_MOUSE_AREA_THICKNESS)),
    )
    .on_press(AppEvent::ResizeWindow(Direction::SouthEast))
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

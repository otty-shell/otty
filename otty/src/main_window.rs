use iced::alignment;
use iced::widget::{button, column, container, row, slider, text};
use iced::{Element, Length, Size, Subscription, Task, Theme, window};
use otty_ui_term::TerminalView;
use otty_ui_term::settings::{
    BackendSettings, FontSettings, LocalSessionOptions, SessionKind, Settings,
    ThemeSettings,
};
use std::path::Path;

use crate::tab_bar;
use crate::theme::{AppThemeId, ThemeManager};

/// Represents the currently active high-level view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarView {
    MainLayout,
    Settings,
}

#[derive(Debug, Clone)]
pub enum Event {
    Terminal(otty_ui_term::Event),
    NewTab,
    CloseTab(u64),
    ActivateTab(u64),
    WindowEvent(window::Event),
    SidebarViewChanged(SidebarView),
    SelectTheme(AppThemeId),
    UiFontSizeChanged(f32),
    TerminalFontSizeChanged(f32),
}

pub struct Tab {
    pub id: u64,
    pub title: String,
    pub terminal: otty_ui_term::Terminal,
}

pub struct App {
    shell_name: String,
    pub(crate) settings: Settings,
    pub(crate) tabs: Vec<Tab>,
    pub(crate) active_tab_index: usize,
    pub(crate) next_tab_id: u64,
    pub(crate) window_size: Size,
    pub(crate) active_view: SidebarView,
    pub(crate) theme_manager: ThemeManager,
}

impl App {
    pub fn new() -> (Self, Task<Event>) {
        let shell_path =
            std::env::var("SHELL").expect("SHELL variable is not defined");
        let shell_name = Path::new(&shell_path)
            .file_name()
            .and_then(|name| name.to_str())
            .map(ToString::to_string)
            .unwrap_or_else(|| shell_path.clone());

        let session_options =
            LocalSessionOptions::default().with_program(&shell_path);
        let session = SessionKind::from_local_options(session_options);

        let theme_manager = ThemeManager::new();
        let current_theme = theme_manager.current();

        let font_settings = FontSettings {
            size: current_theme.font_size_terminal,
            ..FontSettings::default()
        };
        let theme_settings = ThemeSettings::new(Box::new(
            current_theme.terminal_palette.clone(),
        ));

        let settings = Settings {
            font: font_settings,
            theme: theme_settings,
            backend: BackendSettings::default().with_session(session),
        };

        let mut app = App {
            shell_name,
            settings: settings.clone(),
            tabs: Vec::new(),
            active_tab_index: 0,
            next_tab_id: 0,
            window_size: Size {
                width: 1280.0,
                height: 720.0,
            },
            active_view: SidebarView::MainLayout,
            theme_manager,
        };

        let task = app.create_initial_tab();
        (app, task)
    }

    pub fn title(&self) -> String {
        String::from("OTTY")
    }

    pub fn theme(&self) -> Theme {
        self.theme_manager.iced_theme()
    }

    pub fn subscription(&self) -> Subscription<Event> {
        let mut subscriptions = Vec::new();

        for tab in &self.tabs {
            subscriptions.push(Subscription::run_with_id(
                tab.id,
                tab.terminal.subscription(),
            ));
        }

        let term_subs = Subscription::batch(subscriptions).map(Event::Terminal);
        let win_subs =
            window::events().map(|(_id, event)| Event::WindowEvent(event));

        Subscription::batch(vec![term_subs, win_subs])
    }

    pub fn update(&mut self, event: Event) -> Task<Event> {
        match event {
            Event::Terminal(inner) => self.update_terminal(inner),
            Event::NewTab => {
                self.active_view = SidebarView::MainLayout;
                self.create_tab()
            },
            Event::CloseTab(id) => self.close_tab(id),
            Event::ActivateTab(id) => {
                self.activate_tab(id);
                self.active_view = SidebarView::MainLayout;
                Task::none()
            },
            Event::SidebarViewChanged(view) => {
                self.active_view = view;
                Task::none()
            },
            Event::SelectTheme(id) => {
                self.theme_manager.set_current(id);
                self.apply_theme_to_settings_and_tabs();
                Task::none()
            },
            Event::UiFontSizeChanged(size) => {
                self.theme_manager.set_font_size_ui(size);
                Task::none()
            },
            Event::TerminalFontSizeChanged(size) => {
                self.theme_manager.set_font_size_terminal(size);
                self.apply_terminal_font_to_settings_and_tabs();
                Task::none()
            },
            Event::WindowEvent(window::Event::Resized(size)) => {
                self.window_size = size;
                Task::none()
            },
            Event::WindowEvent(_) => Task::none(),
        }
    }

    pub fn view(&self) -> Element<Event, Theme, iced::Renderer> {
        let tabs_row = tab_bar::view_tab_bar(self);
        let main_content: Element<Event, Theme, iced::Renderer> =
            match self.active_view {
                SidebarView::MainLayout => {
                    let content = self.view_active_terminal();
                    column![tabs_row, content]
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .into()
                },
                SidebarView::Settings => {
                    let settings_view = self.view_settings();
                    column![tabs_row, settings_view]
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .into()
                },
            };

        row![crate::sidebar::view_sidebar(self.active_view), main_content]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_settings(&self) -> Element<Event, Theme, iced::Renderer> {
        let current = self.theme_manager.current();

        let theme_row = {
            let mut row_view = row![text("Theme:")].spacing(8);

            for preset in self.theme_manager.presets() {
                let is_active = preset.id == current.id;
                let label = if is_active {
                    format!("{}*", preset.name)
                } else {
                    preset.name.clone()
                };

                let button =
                    button(text(label)).on_press(Event::SelectTheme(preset.id));
                row_view = row_view.push(button);
            }

            row_view
        };

        let ui_font_row = row![
            text("UI font size"),
            slider(10.0..=24.0, current.font_size_ui, Event::UiFontSizeChanged),
            text(format!("{:.0}", current.font_size_ui)),
        ]
        .spacing(8);

        let terminal_font_row = row![
            text("Terminal font size"),
            slider(
                8.0..=32.0,
                current.font_size_terminal,
                Event::TerminalFontSizeChanged
            ),
            text(format!("{:.0}", current.font_size_terminal)),
        ]
        .spacing(8);

        let content = column![
            text("Appearance").size(18),
            theme_row,
            ui_font_row,
            terminal_font_row,
        ]
        .spacing(16)
        .padding(16);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_active_terminal(&self) -> Element<Event, Theme, iced::Renderer> {
        if let Some(tab) = self.tabs.get(self.active_tab_index) {
            container(TerminalView::show(&tab.terminal).map(Event::Terminal))
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        } else {
            container(text("No tabs"))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(alignment::Horizontal::Center)
                .align_y(alignment::Vertical::Center)
                .into()
        }
    }

    fn update_terminal(&mut self, event: otty_ui_term::Event) -> Task<Event> {
        use otty_ui_term::Event::*;

        let id = *event.terminal_id();
        if let Some(index) = self.tabs.iter().position(|tab| tab.id == id) {
            match event {
                Shutdown { .. } => {
                    return self.close_tab(id);
                },
                TitleChanged { title, .. } => {
                    self.tabs[index].title = title;
                },
                ResetTitle { .. } => {
                    self.tabs[index].title = self.shell_name.clone();
                },
                other => self.tabs[index].terminal.handle(other),
            }
        }

        Task::none()
    }

    fn create_initial_tab(&mut self) -> Task<Event> {
        self.create_tab()
    }

    fn create_tab(&mut self) -> Task<Event> {
        let tab_id = self.next_tab_id;
        self.next_tab_id += 1;

        let terminal =
            otty_ui_term::Terminal::new(tab_id, self.settings.clone())
                .expect("failed to create the new terminal instance");

        self.tabs.push(Tab {
            id: tab_id,
            title: self.shell_name.clone(),
            terminal,
        });

        self.active_tab_index = self.tabs.len() - 1;

        if let Some(active) = self.tabs.last() {
            return TerminalView::focus(active.terminal.widget_id());
        }

        Task::none()
    }

    fn apply_theme_to_settings_and_tabs(&mut self) {
        let palette = self.theme_manager.terminal_palette();
        self.settings.theme = ThemeSettings::new(Box::new(palette.clone()));

        for tab in &mut self.tabs {
            tab.terminal.change_theme(palette.clone());
        }
    }

    fn apply_terminal_font_to_settings_and_tabs(&mut self) {
        let current = self.theme_manager.current();
        let font_settings = FontSettings {
            size: current.font_size_terminal,
            ..FontSettings::default()
        };

        self.settings.font = font_settings.clone();

        for tab in &mut self.tabs {
            tab.terminal.change_font(font_settings.clone());
        }
    }

    fn close_tab(&mut self, id: u64) -> Task<Event> {
        if self.tabs.len() == 1 {
            return window::get_latest().and_then(window::close);
        }

        if let Some(index) = self.tabs.iter().position(|tab| tab.id == id) {
            self.tabs.remove(index);

            if self.active_tab_index >= self.tabs.len() {
                self.active_tab_index = self.tabs.len().saturating_sub(1);
            }

            if let Some(active) = self.tabs.get(self.active_tab_index) {
                return TerminalView::focus(active.terminal.widget_id());
            }
        }

        Task::none()
    }

    fn activate_tab(&mut self, id: u64) {
        if let Some(index) = self.tabs.iter().position(|tab| tab.id == id) {
            self.active_tab_index = index;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::ThemeManager;

    #[test]
    fn default_tab_titles_are_indexed_from_one() {
        let settings = Settings::default();
        let mut app = App {
            shell_name: String::from("zsh"),
            settings,
            tabs: Vec::new(),
            active_tab_index: 0,
            next_tab_id: 0,
            window_size: Size {
                width: 1280.0,
                height: 720.0,
            },
            active_view: SidebarView::MainLayout,
            theme_manager: ThemeManager::new(),
        };

        let _ = app.create_tab();
        let _ = app.create_tab();

        assert_eq!(app.tabs[0].title, "zsh");
        assert_eq!(app.tabs[1].title, "zsh");
    }

    #[test]
    fn closing_tab_updates_active_index() {
        let settings = Settings::default();
        let mut app = App {
            shell_name: String::from("zsh"),
            settings,
            tabs: Vec::new(),
            active_tab_index: 0,
            next_tab_id: 0,
            window_size: Size {
                width: 1280.0,
                height: 720.0,
            },
            active_view: SidebarView::MainLayout,
            theme_manager: ThemeManager::new(),
        };

        let _ = app.create_tab();
        let _ = app.create_tab();
        let first_id = app.tabs[0].id;

        let _ = app.close_tab(first_id);

        assert_eq!(app.tabs.len(), 1);
        assert_eq!(app.active_tab_index, 0);
    }

    #[test]
    fn activating_tab_from_settings_switches_to_main_layout() {
        let settings = Settings::default();
        let mut app = App {
            shell_name: String::from("zsh"),
            settings,
            tabs: Vec::new(),
            active_tab_index: 0,
            next_tab_id: 0,
            window_size: Size {
                width: 1280.0,
                height: 720.0,
            },
            active_view: SidebarView::Settings,
            theme_manager: ThemeManager::new(),
        };

        let _ = app.create_tab();
        let first_id = app.tabs[0].id;

        let _ = app.update(Event::ActivateTab(first_id));

        assert!(matches!(app.active_view, SidebarView::MainLayout));
    }

    #[test]
    fn new_tab_switches_to_main_layout() {
        let settings = Settings::default();
        let mut app = App {
            shell_name: String::from("zsh"),
            settings,
            tabs: Vec::new(),
            active_tab_index: 0,
            next_tab_id: 0,
            window_size: Size {
                width: 1280.0,
                height: 720.0,
            },
            active_view: SidebarView::Settings,
            theme_manager: ThemeManager::new(),
        };

        let _ = app.update(Event::NewTab);

        assert!(matches!(app.active_view, SidebarView::MainLayout));
    }

    #[test]
    fn default_active_view_is_main_layout() {
        let (_, task) = App::new();
        drop(task);

        let settings = Settings::default();
        let app = App {
            shell_name: String::from("zsh"),
            settings,
            tabs: Vec::new(),
            active_tab_index: 0,
            next_tab_id: 0,
            window_size: Size {
                width: 1280.0,
                height: 720.0,
            },
            active_view: SidebarView::MainLayout,
            theme_manager: ThemeManager::new(),
        };

        assert!(matches!(app.active_view, SidebarView::MainLayout));
    }

    #[test]
    fn sidebar_view_changed_updates_active_view() {
        let settings = Settings::default();
        let mut app = App {
            shell_name: String::from("zsh"),
            settings,
            tabs: Vec::new(),
            active_tab_index: 0,
            next_tab_id: 0,
            window_size: Size {
                width: 1280.0,
                height: 720.0,
            },
            active_view: SidebarView::MainLayout,
            theme_manager: ThemeManager::new(),
        };

        let _ = app.update(Event::SidebarViewChanged(SidebarView::Settings));

        assert!(matches!(app.active_view, SidebarView::Settings));
    }
}

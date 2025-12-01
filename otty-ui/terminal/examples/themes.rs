use iced::advanced::graphics::core::Element;
use iced::widget::{button, column, container, row};
use iced::{Length, Size, Subscription, Task, Theme, window};
use otty_ui_term::settings::{LocalSessionOptions, SessionKind};
use otty_ui_term::{ColorPalette, TerminalView};

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .antialiasing(false)
        .window_size(Size {
            width: 1280.0,
            height: 720.0,
        })
        .title(App::title)
        .subscription(App::subscription)
        .run()
}

#[derive(Debug, Clone)]
pub enum Event {
    Terminal(otty_ui_term::Event),
    ThemeChanged(Box<otty_ui_term::ColorPalette>),
}

struct App {
    title: String,
    term: otty_ui_term::Terminal,
}

impl App {
    fn new() -> (Self, Task<Event>) {
        let system_shell =
            std::env::var("SHELL").expect("SHELL variable is not defined");

        let session_options =
            LocalSessionOptions::default().with_program(&system_shell);
        let session = SessionKind::from_local_options(session_options);
        let term_id = 0;
        let term_settings = otty_ui_term::settings::Settings {
            backend: otty_ui_term::settings::BackendSettings::default()
                .with_session(session),
            ..Default::default()
        };

        (
            Self {
                title: String::from("Terminal app"),
                term: otty_ui_term::Terminal::new(
                    term_id,
                    term_settings.clone(),
                )
                .expect("failed to create the new terminal instance"),
            },
            Task::none(),
        )
    }

    fn title(&self) -> String {
        self.title.clone()
    }

    fn subscription(&self) -> Subscription<Event> {
        self.term.subscription().map(Event::Terminal)
    }

    fn update(&mut self, event: Event) -> Task<Event> {
        use otty_ui_term::Event::*;

        match event {
            Event::ThemeChanged(theme) => {
                self.term.change_theme(*theme);
            },
            Event::Terminal(inner) => match inner {
                Shutdown { .. } => {
                    return window::latest().and_then(window::close);
                },
                TitleChanged { title, .. } => {
                    self.title = title;
                },
                event => self.term.handle(event),
            },
        }

        Task::none()
    }

    fn view(&'_ self) -> Element<'_, Event, Theme, iced::Renderer> {
        let content = column![
            row![
                button("default").width(Length::Fill).padding(8).on_press(
                    Event::ThemeChanged(Box::new(ColorPalette::default(),))
                ),
                button("ubuntu").width(Length::Fill).padding(8).on_press(
                    Event::ThemeChanged(Box::new(otty_ui_term::ColorPalette {
                        background: String::from("#300A24"),
                        foreground: String::from("#FFFFFF"),
                        black: String::from("#2E3436"),
                        red: String::from("#CC0000"),
                        green: String::from("#4E9A06"),
                        yellow: String::from("#C4A000"),
                        blue: String::from("#3465A4"),
                        magenta: String::from("#75507B"),
                        cyan: String::from("#06989A"),
                        white: String::from("#D3D7CF"),
                        bright_black: String::from("#555753"),
                        bright_red: String::from("#EF2929"),
                        bright_green: String::from("#8AE234"),
                        bright_yellow: String::from("#FCE94F"),
                        bright_blue: String::from("#729FCF"),
                        bright_magenta: String::from("#AD7FA8"),
                        bright_cyan: String::from("#34E2E2"),
                        bright_white: String::from("#EEEEEC"),
                        ..Default::default()
                    }))
                ),
                button("3024 Day").width(Length::Fill).padding(8).on_press(
                    Event::ThemeChanged(Box::new(otty_ui_term::ColorPalette {
                        background: String::from("#F7F7F7"),
                        foreground: String::from("#4A4543"),
                        black: String::from("#090300"),
                        red: String::from("#DB2D20"),
                        green: String::from("#01A252"),
                        yellow: String::from("#FDED02"),
                        blue: String::from("#01A0E4"),
                        magenta: String::from("#A16A94"),
                        cyan: String::from("#B5E4F4"),
                        white: String::from("#A5A2A2"),
                        bright_black: String::from("#5C5855"),
                        bright_red: String::from("#E8BBD0"),
                        bright_green: String::from("#3A3432"),
                        bright_yellow: String::from("#4A4543"),
                        bright_blue: String::from("#807D7C"),
                        bright_magenta: String::from("#D6D5D4"),
                        bright_cyan: String::from("#CDAB53"),
                        bright_white: String::from("#F7F7F7"),
                        ..Default::default()
                    }))
                ),
            ],
            row![TerminalView::show(&self.term).map(Event::Terminal)]
        ];

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

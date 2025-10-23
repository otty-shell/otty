use iced::advanced::graphics::core::Element;
use iced::font::{Family, Stretch, Weight};
use iced::widget::{button, column, container, row};
use iced::{window, Font, Length, Size, Subscription, Task, Theme};
use iced_term::TerminalView;

const TERM_FONT_JET_BRAINS_BYTES: &[u8] = include_bytes!(
    "../assets/fonts/JetBrains/JetBrainsMonoNerdFontMono-Bold.ttf"
);

fn main() -> iced::Result {
    iced::application(App::title, App::update, App::view)
        .antialiasing(false)
        .window_size(Size {
            width: 1280.0,
            height: 720.0,
        })
        .subscription(App::subscription)
        .font(TERM_FONT_JET_BRAINS_BYTES)
        .run_with(App::new)
}

#[derive(Debug, Clone)]
pub enum Event {
    Terminal(iced_term::Event),
    ThemeChanged(Box<iced_term::ColorPalette>),
}

struct App {
    title: String,
    term: iced_term::Terminal,
}

impl App {
    fn new() -> (Self, Task<Event>) {
        let system_shell = std::env::var("SHELL")
            .expect("SHELL variable is not defined")
            .to_string();
        let term_id = 0;
        let term_settings = iced_term::settings::Settings {
            font: iced_term::settings::FontSettings {
                size: 14.0,
                font_type: Font {
                    weight: Weight::Bold,
                    family: Family::Name("JetBrainsMono Nerd Font Mono"),
                    stretch: Stretch::Normal,
                    ..Default::default()
                },
                ..Default::default()
            },
            theme: iced_term::settings::ThemeSettings::default(),
            backend: iced_term::settings::BackendSettings {
                program: system_shell,
                ..Default::default()
            },
        };

        (
            Self {
                title: String::from("Terminal app"),
                term: iced_term::Terminal::new(term_id, term_settings.clone())
                    .expect("failed to create the new terminal instance"),
            },
            Task::none(),
        )
    }

    fn title(&self) -> String {
        self.title.clone()
    }

    fn subscription(&self) -> Subscription<Event> {
        Subscription::run_with_id(self.term.id, self.term.subscription())
            .map(Event::Terminal)
    }

    fn update(&mut self, event: Event) -> Task<Event> {
        match event {
            Event::ThemeChanged(palette) => {
                self.term.handle(iced_term::Command::ChangeTheme(palette));
            },
            Event::Terminal(iced_term::Event::BackendCall(_, cmd)) => {
                match self.term.handle(iced_term::Command::ProxyToBackend(cmd))
                {
                    iced_term::actions::Action::Shutdown => {
                        return window::get_latest().and_then(window::close)
                    },
                    iced_term::actions::Action::ChangeTitle(title) => {
                        self.title = title;
                    },
                    _ => {},
                }
            },
        }

        Task::none()
    }

    fn view(&self) -> Element<Event, Theme, iced::Renderer> {
        let content = column![
            row![
                button("default")
                    .width(Length::Fill)
                    .padding(8)
                    .on_press(Event::ThemeChanged(Box::default())),
                button("ubuntu").width(Length::Fill).padding(8).on_press(
                    Event::ThemeChanged(Box::new(iced_term::ColorPalette {
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
                    Event::ThemeChanged(Box::new(iced_term::ColorPalette {
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

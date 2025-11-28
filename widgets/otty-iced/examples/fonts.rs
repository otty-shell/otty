use iced::advanced::graphics::core::Element;
use iced::font::{Family, Weight};
use iced::widget::{button, column, container, row};
use iced::{window, Font, Length, Size, Subscription, Task, Theme};
use otty_iced::TerminalView;
use otty_iced::settings::{LocalSessionOptions, SessionKind};

const TERM_FONT_JET_BRAINS_BYTES: &[u8] = include_bytes!(
    "../../../assets/fonts/JetBrains/JetBrainsMonoNerdFontMono-Bold.ttf"
);

const TERM_FONT_3270_BYTES: &[u8] =
    include_bytes!("../../../assets/fonts/3270/3270NerdFont-Regular.ttf");

fn main() -> iced::Result {
    iced::application(App::title, App::update, App::view)
        .antialiasing(false)
        .window_size(Size {
            width: 1280.0,
            height: 720.0,
        })
        .subscription(App::subscription)
        .font(TERM_FONT_JET_BRAINS_BYTES)
        .font(TERM_FONT_3270_BYTES)
        .run_with(App::new)
}

#[derive(Debug, Clone)]
pub enum Event {
    Terminal(otty_iced::Event),
    FontChanged(String),
    FontSizeInc,
    FontSizeDec,
}

struct App {
    title: String,
    term: otty_iced::Terminal,
    font_setting: otty_iced::settings::FontSettings,
}

impl App {
    fn new() -> (Self, Task<Event>) {
        let system_shell =
            std::env::var("SHELL").expect("SHELL variable is not defined");

        let session_options =
            LocalSessionOptions::default().with_program(&system_shell);
        let session = SessionKind::from_local_options(session_options);
        let term_id = 0;
        let term_settings = otty_iced::settings::Settings {
            backend: otty_iced::settings::BackendSettings::default()
                .with_session(session),
            ..Default::default()
        };

        (
            Self {
                title: String::from("fonts"),
                term: otty_iced::Terminal::new(term_id, term_settings.clone())
                    .expect("failed to create the new terminal instance"),
                font_setting: term_settings.font,
            },
            Task::none(),
        )
    }

    fn title(&self) -> String {
        self.title.clone()
    }

    fn subscription(&self) -> Subscription<Event> {
        let id = self.term.id;
        let subscription = self.term.subscription();
        Subscription::run_with_id(id, subscription).map(Event::Terminal)
    }

    fn update(&mut self, event: Event) -> Task<Event> {
        match event {
            Event::FontChanged(name) => {
                if name.as_str() == "3270" {
                    self.font_setting.font_type = Font {
                        weight: Weight::Normal,
                        family: Family::Name("3270 Nerd Font"),
                        ..Font::default()
                    };
                } else {
                    self.font_setting.font_type = Font {
                        weight: Weight::Bold,
                        family: Family::Name("JetBrainsMono Nerd Font Mono"),
                        ..Font::default()
                    };
                };

                self.term.change_font(self.font_setting.clone());
            },
            Event::FontSizeInc => {
                self.font_setting.size += 1.0;
                self.term.change_font(self.font_setting.clone());
            },
            Event::FontSizeDec => {
                if self.font_setting.size > 0.0 {
                    self.font_setting.size -= 1.0;
                    self.term.change_font(self.font_setting.clone());
                }
            },
            Event::Terminal(inner) => match inner {
                otty_iced::Event::Shutdown { .. } => {
                    return window::get_latest().and_then(window::close);
                },
                otty_iced::Event::TitleChanged { title, .. } => {
                    self.title = title;
                }
                event => self.term.handle(event),
            },
        }

        Task::none()
    }

    fn view(&'_ self) -> Element<'_, Event, Theme, iced::Renderer> {
        let content = column![
            row![
                button("JetBrains")
                    .width(Length::Fill)
                    .padding(8)
                    .on_press(Event::FontChanged("JetBrains Mono".to_string())),
                button("3270")
                    .width(Length::Fill)
                    .padding(8)
                    .on_press(Event::FontChanged("3270".to_string())),
            ],
            row![
                button("+ size")
                    .width(Length::Fill)
                    .padding(8)
                    .on_press(Event::FontSizeInc),
                button("- size")
                    .width(Length::Fill)
                    .padding(8)
                    .on_press(Event::FontSizeDec),
            ],
            row![TerminalView::show(&self.term).map(Event::Terminal)],
        ];

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

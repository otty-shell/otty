use iced::advanced::graphics::core::Element;
use iced::font::{Family, Weight};
use iced::widget::container;
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
                    ..Font::default()
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
                title: String::from("full_screen"),
                term: iced_term::Terminal::new(term_id, term_settings)
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
        container(TerminalView::show(&self.term).map(Event::Terminal))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

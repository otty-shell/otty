use iced::advanced::graphics::core::Element;
use iced::widget::container;
use iced::{Length, Size, Subscription, Task, Theme, window};
use otty_iced::TerminalView;
use otty_iced::settings::{LocalSessionOptions, SessionKind};

fn main() -> iced::Result {
    iced::application(App::title, App::update, App::view)
        .window_size(Size {
            width: 1280.0,
            height: 720.0,
        })
        .subscription(App::subscription)
        .run_with(App::new)
}

#[derive(Debug, Clone)]
pub enum Event {
    Terminal(otty_iced::Event),
}

struct App {
    title: String,
    term: otty_iced::Terminal,
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
                title: String::from("full_screen"),
                term: otty_iced::Terminal::new(term_id, term_settings)
                    .expect("failed to create the new terminal instance"),
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
        use otty_iced::Event::*;

        match event {
            Event::Terminal(inner) => match inner {
                Shutdown { .. } => {
                    return window::get_latest().and_then(window::close);
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
        container(TerminalView::show(&self.term).map(Event::Terminal))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

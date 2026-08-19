use iced::advanced::graphics::core::Element;
use iced::widget::container;
use iced::{Length, Size, Subscription, Task, Theme, window};
use otty_ui_term::TerminalView;
use otty_ui_term::settings::{LocalSessionOptions, SessionKind};

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title(App::title)
        .window_size(Size {
            width: 1280.0,
            height: 720.0,
        })
        .subscription(App::subscription)
        .run()
}

#[derive(Debug, Clone)]
pub enum Event {
    Terminal(otty_ui_term::Event),
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
                title: String::from("full_screen"),
                term: otty_ui_term::Terminal::new(term_id, term_settings)
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
        container(TerminalView::show(&self.term).map(Event::Terminal))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

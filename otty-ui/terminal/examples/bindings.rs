use iced::keyboard::Modifiers;
use iced::widget::container;
use iced::{Element, Length, Size, Subscription, Task, Theme, window};
use otty_ui_term::bindings::{
    Binding, BindingAction, InputKind, KeyboardBinding,
};
use otty_ui_term::settings::{LocalSessionOptions, SessionKind};
use otty_ui_term::{self, SurfaceMode, TerminalView, generate_bindings};

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

        let custom_bindings = vec![
            (
                Binding {
                    target: InputKind::Char(String::from("c")),
                    modifiers: Modifiers::SHIFT,
                    mode_include: SurfaceMode::ALT_SCREEN,
                    mode_exclude: SurfaceMode::empty(),
                },
                BindingAction::Paste,
            ),
            (
                Binding {
                    target: InputKind::Char(String::from("a")),
                    modifiers: Modifiers::SHIFT | Modifiers::CTRL,
                    mode_include: SurfaceMode::empty(),
                    mode_exclude: SurfaceMode::empty(),
                },
                BindingAction::Char('B'),
            ),
            (
                Binding {
                    target: InputKind::Char(String::from("b")),
                    modifiers: Modifiers::SHIFT | Modifiers::CTRL,
                    mode_include: SurfaceMode::empty(),
                    mode_exclude: SurfaceMode::empty(),
                },
                BindingAction::Esc("\x1b[5~".into()),
            ),
        ];

        let mut term = otty_ui_term::Terminal::new(term_id, term_settings)
            .expect("failed to create the new terminal instance");

        term.add_bindings(custom_bindings);

        // You can also use generate_bindings macros
        let custom_bindings = generate_bindings!(
            KeyboardBinding;
            "l", Modifiers::SHIFT; BindingAction::Char('K');
        );
        term.add_bindings(custom_bindings);

        (
            Self {
                title: String::from("custom_bindings"),
                term,
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

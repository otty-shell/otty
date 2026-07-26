use gpui::prelude::*;
use gpui::{
    App, Application, Bounds, Context, Entity, KeyBinding, Render,
    Subscription, Window, WindowBounds, WindowOptions, div, px, size,
};
use otty_ui_term_gpui::{
    Copy, LocalBackend, LocalOptions, Paste, Terminal, TerminalConfig,
    TerminalEvent,
};

struct FullScreen {
    terminal: Entity<Terminal>,
    _events: Subscription,
}

impl FullScreen {
    fn new(window: &Window, cx: &mut Context<Self>) -> Self {
        let terminal = cx.new(|cx| {
            Terminal::new(
                TerminalConfig::default(),
                LocalBackend::new(LocalOptions::default()),
                cx,
            )
        });
        let events =
            cx.subscribe_in(&terminal, window, |_, _, event, window, _| {
                match event {
                    TerminalEvent::TitleChanged(Some(title)) => {
                        window.set_window_title(title);
                    },
                    TerminalEvent::TitleChanged(None) => {
                        window.set_window_title("OTTY GPUI Terminal");
                    },
                    _ => {},
                }
            });

        Self {
            terminal,
            _events: events,
        }
    }
}

impl Render for FullScreen {
    fn render(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div().size_full().p_2().child(self.terminal.clone())
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        cx.bind_keys([
            KeyBinding::new("ctrl-shift-c", Copy, None),
            KeyBinding::new("ctrl-shift-v", Paste, None),
        ]);
        let bounds = Bounds::centered(None, size(px(900.0), px(600.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |window, cx| cx.new(|cx| FullScreen::new(window, cx)),
        )
        .expect("open terminal window");
        cx.activate(true);
    });
}

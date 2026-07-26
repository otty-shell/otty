use gpui::prelude::*;
use gpui::{
    App, Application, Bounds, ClickEvent, Context, Entity, Render, Window,
    WindowBounds, WindowOptions, div, px, size,
};
use otty_ui_term_gpui::{
    LocalBackend, LocalOptions, Terminal, TerminalConfig, TerminalFont,
};

struct Fonts {
    terminal: Entity<Terminal>,
    large: bool,
}

impl Fonts {
    fn new(cx: &mut Context<Self>) -> Self {
        let terminal = cx.new(|cx| {
            Terminal::new(
                TerminalConfig::default(),
                LocalBackend::new(LocalOptions::default()),
                cx,
            )
        });

        Self {
            terminal,
            large: false,
        }
    }

    fn toggle_font(
        &mut self,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.large = !self.large;
        let size = if self.large { 20.0 } else { 14.0 };
        let font = TerminalFont::monospace(size).expect("valid example font");

        self.terminal.update(cx, |terminal, cx| {
            terminal.set_font(font, cx);
        });
    }
}

impl Render for Fonts {
    fn render(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .gap_2()
            .p_2()
            .child(
                div()
                    .id("toggle-font")
                    .px_3()
                    .py_1()
                    .rounded_md()
                    .bg(gpui::rgb(0x5b6ee1))
                    .text_color(gpui::white())
                    .cursor_pointer()
                    .child("Toggle 14/20 px")
                    .on_click(cx.listener(Self::toggle_font)),
            )
            .child(div().flex_1().child(self.terminal.clone()))
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(900.0), px(600.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(Fonts::new),
        )
        .expect("open fonts example");
        cx.activate(true);
    });
}

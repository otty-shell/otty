use gpui::prelude::*;
use gpui::{
    App, Application, Bounds, ClickEvent, Context, Entity, Render, Window,
    WindowBounds, WindowOptions, div, px, size,
};
use otty_ui_term_gpui::{LocalBackend, LocalOptions, Terminal, TerminalConfig};

struct BackendSwitching {
    terminal: Entity<Terminal>,
    use_sh: bool,
}

impl BackendSwitching {
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
            use_sh: false,
        }
    }

    fn replace_backend(
        &mut self,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.use_sh = !self.use_sh;
        let options = if self.use_sh {
            LocalOptions::new("/bin/sh")
        } else {
            LocalOptions::default()
        };

        self.terminal.update(cx, |terminal, cx| {
            terminal.replace_backend(LocalBackend::new(options), cx);
        });
    }
}

impl Render for BackendSwitching {
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
                    .id("replace-backend")
                    .px_3()
                    .py_1()
                    .rounded_md()
                    .bg(gpui::rgb(0x5b6ee1))
                    .text_color(gpui::white())
                    .cursor_pointer()
                    .child("Replace backend in the same Entity")
                    .on_click(cx.listener(Self::replace_backend)),
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
            |_, cx| cx.new(BackendSwitching::new),
        )
        .expect("open backend switching example");
        cx.activate(true);
    });
}

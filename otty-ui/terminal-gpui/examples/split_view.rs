use gpui::prelude::*;
use gpui::{
    App, Application, Bounds, Context, Entity, Render, Window, WindowBounds,
    WindowOptions, div, px, size,
};
use otty_ui_term_gpui::{LocalBackend, LocalOptions, Terminal, TerminalConfig};

struct SplitView {
    left: Entity<Terminal>,
    right: Entity<Terminal>,
}

impl SplitView {
    fn new(cx: &mut Context<Self>) -> Self {
        let left = cx.new(|cx| {
            Terminal::new(
                TerminalConfig::default(),
                LocalBackend::new(LocalOptions::default()),
                cx,
            )
        });
        let right = cx.new(|cx| {
            Terminal::new(
                TerminalConfig::default(),
                LocalBackend::new(LocalOptions::default()),
                cx,
            )
        });

        Self { left, right }
    }
}

impl Render for SplitView {
    fn render(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .flex()
            .size_full()
            .gap_2()
            .p_2()
            .child(div().flex_1().h_full().child(self.left.clone()))
            .child(div().flex_1().h_full().child(self.right.clone()))
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1200.0), px(700.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(SplitView::new),
        )
        .expect("open split terminal window");
        cx.activate(true);
    });
}

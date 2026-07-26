use gpui::prelude::*;
use gpui::{
    App, Application, Bounds, Context, Entity, Render, Subscription, Window,
    WindowBounds, WindowOptions, div, px, size,
};
use otty_ui_term_gpui::{
    BlockUiMode, LocalBackend, LocalOptions, Terminal, TerminalBehavior,
    TerminalConfig,
};

struct BlocksOverlay {
    terminal: Entity<Terminal>,
    _updates: Subscription,
}

impl BlocksOverlay {
    fn new(cx: &mut Context<Self>) -> Self {
        let config = TerminalConfig::new(
            Default::default(),
            Default::default(),
            Default::default(),
            TerminalBehavior::default()
                .with_block_ui_mode(BlockUiMode::ExternalOverlay),
            Default::default(),
        );
        let terminal = cx.new(|cx| {
            Terminal::new(
                config,
                LocalBackend::new(LocalOptions::default()),
                cx,
            )
        });
        let updates = cx.observe(&terminal, |_, _, cx| cx.notify());

        Self {
            terminal,
            _updates: updates,
        }
    }
}

impl Render for BlocksOverlay {
    fn render(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let blocks = self
            .terminal
            .read(cx)
            .blocks()
            .iter()
            .map(|block| {
                let command =
                    block.meta.cmd.as_deref().unwrap_or("terminal block");
                div()
                    .px_2()
                    .py_1()
                    .border_b_1()
                    .border_color(gpui::rgb(0x4b5563))
                    .child(command.to_string())
            })
            .collect::<Vec<_>>();

        div()
            .relative()
            .size_full()
            .p_2()
            .child(self.terminal.clone())
            .child(
                div()
                    .absolute()
                    .top_4()
                    .right_4()
                    .w(px(240.0))
                    .max_h(px(400.0))
                    .overflow_hidden()
                    .rounded_md()
                    .bg(gpui::rgba(0x111827e6))
                    .text_color(gpui::white())
                    .children(blocks),
            )
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1000.0), px(650.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(BlocksOverlay::new),
        )
        .expect("open blocks overlay example");
        cx.activate(true);
    });
}

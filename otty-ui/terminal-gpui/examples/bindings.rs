use gpui::prelude::*;
use gpui::{
    App, Application, Bounds, Context, Entity, Keystroke, Render, Window,
    WindowBounds, WindowOptions, div, px, size,
};
use otty_libterm::surface::SurfaceMode;
use otty_ui_term_gpui::{
    BindingAction, LocalBackend, LocalOptions, Terminal, TerminalBinding,
    TerminalBindings, TerminalConfig,
};

struct Bindings {
    terminal: Entity<Terminal>,
}

impl Bindings {
    fn new(cx: &mut Context<Self>) -> Self {
        let mut bindings = TerminalBindings::default();
        bindings.add(TerminalBinding::new(
            Keystroke::parse("ctrl-g").expect("valid example keystroke"),
            SurfaceMode::empty(),
            SurfaceMode::empty(),
            BindingAction::Bytes(b"echo custom GPUI binding\r".to_vec()),
        ));
        let config = TerminalConfig::new(
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            bindings,
        );
        let terminal = cx.new(|cx| {
            Terminal::new(
                config,
                LocalBackend::new(LocalOptions::default()),
                cx,
            )
        });

        Self { terminal }
    }
}

impl Render for Bindings {
    fn render(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .gap_2()
            .p_2()
            .child("Press Ctrl-G to execute the custom binding")
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
            |_, cx| cx.new(Bindings::new),
        )
        .expect("open bindings example");
        cx.activate(true);
    });
}

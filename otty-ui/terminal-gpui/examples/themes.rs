use gpui::prelude::*;
use gpui::{
    App, Application, Bounds, ClickEvent, Context, Entity, Render, Window,
    WindowBounds, WindowOptions, div, px, size,
};
use otty_ui_term_gpui::{
    ColorPalette, LocalBackend, LocalOptions, Terminal, TerminalColor,
    TerminalConfig, TerminalTheme,
};

struct Themes {
    terminal: Entity<Terminal>,
    light: bool,
}

impl Themes {
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
            light: false,
        }
    }

    fn toggle_theme(
        &mut self,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.light = !self.light;
        let (background, foreground) = if self.light {
            (
                TerminalColor::from_rgb(0xf4, 0xf1, 0xe8),
                TerminalColor::from_rgb(0x20, 0x25, 0x2b),
            )
        } else {
            (
                TerminalColor::from_rgb(0x10, 0x12, 0x18),
                TerminalColor::from_rgb(0xea, 0xea, 0xea),
            )
        };
        let palette = ColorPalette::default()
            .with_background(background)
            .with_foreground(foreground);
        let theme = TerminalTheme::new(palette);

        self.terminal.update(cx, |terminal, cx| {
            terminal.set_theme(theme, cx);
        });
    }
}

impl Render for Themes {
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
                    .id("toggle-theme")
                    .px_3()
                    .py_1()
                    .rounded_md()
                    .bg(gpui::rgb(0x5b6ee1))
                    .text_color(gpui::white())
                    .cursor_pointer()
                    .child("Toggle terminal theme")
                    .on_click(cx.listener(Self::toggle_theme)),
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
            |_, cx| cx.new(Themes::new),
        )
        .expect("open themes example");
        cx.activate(true);
    });
}

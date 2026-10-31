//! Gallery of the kit components in every state, in OTTY Dark and OTTY Light.
//!
//! Run with `cargo run -p otty-ui-kit --example gallery`.

use iced::widget::{Space, column, container, row, scrollable, svg, text};
use iced::{Element, Length, Padding, Theme};
use otty_ui_kit::components::button::{Button, ButtonKind, ButtonSize};
use otty_ui_kit::components::icon_button::{
    IconButton, IconButtonSize, IconTone,
};
use otty_ui_kit::components::kbd::{kbd, shortcut_text};
use otty_ui_kit::components::menu::{Menu, MenuItem};
use otty_ui_kit::components::segmented::segmented;
use otty_ui_kit::components::select::select;
use otty_ui_kit::components::switch::Switch;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, spacing, typography};

/// Plus icon used by the icon buttons.
const PLUS_ICON: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" fill="none" stroke="black" stroke-width="1.5" stroke-linecap="round"><path d="M8 3v10M3 8h10"/></svg>"#;

const KINDS: [(&str, ButtonKind); 4] = [
    ("Primary", ButtonKind::Primary),
    ("Secondary", ButtonKind::Secondary),
    ("Ghost", ButtonKind::Ghost),
    ("Danger", ButtonKind::Danger),
];
const TYPES: [&str; 3] = ["Command", "Script", "SSH"];
const TARGETS: [&str; 4] = ["This computer", "prod-1", "staging-2", "dev-box"];

fn main() -> iced::Result {
    let mut app = iced::application(Gallery::new, update, view).theme(
        |state: &Gallery| {
            if state.dark {
                Theme::Dark
            } else {
                Theme::Light
            }
        },
    );
    for bytes in fonts::all_bytes() {
        app = app.font(bytes);
    }

    app.default_font(fonts::UI).run()
}

struct Gallery {
    dark: bool,
    switch_on: bool,
    kind: &'static str,
    target: Option<&'static str>,
    clicks: u32,
}

impl Gallery {
    /// Starts in OTTY Light when `light` is the first argument.
    fn new() -> Self {
        let light = std::env::args().nth(1).as_deref() == Some("light");

        Self {
            dark: !light,
            switch_on: true,
            kind: TYPES[0],
            target: Some(TARGETS[1]),
            clicks: 0,
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    Clicked,
    Switched(bool),
    Kind(&'static str),
    Target(&'static str),
}

fn update(state: &mut Gallery, message: Message) {
    match message {
        Message::ToggleTheme => state.dark = !state.dark,
        Message::Clicked => state.clicks += 1,
        Message::Switched(on) => state.switch_on = on,
        Message::Kind(kind) => state.kind = kind,
        Message::Target(target) => state.target = Some(target),
    }
}

fn view(state: &Gallery) -> Element<'_, Message> {
    let palette = if state.dark {
        UiPalette::otty_dark()
    } else {
        UiPalette::otty_light()
    };
    let p = &palette;

    let theme_name = if state.dark {
        "OTTY Dark"
    } else {
        "OTTY Light"
    };
    let header = row![
        Button::new(format!("Theme: {theme_name}"), p)
            .kind(ButtonKind::Secondary)
            .on_press(Message::ToggleTheme),
        label(format!("clicks: {}", state.clicks), p),
    ]
    .spacing(spacing::SPACE_4)
    .align_y(iced::Alignment::Center);

    let mut buttons = column![].spacing(spacing::SPACE_3);
    for size in [ButtonSize::Form, ButtonSize::Small] {
        let mut idle = row![].spacing(spacing::SPACE_3);
        let mut disabled = row![].spacing(spacing::SPACE_3);
        for (name, kind) in KINDS {
            idle = idle.push(
                Button::new(name, p)
                    .kind(kind)
                    .size(size)
                    .on_press(Message::Clicked),
            );
            disabled = disabled
                .push(Button::<Message>::new(name, p).kind(kind).size(size));
        }
        idle = idle.push(
            Button::new("Save", p)
                .kind(ButtonKind::Primary)
                .size(size)
                .shortcut("Ctrl S")
                .on_press(Message::Clicked),
        );
        buttons = buttons.push(idle).push(disabled);
    }

    let icon = || svg::Handle::from_memory(PLUS_ICON);
    let icon_buttons = row![
        IconButton::new(icon(), "New session", p).on_press(Message::Clicked),
        IconButton::new(icon(), "New session", p)
            .tone(IconTone::Accent)
            .on_press(Message::Clicked),
        IconButton::new(icon(), "Add", p)
            .size(IconButtonSize::Small)
            .on_press(Message::Clicked),
        IconButton::new(icon(), "Add", p)
            .size(IconButtonSize::Small)
            .tone(IconTone::Accent)
            .on_press(Message::Clicked),
    ]
    .spacing(spacing::SPACE_3);

    let keys = row![
        kbd("Ctrl T", p),
        kbd(
            shortcut_text(
                iced::keyboard::Modifiers::CTRL
                    | iced::keyboard::Modifiers::SHIFT,
                "c"
            ),
            p
        ),
        kbd("Alt 1", p),
    ]
    .spacing(spacing::SPACE_3);

    let menu = Menu::new(p)
        .heading("From launches")
        .item(
            MenuItem::new("New session")
                .shortcut("Ctrl T")
                .on_press(Message::Clicked),
        )
        .item(
            MenuItem::new("Active (keyboard)")
                .active(true)
                .on_press(Message::Clicked),
        )
        .item(
            MenuItem::new("Split pane")
                .shortcut("Ctrl Shift D")
                .on_press(Message::Clicked),
        )
        .separator()
        .item(MenuItem::new("Disabled item"))
        .item(
            MenuItem::new("Close tab")
                .shortcut("Ctrl W")
                .on_press(Message::Clicked),
        );

    let switches = column![
        Switch::new(state.switch_on, p)
            .label("Show command blocks")
            .on_toggle(Message::Switched),
        Switch::new(false, p)
            .label("Off")
            .on_toggle(Message::Switched),
        Switch::<Message>::new(true, p).label("On, static"),
    ]
    .spacing(spacing::SPACE_3);

    let segments = column![
        segmented(&TYPES, &state.kind, Message::Kind, p),
        segmented(&TYPES, &TYPES[2], Message::Kind, p),
    ]
    .spacing(spacing::SPACE_3);

    let dropdown = select(&TARGETS, state.target, Message::Target, p);

    let body = column![
        header,
        section(
            "Button (form 34, small 28; second row disabled)",
            buttons.into(),
            p
        ),
        section(
            "IconButton (hover for tooltip; md, accent, sm)",
            icon_buttons.into(),
            p
        ),
        section("Kbd", keys.into(), p),
        section("Menu", menu.into(), p),
        section("Switch", switches.into(), p),
        section("Segmented", segments.into(), p),
        container(section("Select", dropdown, p)).width(280),
        Space::new().height(240),
    ]
    .spacing(spacing::SPACE_6)
    .padding(Padding::from(spacing::SPACE_6));

    let canvas = palette.surface_canvas();
    container(scrollable(body))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_| container::Style {
            background: Some(canvas.into()),
            ..container::Style::default()
        })
        .into()
}

fn label<'a>(content: String, palette: &UiPalette) -> Element<'a, Message> {
    let style = typography::SMALL;

    text(content)
        .size(style.size())
        .color(palette.text_muted())
        .into()
}

fn section<'a>(
    title: &'a str,
    content: Element<'a, Message>,
    palette: &UiPalette,
) -> Element<'a, Message> {
    column![label(title.to_owned(), palette), content]
        .spacing(spacing::SPACE_3)
        .into()
}

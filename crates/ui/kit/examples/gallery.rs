//! Gallery of the kit components in every state, in OTTY Dark and OTTY Light.
//!
//! Run with `cargo run -p otty-ui-kit --example gallery`; add `light` for
//! OTTY Light, and `rows`, `fields` or `settings` to show one group of
//! the second set.

use iced::widget::{Space, column, container, row, scrollable, svg, text};
use iced::{Element, Length, Padding, Theme};
use otty_ui_kit::components::button::{Button, ButtonKind, ButtonSize};
use otty_ui_kit::components::form_section::FormSection;
use otty_ui_kit::components::icon_button::{
    IconButton, IconButtonSize, IconTone,
};
use otty_ui_kit::components::kbd::{kbd, shortcut_text};
use otty_ui_kit::components::key_value_field::KeyValueField;
use otty_ui_kit::components::menu::{Menu, MenuItem};
use otty_ui_kit::components::rail_button::{RailButton, rail_separator};
use otty_ui_kit::components::segmented::segmented;
use otty_ui_kit::components::select::select;
use otty_ui_kit::components::setting_row::SettingRow;
use otty_ui_kit::components::switch::Switch;
use otty_ui_kit::components::tab_strip::{Tab, TabStrip};
use otty_ui_kit::components::text_field::TextField;
use otty_ui_kit::components::token_field::{TokenField, TokenList};
use otty_ui_kit::components::tree_row::TreeRow;
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
/// Width of the tree panel.
const PANEL_WIDTH: f32 = 260.0;
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

/// Which sections the gallery shows.
#[derive(PartialEq)]
enum Part {
    All,
    Rows,
    Fields,
    Settings,
}

struct Gallery {
    dark: bool,
    part: Part,
    switch_on: bool,
    kind: &'static str,
    target: Option<&'static str>,
    clicks: u32,
    path: String,
    tokens: TokenList,
    empty_tokens: TokenList,
    variables: Vec<(String, String)>,
    tab: usize,
    tree_selected: usize,
    rail: usize,
}

impl Gallery {
    /// Starts in OTTY Light with the `light` argument; `rows`, `fields` and `settings` show only
    /// the sections of the second component set.
    fn new() -> Self {
        let args: Vec<String> = std::env::args().collect();
        let light = args.iter().any(|arg| arg == "light");

        Self {
            dark: !light,
            part: if args.iter().any(|arg| arg == "rows") {
                Part::Rows
            } else if args.iter().any(|arg| arg == "fields") {
                Part::Fields
            } else if args.iter().any(|arg| arg == "settings") {
                Part::Settings
            } else {
                Part::All
            },
            switch_on: true,
            kind: TYPES[0],
            target: Some(TARGETS[1]),
            clicks: 0,
            path: "~/projects/otty".to_owned(),
            empty_tokens: TokenList::default(),
            tokens: TokenList::new(vec![
                "--color=always".to_owned(),
                "-n".to_owned(),
                "--very-long-argument-value-that-wraps-to-the-next-line"
                    .to_owned(),
            ]),
            variables: vec![
                ("RUST_LOG".to_owned(), "debug".to_owned()),
                ("PATH".to_owned(), "/usr/local/bin:/usr/bin".to_owned()),
            ],
            tab: 0,
            tree_selected: 2,
            rail: 0,
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
    Path(String),
    TokenInput(String),
    TokenSubmit,
    TokenBackspace,
    TokenRemove(usize),
    VarName(usize, String),
    VarValue(usize, String),
    VarRemove(usize),
    VarAdd,
    Tab(usize),

    Tree(usize),
    Rail(usize),
}

fn update(state: &mut Gallery, message: Message) {
    match message {
        Message::ToggleTheme => state.dark = !state.dark,
        Message::Clicked => state.clicks += 1,
        Message::Switched(on) => state.switch_on = on,
        Message::Kind(kind) => state.kind = kind,
        Message::Target(target) => state.target = Some(target),
        Message::Path(path) => state.path = path,
        Message::TokenInput(input) => state.tokens.set_input(input),
        Message::TokenSubmit => {
            state.tokens.submit();
        },
        Message::TokenBackspace => {
            state.tokens.backspace();
        },
        Message::TokenRemove(index) => {
            state.tokens.remove(index);
        },
        Message::VarName(index, name) => state.variables[index].0 = name,
        Message::VarValue(index, value) => state.variables[index].1 = value,
        Message::VarRemove(index) => {
            state.variables.remove(index);
        },
        Message::VarAdd => state.variables.push(Default::default()),
        Message::Tab(index) => state.tab = index,
        Message::Tree(index) => state.tree_selected = index,
        Message::Rail(index) => state.rail = index,
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

    let tree = column![
        TreeRow::new("Servers", p)
            .caret(true)
            .count(3)
            .selected(state.tree_selected == 0)
            .on_press(Message::Tree(0)),
        TreeRow::new("prod-1", p)
            .depth(1)
            .caret(false)
            .count(12)
            .selected(state.tree_selected == 1)
            .on_press(Message::Tree(1)),
        TreeRow::new("A very long session title that has to be cut at the end of the row", p)
            .depth(2)
            .selected(state.tree_selected == 2)
            .on_press(Message::Tree(2)),
        TreeRow::new("Static row (no message)", p).depth(3),
    ]
    .width(PANEL_WIDTH);

    let rail = column![
        RailButton::new(icon(), "Sessions", p)
            .active(state.rail == 0)
            .on_press(Message::Rail(0)),
        RailButton::new(icon(), "Files", p)
            .active(state.rail == 1)
            .on_press(Message::Rail(1)),
        rail_separator(p),
        RailButton::new(icon(), "Settings", p)
            .active(state.rail == 2)
            .on_press(Message::Rail(2)),
    ]
    .width(48);

    let mut tabs = TabStrip::new(p).on_new(Message::Clicked);
    for (index, title) in [
        "main",
        "build logs",
        "ssh prod-1 (a very long tab title for cutting)",
    ]
    .into_iter()
    .enumerate()
    {
        tabs = tabs.tab(
            Tab::new(title)
                .active(state.tab == index)
                .on_press(Message::Tab(index))
                .on_close(Message::Clicked),
        );
    }
    let mut many = TabStrip::new(p).on_new(Message::Clicked);
    for index in 0..14 {
        many = many.tab(
            Tab::new(format!("session {index}"))
                .active(index == 3)
                .on_press(Message::Clicked),
        );
    }

    let fields = column![
        TextField::new("Working directory", &state.path, p)
            .optional(true)
            .hint("Defaults to the home directory.")
            .action("Browse", Message::Clicked)
            .on_input(Message::Path),
        TextField::new("Program", "", p)
            .placeholder("/usr/bin/zsh")
            .hint("Path or name of the program.")
            .on_input(Message::Path),
        TextField::new("Host", "prod..", p)
            .error("Host name cannot contain two dots.")
            .hint("Hidden: error replaces the hint.")
            .on_input(Message::Path),
        TextField::<Message>::new("Disabled", "read only", p)
            .hint("No input handler."),
        TextField::new("Port", "22", p)
            .width(120)
            .on_input(Message::Path),
    ]
    .spacing(spacing::SPACE_4);

    let token_field = column![
        TokenField::new("Arguments", &state.tokens, p)
            .placeholder("Add argument")
            .hint("Enter adds, Backspace on empty input removes the last.")
            .on_input(Message::TokenInput)
            .on_submit(Message::TokenSubmit)
            .on_backspace_empty(Message::TokenBackspace)
            .on_remove(Message::TokenRemove),
        TokenField::<Message>::new("Disabled", &state.empty_tokens, p),
    ]
    .spacing(spacing::SPACE_4);

    let key_values =
        KeyValueField::new("Environment variables", &state.variables, p)
            .optional(true)
            .hint("Passed to the program as-is.")
            .on_name(Message::VarName)
            .on_value(Message::VarValue)
            .on_remove(Message::VarRemove)
            .on_add(Message::VarAdd);

    let settings = column![
        SettingRow::new("Command blocks", p)
            .description("Group terminal output by command.")
            .control(
                Switch::new(state.switch_on, p).on_toggle(Message::Switched)
            ),
        SettingRow::new("Default target", p)
            .description("Used by New session.")
            .control(select(&TARGETS, state.target, Message::Target, p)),
        SettingRow::new("No description", p).control(segmented(
            &TYPES,
            &state.kind,
            Message::Kind,
            p
        )),
    ];

    let form = FormSection::new("Command", p)
        .field(
            TextField::new("Program", "/usr/bin/zsh", p)
                .on_input(Message::Path),
        )
        .field(
            TextField::new("Arguments", "-l", p)
                .optional(true)
                .on_input(Message::Path),
        );

    let first: Vec<Element<'_, Message>> = vec![
        section(
            "Button (form 34, small 28; second row disabled)",
            buttons.into(),
            p,
        ),
        section(
            "IconButton (hover for tooltip; md, accent, sm)",
            icon_buttons.into(),
            p,
        ),
        section("Kbd", keys.into(), p),
        section("Menu", menu.into(), p),
        section("Switch", switches.into(), p),
        section("Segmented", segments.into(), p),
        container(section("Select", dropdown, p)).width(280).into(),
    ];
    let rows: Vec<Element<'_, Message>> = vec![
        section(
            "TreeRow (depth 0-3; selected, hover, static; long title cut)",
            tree.into(),
            p,
        ),
        section("RailButton (active, idle, separator)", rail.into(), p),
        section("TabStrip (active tab, hover, close, plus)", tabs.into(), p),
        section("TabStrip (many tabs: clipped at the edge)", many.into(), p),
    ];
    let form_fields: Vec<Element<'_, Message>> = vec![
        section(
            "TextField (optional+hint+action, placeholder, error, disabled, short)",
            fields.into(),
            p,
        ),
        section("TokenField (wraps; disabled)", token_field.into(), p),
    ];
    let settings_parts: Vec<Element<'_, Message>> = vec![
        section("KeyValueField", key_values.into(), p),
        section("SettingRow", settings.into(), p),
        section("FormSection", form.into(), p),
    ];
    let mut body = column![header];
    if state.part == Part::All {
        body = body.extend(first);
    }
    if matches!(state.part, Part::All | Part::Rows) {
        body = body.extend(rows);
    }
    if matches!(state.part, Part::All | Part::Fields) {
        body = body.extend(form_fields);
    }
    if matches!(state.part, Part::All | Part::Settings) {
        body = body.extend(settings_parts);
    }
    let body = body
        .push(Space::new().height(240))
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

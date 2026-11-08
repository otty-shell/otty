//! Gallery of the kit components in every state, in OTTY Dark and OTTY Light.
//!
//! Run with `cargo run -p otty-ui-kit --example gallery`; add `light` for
//! OTTY Light, and `rows`, `fields`, `settings`, `inspector` or `data` to
//! show one group of the second set.

use iced::widget::pane_grid::{self, PaneGrid};
use iced::widget::{
    Column, Space, column, container, row, scrollable, stack, svg, text,
    text_editor,
};
use iced::{Element, Length, Padding, Theme};
use otty_ui_kit::components::action_tile::ActionTile;
use otty_ui_kit::components::button::{Button, ButtonKind, ButtonSize};
use otty_ui_kit::components::chip::Chip;
use otty_ui_kit::components::command_box::CommandBox;
use otty_ui_kit::components::command_palette::{CommandPalette, PaletteAction};
use otty_ui_kit::components::data_table::{
    CellPart, ColumnAlign, DataTable, SortDirection, TableCell, TableColumn,
};
use otty_ui_kit::components::data_tree::{DataRow, DataTree};
use otty_ui_kit::components::dialog::Dialog;
use otty_ui_kit::components::empty_card::EmptyCard;
use otty_ui_kit::components::floating_note::FloatingNote;
use otty_ui_kit::components::form_page::FormPage;
use otty_ui_kit::components::form_section::FormSection;
use otty_ui_kit::components::icon_button::{
    IconButton, IconButtonSize, IconTone,
};
use otty_ui_kit::components::inspector_header::InspectorHeader;
use otty_ui_kit::components::inspector_tabs::inspector_tabs;
use otty_ui_kit::components::kbd::{kbd, shortcut_text};
use otty_ui_kit::components::key_value_field::KeyValueField;
use otty_ui_kit::components::key_value_list::{KeyValueList, ValuePart};
use otty_ui_kit::components::log_view::{LogView, LogViewState};
use otty_ui_kit::components::mapping_field::{MappingField, MappingRow};
use otty_ui_kit::components::menu::{Menu, MenuItem};
use otty_ui_kit::components::mode_menu::{
    ModeMenu, ModeMenuSection, ModePin, ModeRow,
};
use otty_ui_kit::components::mode_switcher::{ModeSwitcher, ModeTab};
use otty_ui_kit::components::notice::Notice;
use otty_ui_kit::components::palette_list::{PaletteItem, PaletteList};
use otty_ui_kit::components::pane_header::PaneHeader;
use otty_ui_kit::components::panel_section::PanelSection;
use otty_ui_kit::components::progress_bar::ProgressBar;
use otty_ui_kit::components::screen_area::ScreenArea;
use otty_ui_kit::components::segmented::{segmented, segmented_with_disabled};
use otty_ui_kit::components::select::select;
use otty_ui_kit::components::session_group::SessionGroup;
use otty_ui_kit::components::session_row::{SessionRow, SessionTone};
use otty_ui_kit::components::setting_row::SettingRow;
use otty_ui_kit::components::split::{self, ResizableGrid};
use otty_ui_kit::components::split_button::SplitButton;
use otty_ui_kit::components::status_badge::{StatusBadge, StatusTone};
use otty_ui_kit::components::status_bar::{StatusBar, StatusItem};
use otty_ui_kit::components::switch::Switch;
use otty_ui_kit::components::tag_row::TagRow;
use otty_ui_kit::components::tag_search::{
    SearchRow, Tag, TagKey, TagSchema, TagSearch, TagSearchAction,
};
use otty_ui_kit::components::tag_search_field::TagSearchField;
use otty_ui_kit::components::target_dot::{TargetKind, target_dot};
use otty_ui_kit::components::text_field::TextField;
use otty_ui_kit::components::title_search::TitleSearch;
use otty_ui_kit::components::token_field::{TokenField, TokenList};
use otty_ui_kit::components::toolbar::{Toolbar, ToolbarItem};
use otty_ui_kit::components::tree_row::TreeRow;
use otty_ui_kit::components::{divider, scrollbar};
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, size, spacing, typography};

/// Plus icon used by the icon buttons.
const PLUS_ICON: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" fill="none" stroke="black" stroke-width="1.5" stroke-linecap="round"><path d="M8 3v10M3 8h10"/></svg>"#;

/// Terminal mode icon.
const TERMINAL_ICON: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="black" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="4" width="18" height="16" rx="2"/><path d="m7 9 3 3-3 3M13 15h4"/></svg>"#;

/// Containers mode icon.
const CONTAINERS_ICON: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="black" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="10" width="18" height="9" rx="2"/><path d="M6 10V7h4v3M10 10V5h4v5M14 10V7h4v3"/></svg>"#;

/// Stand-in icon for modes that do not exist yet.
const GRID_ICON: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="black" stroke-width="2" stroke-linejoin="round"><rect x="4" y="4" width="7" height="7" rx="1"/><rect x="13" y="4" width="7" height="7" rx="1"/><rect x="4" y="13" width="7" height="7" rx="1"/><rect x="13" y="13" width="7" height="7" rx="1"/></svg>"#;

/// Search icon of the title search.
const SEARCH_ICON: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="black" stroke-width="2"><circle cx="11" cy="11" r="7"/><path d="m20 20-3.5-3.5"/></svg>"#;

/// Folder icon of the empty card.
const FOLDER_ICON: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="black" stroke-width="1.8"><path d="M3 6a1 1 0 0 1 1-1h5l2 2h9a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1z"/></svg>"#;

/// Launch icon of the empty card.
const LAUNCH_ICON: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="black" stroke-width="1.8"><circle cx="12" cy="12" r="9"/><path d="m10 8 6 4-6 4z"/></svg>"#;

/// Error icon of the notice.
const ERROR_ICON: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="black" stroke-width="2"><circle cx="12" cy="12" r="9"/><path d="M12 8v5M12 16.5v.5"/></svg>"#;

/// Product logo, passed to the empty card as an element.
const LOGO: &[u8] = include_bytes!("../../../../assets/logo/logo-small.svg");

/// Stroke icons of the resource actions, from `assets/svg`.
const START_ICON: &[u8] = include_bytes!("../../../../assets/svg/start.svg");
const STOP_ICON: &[u8] = include_bytes!("../../../../assets/svg/stop.svg");
const RESTART_ICON: &[u8] =
    include_bytes!("../../../../assets/svg/restart.svg");
const PAUSE_ICON: &[u8] = include_bytes!("../../../../assets/svg/pause.svg");
const RESUME_ICON: &[u8] = include_bytes!("../../../../assets/svg/resume.svg");
const TRASH_ICON: &[u8] = include_bytes!("../../../../assets/svg/trash.svg");
const SHELL_ICON: &[u8] = include_bytes!("../../../../assets/svg/shell.svg");
const PULL_ICON: &[u8] = include_bytes!("../../../../assets/svg/pull.svg");
const PRUNE_ICON: &[u8] = include_bytes!("../../../../assets/svg/prune.svg");
const CREATE_ICON: &[u8] = include_bytes!("../../../../assets/svg/plus.svg");
const MORE_ICON: &[u8] = include_bytes!("../../../../assets/svg/more.svg");
const REFRESH_ICON: &[u8] =
    include_bytes!("../../../../assets/svg/refresh.svg");
const COPY_ICON: &[u8] = include_bytes!("../../../../assets/svg/copy.svg");

/// The action icons with their names, in the order the gallery shows them.
const ACTION_ICONS: [(&str, &[u8]); 13] = [
    ("Start", START_ICON),
    ("Stop", STOP_ICON),
    ("Restart", RESTART_ICON),
    ("Pause", PAUSE_ICON),
    ("Resume", RESUME_ICON),
    ("Remove", TRASH_ICON),
    ("Shell", SHELL_ICON),
    ("Pull", PULL_ICON),
    ("Prune", PRUNE_ICON),
    ("Create", CREATE_ICON),
    ("More", MORE_ICON),
    ("Refresh", REFRESH_ICON),
    ("Copy JSON", COPY_ICON),
];

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

const INSPECTOR_TABS: [&str; 3] = ["Overview", "Logs", "Inspect"];

const PROTOCOLS: [&str; 2] = ["tcp", "udp"];

/// Which sections the gallery shows.
#[derive(PartialEq)]
enum Part {
    All,
    Rows,
    Fields,
    Settings,
    Inspector,
    Data,
}

struct Gallery {
    inspector_tab: &'static str,
    log: LogViewState,
    log_lines: u32,
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
    tree_selected: usize,
    dialog_open: bool,
    wide_dialog_open: bool,
    sessions_open: bool,
    menu_open: bool,
    commands: PaletteList<u32>,
    no_commands: PaletteList<u32>,
    table_selected: Option<usize>,
    table_menu_at: Option<iced::Point>,
    notice_open: bool,
    table_sort: (usize, SortDirection),
    table_event: String,
    toolbar_filter: String,
    tag_search: TagSearch,
    inspector_event: String,
    data_tree_state_open: bool,
    data_tree_mounts_open: bool,
    split_panes: pane_grid::State<&'static str>,
}

impl Gallery {
    /// Starts in OTTY Light with the `light` argument; `rows`, `fields` and `settings` show only
    /// the sections of the second component set.
    fn new() -> Self {
        let args: Vec<String> = std::env::args().collect();
        let light = args.iter().any(|arg| arg == "light");

        let mut log = LogViewState::new();
        let mut tag_search = TagSearch::default();
        let _ = tag_search.set_tag("status", "Running");

        log.push_lines((1..=30).map(|n| {
            format!("2026-11-01T10:00:{n:02}Z INFO request {n} served")
        }));
        log.push_lines([
            "2026-11-01T10:00:31Z WARN upstream answered slowly: \
             0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                .to_owned(),
        ]);

        Self {
            inspector_tab: INSPECTOR_TABS[0],
            log,
            log_lines: 31,
            dark: !light,
            part: if args.iter().any(|arg| arg == "inspector") {
                Part::Inspector
            } else if args.iter().any(|arg| arg == "rows") {
                Part::Rows
            } else if args.iter().any(|arg| arg == "fields") {
                Part::Fields
            } else if args.iter().any(|arg| arg == "settings") {
                Part::Settings
            } else if args.iter().any(|arg| arg == "data") {
                Part::Data
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
            tree_selected: 2,
            dialog_open: false,
            wide_dialog_open: false,
            sessions_open: true,
            menu_open: false,
            commands: palette_list("la"),
            no_commands: palette_list("docker"),
            table_selected: Some(1),
            table_menu_at: None,
            notice_open: true,
            table_sort: (0, SortDirection::Ascending),
            table_event: "press a title, a Compose link or a row ⋯".to_owned(),
            toolbar_filter: String::new(),
            tag_search,
            inspector_event: "press a link in the list".to_owned(),
            data_tree_state_open: true,
            data_tree_mounts_open: false,
            split_panes: pane_grid::State::with_configuration(
                pane_grid::Configuration::Split {
                    axis: pane_grid::Axis::Vertical,
                    ratio: 0.5,
                    a: Box::new(pane_grid::Configuration::Pane("left")),
                    b: Box::new(pane_grid::Configuration::Pane("right")),
                },
            ),
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

    Tree(usize),
    OpenDialog,
    OpenWideDialog,
    CloseDialog,
    ToggleSessions,
    ToggleMenu,
    Palette(PaletteAction),
    InspectorTab(&'static str),
    Log(text_editor::Action),
    Follow(bool),
    AppendLog,

    TableRow(usize),
    TableMenu(usize, iced::Point),
    DismissNotice,
    ShowNotice,
    TableSort(usize),
    TableLink(usize, usize, usize),
    TableRowMenu(usize, iced::Point),
    ToolbarFilter(String),
    ToolbarAction(&'static str),
    TagSearch(TagSearchAction),
    InspectorLink(&'static str),
    ToggleDataTree(&'static str),
    SplitResized(pane_grid::ResizeEvent),
}

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

fn update(state: &mut Gallery, message: Message) {
    match message {
        Message::ToggleTheme => state.dark = !state.dark,
        Message::Clicked => state.clicks += 1,
        Message::Switched(on) => state.switch_on = on,
        Message::SplitResized(event) => {
            state.split_panes.resize(event.split, event.ratio);
        },
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
        Message::Tree(index) => state.tree_selected = index,
        Message::OpenDialog => state.dialog_open = true,
        Message::OpenWideDialog => state.wide_dialog_open = true,
        Message::CloseDialog => {
            state.dialog_open = false;
            state.wide_dialog_open = false;
        },
        Message::ToggleSessions => state.sessions_open = !state.sessions_open,
        Message::ToggleMenu => state.menu_open = !state.menu_open,
        Message::InspectorTab(tab) => state.inspector_tab = tab,
        Message::Log(action) => state.log.perform(action),
        Message::Follow(is_following) => {
            state.log.set_following(is_following);
        },
        Message::AppendLog => {
            state.log_lines += 1;
            let line =
                format!("2026-11-01T10:01:00Z INFO line {}", state.log_lines);
            state.log.push_lines([line]);
        },
        Message::Palette(action) => match action {
            PaletteAction::QueryChanged(query) => {
                state.commands.set_query(query);
            },
            PaletteAction::MoveUp => state.commands.move_up(),
            PaletteAction::MoveDown => state.commands.move_down(),
            PaletteAction::RunActive | PaletteAction::RunItem(_) => {
                state.clicks += 1;
            },
            PaletteAction::Close => state.commands.set_query(String::new()),
        },
        Message::TableRow(index) => state.table_selected = Some(index),
        Message::TableMenu(index, point) => {
            state.table_selected = Some(index);
            state.table_menu_at = Some(point);
        },
        Message::DismissNotice => state.notice_open = false,
        Message::ShowNotice => state.notice_open = true,
        Message::TableSort(column) => {
            let (sorted, direction) = state.table_sort;
            let direction = match (sorted == column, direction) {
                (true, SortDirection::Ascending) => SortDirection::Descending,
                _ => SortDirection::Ascending,
            };
            state.table_sort = (column, direction);
            state.table_event = format!("sort column {column}, {direction:?}");
        },
        Message::TableLink(row, column, link) => {
            state.table_event =
                format!("link {link} in row {row}, column {column}");
        },
        Message::TableRowMenu(row, point) => {
            state.table_event = format!(
                "⋯ of row {row}, menu at {:.0}, {:.0}",
                point.x, point.y
            );
        },
        Message::ToolbarFilter(filter) => state.toolbar_filter = filter,
        Message::ToolbarAction(action) => {
            state.table_event = format!("toolbar: {action}");
        },
        Message::TagSearch(action) => {
            let _ = state.tag_search.perform(action, &tag_schema());
        },
        Message::InspectorLink(link) => {
            state.inspector_event = format!("link: {link}");
        },
        Message::ToggleDataTree("State") => {
            state.data_tree_state_open = !state.data_tree_state_open;
        },
        Message::ToggleDataTree(_) => {
            state.data_tree_mounts_open = !state.data_tree_mounts_open;
        },
    }
}

/// What the gallery's tag search knows: its keys and twelve rows, enough for
/// the suggestion list to scroll.
fn tag_schema() -> TagSchema {
    let rows = (1..=12)
        .map(|index| {
            let name = format!("web-{index}");
            let status = if index % 3 == 0 { "Exited" } else { "Running" };

            SearchRow::new(name.as_str())
                .value("name", name)
                .value("status", status)
                .value("image", format!("nginx:1.{index}"))
                .value("ports", format!("80{index:02}:80/tcp"))
                .value("ports", format!("84{index:02}:443/tcp"))
        })
        .collect();

    let keys = vec![
        TagKey::new("name", "Name"),
        TagKey::new("status", "Status"),
        TagKey::new("image", "Image"),
        TagKey::new("ports", "Ports"),
    ];

    TagSchema::new(keys, rows)
}

/// A tree as the Inspect tab of a container shows it.
fn data_tree<'a>(state: &Gallery, palette: &UiPalette) -> Element<'a, Message> {
    let general = vec![
        DataRow::new(
            0,
            "Id",
            "sha256:0123456789abcdef0123456789abcdef0123456789abcdef",
        ),
        DataRow::with_parts(
            0,
            "Image",
            [ValuePart::link(
                "nginx:1.27",
                Message::InspectorLink("image"),
            )],
        ),
        DataRow::new(0, "Name", "/web-1"),
        DataRow::with_parts(
            0,
            "Ports",
            [
                ValuePart::link(
                    "8080:80/tcp",
                    Message::InspectorLink("http://localhost:8080"),
                ),
                ValuePart::text(", "),
                ValuePart::link(
                    "8443:443/tcp",
                    Message::InspectorLink("http://localhost:8443"),
                ),
            ],
        ),
        DataRow::new(0, "RestartCount", "0"),
        DataRow::heading(0, "Networks"),
        DataRow::heading_link(
            1,
            "otty-fx-net",
            Message::InspectorLink("network"),
        ),
        DataRow::new(2, "IPAddress", "172.18.0.2"),
    ];
    let state_rows = vec![
        DataRow::new(0, "Status", "running"),
        DataRow::heading(0, "Health"),
        DataRow::new(1, "FailingStreak", "0"),
        DataRow::heading(1, "Log"),
        DataRow::heading(2, "1"),
        DataRow::new(3, "ExitCode", "0"),
        DataRow::new(3, "Output", "ok"),
    ];
    let mounts_rows = vec![
        DataRow::heading(0, "1"),
        DataRow::new(1, "Destination", "/usr/share/nginx/html"),
        DataRow::with_parts(
            1,
            "Name",
            [ValuePart::link(
                "otty-fx-data",
                Message::InspectorLink("volume"),
            )],
        ),
        DataRow::new(1, "Type", "volume"),
    ];

    DataTree::new(palette)
        .group("General", general)
        .section(
            "State",
            state.data_tree_state_open,
            state_rows,
            Message::ToggleDataTree("State"),
        )
        .section(
            "Mounts",
            state.data_tree_mounts_open,
            mounts_rows,
            Message::ToggleDataTree("Mounts"),
        )
        .into()
}

/// Palette entries as in the V4Palette screen, with `query` typed.
fn palette_list(query: &str) -> PaletteList<u32> {
    let mut list = PaletteList::new(vec![
        PaletteItem::new(1, "Launches", "laptop")
            .with_target(TargetKind::Ssh)
            .with_detail("ubuntu@192.168.1.20")
            .with_shortcut("Alt 1"),
        PaletteItem::new(2, "Sessions", "zsh  ~/projects/otty")
            .with_target(TargetKind::Local),
        PaletteItem::new(3, "Sessions", "htop on laptop")
            .with_target(TargetKind::Ssh),
        PaletteItem::new(4, "Actions", "Settings: Language"),
        PaletteItem::new(5, "Actions", "New launch…")
            .with_shortcut("Ctrl Shift T"),
    ]);
    list.set_query(query.to_owned());

    list
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
        IconButton::new(svg::Handle::from_memory(TRASH_ICON), "Remove", p)
            .tone(IconTone::Danger)
            .on_press(Message::Clicked),
        IconButton::new(svg::Handle::from_memory(TRASH_ICON), "Remove", p)
            .tone(IconTone::Danger)
            .on_press_maybe(None),
        IconButton::new(svg::Handle::from_memory(STOP_ICON), "Stop", p)
            .on_press_maybe(None),
    ]
    .spacing(spacing::SPACE_3);

    let keys = row![
        kbd("Ctrl T", p),
        kbd(
            shortcut_text(
                iced::keyboard::Modifiers::CTRL
                    | iced::keyboard::Modifiers::SHIFT,
                "c",
                "Super"
            ),
            p
        ),
        kbd("Alt 1", p),
    ]
    .spacing(spacing::SPACE_3);

    let dividers = column![
        divider::horizontal(p),
        container(divider::vertical(p)).height(24),
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
        // A label stays on one line; without room it ends with "…".
        container(
            Switch::new(state.switch_on, p)
                .label("Remove the container when it stops")
                .on_toggle(Message::Switched),
        )
        .width(180),
    ]
    .spacing(spacing::SPACE_3);

    let split_grid = PaneGrid::new(&state.split_panes, |_, title, _| {
        let rows = (1..=30).map(|n| text(format!("{title} {n}")).into());
        let list = scrollable(Column::with_children(rows))
            .direction(scrollbar::vertical())
            .style(scrollbar::style(p))
            .width(Length::Fill)
            .height(Length::Fill);

        pane_grid::Content::new(list)
    })
    .spacing(size::LINE)
    .min_size(50)
    .style(split::style(p));
    let split_view: Element<'_, Message> = container(
        ResizableGrid::new(split_grid, &state.split_panes, 50.0, p)
            .on_resize(Message::SplitResized),
    )
    .width(360)
    .height(140)
    .into();

    let segments = column![
        segmented(&TYPES, &state.kind, Message::Kind, p),
        segmented(&TYPES, &TYPES[2], Message::Kind, p),
        segmented_with_disabled(
            &TYPES,
            &TYPES[1..],
            &TYPES[0],
            Message::Kind,
            p
        ),
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

    let fields = column![
        TextField::new("Working directory", &state.path, p)
            .optional("optional")
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
            .on_remove(Message::TokenRemove)
            .remove_label("Remove"),
        TokenField::<Message>::new("Disabled", &state.empty_tokens, p),
    ]
    .spacing(spacing::SPACE_4);

    let key_values =
        KeyValueField::new("Environment variables", &state.variables, p)
            .optional("optional")
            .hint("Passed to the program as-is.")
            .on_name(Message::VarName)
            .on_value(Message::VarValue)
            .on_remove(Message::VarRemove)
            .on_add(Message::VarAdd)
            .placeholders("Name", "Value")
            .add_label("Add variable")
            .remove_label("Remove variable");

    let protocol =
        |picked: &'static str| segmented(&PROTOCOLS, &picked, Message::Kind, p);
    let ports = MappingField::new("Host port", "Container port", p)
        .row(
            MappingRow::new("8081", "80")
                .placeholders("8080", "80")
                .control(protocol(PROTOCOLS[0]))
                .on_source(Message::Path)
                .on_target(Message::Path)
                .on_remove(Message::Clicked)
                .remove_label("Remove port 8081"),
        )
        .row(
            MappingRow::new("80800", "443")
                .invalid(true, false)
                .error("A port is a number from 1 to 65535.")
                .control(protocol(PROTOCOLS[1]))
                .on_source(Message::Path)
                .on_target(Message::Path)
                .on_remove(Message::Clicked)
                .remove_label("Remove port 80800"),
        )
        .add_label("Add port")
        .on_add(Message::Clicked)
        .hint("The image listens on 80/tcp.");
    let volumes = MappingField::new(
        "Volume or folder on this computer",
        "Path in the container",
        p,
    )
    .row(
        MappingRow::new("~/projects/otty-site/dist", "/usr/share/nginx/html")
            .control(Switch::new(true, p).label("Read only"))
            .detail(
                row![
                    Switch::new(false, p)
                        .label("Let the container read this folder (:z)"),
                    text("Changes the SELinux labels of the folder's files.")
                        .size(typography::SMALL.size())
                        .color(palette.text_muted()),
                ]
                .spacing(spacing::SPACE_3),
            )
            .on_remove(Message::Clicked)
            .remove_label("Remove this volume"),
    )
    .add_label("Add volume");
    let mappings = column![ports, volumes].spacing(spacing::SPACE_6);
    let narrow_page = FormPage::new(p)
        .section(
            row![
                text("Run container").width(Length::Fill),
                Button::new("Far action", p).on_press(Message::Clicked),
            ]
            .align_y(iced::Alignment::Center),
        )
        .section(FormSection::new("Image", p).field(
            TextField::new("Image", "nginx:1.27", p).on_input(Message::Path),
        ))
        .section(
            FormSection::new("Command", p).field(
                TextField::new("Command", "", p).on_input(Message::Path),
            ),
        )
        .footer(
            row![
                text("A hint beside the buttons. It scrolls with the form.")
                    .size(typography::SMALL.size())
                    .color(palette.text_muted())
                    .width(Length::Fill),
                Button::new("Cancel", p)
                    .kind(ButtonKind::Ghost)
                    .on_press(Message::Clicked),
                Button::new("Run", p)
                    .kind(ButtonKind::Primary)
                    .on_press(Message::Clicked),
            ]
            .spacing(spacing::SPACE_3)
            .align_y(iced::Alignment::Center),
        );
    let footer_page = FormPage::new(p)
        .section(FormSection::new("Image", p).field(
            TextField::new("Image", "nginx:1.27", p).on_input(Message::Path),
        ))
        .section(
            FormSection::new("Command", p).field(
                TextField::new("Command", "", p).on_input(Message::Path),
            ),
        )
        .footer(
            row![
                text("Starts in the background on Docker 27.3.")
                    .size(typography::SMALL.size())
                    .color(palette.text_muted())
                    .width(Length::Fill),
                Button::new("Cancel", p)
                    .kind(ButtonKind::Ghost)
                    .on_press(Message::Clicked),
                Button::new("Run", p)
                    .kind(ButtonKind::Primary)
                    .on_press(Message::Clicked),
            ]
            .spacing(spacing::SPACE_3)
            .align_y(iced::Alignment::Center),
        );

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
                .optional("optional")
                .on_input(Message::Path),
        );

    let page = FormPage::new(p)
        .section(text("Settings"))
        .section(FormSection::new("General", p).field(
            SettingRow::new("Language", p).control(segmented(
                &TYPES,
                &state.kind,
                Message::Kind,
                p,
            )),
        ))
        .section(FormSection::new("Terminal", p).field(
            TextField::new("Shell", "/usr/bin/zsh", p).on_input(Message::Path),
        ))
        .footer(
            Button::new("Save", p)
                .kind(ButtonKind::Primary)
                .on_press(Message::Switched(true)),
        );

    let open_dialog = Button::new("Open dialog", p)
        .kind(ButtonKind::Secondary)
        .on_press(Message::OpenDialog);
    let open_wide_dialog = Button::new("Open wide dialog", p)
        .kind(ButtonKind::Secondary)
        .on_press(Message::OpenWideDialog);
    let dialogs = row![open_dialog, open_wide_dialog].spacing(spacing::SPACE_3);

    let plus = || svg::Handle::from_memory(PLUS_ICON);
    let split = |is_open: bool| {
        SplitButton::new(
            plus(),
            "New terminal (Ctrl T)",
            "More ways to start a session",
            p,
        )
        .menu_open(is_open)
        .on_press(Message::Clicked)
        .on_menu(Message::ToggleMenu)
    };
    let split_buttons = row![split(state.menu_open), split(true)]
        .spacing(spacing::SPACE_4)
        .align_y(iced::Alignment::Center);

    let mode_tab =
        |bytes: &'static [u8], name: &'static str, keys: &'static str| {
            ModeTab::new(svg::Handle::from_memory(bytes), name)
                .shortcut(keys)
                .on_press(Message::Clicked)
        };
    let mode_switchers = row![
        ModeSwitcher::new(p)
            .tab(
                mode_tab(TERMINAL_ICON, "Terminal", "Ctrl Shift 1")
                    .active(true)
            )
            .tab(mode_tab(CONTAINERS_ICON, "Containers", "Ctrl Shift 2"))
            .more("All modes (Ctrl Shift M)", false, Message::Clicked),
        ModeSwitcher::new(p)
            .tab(mode_tab(TERMINAL_ICON, "Terminal", "Ctrl Shift 1"))
            .tab(mode_tab(CONTAINERS_ICON, "Containers", "Ctrl Shift 2"))
            .tab(
                mode_tab(GRID_ICON, "Grid", "Ctrl Shift 3")
                    .active(true)
                    .temporary(true)
            )
            .more("All modes (Ctrl Shift M)", true, Message::Clicked),
    ]
    .spacing(spacing::SPACE_6)
    .align_y(iced::Alignment::Center);

    let pinned = |label: &str| ModePin::Pinned {
        label: label.to_owned(),
        on_press: Message::Clicked,
    };
    let mode_row =
        |bytes: &'static [u8], name: &'static str, keys: &'static str| {
            ModeRow::new(svg::Handle::from_memory(bytes), name)
                .shortcut(keys)
                .on_press(Message::Clicked)
        };
    let full_menu = ModeMenu::new(p)
        .section(
            ModeMenuSection::new("Pinned")
                .note("4 of 4")
                .row(mode_row(TERMINAL_ICON, "Terminal", "Ctrl Shift 1"))
                .row(
                    mode_row(CONTAINERS_ICON, "Containers", "Ctrl Shift 2")
                        .pin(pinned("Unpin")),
                )
                .row(
                    mode_row(GRID_ICON, "Grid", "Ctrl Shift 3")
                        .pin(pinned("Unpin")),
                )
                .row(
                    mode_row(GRID_ICON, "Grid two", "Ctrl Shift 4")
                        .pin(pinned("Unpin")),
                )
                .row(
                    mode_row(GRID_ICON, "Grid three", "Ctrl Shift 5")
                        .pin(pinned("Unpin")),
                ),
        )
        .section(
            ModeMenuSection::new("Other modes").row(
                ModeRow::new(svg::Handle::from_memory(GRID_ICON), "Grid four")
                    .on_press(Message::Clicked)
                    .pin(ModePin::Disabled {
                        hint: "Unpin another mode first".to_owned(),
                    }),
            ),
        );
    let short_menu = ModeMenu::new(p)
        .section(
            ModeMenuSection::new("Pinned")
                .note("2 of 4")
                .row(mode_row(TERMINAL_ICON, "Terminal", "Ctrl Shift 1"))
                .row(
                    mode_row(CONTAINERS_ICON, "Containers", "Ctrl Shift 2")
                        .pin(pinned("Unpin")),
                ),
        )
        .section(
            ModeMenuSection::new("Other modes")
                .row(
                    ModeRow::new(svg::Handle::from_memory(GRID_ICON), "Grid")
                        .on_press(Message::Clicked)
                        .pin(ModePin::Unpinned {
                            label: "Pin".to_owned(),
                            on_press: Message::Clicked,
                        }),
                )
                .hint("Every other mode is pinned."),
        );
    let mode_menus = row![full_menu, short_menu]
        .spacing(spacing::SPACE_6)
        .align_y(iced::Alignment::Start);

    let pane_headers = column![
        PaneHeader::new("zsh", p)
            .folder("~/projects/otty")
            .focused(true)
            .trailing(Chip::new("fedora", p).target(TargetKind::Local)),
        PaneHeader::new("claude", p)
            .folder("~/projects/otty")
            .trailing(Chip::new("needs you", p)),
    ];
    let inspector_header = InspectorHeader::new("web-1", p)
        .status(
            StatusBadge::new(
                "Exited (137) 3 hours ago, stopped by the out of memory killer",
                StatusTone::Error,
                p,
            )
            .wrapping(true),
        )
        .action(
            Button::new("Stop", p)
                .size(ButtonSize::Small)
                .on_press(Message::Clicked),
        )
        .action(
            Button::new("Restart", p)
                .size(ButtonSize::Small)
                .on_press(Message::Clicked),
        )
        .action(
            Button::new("Remove", p)
                .size(ButtonSize::Small)
                .kind(ButtonKind::Danger),
        )
        .hint("Stop the container before removing it.");
    let wide_tabs = inspector_tabs(
        &INSPECTOR_TABS,
        &state.inspector_tab,
        Message::InspectorTab,
        p,
    );
    let narrow_tabs = container(inspector_tabs(
        &INSPECTOR_TABS,
        &state.inspector_tab,
        Message::InspectorTab,
        p,
    ))
    .width(120);
    // The same list twice: two columns at 320px, stacked at 200px.
    let inspector_list = || -> Element<'_, Message> {
        KeyValueList::new(p)
        .entry_parts(
            "Image",
            [ValuePart::link(
                "nginx:1.27-alpine",
                Message::InspectorLink("image"),
            )],
        )
        .entry_parts(
            "Ports",
            [
                ValuePart::link(
                    "8080:80/tcp",
                    Message::InspectorLink("http://localhost:8080"),
                ),
                ValuePart::text(", "),
                ValuePart::link(
                    "8443:443/tcp",
                    Message::InspectorLink("http://localhost:8443"),
                ),
                ValuePart::text(", 53/udp"),
            ],
        )
        .entry_parts(
            "Mounts",
            [
                ValuePart::link(
                    "otty-site-data",
                    Message::InspectorLink("volume"),
                ),
                ValuePart::text(" → /usr/share/nginx/html, /srv/site → /srv"),
            ],
        )
        .entry(
            "ID",
            "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        )
        .entry("Created", "2026-11-01 18:30 UTC")
        .into()
    };
    let inspector_header: Element<'_, Message> = inspector_header.into();
    let inspector = container(
        column![
            inspector_header,
            wide_tabs,
            narrow_tabs,
            inspector_list(),
            container(inspector_list()).width(200),
            label(state.inspector_event.clone(), p),
        ]
        .spacing(spacing::SPACE_4),
    )
    .width(320);
    let waiting = match state.log.pending_count() {
        0 => String::new(),
        1 => "1 new line".to_owned(),
        count => format!("{count} new lines"),
    };
    let log_view = column![
        row![
            Button::new("Add line", p)
                .size(ButtonSize::Small)
                .on_press(Message::AppendLog),
            Space::new().width(Length::Fill),
            text(waiting).size(12).color(p.text_muted()),
            Switch::new(state.log.is_following(), p)
                .label("Follow")
                .on_toggle(Message::Follow),
        ]
        .spacing(spacing::SPACE_3)
        .align_y(iced::Alignment::Center),
        LogView::new(&state.log, p)
            .height(200)
            .on_action(Message::Log),
    ]
    .spacing(spacing::SPACE_3);
    let title_search = TitleSearch::new(
        "zsh  ~/projects/otty",
        "Ctrl K",
        svg::Handle::from_memory(SEARCH_ICON),
        p,
    )
    .on_press(Message::Clicked);
    let floating_note = FloatingNote::new(p)
        .text("Focus mode")
        .muted("Panels are hidden")
        .kbd("Ctrl Shift F")
        .muted("to show them");
    let logo = svg(svg::Handle::from_memory(LOGO))
        .width(Length::Fill)
        .height(Length::Fill);
    let command_palette = |list| {
        CommandPalette::new(list, svg::Handle::from_memory(SEARCH_ICON), p)
            .placeholder("Search sessions, launches and actions")
            .no_matches("No matches")
            .hint("↑↓", "move")
            .hint("Enter", "open")
    };
    let palettes = column![
        command_palette(&state.commands).on_action(Message::Palette),
        command_palette(&state.no_commands),
    ]
    .spacing(spacing::SPACE_6);

    let command_box = column![
        CommandBox::new(
            "sudo systemctl start docker",
            svg::Handle::from_memory(COPY_ICON),
            "Copy",
            p,
        )
        .on_copy(Message::Clicked),
        container(
            CommandBox::new(
                "systemctl --user enable --now podman.socket",
                svg::Handle::from_memory(COPY_ICON),
                "Copy",
                p,
            )
            .on_copy(Message::Clicked),
        )
        .width(240),
        ActionTile::new(
            svg::Handle::from_memory(PLUS_ICON),
            "Install Docker",
            p,
        )
        .note("docs.docker.com")
        .on_press(Message::Clicked),
    ]
    .spacing(spacing::SPACE_2)
    .width(460);

    let empty_card = EmptyCard::new(
        "Start a session",
        "Open a terminal on this computer or run one of your launches.",
        logo,
        p,
    )
    .action(
        ActionTile::new(svg::Handle::from_memory(PLUS_ICON), "New terminal", p)
            .shortcut("Ctrl T")
            .accent(true)
            .on_press(Message::Clicked),
    )
    .action(
        ActionTile::new(
            svg::Handle::from_memory(FOLDER_ICON),
            "Terminal in a folder…",
            p,
        )
        .shortcut("Ctrl Shift T")
        .on_press(Message::Clicked),
    )
    .action(
        ActionTile::new(
            svg::Handle::from_memory(LAUNCH_ICON),
            "New launch…",
            p,
        )
        .on_press(Message::Clicked),
    );

    let sessions_body = column![
        SessionGroup::new("fedora", TargetKind::Local, p).note("this computer"),
        SessionRow::new("zsh  ~/projects/otty", p)
            .selected(true)
            .on_press(Message::Clicked)
            .on_close(Message::Clicked, "Close session"),
        SessionRow::new("claude  Review tree indent", p)
            .state("needs you", SessionTone::Waiting)
            .on_press(Message::Clicked),
        SessionRow::new("cargo test", p)
            .state("done", SessionTone::Done)
            .on_press(Message::Clicked),
        SessionRow::new("make deploy", p)
            .state("failed", SessionTone::Failed)
            .on_press(Message::Clicked),
        SessionGroup::new("laptop", TargetKind::Ssh, p).note("ssh"),
        SessionRow::new("htop", p)
            .state("connecting", SessionTone::Muted)
            .on_press(Message::Clicked),
        SessionRow::new(
            "A very long session title that has to be cut at the row edge",
            p
        )
        .on_press(Message::Clicked),
    ]
    .spacing(1);
    let sessions = PanelSection::new("Sessions", p)
        .open(state.sessions_open)
        .count(5)
        .action(split(state.menu_open))
        .body(sessions_body)
        .on_toggle(Message::ToggleSessions);
    let collapsed = PanelSection::new("Launches", p)
        .open(false)
        .count(12)
        .action(
            IconButton::new(plus(), "Add", p)
                .size(IconButtonSize::Small)
                .on_press(Message::Clicked),
        )
        .on_toggle(Message::Clicked);
    let panel =
        container(column![sessions, collapsed].spacing(spacing::SPACE_3))
            .width(PANEL_WIDTH)
            .style({
                let background = p.surface_panel();
                move |_| container::Style {
                    background: Some(background.into()),
                    ..container::Style::default()
                }
            });

    // The same section at the panel's minimum width (128) and a little more.
    let narrow_panel = |width: f32| {
        let body = column![
            SessionGroup::new("fedora", TargetKind::Local, p)
                .note("this computer"),
            SessionRow::new("claude", p)
                .folder("otty")
                .state("needs you", SessionTone::Waiting)
                .on_press(Message::Clicked)
                .on_close(Message::Clicked, "Close session"),
            SessionRow::new("A long session title", p)
                .on_press(Message::Clicked),
        ]
        .spacing(1);

        container(
            PanelSection::new("Sessions", p)
                .open(true)
                .count(2)
                .action(split(false))
                .body(body),
        )
        .width(width)
        .style({
            let background = p.surface_panel();
            move |_| container::Style {
                background: Some(background.into()),
                ..container::Style::default()
            }
        })
    };

    let dots = row![
        target_dot(TargetKind::Local, p),
        target_dot(TargetKind::Ssh, p),
        target_dot(TargetKind::Prod, p),
        target_dot(TargetKind::Staging, p),
    ]
    .spacing(spacing::SPACE_3);
    let chips = row![
        Chip::new("fedora", p)
            .target(TargetKind::Local)
            .note("this computer"),
        Chip::new("prod-1", p).target(TargetKind::Prod),
        Chip::new("~/projects/otty", p).mono(true),
        Chip::new("compose: otty-site", p)
            .on_clear("Show all containers", Message::Clicked),
    ]
    .spacing(spacing::SPACE_3)
    .align_y(iced::Alignment::Center);
    let status_bar = StatusBar::new(p)
        .left(StatusItem::target(TargetKind::Ssh, "laptop"))
        .left(StatusItem::mono("~/projects/otty"))
        .right(StatusItem::text("3 sessions"))
        .right(StatusItem::button("Settings", Message::Clicked));

    let first: Vec<Element<'_, Message>> = vec![
        section(
            "Button (form 34, small 28; second row disabled)",
            buttons.into(),
            p,
        ),
        section(
            "IconButton (hover the edge of a button: the icon lights up; md, accent, sm, danger, disabled danger, disabled)",
            icon_buttons.into(),
            p,
        ),
        section(
            "Dialog (with actions; wide, with a close cross)",
            dialogs.into(),
            p,
        ),
        section("Kbd", keys.into(), p),
        section("TargetDot (local, ssh, prod, staging)", dots.into(), p),
        section(
            "Chip (target + note, target, mono path, clearable)",
            chips.into(),
            p,
        ),
        section("StatusBar (items left and right)", status_bar.into(), p),
        section("Divider", dividers.into(), p),
        section("Menu", menu.into(), p),
        section("Switch", switches.into(), p),
        section(
            "ResizableGrid (a drag on the line resizes; a drag on the bar beside it scrolls)",
            split_view,
            p,
        ),
        section("Segmented", segments.into(), p),
        container(section("Select", dropdown, p)).width(280).into(),
    ];
    let rows: Vec<Element<'_, Message>> = vec![
        section(
            "TreeRow (depth 0-3; selected, hover, static; long title cut)",
            tree.into(),
            p,
        ),
        section(
            "PanelSection (open with SessionGroup + SessionRow: selected, state words, long title; collapsed)",
            panel.into(),
            p,
        ),
        section(
            "PanelSection narrow (170 and 128 wide: titles and names stay whole, notes give way, nothing wraps or reaches a button)",
            row![narrow_panel(170.0), narrow_panel(128.0)]
                .spacing(spacing::SPACE_4)
                .into(),
            p,
        ),
        section(
            "ModeSwitcher (active tab; temporary tab with the menu open)",
            mode_switchers.into(),
            p,
        ),
        section(
            "ModeMenu (full pinned list with a disabled pin; unpinned row and hint)",
            mode_menus.into(),
            p,
        ),
        section(
            "SplitButton (idle; menu open shows the frame)",
            split_buttons.into(),
            p,
        ),
        section(
            "PaneHeader (focused with chip; other pane)",
            pane_headers.into(),
            p,
        ),
        section("TitleSearch", title_search.into(), p),
        section("FloatingNote", floating_note.into(), p),
        section(
            "EmptyCard (ActionTile: accent icon, shortcut)",
            empty_card.into(),
            p,
        ),
        section("CommandBox", command_box.into(), p),
        section(
            "CommandPalette (query \"la\": active item, match in bold; no matches)",
            palettes.into(),
            p,
        ),
    ];
    let inspector_parts: Vec<Element<'_, Message>> = vec![
        section(
            "InspectorHeader (long status wraps) + InspectorTabs (a select when narrow, second row) + KeyValueList (values wrap at spaces, / - : . first; links underline under the pointer, the last press is printed)",
            inspector.into(),
            p,
        ),
        section(
            "LogView with the Follow switch (Add line is gallery only)",
            container(log_view).width(480).into(),
            p,
        ),
        section(
            "DataTree (group, sections that open and close, indented and numbered rows; values wrap; stacked at 200 wide; values and a heading can be links, the last press is printed)",
            column![
                row![
                    container(data_tree(state, p)).width(360),
                    container(data_tree(state, p)).width(200),
                ]
                .spacing(spacing::SPACE_6),
                label(state.inspector_event.clone(), p),
            ]
            .spacing(spacing::SPACE_3)
            .into(),
            p,
        ),
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
        section(
            "FormPage in a ScreenArea (sections, footer; the area scrolls)",
            container(ScreenArea::new("page", page, p))
                .height(280)
                .into(),
            p,
        ),
        section(
            "MappingField (ports: protocol, error row; volumes: read-only, detail under the row, disabled)",
            container(mappings).width(720).into(),
            p,
        ),
        section(
            "FormPage with a footer bar in a ScreenArea (the bar comes after the sections)",
            container(ScreenArea::new("footer", footer_page, p))
                .height(260)
                .into(),
            p,
        ),
        section(
            "ScreenArea narrower than the minimum of a screen (320 wide, minimum 592): scrolls both ways, fields and bar together",
            container(ScreenArea::new("narrow", narrow_page, p))
                .width(320)
                .height(240)
                .into(),
            p,
        ),
    ];
    let mut body = column![header];
    if state.part == Part::All {
        body = body.extend(first);
    }
    if matches!(state.part, Part::All | Part::Rows) {
        body = body.extend(rows);
    }
    if matches!(state.part, Part::All | Part::Inspector) {
        body = body.extend(inspector_parts);
    }
    if matches!(state.part, Part::All | Part::Fields) {
        body = body.extend(form_fields);
    }
    if matches!(state.part, Part::All | Part::Settings) {
        body = body.extend(settings_parts);
    }
    if matches!(state.part, Part::All | Part::Data) {
        body = body.extend(data_parts(state, p));
    }
    let body = body
        .push(Space::new().height(240))
        .spacing(spacing::SPACE_6)
        .padding(Padding::from(spacing::SPACE_6));

    let canvas = palette.surface_canvas();
    let page = container(scrollable(body))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_| container::Style {
            background: Some(canvas.into()),
            ..container::Style::default()
        });

    if state.wide_dialog_open {
        let dialog = Dialog::new("Settings", p)
            .width(size::DIALOG_WIDE)
            .on_close(Message::CloseDialog, "Close")
            .body(
                SettingRow::new("Command blocks", p)
                    .description("Group terminal output by command.")
                    .control(
                        Switch::new(state.switch_on, p)
                            .on_toggle(Message::Switched),
                    ),
            );

        return stack![page, Element::from(dialog)].into();
    }
    if !state.dialog_open {
        return page.into();
    }

    let dialog = Dialog::new("Delete session", p)
        .body(
            text("The session and its history are removed.")
                .color(palette.text_body()),
        )
        .action(
            Button::new("Cancel", p)
                .kind(ButtonKind::Ghost)
                .on_press(Message::CloseDialog),
        )
        .action(
            Button::new("Delete", p)
                .kind(ButtonKind::Primary)
                .on_press(Message::CloseDialog),
        );

    stack![page, Element::from(dialog)].into()
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

/// Sections of the data components: tables, status badges, the notice and
/// the tree row count tones.
fn data_parts<'a>(
    state: &'a Gallery,
    p: &UiPalette,
) -> Vec<Element<'a, Message>> {
    let columns = || {
        vec![
            TableColumn::new("Name", 3),
            TableColumn::new("Status", 2),
            TableColumn::new("Image", 2),
            TableColumn::new("Ports", 2),
            TableColumn::new("Size", 1).align(ColumnAlign::End),
            TableColumn::new("Age", 1),
        ]
    };
    let containers = [
        (
            "otty-site-web-1",
            "Running",
            StatusTone::Ok,
            "nginx:1.27",
            "8080→80/tcp",
            "192 MB",
        ),
        (
            "otty-site-build-1",
            "Exited (1)",
            StatusTone::Error,
            "node:22",
            "—",
            "1.12 GB",
        ),
        (
            "mailpit",
            "Restarting",
            StatusTone::Warn,
            "axllent/mailpit",
            "8025→8025/tcp",
            "34 MB",
        ),
        (
            "hello",
            "Exited (0)",
            StatusTone::Neutral,
            "hello-world",
            "—",
            "13 kB",
        ),
        (
            "a-container-with-a-very-long-name-that-ends-with-an-ellipsis",
            "Running",
            StatusTone::Ok,
            "registry.example.com/team/very-long-image-name:2026.11.01",
            "5432→5432/tcp, 5433→5433/tcp",
            "438 MB",
        ),
    ];
    let mut table = DataTable::new(columns(), p)
        .selected(state.table_selected)
        .empty_message("No containers")
        .on_row_press(Message::TableRow)
        .on_row_right_press(Message::TableMenu);
    for (name, status, tone, image, ports, size) in containers {
        table = table.row(vec![
            TableCell::mono(name),
            TableCell::badge(status, tone),
            TableCell::muted_mono(image),
            TableCell::muted_mono(ports),
            TableCell::number(size),
            TableCell::muted("2 h"),
        ]);
    }
    let plain = DataTable::<Message>::new(
        vec![TableColumn::new("Plain", 1), TableColumn::new("Muted", 1)],
        p,
    )
    .row(vec![
        TableCell::plain("Plain text"),
        TableCell::muted("Muted text"),
    ]);
    let empty = DataTable::<Message>::new(columns(), p)
        .empty_message("Nothing matches “web”.");
    let narrow_table = |width: f32| {
        let table = DataTable::<Message>::new(
            vec![
                TableColumn::new("Name", 3).min_width(140),
                TableColumn::new("Status", 2).min_width(110),
                TableColumn::new("Size", 1)
                    .align(ColumnAlign::End)
                    .min_width(84),
                TableColumn::new("Age", 1).min_width(64),
            ],
            p,
        )
        .on_row_menu("More actions", |_, _| Message::Clicked)
        .row(vec![
            TableCell::mono("otty-site-web-1"),
            TableCell::badge("Running", StatusTone::Ok),
            TableCell::number("192 MB"),
            TableCell::muted("2 h"),
        ])
        .row(vec![
            TableCell::mono("postgres-dev-db-1"),
            TableCell::badge("Exited (1)", StatusTone::Error),
            TableCell::number("1.2 GB"),
            TableCell::muted("3 d"),
        ]);

        container(container(table).width(width).height(110))
    };
    let narrow_rows = column![
        TreeRow::<Message>::new("Containers", p)
            .depth(1)
            .state("2 running"),
        TreeRow::<Message>::new("Docker 29.9.1", p)
            .caret(true)
            .dot(TargetKind::Local),
    ];

    let badges = row![
        StatusBadge::new("Running", StatusTone::Ok, p),
        StatusBadge::new("Restarting", StatusTone::Warn, p),
        StatusBadge::new("Exited (1)", StatusTone::Error, p),
        StatusBadge::new("Exited (0)", StatusTone::Neutral, p),
    ]
    .spacing(spacing::SPACE_6);

    let notice: Element<'a, Message> = if state.notice_open {
        Notice::new(
            "Error response from daemon: cannot stop container mailpit: \
             permission denied.",
            svg::Handle::from_memory(ERROR_ICON),
            p,
        )
        .on_dismiss(Message::DismissNotice, "Dismiss")
        .into()
    } else {
        Button::new("Show notice", p)
            .kind(ButtonKind::Secondary)
            .on_press(Message::ShowNotice)
            .into()
    };

    let tones = column![
        TreeRow::<Message>::new("Containers", p)
            .depth(1)
            .state("3 running"),
        TreeRow::<Message>::new("Pods", p)
            .depth(1)
            .state("2 failing")
            .count_tone(SessionTone::Failed),
        TreeRow::<Message>::new("Deployments", p)
            .depth(1)
            .count(1)
            .count_tone(SessionTone::Waiting),
        TreeRow::<Message>::new("Jobs", p)
            .depth(1)
            .count(4)
            .count_tone(SessionTone::Done),
    ]
    .width(PANEL_WIDTH);

    let progress = column![
        ProgressBar::new(0.0, p),
        row![
            label(String::from("Pulling nginx:1.27-alpine"), p),
            ProgressBar::new(0.32, p),
            label(String::from("15 MB of 47 MB"), p),
        ]
        .spacing(spacing::SPACE_3)
        .align_y(iced::Alignment::Center),
        ProgressBar::new(1.0, p),
        ProgressBar::new(0.6, p).width(240.0),
    ]
    .spacing(spacing::SPACE_4);

    let right_click = match state.table_menu_at {
        Some(point) => format!(
            "last right click at {:.0}, {:.0} (gallery scroll coordinates)",
            point.x, point.y
        ),
        None => "right click a row to see where the menu would open".to_owned(),
    };

    let (sorted, direction) = state.table_sort;
    let mut sortable = DataTable::new(
        vec![
            TableColumn::new("Name", 3).sortable(true),
            TableColumn::new("Status", 2).sortable(true),
            TableColumn::new("Compose", 2),
            TableColumn::new("Ports", 3),
            TableColumn::new("Size", 1)
                .align(ColumnAlign::End)
                .sortable(true),
        ],
        p,
    )
    .selected(Some(1))
    .sorted_by(sorted, direction)
    .on_sort(Message::TableSort)
    .on_link_press(Message::TableLink)
    .on_row_menu("More actions", Message::TableRowMenu);
    for (name, status, tone, compose, size) in [
        (
            "otty-site-web-1",
            "Running",
            StatusTone::Ok,
            "otty-site",
            "192 MB",
        ),
        (
            "otty-site-db-1",
            "Running",
            StatusTone::Ok,
            "otty-site",
            "438 MB",
        ),
        ("mailpit", "Restarting", StatusTone::Warn, "", "34 MB"),
    ] {
        let compose = if compose.is_empty() {
            TableCell::muted("")
        } else {
            TableCell::link(compose)
        };
        // Two port links with text between and after them.
        let ports = if name == "mailpit" {
            TableCell::muted_mono("—")
        } else {
            TableCell::parts(vec![
                CellPart::link("8080:80/tcp"),
                CellPart::text(", "),
                CellPart::link("5432/tcp"),
                CellPart::text(", 53/udp"),
            ])
        };
        sortable = sortable.row(vec![
            TableCell::mono(name),
            TableCell::badge(status, tone),
            compose,
            ports,
            TableCell::number(size),
        ]);
    }
    sortable = sortable.row_without_menu(vec![
        TableCell::mono("hello"),
        TableCell::badge("Exited (0)", StatusTone::Neutral),
        TableCell::muted(""),
        TableCell::muted_mono("—"),
        TableCell::number("13 kB"),
    ]);

    let toolbar = |width: f32| -> Element<'a, Message> {
        let palette = p.clone();
        let filter = state.toolbar_filter.as_str();
        let item = |icon: &'static [u8], name: &'static str| {
            ToolbarItem::new(svg::Handle::from_memory(icon), name)
                .on_press(Message::ToolbarAction(name))
        };
        let bar = Toolbar::new("More actions", p)
            .field(120.0, 220.0, move || {
                TextField::new("", filter, &palette)
                    .placeholder("Filter by name")
                    .on_input(Message::ToolbarFilter)
                    .into()
            })
            .item(item(CREATE_ICON, "Run container"))
            .item(item(PULL_ICON, "Pull image"))
            .item(
                ToolbarItem::new(svg::Handle::from_memory(PRUNE_ICON), "Prune")
                    .on_press_maybe(None),
            )
            .item(item(REFRESH_ICON, "Refresh"));

        container(bar).width(width).into()
    };
    let toolbars = column![
        toolbar(640.0),
        toolbar(330.0),
        toolbar(230.0),
        label(state.table_event.clone(), p),
    ]
    .spacing(spacing::SPACE_4);

    let tag_field_palette = p.clone();
    let search = &state.tag_search;
    let schema = tag_schema();
    let suggestions = search.suggestions(&schema);
    let passing = schema
        .rows()
        .iter()
        .filter(|row| search.passes(schema.keys(), row))
        .count();
    let long_tags = [
        Tag::new("status", "Running"),
        Tag::new(
            "image",
            "registry.example.com/team/very-long-image-name/with/many/parts:1.2.3",
        ),
    ];
    let tag_bar = Toolbar::new("More actions", p)
        .field(160.0, 320.0, move || {
            TagSearchField::new(search, suggestions.clone(), &tag_field_palette)
                .placeholder("Filter: name or key:value")
                .on_action(Message::TagSearch)
                .into()
        })
        .item(
            ToolbarItem::new(svg::Handle::from_memory(REFRESH_ICON), "Refresh")
                .on_press(Message::ToolbarAction("Refresh")),
        );
    let tag_search_parts = column![
        container(tag_bar).width(640),
        container(
            TagRow::new(search.tags(), schema.keys(), p)
                .remove_label("Remove this tag")
                .clear_all_label("Clear all")
                .on_action(Message::TagSearch)
        )
        .width(640),
        label(
            "A long tag wraps and its chip grows (240 wide):".to_owned(),
            p
        ),
        container(
            TagRow::new(&long_tags, schema.keys(), p)
                .remove_label("Remove this tag")
                .clear_all_label("Clear all")
        )
        .width(240),
        label(format!("{passing} of 12 rows pass"), p),
    ]
    .spacing(spacing::SPACE_3);

    let mut icons = row![].spacing(spacing::SPACE_3);
    for (name, bytes) in ACTION_ICONS {
        icons = icons.push(
            IconButton::new(svg::Handle::from_memory(bytes), name, p)
                .on_press(Message::ToolbarAction(name)),
        );
    }

    vec![
        section(
            "DataTable (sortable Name, Status and Size; Compose links; row ⋯, none on the last row)",
            column![
                container(sortable).height(200),
                label(state.table_event.clone(), p)
            ]
            .spacing(spacing::SPACE_2)
            .into(),
            p,
        ),
        section(
            "TagSearchField and TagRow (focus the field for the keys; try sta Enter, run Enter, Backspace, a cross, Clear all)",
            tag_search_parts.into(),
            p,
        ),
        section(
            "Toolbar (640, 330 and 230 wide: the filter shrinks, then buttons move into ⋯; Prune disabled)",
            toolbars.into(),
            p,
        ),
        section(
            "Action icons (assets/svg; hover for the name)",
            icons.into(),
            p,
        ),
        section(
            "DataTable (selected row, every cell kind, long text cut with …; click and right click)",
            column![container(table).height(240), label(right_click, p)]
                .spacing(spacing::SPACE_2)
                .into(),
            p,
        ),
        section(
            "DataTable minimum widths (520 wide: Size and Age keep theirs while Name shrinks; 320 wide: the table scrolls sideways with its header)",
            column![narrow_table(520.0), narrow_table(320.0)]
                .spacing(spacing::SPACE_4)
                .into(),
            p,
        ),
        section(
            "TreeRow narrow (200, 150 and 90 wide: the label stays whole, the note gives way, then the label ends with …)",
            column![
                container(narrow_rows.width(200)),
                container(
                    column![
                        TreeRow::<Message>::new("Containers", p)
                            .depth(1)
                            .state("2 running"),
                    ]
                    .width(150)
                ),
                container(
                    column![
                        TreeRow::<Message>::new("Containers", p)
                            .depth(1)
                            .state("2 running"),
                    ]
                    .width(90)
                ),
            ]
            .spacing(spacing::SPACE_2)
            .into(),
            p,
        ),
        section(
            "DataTable (plain and muted cells)",
            container(plain).height(80).into(),
            p,
        ),
        section(
            "DataTable (no rows: empty message)",
            container(empty).height(90).into(),
            p,
        ),
        section("StatusBadge (ok, warn, error, neutral)", badges.into(), p),
        section(
            "Notice (error; dismiss hides it)",
            container(notice).width(560).into(),
            p,
        ),
        section(
            "TreeRow count tone (default, failed, waiting, done)",
            tones.into(),
            p,
        ),
        section(
            "ProgressBar (empty, a third, full; 240 wide)",
            progress.into(),
            p,
        ),
    ]
}

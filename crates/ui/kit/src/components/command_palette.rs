//! `CommandPalette`: search box over a grouped list of things to run.
//!
//! The rules for matching and the active entry live in [`PaletteList`];
//! [`CommandPalette`] only draws it and reports what the user did as a
//! [`PaletteAction`].
//!
//! Keys are handled by the palette itself while its input has focus: ↑ and ↓
//! send [`PaletteAction::MoveUp`] and [`PaletteAction::MoveDown`], Enter
//! sends [`PaletteAction::RunActive`] and Esc sends [`PaletteAction::Close`].
//! The app focuses the input when it opens the palette, with
//! `iced::widget::operation::focus(input_id())`.

use std::ops::Range;

use iced::keyboard::key::Named;
use iced::widget::text::Wrapping;
use iced::widget::{
    Id, button, column, container, row, scrollable, svg, text, text_input,
};
use iced::{Alignment, Border, Color, Element, Length, Padding, Theme};

use super::input_keys::InputKeys;
use super::kbd::kbd;
use super::palette_list::{PaletteItem, PaletteList};
use super::target_dot::target_dot;
use super::{divider, menu, scrollbar};
use crate::palette::UiPalette;
use crate::{fonts, radius, size, spacing, typography};

/// Gap between the search icon, the input and the Esc hint (`gap: 10px`).
const INPUT_GAP: f32 = 10.0;
/// Side padding of the input row and of each entry (`padding: 0 10px`).
const PADDING_X: f32 = 10.0;
/// Edge of the search icon (`width: 14px; height: 14px`).
const ICON_SIZE: f32 = 14.0;
/// Font size of the input (`font-size: 14px`).
const INPUT_TEXT_SIZE: f32 = 14.0;
/// Space between the line under the input and the list (`margin-bottom:
/// 4px`).
const LIST_GAP: f32 = spacing::SPACE_1;
/// Tallest the list grows before it scrolls. The design shows no limit;
/// twelve entries keep the palette inside a small window.
const LIST_MAX_HEIGHT: f32 = 12.0 * size::PALETTE_ITEM;
/// Padding of the footer (`padding: 8px 10px 4px`).
const FOOTER_PADDING: Padding = Padding {
    top: spacing::SPACE_3,
    right: PADDING_X,
    bottom: spacing::SPACE_1,
    left: PADDING_X,
};
/// Key name on the close hint in the input row.
const ESC_KEY: &str = "Esc";
/// Opacity of the text selection in the input.
const SELECTION_ALPHA: f32 = 0.35;

/// What the user did in a [`CommandPalette`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaletteAction {
    /// The text in the input changed; the app calls
    /// [`PaletteList::set_query`].
    QueryChanged(String),
    /// ↑ was pressed; the app calls [`PaletteList::move_up`].
    MoveUp,
    /// ↓ was pressed; the app calls [`PaletteList::move_down`].
    MoveDown,
    /// Enter was pressed; the app runs [`PaletteList::active_key`].
    RunActive,
    /// The entry at this index among the matches was clicked; the app runs
    /// [`PaletteList::key_at`].
    RunItem(usize),
    /// Esc was pressed or the Esc hint was clicked.
    Close,
}

/// Floating palette: a search input, the matching entries under their group
/// labels with the matched part of each title in `text-strong` semibold, and
/// a footer with key hints. Without an action handler the palette is shown
/// but does nothing.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use iced::widget::svg;
/// use otty_ui_kit::components::command_palette::{
///     CommandPalette, PaletteAction,
/// };
/// use otty_ui_kit::components::palette_list::{PaletteItem, PaletteList};
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Clone)]
/// enum Message {
///     Palette(PaletteAction),
/// }
///
/// let palette = UiPalette::otty_dark();
/// let icon = svg::Handle::from_memory(&b"<svg/>"[..]);
/// let list = PaletteList::new(vec![
///     PaletteItem::new(1, "Sessions", "zsh  ~/projects/otty"),
///     PaletteItem::new(2, "Actions", "New launch…"),
/// ]);
/// let view: Element<'_, Message> = CommandPalette::new(&list, icon, &palette)
///     .placeholder("Search sessions, launches and actions")
///     .no_matches("No matches")
///     .hint("↑↓", "move")
///     .hint("Enter", "open")
///     .on_action(Message::Palette)
///     .into();
/// ```
pub struct CommandPalette<'a, K, Message> {
    list: &'a PaletteList<K>,
    search_icon: svg::Handle,
    palette: UiPalette,
    placeholder: String,
    no_matches: Option<String>,
    hints: Vec<(String, String)>,
    on_action: Option<Box<dyn Fn(PaletteAction) -> Message + 'a>>,
}

impl<'a, K, Message> CommandPalette<'a, K, Message> {
    /// Creates a palette that shows `list`, with `search_icon` before the
    /// input.
    pub fn new(
        list: &'a PaletteList<K>,
        search_icon: svg::Handle,
        palette: &UiPalette,
    ) -> Self {
        Self {
            list,
            search_icon,
            palette: palette.clone(),
            placeholder: String::new(),
            no_matches: None,
            hints: Vec::new(),
            on_action: None,
        }
    }

    /// Sets the text shown while the input is empty.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Sets the muted line shown when nothing matches the query.
    pub fn no_matches(mut self, text: impl Into<String>) -> Self {
        self.no_matches = Some(text.into());
        self
    }

    /// Adds a key hint to the footer, such as `Enter` and `open`. The footer
    /// is hidden while there are no hints.
    pub fn hint(
        mut self,
        keys: impl Into<String>,
        label: impl Into<String>,
    ) -> Self {
        self.hints.push((keys.into(), label.into()));
        self
    }

    /// Sets the function that maps a [`PaletteAction`] to the app message.
    pub fn on_action(
        mut self,
        on_action: impl Fn(PaletteAction) -> Message + 'a,
    ) -> Self {
        self.on_action = Some(Box::new(on_action));
        self
    }
}

impl<'a, K, Message: Clone + 'a> From<CommandPalette<'a, K, Message>>
    for Element<'a, Message, Theme>
{
    fn from(value: CommandPalette<'a, K, Message>) -> Self {
        let palette = value.palette;
        let send = |action: PaletteAction| {
            value.on_action.as_ref().map(|on_action| on_action(action))
        };

        let move_up = send(PaletteAction::MoveUp);
        let move_down = send(PaletteAction::MoveDown);
        let run_active = send(PaletteAction::RunActive);
        let close = send(PaletteAction::Close);
        let entries = entries_view(
            value.list,
            value.no_matches.as_deref(),
            &send,
            &palette,
        );
        let on_input = value.on_action.map(|on_action| {
            move |query| on_action(PaletteAction::QueryChanged(query))
        });

        let input = text_input(&value.placeholder, value.list.query())
            .id(input_id())
            .font(fonts::UI)
            .size(INPUT_TEXT_SIZE)
            .padding(0)
            .on_input_maybe(on_input)
            .on_submit_maybe(run_active)
            .style(input_style(&palette));
        let mut keys = InputKeys::new(input);
        if let Some(message) = move_up {
            keys = keys.on_key(Named::ArrowUp, message);
        }
        if let Some(message) = move_down {
            keys = keys.on_key(Named::ArrowDown, message);
        }
        if let Some(message) = close.clone() {
            keys = keys.on_key(Named::Escape, message);
        }

        let muted = palette.text_muted();
        let input_row = row![
            svg(value.search_icon)
                .width(ICON_SIZE)
                .height(ICON_SIZE)
                .style(move |_, _| svg::Style { color: Some(muted) }),
            keys,
            button(kbd(ESC_KEY, &palette))
                .padding(0)
                .on_press_maybe(close)
                .style(|_, _| button::Style::default()),
        ]
        .spacing(INPUT_GAP)
        .align_y(Alignment::Center)
        .height(size::PALETTE_INPUT)
        .padding(Padding::from([0.0, PADDING_X]));

        let mut body = column![
            input_row,
            divider::horizontal(&palette),
            container(entries).padding(Padding {
                top: LIST_GAP,
                ..Padding::ZERO
            }),
        ];
        if !value.hints.is_empty() {
            body = body.push(footer_view(value.hints, &palette));
        }

        let background = palette.surface_overlay();
        let shadow = palette.shadow_popover();

        container(body)
            .width(size::PALETTE)
            .padding(spacing::SPACE_2)
            .style(move |_| container::Style {
                background: Some(background.into()),
                border: Border {
                    radius: radius::RADIUS_LG.into(),
                    ..Border::default()
                },
                shadow,
                ..container::Style::default()
            })
            .into()
    }
}

/// Id of the palette input. The app focuses it when the palette opens:
/// `iced::widget::operation::focus(input_id())`.
pub const fn input_id() -> Id {
    Id::new("otty-command-palette-input")
}

/// Splits `title` into pieces at the edges of `ranges`. Each piece says
/// whether it is part of a match.
fn title_pieces<'t>(
    title: &'t str,
    ranges: &[Range<usize>],
) -> Vec<(&'t str, bool)> {
    let mut pieces = Vec::new();
    let mut start = 0;

    for range in ranges {
        if start < range.start {
            pieces.push((&title[start..range.start], false));
        }
        pieces.push((&title[range.clone()], true));
        start = range.end;
    }
    if start < title.len() {
        pieces.push((&title[start..], false));
    }

    pieces
}

/// Builds the scrolling list: a group label before the first entry of each
/// group, then the entries, or the `no_matches` line.
fn entries_view<'a, K, Message: Clone + 'a>(
    list: &'a PaletteList<K>,
    no_matches: Option<&str>,
    send: &dyn Fn(PaletteAction) -> Option<Message>,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let mut entries = column![].width(Length::Fill);

    let mut group: Option<&str> = None;
    for (index, (item, ranges)) in list.matches().enumerate() {
        if group != Some(item.group()) {
            entries = entries
                .push(menu::heading_view(item.group().to_owned(), palette));
            group = Some(item.group());
        }

        let is_active = list.active() == Some(index);
        let on_press = send(PaletteAction::RunItem(index));
        entries =
            entries.push(item_view(item, ranges, is_active, on_press, palette));
    }

    if list.match_count() == 0
        && let Some(no_matches) = no_matches
    {
        entries = entries.push(no_matches_view(no_matches, palette));
    }

    container(
        scrollable(entries)
            .width(Length::Fill)
            .direction(scrollbar::vertical())
            .style(scrollbar::style(palette)),
    )
    .max_height(LIST_MAX_HEIGHT)
    .into()
}

/// Builds one entry: target dot, title with the matched pieces in
/// `text-strong` semibold, muted mono detail and shortcut hint.
fn item_view<'a, K, Message: Clone + 'a>(
    item: &'a PaletteItem<K>,
    ranges: &[Range<usize>],
    is_active: bool,
    on_press: Option<Message>,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let body = typography::BODY;
    let caption = typography::CAPTION;
    let strong = palette.text_strong();

    let mut title = row![];
    for (piece, is_match) in title_pieces(item.title(), ranges) {
        let part = text(piece).size(body.size()).wrapping(Wrapping::None);
        title = title.push(if is_match {
            part.font(fonts::UI_SEMIBOLD).color(strong)
        } else {
            part.font(fonts::UI)
        });
    }

    let mut content = row![]
        .spacing(spacing::SPACE_3)
        .height(Length::Fill)
        .align_y(Alignment::Center);
    if let Some(kind) = item.target() {
        content = content.push(target_dot(kind, palette));
    }
    content = content.push(container(title).width(Length::Fill).clip(true));
    if let Some(detail) = item.detail() {
        content = content.push(
            text(detail)
                .font(fonts::MONO)
                .size(caption.size())
                .color(palette.text_muted())
                .wrapping(Wrapping::None),
        );
    }
    if let Some(shortcut) = item.shortcut() {
        content = content.push(kbd(shortcut, palette));
    }

    button(content)
        .width(Length::Fill)
        .height(size::PALETTE_ITEM)
        .padding(Padding::from([0.0, PADDING_X]))
        .on_press_maybe(on_press)
        .style(menu::item_style(palette, is_active))
        .into()
}

/// Builds the muted line shown when nothing matches.
fn no_matches_view<'a, Message: 'a>(
    no_matches: &str,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let style = typography::BODY;

    container(
        text(no_matches.to_owned())
            .size(style.size())
            .font(fonts::UI)
            .color(palette.text_muted()),
    )
    .height(size::PALETTE_ITEM)
    .padding(Padding::from([0.0, PADDING_X]))
    .align_y(Alignment::Center)
    .into()
}

/// Builds the footer: each key hint followed by its muted label.
fn footer_view<'a, Message: 'a>(
    hints: Vec<(String, String)>,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let style = typography::CAPTION;

    let mut footer = row![]
        .spacing(spacing::SPACE_5)
        .padding(FOOTER_PADDING)
        .align_y(Alignment::Center);
    for (keys, label) in hints {
        footer = footer.push(
            row![
                kbd(keys, palette),
                text(label)
                    .size(style.size())
                    .font(fonts::UI)
                    .color(palette.text_muted()),
            ]
            .spacing(spacing::SPACE_1)
            .align_y(Alignment::Center),
        );
    }

    footer.into()
}

/// Style of the input: no frame of its own, `text-strong` value and
/// `text-muted` placeholder.
fn input_style(
    palette: &UiPalette,
) -> impl Fn(&Theme, text_input::Status) -> text_input::Style + 'static {
    let value = palette.text_strong();
    let muted = palette.text_muted();
    let selection = Color {
        a: SELECTION_ALPHA,
        ..palette.accent()
    };

    move |_, _| text_input::Style {
        background: Color::TRANSPARENT.into(),
        border: Border::default(),
        icon: muted,
        placeholder: muted,
        value,
        selection,
    }
}

#[cfg(test)]
mod tests {
    use iced::keyboard::key::Named;
    use iced_test::simulator;

    use super::*;
    use crate::components::glyph;

    /// Byte ranges written as `(start, end)` pairs.
    type Spans = &'static [(usize, usize)];
    /// Title pieces and whether each one is part of a match.
    type Pieces = &'static [(&'static str, bool)];

    fn list() -> PaletteList<u32> {
        PaletteList::new(vec![
            PaletteItem::new(1, "Sessions", "zsh")
                .with_detail("~/projects/otty"),
            PaletteItem::new(2, "Actions", "New launch…")
                .with_shortcut("Ctrl Shift L"),
            PaletteItem::new(3, "Sessions", "htop"),
        ])
    }

    fn view(list: &PaletteList<u32>) -> Element<'_, PaletteAction> {
        let palette = UiPalette::otty_dark();

        CommandPalette::new(list, glyph::plus(), &palette)
            .placeholder("Search")
            .no_matches("No matches")
            .hint("↑↓", "move")
            .hint("Enter", "open")
            .on_action(|action| action)
            .into()
    }

    /// Focuses the input, then taps `key`.
    fn tap_in_input(key: Named) -> Vec<PaletteAction> {
        let list = list();
        let mut ui = simulator(view(&list));

        ui.click(input_id()).expect("input should be visible");
        ui.tap_key(key);

        ui.into_messages().collect()
    }

    #[test]
    fn given_focused_input_when_typing_then_query_changes() {
        let list = list();
        let mut ui = simulator(view(&list));

        ui.click(input_id()).expect("input should be visible");
        ui.typewrite("h");

        let messages: Vec<PaletteAction> = ui.into_messages().collect();
        assert_eq!(messages, vec![PaletteAction::QueryChanged("h".to_owned())]);
    }

    #[test]
    fn given_items_when_one_clicked_then_its_display_index_is_run() {
        let list = list();
        let mut ui = simulator(view(&list));

        ui.click("New launch…").expect("item should be visible");

        let messages: Vec<PaletteAction> = ui.into_messages().collect();
        assert_eq!(messages, vec![PaletteAction::RunItem(2)]);
    }

    #[test]
    fn given_focused_input_when_keys_pressed_then_each_sends_its_action() {
        let cases = [
            (Named::ArrowDown, PaletteAction::MoveDown),
            (Named::ArrowUp, PaletteAction::MoveUp),
            (Named::Enter, PaletteAction::RunActive),
            (Named::Escape, PaletteAction::Close),
        ];

        for (key, expected) in cases {
            assert_eq!(tap_in_input(key), vec![expected], "key {key:?}");
        }
    }

    #[test]
    fn given_unfocused_input_when_arrow_pressed_then_nothing_is_sent() {
        let list = list();
        let mut ui = simulator(view(&list));

        ui.tap_key(Named::ArrowDown);
        ui.tap_key(Named::Escape);

        assert_eq!(ui.into_messages().count(), 0);
    }

    #[test]
    fn given_esc_hint_when_clicked_then_palette_closes() {
        let list = list();
        let mut ui = simulator(view(&list));

        ui.click("Esc").expect("Esc hint should be visible");

        let messages: Vec<PaletteAction> = ui.into_messages().collect();
        assert_eq!(messages, vec![PaletteAction::Close]);
    }

    #[test]
    fn given_items_when_rendered_then_groups_details_and_hints_are_shown() {
        let list = list();
        let mut ui = simulator(view(&list));

        for label in [
            "Sessions",
            "Actions",
            "zsh",
            "~/projects/otty",
            "Ctrl Shift L",
            "move",
            "open",
        ] {
            assert!(ui.find(label).is_ok(), "{label:?} should be shown");
        }
        assert!(ui.find("No matches").is_err());
    }

    #[test]
    fn given_query_when_rendered_then_title_is_split_at_the_match() {
        let mut list = list();
        list.set_query("LA".to_owned());
        let mut ui = simulator(view(&list));

        assert!(ui.find("la").is_ok());
        assert!(ui.find("unch…").is_ok());
        assert!(ui.find("Sessions").is_err());
    }

    #[test]
    fn given_no_match_when_rendered_then_only_the_empty_line_is_shown() {
        let mut list = list();
        list.set_query("zzz".to_owned());
        let mut ui = simulator(view(&list));

        assert!(ui.find("No matches").is_ok());
        assert!(ui.find("Sessions").is_err());
        assert!(ui.find("Actions").is_err());
    }

    #[test]
    fn given_ranges_when_title_split_then_pieces_cover_the_title() {
        // (title, ranges as (start, end) bytes, expected pieces)
        let cases: [(&str, Spans, Pieces); 5] = [
            ("laptop", &[], &[("laptop", false)]),
            ("laptop", &[(0, 2)], &[("la", true), ("ptop", false)]),
            (
                "htop on laptop",
                &[(8, 10)],
                &[("htop on ", false), ("la", true), ("ptop", false)],
            ),
            (
                "banana",
                &[(1, 3), (3, 5)],
                &[("b", false), ("an", true), ("an", true), ("a", false)],
            ),
            (
                "Сервер Продакшн",
                &[(13, 21)],
                &[("Сервер ", false), ("Прод", true), ("акшн", false)],
            ),
        ];

        for (title, spans, expected) in cases {
            let ranges: Vec<Range<usize>> =
                spans.iter().map(|(start, end)| *start..*end).collect();

            assert_eq!(title_pieces(title, &ranges), expected, "{title:?}");
        }
    }
}

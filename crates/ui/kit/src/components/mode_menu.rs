//! `ModeMenu`: the All modes menu with pinned and other modes.

use iced::widget::text::Wrapping;
use iced::widget::{Space, button, column, container, row, svg, text, tooltip};
use iced::{Alignment, Border, Color, Element, Length, Padding};

use super::{faded, glyph, icon_button, kbd, menu};
use crate::palette::UiPalette;
use crate::{fonts, radius, size, spacing};

/// Padding around the sections (`padding: 5px`, as in `Menu`).
const MENU_PADDING: f32 = menu::MENU_PADDING;
/// Text size of a mode name and of a hint line.
const NAME_TEXT_SIZE: f32 = 12.5;
/// Text size of a section label and its note (`font-size: 12px`).
const LABEL_TEXT_SIZE: f32 = 12.0;
/// Left padding of a row (`padding: 0 6px 0 8px`).
const ROW_PADDING_LEFT: f32 = 8.0;
/// Right padding of a row.
const ROW_PADDING_RIGHT: f32 = 6.0;
/// Gap between the icon tile, the name and the shortcut (`gap: 10px`).
const ROW_GAP: f32 = 10.0;
/// Corner radius of the icon tile (`border-radius: 5px`).
const TILE_RADIUS: f32 = 5.0;
/// Edge of the icon inside the tile (13px in the design).
const TILE_ICON_SIZE: f32 = 13.0;
/// Edge of the pin icon (14px in the design).
const PIN_ICON_SIZE: f32 = 14.0;

/// State of the pin toggle at the end of a [`ModeRow`].
pub enum ModePin<Message> {
    /// No toggle, for Terminal, which cannot be unpinned.
    Hidden,
    /// Pinned: accent pin; pressing it unpins. `label` is the tooltip.
    Pinned {
        /// Tooltip, for example "Unpin".
        label: String,
        /// Message sent on press.
        on_press: Message,
    },
    /// Not pinned: muted pin; pressing it pins. `label` is the tooltip.
    Unpinned {
        /// Tooltip, for example "Pin".
        label: String,
        /// Message sent on press.
        on_press: Message,
    },
    /// Cannot be pinned now. Dimmed, sends nothing; `hint` is the tooltip.
    Disabled {
        /// Tooltip that says why, for example "Unpin another mode first".
        hint: String,
    },
}

/// One mode in a [`ModeMenuSection`].
pub struct ModeRow<Message> {
    icon: svg::Handle,
    name: String,
    shortcut: Option<String>,
    pin: ModePin<Message>,
    on_press: Option<Message>,
}

impl<Message> ModeRow<Message> {
    /// Creates a row with an `icon`, the mode `name` and no pin toggle.
    pub fn new(icon: svg::Handle, name: impl Into<String>) -> Self {
        Self {
            icon,
            name: name.into(),
            shortcut: None,
            pin: ModePin::Hidden,
            on_press: None,
        }
    }

    /// Shows a shortcut hint after the name.
    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// Sets the pin toggle.
    pub fn pin(mut self, pin: ModePin<Message>) -> Self {
        self.pin = pin;
        self
    }

    /// Sets the message sent when the row (not its pin) is pressed.
    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }
}

/// A labeled group of rows in a [`ModeMenu`].
pub struct ModeMenuSection<Message> {
    label: String,
    note: Option<String>,
    rows: Vec<ModeRow<Message>>,
    hint: Option<String>,
}

impl<Message> ModeMenuSection<Message> {
    /// Creates an empty section with a `label` such as "Pinned".
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            note: None,
            rows: Vec::new(),
            hint: None,
        }
    }

    /// Shows a note on the right of the label, for example "1 of 4".
    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }

    /// Appends a row.
    pub fn row(mut self, row: ModeRow<Message>) -> Self {
        self.rows.push(row);
        self
    }

    /// Shows a muted hint line under the rows, for an empty section.
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }
}

/// Floating menu of all modes, opened by the "⋯" button of the
/// [`ModeSwitcher`](super::mode_switcher::ModeSwitcher).
///
/// # Example
///
/// ```
/// use iced::Element;
/// use iced::widget::svg;
/// use otty_ui_kit::components::mode_menu::{
///     ModeMenu, ModeMenuSection, ModePin, ModeRow,
/// };
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Clone)]
/// enum Message {
///     Open(&'static str),
///     Unpin(&'static str),
/// }
///
/// let palette = UiPalette::otty_dark();
/// let icon = svg::Handle::from_memory(&b"<svg/>"[..]);
/// let menu: Element<'_, Message> = ModeMenu::new(&palette)
///     .section(
///         ModeMenuSection::new("Pinned").note("1 of 4").row(
///             ModeRow::new(icon, "Docker")
///                 .shortcut("Ctrl Shift 2")
///                 .on_press(Message::Open("docker"))
///                 .pin(ModePin::Pinned {
///                     label: "Unpin".to_owned(),
///                     on_press: Message::Unpin("docker"),
///                 }),
///         ),
///     )
///     .into();
/// ```
pub struct ModeMenu<Message> {
    palette: UiPalette,
    sections: Vec<ModeMenuSection<Message>>,
}

impl<Message> ModeMenu<Message> {
    /// Creates an empty menu.
    pub fn new(palette: &UiPalette) -> Self {
        Self {
            palette: palette.clone(),
            sections: Vec::new(),
        }
    }

    /// Appends a section.
    pub fn section(mut self, section: ModeMenuSection<Message>) -> Self {
        self.sections.push(section);
        self
    }
}

impl<'a, Message: Clone + 'a> From<ModeMenu<Message>> for Element<'a, Message> {
    fn from(value: ModeMenu<Message>) -> Self {
        let palette = value.palette;

        let mut sections = column![];
        for section in value.sections {
            sections = sections.push(section_view(section, &palette));
        }

        let background = palette.surface_overlay();
        let shadow = palette.shadow_popover();
        container(sections)
            .width(size::MODE_MENU)
            .padding(MENU_PADDING)
            .style(move |_| container::Style {
                background: Some(background.into()),
                border: Border {
                    radius: radius::RADIUS_MD.into(),
                    ..Border::default()
                },
                shadow,
                ..container::Style::default()
            })
            .into()
    }
}

/// Builds a section: label row, rows, optional hint.
fn section_view<'a, Message: Clone + 'a>(
    section: ModeMenuSection<Message>,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let mut content = column![label_view(section.label, section.note, palette)];
    for mode in section.rows {
        content = content.push(row_view(mode, palette));
    }
    if let Some(hint) = section.hint {
        content = content.push(hint_view(hint, palette));
    }

    content.into()
}

/// Builds the section label with its right-aligned note (`.ot-menu-label`).
fn label_view<'a, Message: 'a>(
    label: String,
    note: Option<String>,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let muted = palette.text_muted();
    let mut line = row![
        text(label)
            .size(LABEL_TEXT_SIZE)
            .font(fonts::UI)
            .color(muted)
            .width(Length::Fill)
    ];
    if let Some(note) = note {
        line = line.push(
            text(note)
                .size(LABEL_TEXT_SIZE)
                .font(fonts::UI)
                .color(muted),
        );
    }

    container(line)
        .padding(Padding {
            top: 6.0,
            right: 10.0,
            bottom: 4.0,
            left: 10.0,
        })
        .into()
}

/// Builds the muted hint line of an empty section.
fn hint_view<'a, Message: 'a>(
    hint: String,
    palette: &UiPalette,
) -> Element<'a, Message> {
    container(
        text(hint)
            .size(NAME_TEXT_SIZE)
            .font(fonts::UI)
            .color(palette.text_muted())
            .wrapping(Wrapping::Word),
    )
    .padding(Padding::from([spacing::SPACE_2, 10.0]))
    .into()
}

/// Builds one row: the name button (icon tile, name, shortcut) and the pin
/// toggle. The hover background covers the name button only.
fn row_view<'a, Message: Clone + 'a>(
    mode: ModeRow<Message>,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let tile = icon_tile(mode.icon, palette);
    let mut content = row![
        tile,
        text(mode.name)
            .size(NAME_TEXT_SIZE)
            .font(fonts::UI)
            .width(Length::Fill)
    ]
    .spacing(ROW_GAP)
    .height(Length::Fill)
    .align_y(Alignment::Center);
    if let Some(shortcut) = mode.shortcut {
        content = content.push(kbd::kbd(shortcut, palette));
    }

    let name = button(content)
        .width(Length::Fill)
        .height(size::ROW)
        .padding(Padding {
            top: 0.0,
            right: ROW_PADDING_RIGHT,
            bottom: 0.0,
            left: ROW_PADDING_LEFT,
        })
        .on_press_maybe(mode.on_press)
        .style(menu::item_style(palette, false));

    row![name, pin_view(mode.pin, palette)]
        .spacing(spacing::SPACE_1)
        .padding(Padding::ZERO.right(ROW_PADDING_RIGHT))
        .align_y(Alignment::Center)
        .into()
}

/// Builds the 22px icon tile (`.mm-ico`).
fn icon_tile<'a, Message: 'a>(
    icon: svg::Handle,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let muted = palette.text_muted();
    let background = palette.surface_raised();

    container(
        svg(icon)
            .width(TILE_ICON_SIZE)
            .height(TILE_ICON_SIZE)
            .style(move |_, _| svg::Style { color: Some(muted) }),
    )
    .width(size::MODE_ICON_TILE)
    .height(size::MODE_ICON_TILE)
    .center(size::MODE_ICON_TILE)
    .style(move |_| container::Style {
        background: Some(background.into()),
        border: Border {
            radius: TILE_RADIUS.into(),
            ..Border::default()
        },
        ..container::Style::default()
    })
    .into()
}

/// Builds the pin toggle, or a blank of the same width so shortcuts of rows
/// without a toggle line up with the others.
fn pin_view<'a, Message: Clone + 'a>(
    pin: ModePin<Message>,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let (is_pinned, hint, on_press) = match pin {
        ModePin::Hidden => {
            return Space::new()
                .width(size::CONTROL_SM)
                .height(size::CONTROL_SM)
                .into();
        },
        ModePin::Pinned { label, on_press } => (true, label, Some(on_press)),
        ModePin::Unpinned { label, on_press } => (false, label, Some(on_press)),
        ModePin::Disabled { hint } => (false, hint, None),
    };
    let is_enabled = on_press.is_some();

    let idle = pin_color(palette, is_pinned, is_enabled);
    let hover = if is_enabled {
        palette.text_strong()
    } else {
        idle
    };
    let icon = svg(glyph::pin(is_pinned))
        .width(PIN_ICON_SIZE)
        .height(PIN_ICON_SIZE)
        .style(move |_, status| svg::Style {
            color: Some(match status {
                svg::Status::Hovered => hover,
                _ => idle,
            }),
        });

    let raised = palette.surface_raised();
    let control = button(
        container(icon)
            .width(Length::Fill)
            .height(Length::Fill)
            .center(Length::Fill),
    )
    .width(size::CONTROL_SM)
    .height(size::CONTROL_SM)
    .padding(0)
    .on_press_maybe(on_press)
    .style(move |_, status| button::Style {
        background: (is_enabled
            && matches!(
                status,
                button::Status::Hovered | button::Status::Pressed
            ))
        .then(|| raised.into()),
        border: Border {
            radius: radius::RADIUS_XS.into(),
            ..Border::default()
        },
        ..button::Style::default()
    });

    tooltip(
        control,
        icon_button::tooltip_hint(hint, palette),
        tooltip::Position::Bottom,
    )
    .gap(spacing::SPACE_1)
    .into()
}

/// Idle color of a pin: accent when pinned, `text-faint` otherwise, faded
/// when disabled.
fn pin_color(palette: &UiPalette, is_pinned: bool, is_enabled: bool) -> Color {
    let color = if is_pinned {
        palette.accent()
    } else {
        palette.text_faint()
    };

    if is_enabled { color } else { faded(color) }
}

#[cfg(test)]
mod tests {
    use iced::Point;
    use iced_test::{Simulator, simulator};

    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Message {
        OpenDocker,
        OpenTerminal,
        UnpinDocker,
        PinDocker,
    }

    fn terminal_row() -> ModeRow<Message> {
        ModeRow::new(glyph::plus(), "Terminal")
            .shortcut("Ctrl Shift 1")
            .on_press(Message::OpenTerminal)
    }

    fn docker_row(pin: ModePin<Message>) -> ModeRow<Message> {
        ModeRow::new(glyph::plus(), "Docker")
            .shortcut("Ctrl Shift 2")
            .on_press(Message::OpenDocker)
            .pin(pin)
    }

    fn menu_with(
        palette: &UiPalette,
        pin: ModePin<Message>,
    ) -> ModeMenu<Message> {
        ModeMenu::new(palette).section(
            ModeMenuSection::new("Pinned")
                .note("2 of 4")
                .row(terminal_row())
                .row(docker_row(pin)),
        )
    }

    /// Horizontal center of the pin toggle when the menu sits at the origin.
    fn pin_x() -> f32 {
        size::MODE_MENU
            - MENU_PADDING
            - ROW_PADDING_RIGHT
            - size::CONTROL_SM / 2.0
    }

    /// Presses the pin toggle of the row called `name`.
    fn click_pin(ui: &mut Simulator<'_, Message>, name: &str) {
        let y = ui
            .find(name)
            .expect("row name should be visible")
            .bounds()
            .center_y();

        ui.point_at(Point::new(pin_x(), y));
        let _ = ui.simulate(iced_test::simulator::click());
    }

    #[test]
    fn given_menu_when_row_name_pressed_then_row_message_is_sent() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(menu_with(
            &palette,
            ModePin::Unpinned {
                label: "Pin".to_owned(),
                on_press: Message::PinDocker,
            },
        ));

        ui.click("Docker").expect("row name should be visible");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::OpenDocker]);
    }

    #[test]
    fn given_pinned_row_when_pin_pressed_then_unpin_message_is_sent() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(menu_with(
            &palette,
            ModePin::Pinned {
                label: "Unpin".to_owned(),
                on_press: Message::UnpinDocker,
            },
        ));

        click_pin(&mut ui, "Docker");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::UnpinDocker]);
    }

    #[test]
    fn given_unpinned_row_when_pin_pressed_then_pin_message_is_sent() {
        let palette = UiPalette::otty_light();
        let mut ui = simulator(menu_with(
            &palette,
            ModePin::Unpinned {
                label: "Pin".to_owned(),
                on_press: Message::PinDocker,
            },
        ));

        click_pin(&mut ui, "Docker");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::PinDocker]);
    }

    #[test]
    fn given_disabled_pin_when_pressed_then_nothing_is_sent() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(menu_with(
            &palette,
            ModePin::Disabled {
                hint: "Unpin another mode first".to_owned(),
            },
        ));

        click_pin(&mut ui, "Docker");

        assert_eq!(ui.into_messages().count(), 0);
    }

    #[test]
    fn given_terminal_row_without_pin_when_pin_spot_pressed_then_nothing_is_sent()
     {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(menu_with(&palette, ModePin::Hidden));

        click_pin(&mut ui, "Terminal");

        assert_eq!(ui.into_messages().count(), 0);
    }

    #[test]
    fn given_section_with_note_and_hint_then_both_are_shown() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(
            ModeMenu::<Message>::new(&palette).section(
                ModeMenuSection::new("Other modes")
                    .note("0 of 4")
                    .hint("Every mode is pinned."),
            ),
        );

        assert!(ui.find("Other modes").is_ok());
        assert!(ui.find("0 of 4").is_ok());
        assert!(ui.find("Every mode is pinned.").is_ok());
    }

    #[test]
    fn given_menu_when_laid_out_then_it_is_300_wide() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(menu_with(&palette, ModePin::Hidden));

        let label = ui.find("Pinned").expect("label should be visible");
        let note = ui.find("2 of 4").expect("note should be visible");

        // The note is right-aligned: its right edge is the menu width minus
        // the menu and label paddings.
        let right = note.bounds().x + note.bounds().width;
        assert!((right - (size::MODE_MENU - MENU_PADDING - 10.0)).abs() < 1.0);
        assert!(label.bounds().x < note.bounds().x);
    }

    #[test]
    fn given_pin_states_then_pinned_is_accent_and_disabled_is_faded() {
        let palette = UiPalette::otty_dark();

        assert_eq!(pin_color(&palette, true, true), palette.accent());
        assert_eq!(pin_color(&palette, false, true), palette.text_faint());
        assert_eq!(
            pin_color(&palette, false, false),
            faded(palette.text_faint())
        );
    }
}

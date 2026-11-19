//! The chips of the pages: "Used by" on an account page and "Pays
//! through" on a profile page. A chip is an icon, a name and a small
//! number in a line, in a thin frame; a press opens what it names.

use iced::widget::{Row, button, row, svg, text};
use iced::{Alignment, Border, Color, Element, Padding};
use otty_ui_kit::components::small_text::small_text;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, radius, size, spacing, typography};

use super::super::event::AgentsIntent;

/// Edge of the icon of a chip (`size={14}`).
const CHIP_ICON: f32 = 14.0;
/// Gap between the parts of a chip and between two chips (`gap: 8px`).
const CHIP_GAP: f32 = spacing::SPACE_3;
/// Padding of a chip (`padding: 6px 10px`).
const CHIP_PADDING: Padding = Padding {
    top: 6.0,
    right: 10.0,
    bottom: 6.0,
    left: 10.0,
};

/// What one chip shows and what a press on it sends.
pub(super) struct Chip {
    /// The SVG of its icon: the agent's or the provider's.
    pub(super) icon: &'static [u8],
    /// The name, such as "Codex · Work" or "OpenAI · you@example.com".
    pub(super) name: String,
    /// The small number after the name, with its color.
    pub(super) note: Option<(String, Color)>,
    /// What a press sends.
    pub(super) on_press: AgentsIntent,
}

/// The chips side by side; those that do not fit go to the next line.
pub(super) fn view<'a>(
    chips: Vec<Chip>,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let chips = chips.into_iter().map(|one| chip(one, palette));

    Row::with_children(chips)
        .spacing(CHIP_GAP)
        .wrap()
        .vertical_spacing(CHIP_GAP)
        .into()
}

/// One chip: the icon, the name and the note in a line, in a frame that
/// lights under the pointer.
fn chip<'a>(chip: Chip, palette: &UiPalette) -> Element<'a, AgentsIntent> {
    let strong = palette.text_strong();
    let body = typography::BODY;
    let icon = svg(svg::Handle::from_memory(chip.icon))
        .width(CHIP_ICON)
        .height(CHIP_ICON)
        .style(move |_, _| svg::Style {
            color: Some(strong),
        });
    let name = text(chip.name)
        .size(body.size())
        .line_height(iced::Pixels(body.line_height()))
        .font(fonts::UI)
        .color(strong);

    let mut content = row![icon, name]
        .spacing(CHIP_GAP)
        .align_y(Alignment::Center);
    if let Some((note, color)) = chip.note {
        content = content.push(small_text(note, color));
    }

    let line = palette.line();
    let hover = palette.surface_hover();
    button(content)
        .padding(CHIP_PADDING)
        .on_press(chip.on_press)
        .style(move |_, status| button::Style {
            background: matches!(
                status,
                button::Status::Hovered | button::Status::Pressed
            )
            .then(|| hover.into()),
            text_color: strong,
            border: Border {
                color: line,
                width: size::LINE,
                radius: radius::RADIUS_MD.into(),
            },
            ..button::Style::default()
        })
        .into()
}

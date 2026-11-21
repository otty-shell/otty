//! The chips of the pages: "Used by" on an account page, "Pays through"
//! on a profile page, and the profiles and accounts of a project. A chip
//! is an icon, a name and a small number in a line, in a thin frame; a
//! press opens what it names.

use iced::widget::{Row, button, row, text};
use iced::{Alignment, Border, Color, Element, Padding};
use otty_ui_kit::components::session_row::SessionTone;
use otty_ui_kit::components::small_text::small_text;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, radius, size, spacing, typography};

use super::super::event::AgentsIntent;
use super::super::model::{EntryIcon, PaysThroughViewModel};
use super::{entry_icon, money};

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
    /// Its icon: the agent's or the provider's.
    pub(super) icon: EntryIcon,
    /// The name, such as "Coder · Work" or "Example Cloud · you@example.com".
    pub(super) name: String,
    /// The small number after the name, with its color.
    pub(super) note: Option<(String, Color)>,
    /// The tooltip of the number in the tight form: the tooltip of the
    /// mark of money estimated at the list prices.
    pub(super) tip: Option<String>,
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

/// The chips of accounts: the provider icon, the account name and its
/// number in the tight form, in the error color when it needs the user;
/// a press opens the account.
pub(super) fn account_chips<'a>(
    accounts: Vec<PaysThroughViewModel>,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let chips = accounts
        .into_iter()
        .map(|chip| {
            let note = chip.number.as_ref().map(|number| {
                let color = match number.tone() {
                    SessionTone::Failed => palette.status_error(),
                    SessionTone::Waiting => palette.status_warn(),
                    _ => palette.text_muted(),
                };
                (number.text(), color)
            });
            let tip = chip.number.as_ref().and_then(|number| number.tip());

            Chip {
                icon: chip.icon,
                name: chip.name,
                note,
                tip: tip.map(String::from),
                on_press: AgentsIntent::SelectAccount(chip.account),
            }
        })
        .collect();

    view(chips, palette)
}

/// One chip: the icon, the name and the note in a line, in a frame that
/// lights under the pointer.
fn chip<'a>(chip: Chip, palette: &UiPalette) -> Element<'a, AgentsIntent> {
    let strong = palette.text_strong();
    let body = typography::BODY;
    let icon = entry_icon::view(chip.icon, CHIP_ICON, strong, palette);
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
    let framed = button(content)
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
        });

    money::with_tip(framed, chip.tip.as_deref(), palette)
}

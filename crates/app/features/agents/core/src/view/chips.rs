//! The chips of the pages: "Used by" on an account page, "Pays through"
//! on a profile page, and the profiles and accounts of a project. A chip
//! is an icon, a name and a small number in a line, in a thin frame; a
//! press opens what it names. A chip never wraps its texts: in too little
//! room it moves to the next line, and only a chip wider than the whole
//! line cuts its name with "…".

use iced::widget::{Row, button, container, row, text};
use iced::{Alignment, Border, Color, Element, Length, Padding};
use otty_ui_kit::components::ellipsis_text::{EllipsisText, text_width};
use otty_ui_kit::components::session_row::SessionTone;
use otty_ui_kit::components::small_text::small_text;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, radius, size, spacing, typography};

use super::super::event::AgentsIntent;
use super::super::model::{AccountChipViewModel, EntryIcon};
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
    /// The name, such as "Coder · Work" or "you@example.com".
    pub(super) name: String,
    /// The small number after the name, with its color.
    pub(super) note: Option<(String, Color)>,
    /// The tooltip of the chip, such as the whole name of an account with
    /// its provider, or the tooltip of the mark of money estimated at the
    /// list prices.
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

/// The chips of accounts, the one look of every account chip: the
/// provider logo, the account's login or the name the user gave it and its
/// number in the tight form, in the error color when it needs the user.
/// The whole name with the provider is the tooltip, over the tooltip of
/// an estimate; a press opens the account.
pub(super) fn account_chips<'a>(
    accounts: Vec<AccountChipViewModel>,
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
            let full_name = chip.full_name;
            let tip = match chip.number.as_ref().and_then(|number| number.tip())
            {
                Some(estimate) => format!("{full_name}\n{estimate}"),
                None => full_name,
            };

            Chip {
                icon: chip.icon,
                name: chip.name,
                note,
                tip: Some(tip),
                on_press: AgentsIntent::SelectAccount(chip.account),
            }
        })
        .collect();

    view(chips, palette)
}

/// One chip: the icon, the name and the note on one line, in a frame
/// that lights under the pointer. The icon and the note keep their size;
/// the name ends with "…" when the chip is wider than its space.
fn chip<'a>(chip: Chip, palette: &UiPalette) -> Element<'a, AgentsIntent> {
    let strong = palette.text_strong();
    let body = typography::BODY;
    let icon = entry_icon::view(chip.icon, CHIP_ICON, strong, palette);

    // The name leaves the note its width, so only the name gives way.
    let kept_for_note = chip.note.as_ref().map_or(0.0, |(note, _)| {
        CHIP_GAP + text_width(note, typography::SMALL.size(), fonts::UI)
    });
    let name = EllipsisText::new(chip.name, body.size(), fonts::UI, strong)
        .shrink(kept_for_note);
    let name = container(name)
        .height(Length::Fixed(body.line_height()))
        .align_y(Alignment::Center);

    let mut content = row![icon, name]
        .spacing(CHIP_GAP)
        .align_y(Alignment::Center);
    if let Some((note, color)) = chip.note {
        content = content
            .push(small_text(note, color).wrapping(text::Wrapping::None));
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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use iced::widget::column;
    use iced::{Point, Rectangle};
    use iced_test::{Simulator, simulator};
    use otty_app_testing::temp_dir::TempDir;
    use otty_app_theme::theme::ThemeManager;

    use super::*;
    use crate::testing::{chatgpt_login, provider_icon};

    /// Width of the page column in the narrow tests, as the owner asked.
    const NARROW: f32 = 320.0;
    /// A login too long for a chip in [`NARROW`].
    const LONG: &str =
        "someone.with.a.very.long.name@a-long-example-domain.com";
    const NUMBER: &str = "42 % used";

    fn chip_named(name: &str, note: Option<&str>) -> Chip {
        Chip {
            icon: provider_icon("openai"),
            name: name.to_string(),
            note: note.map(|note| (note.to_string(), Color::WHITE)),
            tip: None,
            on_press: AgentsIntent::SelectAccount(chatgpt_login()),
        }
    }

    /// The chips in a column `width` wide.
    fn chips_in(
        width: f32,
        chips: Vec<Chip>,
    ) -> Simulator<'static, AgentsIntent> {
        let themes = ThemeManager::new();
        let row = view(chips, themes.current().ui());

        simulator(container(column![row]).width(width))
    }

    fn bounds(ui: &mut Simulator<'_, AgentsIntent>, text: &str) -> Rectangle {
        ui.find(text).expect("the text should show").bounds()
    }

    /// Issue 38: in a narrow page a name too long for the line is cut on
    /// one line, and the number keeps its size at the end of the chip.
    #[test]
    fn a_long_name_is_cut_on_one_line_and_the_number_keeps_its_size() {
        let body = typography::BODY;
        let mut ui = chips_in(NARROW, vec![chip_named(LONG, Some(NUMBER))]);

        let name = bounds(&mut ui, LONG);
        let number = bounds(&mut ui, NUMBER);

        let whole = text_width(LONG, body.size(), fonts::UI);
        assert!(name.width < whole, "the name is cut: {name:?}");
        assert!(name.height <= body.line_height(), "one line: {name:?}");
        let number_width =
            text_width(NUMBER, typography::SMALL.size(), fonts::UI);
        assert!((number.width - number_width).abs() < 1.0, "{number:?}");
        assert!(number.x >= name.x + name.width, "{number:?} after {name:?}");
        assert!(number.x + number.width <= NARROW, "inside: {number:?}");
    }

    /// Chips that do not fit beside each other go to the next line whole,
    /// before any of them is cut.
    #[test]
    fn chips_go_to_the_next_line_before_they_are_cut() {
        let first = "first.login@example.com";
        let second = "second.login@example.com";
        let mut ui = chips_in(
            NARROW,
            vec![chip_named(first, Some(NUMBER)), chip_named(second, None)],
        );

        let one = bounds(&mut ui, first);
        let two = bounds(&mut ui, second);

        let body = typography::BODY;
        let whole = |name: &str| text_width(name, body.size(), fonts::UI);
        assert!((one.width - whole(first)).abs() < 1.0, "whole: {one:?}");
        assert!((two.width - whole(second)).abs() < 1.0, "whole: {two:?}");
        assert!(two.y > one.y + one.height, "{two:?} is under {one:?}");
    }

    /// Chips that fit stay side by side.
    #[test]
    fn chips_that_fit_stay_on_one_line() {
        let mut ui = chips_in(
            NARROW,
            vec![chip_named("one@x.io", None), chip_named("two@x.io", None)],
        );

        let one = bounds(&mut ui, "one@x.io");
        let two = bounds(&mut ui, "two@x.io");

        assert!((one.y - two.y).abs() < 0.5);
        assert!(two.x > one.x + one.width);
    }

    /// Says whether the account chip whose whole name is `full_name` draws
    /// the picture at `reference`, with the pointer over its login or
    /// away; the first call saves the picture.
    fn chip_draws(full_name: &str, is_over: bool, reference: &Path) -> bool {
        let themes = ThemeManager::new();
        let chip = AccountChipViewModel {
            account: chatgpt_login(),
            icon: provider_icon("openai"),
            name: String::from("you@example.com"),
            full_name: full_name.to_string(),
            number: None,
        };
        let chips = account_chips(vec![chip], themes.current().ui());
        let mut ui = simulator(container(chips).width(400.0).height(120.0));
        if is_over {
            let position: Point = bounds(&mut ui, "you@example.com").center();
            ui.point_at(position);
            let _ = ui.simulate([iced::Event::Mouse(
                iced::mouse::Event::CursorMoved { position },
            )]);
        }

        ui.snapshot(&iced::Theme::Dark)
            .expect("the chip should draw")
            .matches_hash(reference)
            .expect("the reference should be readable")
    }

    /// Issue 38: an account chip shows the login, and the whole name with
    /// the provider shows in its tooltip under the pointer, and nowhere
    /// else.
    #[test]
    fn an_account_chip_shows_its_full_name_on_hover() {
        let folder = TempDir::new("agents-account-chip");
        let away = folder.path().join("away");
        let over = folder.path().join("over");
        let full = "OpenAI · you@example.com";
        let other = "Example Cloud · you@example.com";

        assert!(chip_draws(full, false, &away), "save");
        assert!(chip_draws(other, false, &away), "only in the tooltip");
        assert!(chip_draws(full, true, &over), "save");
        assert!(!chip_draws(other, true, &over), "the tooltip shows it");
        assert!(!chip_draws(full, true, &away), "under the pointer");
    }
}

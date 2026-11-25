//! The chips of the pages: the profiles of a project. A chip is an icon
//! and a name in a line, in a thin frame; a press opens what it names. A
//! chip never wraps its text: in too little room it moves to the next
//! line, and only a chip wider than the whole line cuts its name with "…".

use iced::widget::{Row, button, container, row};
use iced::{Alignment, Border, Element, Length, Padding};
use otty_ui_kit::components::ellipsis_text::EllipsisText;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, radius, size, spacing, typography};

use super::super::event::AgentsIntent;
use super::super::model::EntryIcon;
use super::entry_icon;

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
    /// The icon of the agent.
    pub(super) icon: EntryIcon,
    /// The name, such as "Coder (Work)".
    pub(super) name: String,
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

/// One chip: the icon and the name on one line, in a frame that lights
/// under the pointer. The icon keeps its size; the name ends with "…" when
/// the chip is wider than its space.
fn chip<'a>(chip: Chip, palette: &UiPalette) -> Element<'a, AgentsIntent> {
    let strong = palette.text_strong();
    let body = typography::BODY;
    let icon = entry_icon::view(chip.icon, CHIP_ICON, strong, palette);

    let name = EllipsisText::new(chip.name, body.size(), fonts::UI, strong)
        .shrink(0.0);
    let name = container(name)
        .height(Length::Fixed(body.line_height()))
        .align_y(Alignment::Center);
    let content = row![icon, name]
        .spacing(CHIP_GAP)
        .align_y(Alignment::Center);

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

#[cfg(test)]
mod tests {
    use iced::Rectangle;
    use iced::widget::column;
    use iced_test::{Simulator, simulator};
    use otty_app_theme::theme::ThemeManager;
    use otty_ui_kit::components::ellipsis_text::text_width;

    use super::*;
    use crate::testing::{agent_icon, claude};

    /// Width of the page column in the narrow tests, as the owner asked.
    const NARROW: f32 = 320.0;
    /// A profile name too long for a chip in [`NARROW`].
    const LONG: &str =
        "Claude Code (a very long name of a profile for client work)";

    fn chip_named(name: &str) -> Chip {
        Chip {
            icon: agent_icon("claude-code"),
            name: name.to_string(),
            on_press: AgentsIntent::SelectProfile(claude()),
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
    /// one line, inside the page.
    #[test]
    fn a_long_name_is_cut_on_one_line() {
        let body = typography::BODY;
        let mut ui = chips_in(NARROW, vec![chip_named(LONG)]);

        let name = bounds(&mut ui, LONG);

        let whole = text_width(LONG, body.size(), fonts::UI);
        assert!(name.width < whole, "the name is cut: {name:?}");
        assert!(name.height <= body.line_height(), "one line: {name:?}");
        assert!(name.x + name.width <= NARROW, "inside: {name:?}");
    }

    /// Chips that do not fit beside each other go to the next line whole,
    /// before any of them is cut.
    #[test]
    fn chips_go_to_the_next_line_before_they_are_cut() {
        let first = "Claude Code (first profile)";
        let second = "Codex (the second profile)";
        let mut ui =
            chips_in(NARROW, vec![chip_named(first), chip_named(second)]);

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
            vec![chip_named("one@x.io"), chip_named("two@x.io")],
        );

        let one = bounds(&mut ui, "one@x.io");
        let two = bounds(&mut ui, "two@x.io");

        assert!((one.y - two.y).abs() < 0.5);
        assert!(two.x > one.x + one.width);
    }
}

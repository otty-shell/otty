//! What the accounts or the profiles need from the user as a plain to-do
//! list (spec 2026-11-19, Decision 6): each need its place and title, its
//! line under them and its one button at the right, the rows split by
//! rules and no boxes. The Found by OTTY report and the All accounts and
//! All profiles pages show it.

use iced::widget::{Column, column, row};
use iced::{Alignment, Element, Length};
use otty_ui_kit::components::button::{Button, ButtonKind, ButtonSize};
use otty_ui_kit::components::divider;
use otty_ui_kit::components::small_text::small_text;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::spacing;

use super::super::event::AgentsIntent;
use super::super::model::NeedViewModel;
use super::super::need::NeedAction;

/// Render `needs` as a to-do list; the button of a need sends `on_fix`
/// with what the need asks.
pub(super) fn view<'a>(
    needs: Vec<NeedViewModel>,
    on_fix: fn(NeedAction) -> AgentsIntent,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let mut list = Column::new().spacing(spacing::SPACE_3);

    for (index, need) in needs.into_iter().enumerate() {
        if index > 0 {
            list = list.push(divider::horizontal(palette));
        }

        let place = format!("{} · {}", need.place, need.title);
        let text = column![
            small_text(place, palette.text_strong()),
            small_text(need.line, palette.text_muted()),
        ]
        .spacing(spacing::SPACE_1)
        .width(Length::Fill);

        let mut line = row![text].spacing(spacing::SPACE_4);
        if let Some(fix) = need.fix {
            line = line.push(
                Button::new(fix.label, palette)
                    .kind(ButtonKind::Secondary)
                    .size(ButtonSize::Small)
                    .on_press(on_fix(fix.action)),
            );
        }

        list = list.push(line.align_y(Alignment::Center));
    }

    list.into()
}

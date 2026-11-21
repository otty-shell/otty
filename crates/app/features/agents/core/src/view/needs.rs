//! What the accounts or the profiles need from the user as a plain to-do
//! list (spec 2026-11-19, Decision 6): each need its place and title, a
//! small label naming the plugin that asks when a plugin part gives it,
//! its line under them and its button at the right (two for a format a
//! plugin saw change: Open plugin repository too), the rows split by
//! rules and no boxes. The Found by OTTY report, the All accounts and
//! All profiles pages and the page of an account show it.

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
        let mut heading = row![small_text(place, palette.text_strong())]
            .spacing(spacing::SPACE_2);
        if let Some(plugin) = need.plugin {
            heading = heading.push(small_text(plugin, palette.text_muted()));
        }
        let text =
            column![heading, small_text(need.line, palette.text_muted())]
                .spacing(spacing::SPACE_1)
                .width(Length::Fill);

        let mut line = row![text].spacing(spacing::SPACE_4);
        for fix in need.extra.into_iter().chain(need.fix) {
            let mut button = Button::new(fix.label, palette)
                .kind(ButtonKind::Secondary)
                .size(ButtonSize::Small);
            if let Some(action) = fix.action {
                button = button.on_press(on_fix(action));
            }
            line = line.push(button);
        }

        list = list.push(line.align_y(Alignment::Center));
    }

    list.into()
}

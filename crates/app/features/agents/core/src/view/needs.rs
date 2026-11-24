//! What the accounts or the profiles need from the user as a plain to-do
//! list (spec 2026-11-19, Decision 6) in a warning card: each need its
//! place and title, its line under them, then one line with a small tag
//! naming the plugin that asks when a plugin part gives it, at the left,
//! and its button at the right edge (two for a format a plugin saw
//! change: Open plugin repository too), the needs split by thin rules.
//! The Found by OTTY report, the All accounts and All profiles pages and
//! the pages of an account and of a profile show it.
//!
//! A need is laid out the same at every width (issue 24 of spec
//! 2026-11-23-accounts-apart-from-profiles, in place of issue 21's label
//! and buttons beside the text): the text takes the whole width and
//! wraps, the tag stays on one line and the buttons stay whole. When even
//! the line under the text is too short for both, the label is cut with
//! "…", and its tooltip names it whole.

use iced::widget::{Column, Row, Space, column, responsive, text};
use iced::{Alignment, Element, Length};
use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::components::button::{Button, ButtonKind, ButtonSize};
use otty_ui_kit::components::card::{self, Card, CardTone};
use otty_ui_kit::components::ellipsis_text::EllipsisText;
use otty_ui_kit::components::small_text::small_text;
use otty_ui_kit::components::tag::{tag, tag_width};
use otty_ui_kit::components::tooltip::tooltip;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, size, spacing, typography};

use super::super::event::AgentsIntent;
use super::super::model::NeedViewModel;
use super::super::need::NeedAction;

/// Gap between the label and the buttons.
const SIDE_GAP: f32 = spacing::SPACE_4;
/// Gap between two buttons of a need.
const BUTTON_GAP: f32 = spacing::SPACE_3;
/// Gap between the text of a need and the line of its label and buttons.
const TEXT_GAP: f32 = spacing::SPACE_3;
/// Gap between a need and the rule after it.
const RULE_GAP: f32 = spacing::SPACE_4;
/// Height of the line of the label and the buttons: a small button.
const FOOT_HEIGHT: f32 = size::CONTROL_MD;

/// Render `needs` as the warning card titled `title`; the button of a
/// need sends `on_fix` with what the need asks.
pub(super) fn card<'a>(
    title: &str,
    needs: Vec<NeedViewModel>,
    on_fix: fn(NeedAction) -> AgentsIntent,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    Card::new(view(needs, on_fix, palette), palette)
        .title(title)
        .tone(CardTone::Warning)
        .into()
}

/// Render `needs` as a to-do list split by the rules of a warning card;
/// the button of a need sends `on_fix` with what the need asks.
pub(super) fn view<'a>(
    needs: Vec<NeedViewModel>,
    on_fix: fn(NeedAction) -> AgentsIntent,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let mut list = Column::new().spacing(RULE_GAP);

    for (index, need) in needs.into_iter().enumerate() {
        if index > 0 {
            list = list.push(card::rule(CardTone::Warning, palette));
        }
        list = list.push(need_row(need, on_fix, palette));
    }

    list.into()
}

/// One need: its text across the whole width, then its label and buttons
/// on their own line.
fn need_row<'a>(
    need: NeedViewModel,
    on_fix: fn(NeedAction) -> AgentsIntent,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let text = text_of(&need, palette);
    let has_foot = need.plugin.is_some() || need.fix.is_some();

    let mut row = column![text].spacing(TEXT_GAP);
    if has_foot {
        row = row.push(foot(need, on_fix, palette));
    }

    row.into()
}

/// The line under the text: the label of the plugin at the left, as a
/// tag, and the buttons at the right edge. A label too long for the width
/// the buttons leave is cut, and its tooltip names it whole.
fn foot<'a>(
    need: NeedViewModel,
    on_fix: fn(NeedAction) -> AgentsIntent,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let palette = palette.clone();

    responsive(move |size| {
        let (actions, actions_width) = actions(&need, on_fix, &palette);
        let reserved = match actions {
            Some(_) => actions_width + SIDE_GAP,
            None => 0.0,
        };

        let mut line = Row::new()
            .spacing(SIDE_GAP)
            .height(FOOT_HEIGHT)
            .align_y(Alignment::Center);
        if let Some(plugin) = need.plugin.as_deref() {
            line = line.push(label(plugin, reserved, size.width, &palette));
        }
        line = line.push(Space::new().width(Length::Fill));
        if let Some(actions) = actions {
            line = line.push(actions);
        }

        line.into()
    })
    .height(Length::Shrink)
    .into()
}

/// The heading, the place and the title of the need, and its line under
/// it, wrapping in all the width they get.
fn text_of<'a>(
    need: &NeedViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let body = typography::BODY;
    let heading = i18n::filled(
        Key::TplAgentsNeedHeading,
        &[("place", &need.place), ("title", &need.title)],
    );

    column![
        text(heading)
            .size(body.size())
            .line_height(iced::Pixels(body.line_height()))
            .font(fonts::UI)
            .color(palette.text_strong()),
        small_text(need.line.clone(), palette.text_muted()),
    ]
    .spacing(spacing::SPACE_1)
    .width(Length::Fill)
    .into()
}

/// The label of the plugin that asks: a tag when it fits beside
/// `reserved` width for the buttons in a line `width` wide, else muted
/// text cut with "…", whose tooltip names it whole.
fn label<'a>(
    plugin: &str,
    reserved: f32,
    width: f32,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    if tag_width(plugin) + reserved <= width {
        return tag(plugin.to_string(), FOOT_HEIGHT, palette);
    }

    let small = typography::SMALL;
    let cut = EllipsisText::new(
        plugin,
        small.size(),
        fonts::UI,
        palette.text_muted(),
    )
    .shrink(reserved);
    tooltip(cut, plugin.to_string(), palette)
}

/// The buttons of the need side by side, the second one first, each
/// sending `on_fix` with its action or shown disabled without one, and
/// the width they take; `None` and 0 when the need has no button.
fn actions<'a>(
    need: &NeedViewModel,
    on_fix: fn(NeedAction) -> AgentsIntent,
    palette: &UiPalette,
) -> (Option<Element<'a, AgentsIntent>>, f32) {
    let mut actions = Row::new().spacing(BUTTON_GAP);
    let mut width = 0.0;

    for (index, fix) in need.extra.iter().chain(&need.fix).enumerate() {
        let mut button = Button::new(fix.label.as_str(), palette)
            .kind(ButtonKind::Secondary)
            .size(ButtonSize::Small);
        if let Some(action) = fix.action.clone() {
            button = button.on_press(on_fix(action));
        }

        if index > 0 {
            width += BUTTON_GAP;
        }
        width += button.width();
        actions = actions.push(button);
    }

    match width > 0.0 {
        true => (Some(actions.into()), width),
        false => (None, 0.0),
    }
}

#[cfg(test)]
mod tests {
    use iced::Rectangle;
    use iced::widget::container;
    use iced_test::{Simulator, simulator};
    use otty_app_theme::theme::ThemeManager;
    use otty_ui_kit::components::ellipsis_text::text_width;
    use otty_ui_kit::{fonts, typography};

    use super::*;
    use crate::model::NeedFixViewModel;

    /// A wide card.
    const WIDE: f32 = 900.0;
    /// A narrow card.
    const NARROW: f32 = 360.0;
    /// A card narrower than the label and the button side by side.
    const TINY: f32 = 200.0;
    /// The place of the need, an organization of the Anthropic plugin.
    const PLACE: &str = "Anthropic (Example Org, Claude API Platform)";
    /// The title of the need.
    const TITLE: &str = "the real bill needs an admin key";
    /// The line of the need.
    const LINE: &str = "OTTY shows no money for this organization until it \
                        has an admin key, which reads the real bill. No \
                        agent keeps one.";
    /// The label of the plugin that asks.
    const PLUGIN: &str = "Anthropic plugin";
    /// The label of the button.
    const BUTTON: &str = "Admin key…";
    /// Size of the label of a small button (`font-size: 12.5px`).
    const BUTTON_TEXT: f32 = 12.5;

    fn need_of(plugin: &str) -> NeedViewModel {
        NeedViewModel {
            place: PLACE.to_string(),
            plugin: Some(plugin.to_string()),
            title: TITLE.to_string(),
            line: LINE.to_string(),
            fix: Some(NeedFixViewModel {
                label: BUTTON.to_string(),
                action: Some(NeedAction::OpenPage(
                    "https://example.com".to_string(),
                )),
            }),
            extra: None,
        }
    }

    /// The needs view of `needs` in a card `width` wide.
    fn needs_in(
        width: f32,
        needs: Vec<NeedViewModel>,
    ) -> Simulator<'static, AgentsIntent> {
        let themes = ThemeManager::new();
        let list = view(needs, AgentsIntent::FixNeed, themes.current().ui());

        simulator(container(list).width(width))
    }

    fn bounds(ui: &mut Simulator<'_, AgentsIntent>, text: &str) -> Rectangle {
        ui.find(text).expect("the text should show").bounds()
    }

    fn heading_of(title: &str) -> String {
        i18n::filled(
            Key::TplAgentsNeedHeading,
            &[("place", PLACE), ("title", title)],
        )
    }

    /// Owner's choices of issue 24 of spec
    /// 2026-11-23-accounts-apart-from-profiles: at every width the text
    /// takes the whole width, then one line under it holds the label of
    /// the plugin at the left and the button at the right edge.
    #[test]
    fn at_every_width_the_label_and_the_button_go_on_a_line_under_the_text() {
        for width in [WIDE, NARROW] {
            let mut ui = needs_in(width, vec![need_of(PLUGIN)]);

            let heading = bounds(&mut ui, &heading_of(TITLE));
            let line = bounds(&mut ui, LINE);
            let label = bounds(&mut ui, PLUGIN);
            let button = bounds(&mut ui, BUTTON);

            let text_end = line.y + line.height;
            assert!(label.y >= text_end, "at {width}: {label:?} {line:?}");
            assert!(button.y >= text_end, "at {width}: {button:?} {line:?}");
            assert!(
                label.x - heading.x < spacing::SPACE_3,
                "at {width}: the label starts the line: {label:?}"
            );
            // The label of a small button sits 10 px inside its edge.
            assert!(
                (button.x + button.width - width).abs() < spacing::SPACE_4,
                "at {width}: the button ends the line: {button:?}"
            );
            assert!(
                (label.center_y() - button.center_y()).abs() < 1.0,
                "at {width}: one line: {label:?} {button:?}"
            );
        }
    }

    /// Issue 24: the text wraps across the whole width of the card, not
    /// beside the label and the button.
    #[test]
    fn the_text_of_a_need_takes_the_whole_width() {
        let mut ui = needs_in(NARROW, vec![need_of(PLUGIN)]);

        let line = bounds(&mut ui, LINE);

        assert!(
            line.width > NARROW - 2.0 * spacing::SPACE_6,
            "{line:?} wraps across the card"
        );
    }

    /// Issue 21, kept by issue 24: in a card too narrow even for the
    /// label and the button, the label is cut on one line and the button
    /// stays whole.
    #[test]
    fn a_label_with_no_room_is_cut_on_one_line_and_the_button_stays_whole() {
        let small = typography::SMALL;
        let long = "A plugin with a rather long name plugin";
        let mut ui = needs_in(TINY, vec![need_of(long)]);

        let label = bounds(&mut ui, long);
        let button = bounds(&mut ui, BUTTON);

        assert!(label.height <= small.line_height(), "one line: {label:?}");
        let whole = text_width(long, small.size(), fonts::UI);
        assert!(label.width < whole, "cut: {label:?}");
        assert!(label.x + label.width <= button.x, "{label:?} {button:?}");
        let button_text = text_width(BUTTON, BUTTON_TEXT, fonts::UI);
        assert!(
            (button.width - button_text).abs() < 1.0,
            "whole: {button:?}"
        );
        assert!(button.x + button.width <= TINY, "inside: {button:?}");
    }

    /// Issue 24: the next need starts under the line of the buttons of
    /// the one before, a rule between them.
    #[test]
    fn needs_follow_one_another_under_their_buttons() {
        let second = NeedViewModel {
            title: String::from("asks for new permissions"),
            ..need_of(PLUGIN)
        };
        let mut ui = needs_in(WIDE, vec![need_of(PLUGIN), second]);

        let button = ui
            .find(BUTTON)
            .expect("the first button should show")
            .bounds();
        let next = bounds(&mut ui, &heading_of("asks for new permissions"));

        assert!(
            next.y >= button.y + button.height + spacing::SPACE_4,
            "{next:?} under {button:?} and the rule"
        );
    }
}

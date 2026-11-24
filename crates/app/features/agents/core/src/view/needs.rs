//! What the accounts or the profiles need from the user as a plain to-do
//! list (spec 2026-11-19, Decision 6): each need its place and title, its
//! line under them, then a small label naming the plugin that asks when a
//! plugin part gives it and its button (two for a format a plugin saw
//! change: Open plugin repository too), the rows split by rules and no
//! boxes. The Found by OTTY report, the All accounts and All profiles
//! pages and the page of an account show it.
//!
//! A need fits any width (issue 21 of spec
//! 2026-11-23-accounts-apart-from-profiles): the text takes the width and
//! wraps, the label stays on one line and the buttons stay whole. When the
//! text would keep less than [`TEXT_MIN_WIDTH`] beside them, the label and
//! the buttons go to their own line under the text, at the right. There
//! the label is cut with "…" if even that line is too short, and its
//! tooltip names it whole.

use iced::alignment::Horizontal;
use iced::widget::{Column, Row, column, container, responsive, row};
use iced::{Alignment, Element, Length};
use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::components::button::{Button, ButtonKind, ButtonSize};
use otty_ui_kit::components::divider;
use otty_ui_kit::components::ellipsis_text::{EllipsisText, text_width};
use otty_ui_kit::components::small_text::small_text;
use otty_ui_kit::components::tooltip::tooltip;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, spacing, typography};

use super::super::event::AgentsIntent;
use super::super::model::NeedViewModel;
use super::super::need::NeedAction;

/// Least width the heading and the line of a need keep beside its label
/// and buttons. In less, the label and the buttons go under the text.
const TEXT_MIN_WIDTH: f32 = 320.0;
/// Gap between the text, the label and the buttons.
const SIDE_GAP: f32 = spacing::SPACE_4;
/// Gap between two buttons of a need.
const BUTTON_GAP: f32 = spacing::SPACE_3;

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
        list = list.push(need_row(need, on_fix, palette));
    }

    list.into()
}

/// One need, laid out for the width it gets: its label and buttons beside
/// its text when the text keeps [`TEXT_MIN_WIDTH`], else under it.
fn need_row<'a>(
    need: NeedViewModel,
    on_fix: fn(NeedAction) -> AgentsIntent,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let palette = palette.clone();

    responsive(move |size| {
        let plugin = need.plugin.as_deref();
        let (actions, actions_width) = actions(&need, on_fix, &palette);
        let text = text(&need, &palette);

        match fits_beside(size.width, side_width(plugin, actions_width)) {
            true => beside(text, plugin, actions, &palette),
            false => under(
                text,
                plugin,
                actions,
                actions_width,
                size.width,
                &palette,
            ),
        }
    })
    .height(Length::Shrink)
    .into()
}

/// The text, then the label and the buttons at its right, on its line.
fn beside<'a>(
    text: Element<'a, AgentsIntent>,
    plugin: Option<&str>,
    actions: Option<Element<'a, AgentsIntent>>,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let mut line = row![text].spacing(SIDE_GAP).align_y(Alignment::Center);

    if let Some(plugin) = plugin {
        line = line.push(label(plugin, 0.0, palette));
    }
    if let Some(actions) = actions {
        line = line.push(actions);
    }

    line.into()
}

/// The text, then the label and the buttons, `actions_width` wide, on
/// their own line under it, at the right. A label too long for that line,
/// `width` wide, is cut, and its tooltip names it whole.
fn under<'a>(
    text: Element<'a, AgentsIntent>,
    plugin: Option<&str>,
    actions: Option<Element<'a, AgentsIntent>>,
    actions_width: f32,
    width: f32,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let mut line = row![].spacing(SIDE_GAP).align_y(Alignment::Center);

    if let Some(plugin) = plugin {
        let reserved = match actions {
            Some(_) => actions_width + SIDE_GAP,
            None => 0.0,
        };
        let shown = label(plugin, reserved, palette);
        line = match label_width(plugin) + reserved > width {
            true => line.push(tooltip(shown, plugin.to_string(), palette)),
            false => line.push(shown),
        };
    }
    if let Some(actions) = actions {
        line = line.push(actions);
    }

    let line = container(line)
        .width(Length::Fill)
        .align_x(Horizontal::Right);
    column![text, line].spacing(spacing::SPACE_2).into()
}

/// The heading, the place and the title of the need, and its line under
/// it, wrapping in all the width they get.
fn text<'a>(
    need: &NeedViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let heading = i18n::filled(
        Key::TplAgentsNeedHeading,
        &[("place", &need.place), ("title", &need.title)],
    );

    column![
        small_text(heading, palette.text_strong()),
        small_text(need.line.clone(), palette.text_muted()),
    ]
    .spacing(spacing::SPACE_1)
    .width(Length::Fill)
    .into()
}

/// The label of the plugin that asks, on one line, cut with "…" to leave
/// `reserved` width free for the buttons after it.
fn label<'a>(
    plugin: &str,
    reserved: f32,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let small = typography::SMALL;

    EllipsisText::new(plugin, small.size(), fonts::UI, palette.text_muted())
        .shrink(reserved)
        .into()
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

/// Width of the label of `plugin` on one line.
fn label_width(plugin: &str) -> f32 {
    text_width(plugin, typography::SMALL.size(), fonts::UI)
}

/// Width the label of `plugin` and buttons `actions_width` wide take at
/// the right of the text, with the gaps before them; 0 when there is
/// neither.
fn side_width(plugin: Option<&str>, actions_width: f32) -> f32 {
    let label = plugin.map_or(0.0, |plugin| label_width(plugin) + SIDE_GAP);
    let actions = match actions_width > 0.0 {
        true => actions_width + SIDE_GAP,
        false => 0.0,
    };

    label + actions
}

/// Whether the text keeps [`TEXT_MIN_WIDTH`] of `width` beside a label
/// and buttons `side` wide, or there is nothing beside it.
fn fits_beside(width: f32, side: f32) -> bool {
    side == 0.0 || width - side >= TEXT_MIN_WIDTH
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

    /// A page wide enough for the text, the label and the button side by
    /// side.
    const WIDE: f32 = 900.0;
    /// A page too narrow for the text beside the label and the button.
    const NARROW: f32 = 360.0;
    /// A page narrower than the label and the button side by side.
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

    /// The needs view of `need` in a page `width` wide.
    fn needs_in(
        width: f32,
        need: NeedViewModel,
    ) -> Simulator<'static, AgentsIntent> {
        let themes = ThemeManager::new();
        let list =
            view(vec![need], AgentsIntent::FixNeed, themes.current().ui());

        simulator(container(list).width(width))
    }

    fn bounds(ui: &mut Simulator<'_, AgentsIntent>, text: &str) -> Rectangle {
        ui.find(text).expect("the text should show").bounds()
    }

    fn heading() -> String {
        i18n::filled(
            Key::TplAgentsNeedHeading,
            &[("place", PLACE), ("title", TITLE)],
        )
    }

    /// Whether `part` sits within the height of the text, from the top of
    /// `heading` to the bottom of `line`.
    fn beside(part: Rectangle, heading: Rectangle, line: Rectangle) -> bool {
        part.y < line.y + line.height && part.y + part.height > heading.y
    }

    /// Issue 21 of spec 2026-11-23-accounts-apart-from-profiles: in a wide
    /// page the label and the button sit at the right of the text, on its
    /// line, the label before the button.
    #[test]
    fn at_a_wide_width_the_label_and_the_button_sit_beside_the_text() {
        let mut ui = needs_in(WIDE, need_of(PLUGIN));

        let heading = bounds(&mut ui, &heading());
        let line = bounds(&mut ui, LINE);
        let label = bounds(&mut ui, PLUGIN);
        let button = bounds(&mut ui, BUTTON);

        let text_end = (heading.x + heading.width).max(line.x + line.width);
        assert!(label.x >= text_end, "{label:?} after the text {line:?}");
        assert!(beside(label, heading, line), "{label:?} beside {line:?}");
        assert!(beside(button, heading, line), "{button:?} beside {line:?}");
        assert!(label.x + label.width <= button.x, "{label:?} {button:?}");
        assert!(button.x + button.width <= WIDE, "inside: {button:?}");
    }

    /// Issue 21: in a narrow page the label and the button go to their own
    /// line under the text, whole, the label on one line and apart from
    /// the button.
    #[test]
    fn at_a_narrow_width_the_label_and_the_button_go_under_the_text() {
        let small = typography::SMALL;
        let mut ui = needs_in(NARROW, need_of(PLUGIN));

        let line = bounds(&mut ui, LINE);
        let label = bounds(&mut ui, PLUGIN);
        let button = bounds(&mut ui, BUTTON);

        assert!(label.y >= line.y + line.height, "{label:?} under {line:?}");
        assert!(
            button.y >= line.y + line.height,
            "{button:?} under {line:?}"
        );
        assert!(label.height <= small.line_height(), "one line: {label:?}");
        let whole = text_width(PLUGIN, small.size(), fonts::UI);
        assert!((label.width - whole).abs() < 1.0, "whole: {label:?}");
        assert!(label.x + label.width <= button.x, "{label:?} {button:?}");
        assert!(button.x + button.width <= NARROW, "inside: {button:?}");
    }

    /// Issue 21: in a page too narrow even for the label and the button,
    /// the label is cut on one line and the button stays whole.
    #[test]
    fn a_label_with_no_room_is_cut_on_one_line_and_the_button_stays_whole() {
        let small = typography::SMALL;
        let long = "A plugin with a rather long name plugin";
        let mut ui = needs_in(TINY, need_of(long));

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
}

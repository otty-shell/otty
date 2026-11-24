//! The page of a profile in the main area (spec 2026-11-19, Decision 6):
//! one column, at most as wide as a form and centred, read from top to
//! bottom. The header with the logo, the name, Refresh and "⋯"; "Needs
//! from you" as a to-do list when there is anything; a line with a plain
//! fact; the usage block; and "Profile info" with the plugin that reads
//! the profile. No account: nothing links a profile to an account (spec
//! 2026-11-23-accounts-apart-from-profiles, Decision 1). Each part is a
//! kit card, the needs a warning card (issue 24).

use iced::widget::{Column, column, container, row, svg};
use iced::{Alignment, Element, Length};
use otty_agents::Profile;
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::icons;
use otty_ui_kit::components::card::Card;
use otty_ui_kit::components::ellipsis_text::EllipsisText;
use otty_ui_kit::components::glyph;
use otty_ui_kit::components::heading_row::HeadingRow;
use otty_ui_kit::components::icon_button::IconButton;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, size, spacing, typography};

use super::super::event::AgentsIntent;
use super::super::model::ProfilePageViewModel;
use super::{blocks, entry_icon, needs, usage_block};

/// Edge of the agent logo before the profile name.
pub(super) const LOGO_SIZE: f32 = 26.0;
/// Gap between the parts of the header (`gap: 12px`).
pub(super) const HEADER_GAP: f32 = spacing::SPACE_4;
/// Gap between the buttons of the header.
pub(super) const BUTTON_GAP: f32 = spacing::SPACE_2;
/// Side padding of the page (`padding: 16px 20px`).
pub(super) const PAGE_PADDING_X: f32 = 20.0;
/// Gap between the header and the blocks (`gap: 20px`).
pub(super) const PAGE_GAP: f32 = 20.0;

/// Render the page of a profile: one column as wide as the room, at most a
/// form's width, centred across the main area and starting at its top.
pub(crate) fn view<'a>(
    vm: ProfilePageViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let header = header(&vm, palette);
    let blocks = page_blocks(vm, palette);

    let page = column![header, blocks]
        .spacing(PAGE_GAP)
        .width(Length::Fill)
        .max_width(size::FORM);

    container(page)
        .padding([spacing::SPACE_5, PAGE_PADDING_X])
        .center_x(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// The blocks under the header, in their order: the needs when there are
/// any, the info line and why the sessions could not be read, the usage
/// block and its facts.
fn page_blocks<'a>(
    vm: ProfilePageViewModel,
    palette: &UiPalette,
) -> Column<'a, AgentsIntent> {
    let mut body = Column::new().spacing(blocks::BLOCK_GAP);

    if !vm.needs.is_empty() {
        body = body.push(needs::card(
            i18n::t(Key::AgentsNeedsFromYou),
            vm.needs,
            AgentsIntent::FixNeed,
            palette,
        ));
    }
    if vm.info.is_some() || vm.failure.is_some() {
        body = body.push(blocks::notes(vm.info, vm.failure, palette));
    }
    if let Some(usage) = vm.usage {
        body = body.push(usage_block::card(usage, palette));
    }
    let facts = blocks::facts(vm.facts, palette);
    body.push(Card::new(facts, palette).title(i18n::t(Key::AgentsProfileInfo)))
}

/// The header: the logo and the name, then Refresh and "⋯" at the right
/// end, on the vertical centre of the name. The name ends with "…" when it
/// does not fit beside them.
fn header<'a>(
    vm: &ProfilePageViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let strong = palette.text_strong();
    let logo = entry_icon::view(vm.icon.clone(), LOGO_SIZE, strong, palette);
    let name = EllipsisText::new(
        vm.name.clone(),
        typography::TITLE.size(),
        fonts::UI_SEMIBOLD,
        strong,
    )
    .shrink(0.0);

    HeadingRow::new(name, buttons(&vm.profile, palette))
        .lead(logo)
        .spacing(HEADER_GAP)
        .into()
}

/// Refresh and "⋯", the menu of the profile.
fn buttons<'a>(
    profile: &Profile,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let refresh = IconButton::new(
        svg::Handle::from_memory(icons::REFRESH),
        i18n::t(Key::AgentsRefreshAll),
        palette,
    )
    .on_press(AgentsIntent::Refresh);
    let more =
        IconButton::new(glyph::dots(), i18n::t(Key::MoreActions), palette)
            .on_press(AgentsIntent::OpenProfileMenu(profile.clone()));

    row![refresh, more]
        .spacing(BUTTON_GAP)
        .align_y(Alignment::Center)
        .into()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use iced::{Point, Rectangle, Size};
    use iced_test::Simulator;
    use otty_agents::AgentId;
    use otty_app_theme::theme::ThemeManager;
    use otty_ui_kit::components::screen_area::ScreenArea;

    use super::*;
    use crate::model::{
        FactViewModel, NeedFixViewModel, NeedViewModel, UsageBlockViewModel,
    };
    use crate::need::NeedAction;
    use crate::testing::usage_block;
    use crate::usage::UsageTab;

    /// A wide main area: a window of 1280 or more.
    const WIDE: f32 = 1280.0;
    /// A main area narrower than the least width of a screen.
    const NARROW: f32 = 560.0;
    /// Height of the main area in the tests.
    const HEIGHT: f32 = 1600.0;
    /// The least height of a whole line of text of a table cell.
    const LINE_LEAST: f32 = 12.0;

    fn work() -> Profile {
        Profile::added(
            AgentId::new("codex"),
            "Codex (Work)",
            PathBuf::from("/home/user/.codex-work"),
        )
    }

    fn page() -> ProfilePageViewModel {
        ProfilePageViewModel {
            profile: work(),
            icon: crate::testing::agent_icon("codex"),
            name: String::from("Codex (Work)"),
            needs: vec![NeedViewModel {
                place: String::from("Codex (Work)"),
                plugin: None,
                title: String::from("needs login"),
                line: String::from("Log in to use it."),
                fix: Some(NeedFixViewModel {
                    label: i18n::t(Key::AgentsLogIn).to_string(),
                    action: Some(NeedAction::OpenPage(String::from(
                        "https://example.com/login",
                    ))),
                }),
                extra: None,
            }],
            info: Some(String::from("Codex has no sessions yet.")),
            failure: None,
            usage: Some(usage_block()),
            facts: vec![
                FactViewModel::new(
                    i18n::t(Key::AgentsAgent),
                    String::from("Codex"),
                ),
                FactViewModel::new(
                    i18n::t(Key::AgentsFieldFolder),
                    String::from("~/.codex-work"),
                ),
            ],
        }
    }

    /// The page in the main area's screen, in a main area `width` wide.
    fn in_main_area(
        vm: ProfilePageViewModel,
        width: f32,
        themes: &ThemeManager,
    ) -> Simulator<'_, AgentsIntent> {
        let palette = themes.current().ui();
        let screen = ScreenArea::new("profile", view(vm, palette), palette);

        Simulator::with_size(
            iced::Settings::default(),
            Size::new(width, HEIGHT),
            screen,
        )
    }

    /// Where the text `label` is.
    fn at(ui: &mut Simulator<'_, AgentsIntent>, label: &str) -> Rectangle {
        ui.find(label).expect("the text should show").bounds()
    }

    /// What a press on the text `label` sends.
    fn sent(vm: ProfilePageViewModel, label: &str) -> Vec<AgentsIntent> {
        let themes = ThemeManager::new();
        let mut ui = in_main_area(vm, WIDE, &themes);

        ui.click(label).expect("the text should show");

        ui.into_messages().collect()
    }

    /// What a press at `point` of `ui` sends.
    fn pressed_at(
        mut ui: Simulator<'_, AgentsIntent>,
        point: Point,
    ) -> Vec<AgentsIntent> {
        ui.point_at(point);
        let _ = ui.simulate(iced_test::simulator::click());

        ui.into_messages().collect()
    }

    #[test]
    fn the_page_reads_from_top_to_bottom_in_one_column() {
        for width in [WIDE, NARROW] {
            let themes = ThemeManager::new();
            let mut ui = in_main_area(page(), width, &themes);

            let parts = [
                at(&mut ui, "Codex (Work)"),
                at(&mut ui, i18n::t(Key::AgentsNeedsFromYou)),
                at(&mut ui, "Codex has no sessions yet."),
                at(&mut ui, i18n::t(Key::AgentsUsage)),
                at(&mut ui, "Nov 10"),
                at(&mut ui, "gpt-5.5-codex"),
                at(&mut ui, i18n::t(Key::AgentsProfileInfo)),
            ];

            for pair in parts.windows(2) {
                assert!(
                    pair[1].y >= pair[0].y + pair[0].height,
                    "at {width}: {:?} under {:?}",
                    pair[1],
                    pair[0]
                );
            }
        }
    }

    /// Spec Decision 6: the usage block must not overflow at a narrow
    /// main area; its period drop-down ends inside the page.
    #[test]
    fn in_a_narrow_main_area_the_drop_down_stays_inside_the_page() {
        let themes = ThemeManager::new();
        let mut ui = in_main_area(page(), NARROW, &themes);

        let period = at(&mut ui, i18n::t(Key::AgentsSevenDays));
        let usage = at(&mut ui, i18n::t(Key::AgentsUsage));
        let right_end = NARROW.max(size::SCREEN_MIN) - PAGE_PADDING_X;

        assert!(
            period.x + period.width <= right_end,
            "{period:?} ends inside {right_end}"
        );
        assert!(period.x > usage.x, "{period:?} at the right of {usage:?}");
    }

    /// Issue 14 of spec 2026-11-23, with issues 38 and 39 of spec
    /// 2026-11-19-agent-plugins: on the smallest screen the usage table,
    /// its Tokens and Money columns too, fits the page, so it never
    /// scrolls sideways inside a page that does.
    #[test]
    fn the_usage_table_fits_the_smallest_screen() {
        let themes = ThemeManager::new();
        let mut ui = in_main_area(page(), NARROW, &themes);

        let money = at(&mut ui, i18n::t(Key::AgentsNumberMoney));
        let share = at(&mut ui, i18n::t(Key::AgentsColumnShare));
        let right_end = size::SCREEN_MIN - PAGE_PADDING_X - spacing::SPACE_4;

        assert!(money.x + money.width < share.x, "{money:?} {share:?}");
        assert!(
            share.x + share.width <= right_end + 0.5,
            "the Share column ends at {}, inside {right_end}",
            share.x + share.width
        );
    }

    /// Issue 24 of spec 2026-11-23: the usage block is one card, its title
    /// inside its padding and its period drop-down at the right end of
    /// the title row, the table inside it too.
    #[test]
    fn the_usage_block_is_one_card_with_its_period_in_its_title_row() {
        let themes = ThemeManager::new();
        let mut ui = in_main_area(page(), WIDE, &themes);
        let left = (WIDE - size::FORM) / 2.0;
        let right = (WIDE + size::FORM) / 2.0;

        let usage = at(&mut ui, i18n::t(Key::AgentsUsage));
        let period = at(&mut ui, i18n::t(Key::AgentsSevenDays));
        let row = at(&mut ui, "gpt-5.4");
        let info = at(&mut ui, i18n::t(Key::AgentsProfileInfo));

        assert!(
            (usage.x - (left + spacing::SPACE_5)).abs() < 0.5,
            "{usage:?}"
        );
        assert!(
            (period.center_y() - usage.center_y()).abs() < spacing::SPACE_3,
            "{period:?} on the row of {usage:?}"
        );
        assert!(
            period.x + period.width <= right - spacing::SPACE_5,
            "{period:?} inside the card"
        );
        assert!(row.x >= left + spacing::SPACE_5, "{row:?} inside the card");
        assert!(
            row.y + row.height + spacing::SPACE_5 <= info.y,
            "{row:?} inside the card, over {info:?}"
        );
    }

    #[test]
    fn the_drop_down_tabs_and_rows_send_their_choices() {
        let period = sent(page(), i18n::t(Key::AgentsSevenDays));
        let tab = sent(page(), i18n::t(Key::AgentsTopByProject));
        let row = sent(page(), "gpt-5.4");

        assert_eq!(period, [AgentsIntent::OpenPeriodMenu]);
        assert_eq!(tab, [AgentsIntent::ShowUsageTab(UsageTab::ByProject)]);
        assert!(
            row.contains(&AgentsIntent::PickUsageRow(String::from("gpt-5.4"))),
            "{row:?}"
        );
    }

    #[test]
    fn the_pointer_over_a_row_lights_it_and_leaving_the_table_stops() {
        let themes = ThemeManager::new();
        let mut ui = in_main_area(page(), WIDE, &themes);
        let row = at(&mut ui, "gpt-5.4");

        for position in [
            Point::new(row.center_x(), row.center_y()),
            Point::new(row.center_x(), 1.0),
        ] {
            ui.point_at(position);
            let _ = ui.simulate([iced::Event::Mouse(
                iced::mouse::Event::CursorMoved { position },
            )]);
        }
        let sent: Vec<AgentsIntent> = ui.into_messages().collect();

        assert_eq!(
            sent.first(),
            Some(&AgentsIntent::HoverUsageRow(Some(String::from("gpt-5.4")))),
            "{sent:?}"
        );
        assert_eq!(sent.last(), Some(&AgentsIntent::HoverUsageRow(None)));
    }

    #[test]
    fn a_need_opens_its_fix() {
        let log_in = sent(page(), i18n::t(Key::AgentsLogIn));

        assert_eq!(
            log_in,
            [AgentsIntent::FixNeed(NeedAction::OpenPage(String::from(
                "https://example.com/login"
            )))]
        );
    }

    #[test]
    fn refresh_and_the_more_button_end_the_line_of_the_name() {
        let themes = ThemeManager::new();
        let mut ui = in_main_area(page(), WIDE, &themes);
        let name = at(&mut ui, "Codex (Work)");
        let right_end = (WIDE + size::FORM) / 2.0;
        let more =
            Point::new(right_end - size::CONTROL_MD / 2.0, name.center_y());
        let refresh = Point::new(
            right_end - size::CONTROL_MD - BUTTON_GAP - size::CONTROL_MD / 2.0,
            name.center_y(),
        );

        assert_eq!(
            pressed_at(ui, more),
            [AgentsIntent::OpenProfileMenu(work())]
        );
        assert_eq!(
            pressed_at(in_main_area(page(), WIDE, &themes), refresh),
            [AgentsIntent::Refresh]
        );
    }

    #[test]
    fn a_page_with_nothing_but_its_info_shows_no_empty_blocks() {
        let vm = ProfilePageViewModel {
            needs: Vec::new(),
            info: None,
            usage: None,
            ..page()
        };
        let themes = ThemeManager::new();
        let mut ui = in_main_area(vm, WIDE, &themes);

        ui.find(i18n::t(Key::AgentsProfileInfo))
            .expect("the info shows");
        for gone in [Key::AgentsNeedsFromYou, Key::AgentsUsage] {
            assert!(ui.find(i18n::t(gone)).is_err(), "{gone:?}");
        }
    }

    /// The room between the bottom of the usage table, the line under the
    /// row with the text `last` included, and the title of Profile info.
    fn room_under_the_usage_table(vm: ProfilePageViewModel, last: &str) -> f32 {
        let themes = ThemeManager::new();
        let mut ui = in_main_area(vm, WIDE, &themes);

        let last = at(&mut ui, last);
        let info = at(&mut ui, i18n::t(Key::AgentsProfileInfo));

        assert!(last.height >= LINE_LEAST, "the row is whole: {last:?}");
        info.y - (last.center_y() + size::TABLE_ROW / 2.0 + size::LINE)
    }

    /// Issue 19 of spec 2026-11-23: with no rows the usage table shows its
    /// whole empty row, and the room under it before the next block is
    /// the same as under a table with rows.
    #[test]
    fn an_empty_usage_table_keeps_the_room_under_it_as_one_with_rows() {
        let empty = ProfilePageViewModel {
            usage: Some(UsageBlockViewModel {
                rows: Vec::new(),
                ..usage_block()
            }),
            ..page()
        };

        let with_rows = room_under_the_usage_table(page(), "gpt-5.4");
        let without =
            room_under_the_usage_table(empty, i18n::t(Key::AgentsTopEmpty));

        assert!(with_rows >= blocks::BLOCK_GAP, "{with_rows}");
        assert!(
            (without - with_rows).abs() < 1.0,
            "under the empty row {without}, under the rows {with_rows}"
        );
    }
}

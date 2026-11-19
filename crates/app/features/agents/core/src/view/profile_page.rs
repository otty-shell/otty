//! The page of a profile in the main area (spec 2026-11-19, Decision 6):
//! one column, at most as wide as a form and centred, read from top to
//! bottom. The header with the logo, the name, Refresh and "⋯"; "Needs
//! from you" as a to-do list when there is anything; a line with a plain
//! fact; the usage block; "Pays through" as chips that open their account;
//! "Profile info" and where the numbers come from.

use iced::widget::{Column, column, container, row, svg};
use iced::{Alignment, Element, Length};
use otty_agents::Profile;
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::icons;
use otty_ui_kit::components::ellipsis_text::EllipsisText;
use otty_ui_kit::components::glyph;
use otty_ui_kit::components::heading_row::HeadingRow;
use otty_ui_kit::components::icon_button::IconButton;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, size, spacing, typography};

use super::super::event::AgentsIntent;
use super::super::model::ProfilePageViewModel;
use super::{blocks, chips, needs, sources, usage_block};

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
/// block, the accounts it pays through, its facts and the sources.
fn page_blocks<'a>(
    vm: ProfilePageViewModel,
    palette: &UiPalette,
) -> Column<'a, AgentsIntent> {
    let mut body = Column::new().spacing(blocks::BLOCK_GAP);

    if !vm.needs.is_empty() {
        body = body.push(blocks::titled(
            i18n::t(Key::AgentsNeedsFromYou),
            needs::view(vm.needs, AgentsIntent::FixNeed, palette),
            palette,
        ));
    }
    if vm.info.is_some() || vm.failure.is_some() {
        body = body.push(blocks::notes(vm.info, vm.failure, palette));
    }
    if let Some(usage) = vm.usage {
        body = body.push(usage_block::view(usage, palette));
    }
    if !vm.pays_through.is_empty() {
        body = body.push(blocks::titled(
            i18n::t(Key::AgentsPaysThrough),
            chips::account_chips(vm.pays_through, palette),
            palette,
        ));
    }
    body = body.push(blocks::titled(
        i18n::t(Key::AgentsProfileInfo),
        blocks::facts(vm.facts, palette),
        palette,
    ));
    if !vm.sources.is_empty() {
        body = body.push(sources::view(vm.sources, palette));
    }

    body
}

/// The header: the logo and the name, then Refresh and "⋯" at the right
/// end, on the vertical centre of the name. The name ends with "…" when it
/// does not fit beside them.
fn header<'a>(
    vm: &ProfilePageViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let strong = palette.text_strong();
    let logo = svg(svg::Handle::from_memory(icons::agent_icon(vm.icon)))
        .width(LOGO_SIZE)
        .height(LOGO_SIZE)
        .style(move |_, _| svg::Style {
            color: Some(strong),
        });
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
    use otty_agents::{
        Account, AccountIdentity, AccountKind, AccountLogin, AgentId, FoundIn,
        Provider,
    };
    use otty_app_theme::theme::ThemeManager;
    use otty_ui_kit::components::screen_area::ScreenArea;
    use otty_ui_kit::components::session_row::SessionTone;
    use otty_ui_kit::components::status_badge::StatusTone;

    use super::*;
    use crate::model::{
        FactViewModel, NeedFixViewModel, NeedViewModel, PaysThroughViewModel,
        SourceRowViewModel, UsageBlockViewModel,
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

    fn work() -> Profile {
        Profile::added(
            AgentId::new("codex"),
            "Codex · Work",
            PathBuf::from("/home/user/.codex-work"),
        )
    }

    fn chatgpt() -> Account {
        Account::new(
            Provider::OpenAI,
            AccountIdentity::AccountId(String::from("made-up-account")),
            AccountLogin::Email(String::from("you@example.com")),
            AccountKind::Limits,
            FoundIn::ProfileFolder(PathBuf::from("/home/user/.codex-work")),
        )
    }

    fn page() -> ProfilePageViewModel {
        ProfilePageViewModel {
            profile: work(),
            icon: "codex",
            name: String::from("Codex · Work"),
            needs: vec![NeedViewModel {
                place: String::from("Codex · Work"),
                title: String::from("needs login"),
                line: String::from("Log in to use it."),
                fix: Some(NeedFixViewModel {
                    label: i18n::t(Key::AgentsLogIn).to_string(),
                    action: NeedAction::LogIn(work()),
                }),
            }],
            info: Some(String::from("Codex has no sessions yet.")),
            failure: None,
            usage: Some(usage_block()),
            pays_through: vec![PaysThroughViewModel {
                account: chatgpt(),
                provider: "OpenAI",
                name: String::from("OpenAI · you@example.com"),
                number: Some((String::from("42 % used"), SessionTone::Muted)),
            }],
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
            sources: vec![SourceRowViewModel {
                source: otty_agents::SourceId::SessionFiles,
                number: Some(i18n::t(Key::AgentsTokens)),
                name: "Session files",
                name_tip: "Reads the session files",
                state: i18n::t(Key::AgentsSourceAnswered),
                tone: StatusTone::Ok,
                tip: None,
                updated: Some(String::from("Nov 16, 2026, 10:00")),
            }],
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
                at(&mut ui, "Codex · Work"),
                at(&mut ui, i18n::t(Key::AgentsNeedsFromYou)),
                at(&mut ui, "Codex has no sessions yet."),
                at(&mut ui, i18n::t(Key::AgentsUsage)),
                at(&mut ui, "Nov 10"),
                at(&mut ui, "gpt-5.5-codex"),
                at(&mut ui, i18n::t(Key::AgentsPaysThrough)),
                at(&mut ui, i18n::t(Key::AgentsProfileInfo)),
                at(&mut ui, i18n::t(Key::AgentsSourcesTitle)),
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
    /// main area; its drop-downs end inside the page.
    #[test]
    fn in_a_narrow_main_area_the_drop_downs_stay_inside_the_page() {
        let themes = ThemeManager::new();
        let mut ui = in_main_area(page(), NARROW, &themes);

        let measure = at(&mut ui, i18n::t(Key::AgentsTokens));
        let usage = at(&mut ui, i18n::t(Key::AgentsUsage));
        let right_end = NARROW.max(size::SCREEN_MIN) - PAGE_PADDING_X;

        assert!(
            measure.x + measure.width <= right_end,
            "{measure:?} ends inside {right_end}"
        );
        assert!(measure.x > usage.x, "{measure:?} at the right of {usage:?}");
    }

    #[test]
    fn the_drop_downs_tabs_and_rows_send_their_choices() {
        let period = sent(page(), i18n::t(Key::AgentsSevenDays));
        let measure = sent(page(), i18n::t(Key::AgentsTokens));
        let tab = sent(page(), i18n::t(Key::AgentsTopByProject));
        let row = sent(page(), "gpt-5.4");

        assert_eq!(period, [AgentsIntent::OpenPeriodMenu]);
        assert_eq!(measure, [AgentsIntent::OpenMeasureMenu]);
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
    fn by_account_off_says_why_under_the_tabs() {
        let vm = ProfilePageViewModel {
            usage: Some(UsageBlockViewModel {
                account_reason: Some(String::from("Kilo does not say")),
                ..usage_block()
            }),
            ..page()
        };
        let themes = ThemeManager::new();
        let mut ui = in_main_area(vm.clone(), WIDE, &themes);

        let line = i18n::filled(
            Key::TplAgentsByAccountOff,
            &[("reason", "Kilo does not say")],
        );
        ui.find(line).expect("the reason shows");
        assert_eq!(sent(vm, i18n::t(Key::AgentsByAccount)), []);
    }

    #[test]
    fn a_chip_of_pays_through_opens_its_account_and_a_need_its_fix() {
        let chip = sent(page(), "OpenAI · you@example.com");
        let log_in = sent(page(), i18n::t(Key::AgentsLogIn));

        assert_eq!(chip, [AgentsIntent::SelectAccount(chatgpt())]);
        assert_eq!(log_in, [AgentsIntent::FixNeed(NeedAction::LogIn(work()))]);
    }

    #[test]
    fn refresh_and_the_more_button_end_the_line_of_the_name() {
        let themes = ThemeManager::new();
        let mut ui = in_main_area(page(), WIDE, &themes);
        let name = at(&mut ui, "Codex · Work");
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
            pays_through: Vec::new(),
            sources: Vec::new(),
            ..page()
        };
        let themes = ThemeManager::new();
        let mut ui = in_main_area(vm, WIDE, &themes);

        ui.find(i18n::t(Key::AgentsProfileInfo))
            .expect("the info shows");
        for gone in [
            Key::AgentsNeedsFromYou,
            Key::AgentsUsage,
            Key::AgentsPaysThrough,
            Key::AgentsSourcesTitle,
        ] {
            assert!(ui.find(i18n::t(gone)).is_err(), "{gone:?}");
        }
    }
}

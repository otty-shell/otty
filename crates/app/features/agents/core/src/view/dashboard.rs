//! The dashboard of a profile in the main area (spec Decisions 6, 10, 12
//! and 13): one column, at most as wide as a form and centred, read from
//! top to bottom. The header with the logo, the name, Refresh and "⋯"; the
//! notes, with Log in while the profile is not logged in; the Usage block,
//! the tokens, the facts, the top section and where the numbers come from.

use iced::widget::{Column, column, container, row, svg};
use iced::{Alignment, Element, Length};
use otty_agents::Profile;
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::icons;
use otty_ui_kit::components::button::{Button, ButtonKind, ButtonSize};
use otty_ui_kit::components::ellipsis_text::EllipsisText;
use otty_ui_kit::components::heading_row::HeadingRow;
use otty_ui_kit::components::icon_button::IconButton;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, size, spacing, typography};

use super::super::event::AgentsIntent;
use super::super::model::{BillingViewModel, DashboardViewModel};
use super::{blocks, limits, money, profile_column, tokens};

/// Edge of the agent logo before the profile name.
const LOGO_SIZE: f32 = 26.0;
/// Gap between the parts of the header (`gap: 12px`).
const HEADER_GAP: f32 = spacing::SPACE_4;
/// Gap between the notes and Log in under them.
const NOTES_GAP: f32 = spacing::SPACE_3;
/// Gap between the buttons of the header.
const BUTTON_GAP: f32 = spacing::SPACE_2;
/// Side padding of the page (`padding: 16px 20px`).
const PAGE_PADDING_X: f32 = 20.0;
/// Gap between the header and the blocks (`gap: 20px`).
const PAGE_GAP: f32 = 20.0;

/// Render the dashboard: one column as wide as the room, at most a form's
/// width, centred across the main area and starting at its top.
pub(crate) fn view<'a>(
    vm: DashboardViewModel,
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

/// The blocks under the header, in their order: the notes with Log in
/// while the profile is not logged in, the Usage block, the tokens, then
/// the facts, the top section and the sources.
fn page_blocks<'a>(
    vm: DashboardViewModel,
    palette: &UiPalette,
) -> Column<'a, AgentsIntent> {
    let mut body = Column::new().spacing(blocks::BLOCK_GAP);
    if vm.hint.is_some() || vm.failure.is_some() {
        let hint = vm.hint.map(String::from);
        let mut notes = column![blocks::notes(hint, vm.failure, palette)]
            .spacing(NOTES_GAP);
        if vm.can_log_in {
            notes = notes.push(log_in(&vm.profile, palette));
        }
        body = body.push(notes);
    }
    match vm.billing {
        Some(BillingViewModel::Limits(vm)) => {
            body = body.push(limits::view(vm, palette));
        },
        Some(BillingViewModel::Money(vm)) => {
            body = body.push(money::view(vm, palette));
        },
        None => {},
    }
    if let Some(vm) = vm.tokens {
        body = body.push(tokens::view(vm, palette));
    }

    body.push(profile_column::view(vm.facts, vm.top, vm.sources, palette))
}

/// The header: the logo and the name, then Refresh and "⋯" at the right
/// end, on the vertical centre of the name. The name ends with "…" when it
/// does not fit beside them.
fn header<'a>(
    vm: &DashboardViewModel,
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

/// Refresh and "⋯", the menu with Edit, Log in and Remove.
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
    let more = IconButton::new(
        svg::Handle::from_memory(icons::MORE),
        i18n::t(Key::AgentsMoreActions),
        palette,
    )
    .on_press(AgentsIntent::OpenProfileMenu(profile.clone()));

    row![refresh, more]
        .spacing(BUTTON_GAP)
        .align_y(Alignment::Center)
        .into()
}

/// Log in under the hint of a profile that is not logged in, so the
/// login is one press away, as it is in "⋯".
fn log_in<'a>(
    profile: &Profile,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    Button::new(i18n::t(Key::AgentsLogIn), palette)
        .kind(ButtonKind::Primary)
        .size(ButtonSize::Small)
        .on_press(AgentsIntent::LogIn(profile.clone()))
        .into()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use iced::widget::container;
    use iced::{Point, Rectangle, Size};
    use iced_test::{Simulator, simulator};
    use otty_agents::{AgentId, Profile};
    use otty_app_i18n::{self as i18n, Key};
    use otty_app_theme::theme::ThemeManager;
    use otty_ui_kit::components::screen_area::ScreenArea;
    use otty_ui_kit::components::status_badge::StatusTone;
    use otty_ui_kit::size;

    use super::*;
    use crate::model::{
        BillingViewModel, DayBarViewModel, DayBarsViewModel, FactViewModel,
        LimitsViewModel, MoneyViewModel, SourceRowViewModel, TokensViewModel,
        TopProjectRowViewModel, TopRowsViewModel, TopViewModel, TotalViewModel,
        UsageRowViewModel,
    };
    use crate::top_tab::TopTab;

    /// Width of the main area in the tests.
    const WIDTH: f32 = 1000.0;
    /// A wide main area: a window of 1280 or more.
    const WIDE: f32 = 1280.0;
    /// A medium main area.
    const MEDIUM: f32 = 800.0;
    /// A main area narrower than the least width of a screen.
    const NARROW: f32 = 560.0;
    /// Height of the main area in the tests.
    const HEIGHT: f32 = 1200.0;
    /// A main area whose room, less the page padding, is narrower than a
    /// form.
    const UNDER_FORM: f32 = 680.0;
    /// The right end of the header: the right of the column, as wide as a
    /// form, in the middle of the width.
    const RIGHT_END: f32 = (WIDTH + size::FORM) / 2.0;

    fn work() -> Profile {
        Profile::added(
            AgentId::new("claude-code"),
            "Claude Code · Work",
            PathBuf::from("/home/user/.claude-work"),
        )
    }

    fn dashboard() -> DashboardViewModel {
        DashboardViewModel {
            profile: work(),
            icon: "claude-code",
            name: String::from("Claude Code · Work"),
            can_log_in: false,
            hint: None,
            failure: None,
            billing: Some(BillingViewModel::Limits(LimitsViewModel {
                updated: None,
                windows: vec![UsageRowViewModel {
                    name: String::from("5 hours"),
                    used: String::from("42 % used"),
                    percent: Some(42),
                    resets: None,
                }],
                credits: None,
                is_stale: false,
                recorded: None,
                failure: None,
                log_in_hint: None,
            })),
            tokens: Some(TokensViewModel {
                totals: vec![TotalViewModel {
                    label: i18n::t(Key::AgentsToday),
                    value: String::from("1.2M"),
                    note: String::from("1M in · 0 cached · 0.2M out"),
                }],
                days: None,
                is_stale: false,
                last_read: None,
                failure: None,
            }),
            facts: vec![
                FactViewModel::new(
                    i18n::t(Key::AgentsFieldAccount),
                    String::from("user@example.com"),
                ),
                FactViewModel::new(
                    i18n::t(Key::AgentsFieldPlan),
                    String::from("Max 20x"),
                ),
                FactViewModel::new(
                    i18n::t(Key::AgentsFieldConfigDir),
                    String::from("~/.claude-work"),
                ),
            ],
            top: None,
            sources: Vec::new(),
        }
    }

    fn screen<'a>(
        vm: DashboardViewModel,
        themes: &'a ThemeManager,
    ) -> Element<'a, AgentsIntent> {
        container(view(vm, themes.current().ui()))
            .width(WIDTH)
            .into()
    }

    /// What a press on the text `label` sends.
    fn sent(vm: DashboardViewModel, label: &str) -> Vec<AgentsIntent> {
        let themes = ThemeManager::new();
        let mut ui = simulator(screen(vm, &themes));

        ui.click(label).expect("the text should show");

        ui.into_messages().collect()
    }

    /// What a press at `x`, at the height of the profile name, sends.
    fn sent_at(
        vm: DashboardViewModel,
        x: impl Fn(&mut iced_test::Simulator<'_, AgentsIntent>) -> f32,
    ) -> Vec<AgentsIntent> {
        let themes = ThemeManager::new();
        let mut ui = simulator(screen(vm, &themes));
        let name = ui
            .find("Claude Code · Work")
            .expect("the name should show")
            .bounds();
        let x = x(&mut ui);

        ui.point_at(Point::new(x, name.center_y()));
        let _ = ui.simulate(iced_test::simulator::click());

        ui.into_messages().collect()
    }

    #[test]
    fn the_dashboard_shows_the_header_and_its_blocks() {
        let themes = ThemeManager::new();
        let mut ui = simulator(screen(dashboard(), &themes));

        for shown in [
            "Claude Code · Work",
            i18n::t(Key::AgentsUsage),
            "42 % used",
            i18n::t(Key::AgentsTokensTitle),
            "1.2M",
            i18n::t(Key::AgentsProfileTitle),
            "~/.claude-work",
        ] {
            ui.find(shown).expect("the dashboard part should show");
        }
        assert!(ui.find(i18n::t(Key::AgentsLogIn)).is_err());
        assert!(ui.find(i18n::t(Key::AgentsSourcesTitle)).is_err());
        assert!(ui.find(i18n::t(Key::AgentsEdit)).is_err(), "Edit is in ⋯");
    }

    #[test]
    fn a_profile_that_is_not_logged_in_has_log_in_under_its_hint() {
        let not_logged_in = DashboardViewModel {
            can_log_in: true,
            hint: Some(i18n::t(Key::AgentsNotLoggedInHint)),
            billing: None,
            ..dashboard()
        };
        let themes = ThemeManager::new();
        let mut ui = simulator(screen(not_logged_in.clone(), &themes));

        let name = ui.find("Claude Code · Work").expect("name").bounds();
        let hint = ui
            .find(i18n::t(Key::AgentsNotLoggedInHint))
            .expect("the hint should show")
            .bounds();
        let log_in = ui
            .find(i18n::t(Key::AgentsLogIn))
            .expect("Log in should show")
            .bounds();
        let logged_in = sent(not_logged_in, i18n::t(Key::AgentsLogIn));

        assert!(
            log_in.y > name.y + name.height,
            "Log in should be in the body, not the header: {log_in:?}"
        );
        assert!(log_in.y >= hint.y, "{log_in:?} with {hint:?}");
        assert_eq!(logged_in, vec![AgentsIntent::LogIn(work())]);
    }

    #[test]
    fn refresh_and_the_more_button_end_the_line_of_the_name() {
        let more = sent_at(dashboard(), |_| RIGHT_END - size::CONTROL_MD / 2.0);
        let refreshed = sent_at(dashboard(), |_| {
            RIGHT_END - size::CONTROL_MD - BUTTON_GAP - size::CONTROL_MD / 2.0
        });

        assert_eq!(more, vec![AgentsIntent::OpenProfileMenu(work())]);
        assert_eq!(refreshed, vec![AgentsIntent::Refresh]);
    }

    #[test]
    fn the_money_block_takes_the_place_of_the_limits_and_notes_show() {
        let vm = DashboardViewModel {
            billing: Some(BillingViewModel::Money(MoneyViewModel {
                updated: None,
                rows: vec![UsageRowViewModel {
                    name: i18n::t(Key::AgentsThisMonth).to_string(),
                    used: String::from("$184.75"),
                    percent: None,
                    resets: None,
                }],
                note: None,
                is_stale: false,
                failure: None,
            })),
            hint: Some(i18n::t(Key::AgentsProgramNotFoundHint)),
            failure: Some(String::from("Sessions could not be read: x")),
            ..dashboard()
        };
        let themes = ThemeManager::new();
        let mut ui = simulator(screen(vm, &themes));

        ui.find(i18n::t(Key::AgentsUsage)).expect("Usage shows");
        ui.find("$184.75").expect("the money shows");
        ui.find(i18n::t(Key::AgentsProgramNotFoundHint))
            .expect("the hint shows");
        ui.find("Sessions could not be read: x")
            .expect("the failure shows");
        assert!(ui.find("42 % used").is_err(), "no usage window shows");
    }

    /// Owner, issue 30: the Usage block of an API key lists its money as
    /// rows and has no bars per day; the only bars are the tokens'.
    #[test]
    fn the_usage_block_of_an_api_key_has_no_bars_per_day() {
        let mut state = crate::testing::with_profiles(&[(
            crate::testing::claude(),
            crate::testing::api_key(),
        )]);
        let _ = crate::reducer::reduce(
            &mut state,
            crate::testing::spend_loaded(
                &crate::testing::claude(),
                crate::types::SpendCount::Counted(crate::testing::spend_of(
                    2_000_000, 1_000_000,
                )),
                crate::testing::now(),
            ),
        );
        let _ = crate::reducer::reduce(
            &mut state,
            AgentsIntent::SelectProfile(crate::testing::claude()),
        );
        let shown = state.selected_profile().expect("a profile is selected");
        let vm = DashboardViewModel::from_profile(
            &state,
            shown,
            crate::testing::now(),
        );
        let themes = ThemeManager::new();
        let mut ui = in_main_area(vm, WIDE, &themes);

        let usage = at(&mut ui, i18n::t(Key::AgentsUsage));
        let tokens = at(&mut ui, i18n::t(Key::AgentsTokensTitle));
        let nov_11 = i18n::agents_template(
            Key::TplAgentsMonthDay,
            &[("month", i18n::agents_month(11)), ("day", "11")],
        );
        let first_day = at(&mut ui, &nov_11);

        assert!(usage.y < tokens.y, "{usage:?} above {tokens:?}");
        assert!(
            first_day.y > tokens.y,
            "the first bar day {first_day:?} belongs to the tokens {tokens:?}"
        );
    }

    /// The dashboard with every block: the limits, the tokens with their
    /// bars, the facts, the top section and the sources.
    fn full() -> DashboardViewModel {
        DashboardViewModel {
            tokens: Some(TokensViewModel {
                totals: vec![TotalViewModel {
                    label: i18n::t(Key::AgentsLastThirtyDays),
                    value: String::from("31.6M"),
                    note: String::from("24.1M in · 0 cached · 7.5M out"),
                }],
                days: Some(DayBarsViewModel {
                    bars: vec![DayBarViewModel {
                        value: 1,
                        tip: String::from("2026-11-16: 1 tokens"),
                    }],
                    first_day: String::from("2026-10-18"),
                }),
                is_stale: false,
                last_read: None,
                failure: None,
            }),
            top: Some(TopViewModel {
                tab: TopTab::ByProject,
                rows: TopRowsViewModel::Projects(vec![
                    TopProjectRowViewModel {
                        name: String::from("otty"),
                        sessions: String::from("9"),
                        tokens: String::from("18.4M"),
                    },
                ]),
            }),
            sources: vec![SourceRowViewModel {
                source: otty_agents::SourceId::AnthropicUsageApi,
                number: Some(i18n::t(Key::AgentsNumberLimits)),
                name: "Anthropic usage API (direct request)",
                name_tip: "Asks anthropic.com directly",
                state: i18n::t(Key::AgentsSourceLoginExpired),
                tone: StatusTone::Warn,
                tip: None,
                updated: Some(String::from("Nov 16, 2026, 10:00")),
            }],
            ..dashboard()
        }
    }

    /// The dashboard in the main area's screen, in a main area `width`
    /// wide.
    fn in_main_area(
        vm: DashboardViewModel,
        width: f32,
        themes: &ThemeManager,
    ) -> Simulator<'_, AgentsIntent> {
        let palette = themes.current().ui();
        let screen = ScreenArea::new("dashboard", view(vm, palette), palette);

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

    /// Where the titles of the Usage block, the tokens, the profile, the
    /// top section and the sources are, in a main area `width` wide.
    fn block_titles(width: f32) -> [Rectangle; 5] {
        let themes = ThemeManager::new();
        let mut ui = in_main_area(full(), width, &themes);

        [
            Key::AgentsUsage,
            Key::AgentsTokensTitle,
            Key::AgentsProfileTitle,
            Key::AgentsTopTitle,
            Key::AgentsSourcesTitle,
        ]
        .map(|key| at(&mut ui, i18n::t(key)))
    }

    /// Where the profile name and the titles of the blocks are, in a main
    /// area `width` wide: the header, then the blocks in their order.
    fn dashboard_parts(width: f32) -> [Rectangle; 6] {
        let [usage, tokens, profile, top, sources] = block_titles(width);
        let themes = ThemeManager::new();
        let mut ui = in_main_area(full(), width, &themes);
        let name = at(&mut ui, "Claude Code · Work");

        [name, usage, tokens, profile, top, sources]
    }

    /// Owner, issue 31: one column at every width, read from top to
    /// bottom.
    #[test]
    fn at_every_width_the_dashboard_is_one_column_read_from_top_to_bottom() {
        for width in [WIDE, MEDIUM, NARROW] {
            let parts = dashboard_parts(width);

            for pair in parts.windows(2) {
                assert!(
                    pair[1].y >= pair[0].y + pair[0].height,
                    "at {width}: {:?} under {:?}",
                    pair[1],
                    pair[0]
                );
            }
            let start = parts[1].x;
            for title in &parts[1..] {
                assert!(
                    (title.x - start).abs() < 1.0,
                    "at {width}: {title:?} starts where the column starts"
                );
            }
        }
    }

    #[test]
    fn in_a_wide_main_area_the_column_is_centred_and_as_wide_as_a_form() {
        let themes = ThemeManager::new();
        let mut ui = in_main_area(full(), WIDE, &themes);

        let usage = at(&mut ui, i18n::t(Key::AgentsUsage));
        let first = at(&mut ui, "2026-10-18");
        let today = at(&mut ui, i18n::t(Key::AgentsToday));
        let left = (WIDE - size::FORM) / 2.0;
        let right = (WIDE + size::FORM) / 2.0;

        assert!((usage.x - left).abs() < 1.0, "{usage:?} starts at {left}");
        assert!((first.x - left).abs() < 1.0, "{first:?} starts at {left}");
        assert!(
            (today.x + today.width - right).abs() < 1.0,
            "{today:?} should end at {right}, the right of the column"
        );
    }

    #[test]
    fn in_a_main_area_narrower_than_a_form_the_day_bars_take_the_whole_width() {
        let themes = ThemeManager::new();
        let mut ui = in_main_area(full(), UNDER_FORM, &themes);

        let first = at(&mut ui, "2026-10-18");
        let today = at(&mut ui, i18n::t(Key::AgentsToday));

        assert!((first.x - PAGE_PADDING_X).abs() < 1.0, "{first:?}");
        assert!(
            (today.x + today.width - (UNDER_FORM - PAGE_PADDING_X)).abs() < 1.0,
            "{today:?} should end at the right of the page"
        );
    }

    #[test]
    fn the_dashboard_starts_at_the_top_of_a_tall_main_area() {
        let themes = ThemeManager::new();
        let mut ui = in_main_area(dashboard(), WIDE, &themes);

        let name = at(&mut ui, "Claude Code · Work");

        assert!(
            name.y < spacing::SPACE_5 + LOGO_SIZE,
            "{name:?} should be at the top"
        );
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

    /// Owner, issue 33: the header has the logo, the name and the buttons
    /// only; the plan and the account are in the Profile list alone.
    #[test]
    fn the_header_has_no_plan_and_no_account_and_the_profile_list_has_both() {
        let themes = ThemeManager::new();
        let mut ui = in_main_area(full(), WIDE, &themes);

        let name = at(&mut ui, "Claude Code · Work");
        let profile = at(&mut ui, i18n::t(Key::AgentsProfileTitle));
        let account = at(&mut ui, "user@example.com");
        let plan = at(&mut ui, "Max 20x");
        let more = Point::new(
            (WIDE + size::FORM) / 2.0 - size::CONTROL_MD / 2.0,
            name.center_y(),
        );

        assert!(account.y > profile.y, "{account:?} in the Profile list");
        assert!(plan.y > profile.y, "{plan:?} in the Profile list");
        assert!(
            ui.find("Max 20x · user@example.com").is_err(),
            "no plan and account after the name"
        );
        assert_eq!(
            pressed_at(ui, more),
            vec![AgentsIntent::OpenProfileMenu(work())]
        );
    }

    #[test]
    fn in_the_smallest_main_area_the_buttons_stay_on_the_line_of_the_name() {
        let themes = ThemeManager::new();
        let mut ui = in_main_area(full(), NARROW, &themes);

        let name = at(&mut ui, "Claude Code · Work");
        let source = at(&mut ui, "Nov 16, 2026, 10:00");
        let right_end = NARROW.max(size::SCREEN_MIN) - PAGE_PADDING_X;
        let more =
            Point::new(right_end - size::CONTROL_MD / 2.0, name.center_y());
        let refresh = Point::new(
            right_end - size::CONTROL_MD - BUTTON_GAP - size::CONTROL_MD / 2.0,
            name.center_y(),
        );

        assert!(
            source.x + source.width > NARROW - spacing::SPACE_4,
            "the screen keeps its least width and scrolls sideways: the \
             last cell of the sources table ends at the right end of that \
             width, inside its padding: {source:?}"
        );
        assert_eq!(
            pressed_at(ui, more),
            vec![AgentsIntent::OpenProfileMenu(work())]
        );
        assert_eq!(
            pressed_at(in_main_area(full(), NARROW, &themes), refresh),
            vec![AgentsIntent::Refresh]
        );
    }
}

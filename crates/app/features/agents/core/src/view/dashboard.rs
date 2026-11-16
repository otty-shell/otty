//! The dashboard of a profile in the main area (spec Decisions 6 and 10):
//! the header with the logo, the name, the plan and the account, Refresh
//! and "⋯"; the notes, with Log in while the profile is not logged in, the
//! billing block and the tokens on the left; the facts, the top table and
//! where the numbers come from on the right. In a main area narrower than
//! 900 the right column goes under the left one.

use iced::widget::{Column, column, responsive, row, svg};
use iced::{Alignment, Element, Length};
use otty_agents::Profile;
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::icons;
use otty_ui_kit::components::button::{Button, ButtonKind, ButtonSize};
use otty_ui_kit::components::ellipsis_text::EllipsisText;
use otty_ui_kit::components::heading_row::HeadingRow;
use otty_ui_kit::components::icon_button::IconButton;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, spacing, typography};

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
/// Gap between the header and the columns (`gap: 20px`).
const PAGE_GAP: f32 = 20.0;
/// Gap between the two columns (`gap: 32px`).
const COLUMN_GAP: f32 = 32.0;
/// Gap between the blocks of a column (`gap: 28px`).
const BLOCK_GAP: f32 = 28.0;
/// The share of the width of the left column, against the right one's 2.
const LEFT_SHARE: u16 = 3;
/// The share of the width of the right column.
const RIGHT_SHARE: u16 = 2;
/// The room for the columns, the main area less the page padding, from
/// which the dashboard has two columns: a main area of 900. There the right
/// column gets (860 - 32) × 2 / 5 = 331, room for the minimum columns of
/// the By model table (120 + 3 × 64 = 312) and a little more, so in two
/// columns that table never scrolls sideways; in less room the blocks go
/// in one column of the whole width.
const TWO_COLUMNS_FROM: f32 = 860.0;

/// Render the dashboard. The blocks go in two columns from
/// [`TWO_COLUMNS_FROM`] and in one column below it.
pub(crate) fn view<'a>(
    vm: DashboardViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let header = header(&vm, palette);
    let palette = palette.clone();

    // The width is known only during layout; the blocks are built again
    // from the view model for each width.
    let columns = responsive(move |room| {
        let (left, right) = column_blocks(vm.clone(), &palette);
        if room.width >= TWO_COLUMNS_FROM {
            return row![
                left.width(Length::FillPortion(LEFT_SHARE)),
                right.width(Length::FillPortion(RIGHT_SHARE)),
            ]
            .spacing(COLUMN_GAP)
            .into();
        }

        column![left, right].spacing(BLOCK_GAP).into()
    })
    .height(Length::Shrink);

    column![header, columns]
        .spacing(PAGE_GAP)
        .padding([spacing::SPACE_5, PAGE_PADDING_X])
        .width(Length::Fill)
        .into()
}

/// The blocks of the two columns. The left: the notes with Log in while the
/// profile is not logged in, the billing block and the tokens; the right:
/// the facts, the top table and the sources. In one column the right ones
/// go under the left ones.
fn column_blocks<'a>(
    vm: DashboardViewModel,
    palette: &UiPalette,
) -> (Column<'a, AgentsIntent>, Column<'a, AgentsIntent>) {
    let mut left = Column::new().spacing(BLOCK_GAP);
    if vm.hint.is_some() || vm.failure.is_some() {
        let hint = vm.hint.map(String::from);
        let mut notes = column![blocks::notes(hint, vm.failure, palette)]
            .spacing(NOTES_GAP);
        if vm.can_log_in {
            notes = notes.push(log_in(&vm.profile, palette));
        }
        left = left.push(notes);
    }
    match vm.billing {
        Some(BillingViewModel::Limits(vm)) => {
            left = left.push(limits::view(vm, palette));
        },
        Some(BillingViewModel::Money(vm)) => {
            left = left.push(money::view(vm, palette));
        },
        None => {},
    }
    if let Some(vm) = vm.tokens {
        left = left.push(tokens::view(vm, palette));
    }

    let right = profile_column::view(vm.facts, vm.top, vm.sources, palette);

    (left, column![right])
}

/// The header: the logo and the name, the plan and the account muted
/// after the name, then Refresh and "⋯" at the right end. When the line is
/// too short, the plan and the account move under the name and the
/// buttons stay; the name ends with "…" only when it alone does not fit
/// beside them.
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

    let mut heading = HeadingRow::new(name, buttons(&vm.profile, palette))
        .spacing(HEADER_GAP);
    if let Some(subtitle) = &vm.subtitle {
        heading = heading.note(
            EllipsisText::new(
                subtitle.clone(),
                typography::BODY.size(),
                fonts::UI,
                palette.text_muted(),
            )
            .shrink(0.0),
        );
    }

    row![logo, heading]
        .spacing(HEADER_GAP)
        .align_y(Alignment::Center)
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
        BillingViewModel, DayBarViewModel, DayBarsViewModel, LimitsViewModel,
        MoneyViewModel, SourceRowViewModel, TokensViewModel,
        TopProjectRowViewModel, TopTableViewModel, TotalViewModel,
        WindowViewModel,
    };

    /// Width of the main area in the tests.
    const WIDTH: f32 = 1000.0;
    /// A wide main area: a window of 1280 or more.
    const WIDE: f32 = 1280.0;
    /// A medium main area.
    const MEDIUM: f32 = 800.0;
    /// A main area narrower than the least width of a screen.
    const NARROW: f32 = 560.0;
    /// Height of the main area in the tests.
    const HEIGHT: f32 = 900.0;
    /// The right end of the header: the width less the page padding.
    const RIGHT_END: f32 = WIDTH - PAGE_PADDING_X;

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
            subtitle: Some(String::from("Max · user@example.com")),
            can_log_in: false,
            hint: None,
            failure: None,
            billing: Some(BillingViewModel::Limits(LimitsViewModel {
                from: None,
                windows: vec![WindowViewModel {
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
            facts: vec![(
                i18n::t(Key::AgentsFieldConfigDir),
                String::from("~/.claude-work"),
            )],
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
            "Max · user@example.com",
            i18n::t(Key::AgentsLimits),
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
                note: None,
                totals: vec![TotalViewModel {
                    label: i18n::t(Key::AgentsThisMonth),
                    value: String::from("$184.75"),
                    note: String::from("30.2M tokens"),
                }],
                days: None,
                budget: None,
                unpriced: None,
                is_stale: false,
                last_read: None,
                failure: None,
            })),
            hint: Some(i18n::t(Key::AgentsProgramNotFoundHint)),
            failure: Some(String::from("Sessions could not be read: x")),
            ..dashboard()
        };
        let themes = ThemeManager::new();
        let mut ui = simulator(screen(vm, &themes));

        ui.find(i18n::t(Key::AgentsSpending))
            .expect("Spending shows");
        ui.find("$184.75").expect("the money shows");
        ui.find(i18n::t(Key::AgentsProgramNotFoundHint))
            .expect("the hint shows");
        ui.find("Sessions could not be read: x")
            .expect("the failure shows");
        assert!(ui.find(i18n::t(Key::AgentsLimits)).is_err());
    }

    /// The dashboard with every block: the limits, the tokens with their
    /// bars, the facts, the top projects and the sources.
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
            top: Some(TopTableViewModel::Projects(vec![
                TopProjectRowViewModel {
                    name: String::from("otty"),
                    sessions: String::from("9"),
                    tokens: String::from("18.4M"),
                },
            ])),
            sources: vec![SourceRowViewModel {
                name: String::from("usage request"),
                state: i18n::t(Key::AgentsSourceLoginExpired),
                tone: StatusTone::Warn,
                reason: None,
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

    /// Where the titles of the limits, the tokens, the profile, the top
    /// projects and the sources are, in a main area `width` wide.
    fn block_titles(width: f32) -> [Rectangle; 5] {
        let themes = ThemeManager::new();
        let mut ui = in_main_area(full(), width, &themes);

        [
            Key::AgentsLimits,
            Key::AgentsTokensTitle,
            Key::AgentsProfileTitle,
            Key::AgentsTopProjects,
            Key::AgentsSourcesTitle,
        ]
        .map(|key| at(&mut ui, i18n::t(key)))
    }

    /// Whether the profile column is beside the billing block.
    fn has_two_columns(width: f32) -> bool {
        let [limits, _, profile, _, _] = block_titles(width);

        profile.x > limits.x + 1.0
    }

    #[test]
    fn in_a_wide_main_area_the_profile_column_is_beside_the_billing_block() {
        let [limits, tokens, profile, top, sources] = block_titles(WIDE);

        assert!(profile.x > limits.x + 1.0, "{profile:?} beside {limits:?}");
        assert!(
            (profile.y - limits.y).abs() < 1.0,
            "both columns start at the top: {profile:?} and {limits:?}"
        );
        assert!(tokens.y > limits.y, "{tokens:?} under {limits:?}");
        assert!(top.y > profile.y && sources.y > top.y);
    }

    #[test]
    fn in_a_medium_main_area_the_blocks_go_in_one_column_in_their_order() {
        let titles = block_titles(MEDIUM);

        for pair in titles.windows(2) {
            assert!(pair[1].y > pair[0].y, "{:?} under {:?}", pair[1], pair[0]);
        }
        for title in &titles {
            assert!(
                (title.x - PAGE_PADDING_X).abs() < 1.0,
                "{title:?} starts the line"
            );
        }
    }

    #[test]
    fn the_columns_split_at_a_main_area_of_the_break_and_its_padding() {
        let break_width = TWO_COLUMNS_FROM + 2.0 * PAGE_PADDING_X;

        assert!(has_two_columns(break_width));
        assert!(!has_two_columns(break_width - 1.0));
    }

    #[test]
    fn in_one_column_the_day_bars_take_the_whole_width() {
        let themes = ThemeManager::new();
        let mut ui = in_main_area(full(), MEDIUM, &themes);

        let first = at(&mut ui, "2026-10-18");
        let today = at(&mut ui, i18n::t(Key::AgentsToday));

        assert!((first.x - PAGE_PADDING_X).abs() < 1.0, "{first:?}");
        assert!(
            (today.x + today.width - (MEDIUM - PAGE_PADDING_X)).abs() < 1.0,
            "{today:?} should end at the right of the page"
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

    #[test]
    fn in_a_wide_main_area_the_plan_follows_the_name_and_the_buttons_end_the_line()
     {
        let themes = ThemeManager::new();
        let mut ui = in_main_area(full(), WIDE, &themes);

        let name = at(&mut ui, "Claude Code · Work");
        let plan = at(&mut ui, "Max · user@example.com");
        let more = Point::new(
            WIDE - PAGE_PADDING_X - size::CONTROL_MD / 2.0,
            name.center_y(),
        );

        assert!(
            (plan.center_y() - name.center_y()).abs() < 2.0,
            "{plan:?} on the line of {name:?}"
        );
        assert!(plan.x > name.x + name.width, "{plan:?} after {name:?}");
        assert_eq!(
            pressed_at(ui, more),
            vec![AgentsIntent::OpenProfileMenu(work())]
        );
    }

    /// The dashboard with a plan and an account too long to follow the
    /// name in the smallest main area.
    fn with_a_long_plan() -> DashboardViewModel {
        DashboardViewModel {
            subtitle: Some(String::from(LONG_PLAN)),
            ..full()
        }
    }

    /// A plan and an account too long for the line of the name.
    const LONG_PLAN: &str = "Max 20x · someone.with.a.long.name@example.com";

    #[test]
    fn in_the_smallest_main_area_the_plan_moves_under_the_name_and_the_buttons_stay()
     {
        let themes = ThemeManager::new();
        let mut ui = in_main_area(with_a_long_plan(), NARROW, &themes);

        let name = at(&mut ui, "Claude Code · Work");
        let plan = at(&mut ui, LONG_PLAN);
        let source = at(&mut ui, i18n::t(Key::AgentsSourceLoginExpired));
        let right_end = NARROW.max(size::SCREEN_MIN) - PAGE_PADDING_X;
        let more =
            Point::new(right_end - size::CONTROL_MD / 2.0, name.center_y());
        let refresh = Point::new(
            right_end - size::CONTROL_MD - BUTTON_GAP - size::CONTROL_MD / 2.0,
            name.center_y(),
        );

        assert!(
            plan.y >= name.y + name.height,
            "the plan should be under the name: {plan:?} under {name:?}"
        );
        assert!((plan.x - name.x).abs() < 1.0, "{plan:?} under {name:?}");
        assert!(
            source.x + source.width > NARROW,
            "the screen keeps its least width and scrolls sideways: \
             {source:?}"
        );
        assert_eq!(
            pressed_at(ui, more),
            vec![AgentsIntent::OpenProfileMenu(work())]
        );
        assert_eq!(
            pressed_at(
                in_main_area(with_a_long_plan(), NARROW, &themes),
                refresh
            ),
            vec![AgentsIntent::Refresh]
        );
    }
}

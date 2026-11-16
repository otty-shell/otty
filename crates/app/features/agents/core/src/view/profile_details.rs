//! The details of the selected profile in the main area: its agent, name,
//! state, account and plan, config folder and program, its limits, what is
//! missing, and Log in, Edit, Hide and Remove.

use iced::widget::{Column, column, row, svg, text};
use iced::{Alignment, Color, Element, Length};
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::icons;
use otty_ui_kit::components::button::{Button, ButtonKind};
use otty_ui_kit::components::divider;
use otty_ui_kit::components::session_row::SessionTone;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, spacing, typography};

use super::super::event::AgentsIntent;
use super::super::model::ProfileDetailsViewModel;
use super::fact;
use super::limits::details_limits;

/// Edge of the agent icon before the profile name.
const ICON_SIZE: f32 = 24.0;

/// Render the details of a profile.
pub(crate) fn view<'a>(
    vm: ProfileDetailsViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let muted = palette.text_muted();
    let icon = svg(svg::Handle::from_memory(icons::agent_icon(vm.icon)))
        .width(ICON_SIZE)
        .height(ICON_SIZE)
        .style(move |_, _| svg::Style { color: Some(muted) });
    let title = typography::TITLE;
    let header = row![
        icon,
        text(vm.name)
            .size(title.size())
            .line_height(iced::Pixels(title.line_height()))
            .font(fonts::UI_SEMIBOLD)
            .color(palette.text_strong())
            .width(Length::Fill),
        buttons(&vm.profile, vm.can_log_in, vm.can_remove, palette),
    ]
    .spacing(spacing::SPACE_3)
    .align_y(Alignment::Center);

    let state_color = match vm.tone {
        SessionTone::Failed => palette.status_error(),
        SessionTone::Waiting => palette.status_warn(),
        _ => palette.text_body(),
    };
    let mut facts = Column::new()
        .spacing(spacing::SPACE_2)
        .push(fact(
            Key::AgentsFieldAgent,
            vm.agent.to_string(),
            palette.text_body(),
            palette,
        ))
        .push(fact(Key::AgentsFieldState, vm.state, state_color, palette));
    if let Some(account) = vm.account {
        facts = facts.push(fact(
            Key::AgentsFieldAccount,
            account,
            palette.text_body(),
            palette,
        ));
    }
    if let Some(plan) = vm.plan {
        facts = facts.push(fact(
            Key::AgentsFieldPlan,
            plan,
            palette.text_body(),
            palette,
        ));
    }
    facts = facts.push(fact(
        Key::AgentsFieldConfigDir,
        vm.config_dir,
        palette.text_body(),
        palette,
    ));
    if let Some(program) = vm.program {
        facts = facts.push(fact(
            Key::AgentsFieldProgram,
            program,
            palette.text_body(),
            palette,
        ));
    }

    let mut page = column![header, divider::horizontal(palette), facts]
        .spacing(spacing::SPACE_4);
    if let Some(limits) = vm.limits {
        page = page.push(details_limits(limits, palette));
    }
    if let Some(hint) = vm.hint {
        page = page.push(small_text(hint, palette.text_muted()));
    }
    if let Some(failure) = vm.failure {
        page = page.push(small_text(failure, palette.status_error()));
    }

    page.width(Length::Fill).into()
}

/// Log in when the profile is not logged in, Edit, Hide, and Remove for an
/// added profile.
fn buttons<'a>(
    profile: &otty_agents::Profile,
    can_log_in: bool,
    can_remove: bool,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let mut buttons = row![].spacing(spacing::SPACE_2);
    if can_log_in {
        buttons = buttons.push(
            Button::new(i18n::t(Key::AgentsLogIn), palette)
                .kind(ButtonKind::Primary)
                .on_press(AgentsIntent::LogIn(profile.clone())),
        );
    }
    buttons = buttons
        .push(
            Button::new(i18n::t(Key::AgentsEdit), palette)
                .kind(ButtonKind::Secondary)
                .on_press(AgentsIntent::EditProfile(profile.clone())),
        )
        .push(
            Button::new(i18n::t(Key::AgentsHide), palette)
                .kind(ButtonKind::Secondary)
                .on_press(AgentsIntent::HideProfile(profile.clone())),
        );
    if can_remove {
        buttons = buttons.push(
            Button::new(i18n::t(Key::ButtonRemove), palette)
                .kind(ButtonKind::Danger)
                .on_press(AgentsIntent::RemoveProfile(profile.clone())),
        );
    }

    buttons.into()
}

fn small_text<'a>(
    content: impl text::IntoFragment<'a>,
    color: Color,
) -> Element<'a, AgentsIntent> {
    let small = typography::SMALL;

    text(content)
        .size(small.size())
        .line_height(iced::Pixels(small.line_height()))
        .font(fonts::UI)
        .color(color)
        .into()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use iced_test::simulator;
    use otty_agents::{AgentId, Profile};
    use otty_app_theme::theme::ThemeManager;

    use super::*;
    use crate::model::{LimitBarViewModel, LimitsViewModel};

    fn work() -> Profile {
        Profile::added(
            AgentId::new("claude-code"),
            "Claude Code · Work",
            PathBuf::from("/home/user/.claude-work"),
        )
    }

    fn details() -> ProfileDetailsViewModel {
        ProfileDetailsViewModel {
            profile: work(),
            icon: "claude-code",
            name: String::from("Claude Code · Work"),
            agent: "Claude Code",
            state: String::from("not logged in"),
            tone: SessionTone::Waiting,
            hint: Some(i18n::t(Key::AgentsNotLoggedInHint)),
            account: Some(String::from("user@example.com")),
            plan: Some(String::from("Max")),
            config_dir: String::from("/home/user/.claude-work"),
            program: Some(String::from("/opt/claude")),
            failure: Some(String::from("files: no folder")),
            limits: Some(LimitsViewModel {
                bars: vec![
                    LimitBarViewModel {
                        name: String::from("5 hours"),
                        percent: 42,
                        resets: Some(String::from("resets in 3 h")),
                    },
                    LimitBarViewModel {
                        name: String::from("7 days"),
                        percent: 17,
                        resets: None,
                    },
                ],
                credits: Some(String::from("12.5")),
                source: Some(String::from("usage request")),
                earlier_failures: Some(String::from("Earlier: x")),
                is_stale: true,
                last_read: Some(String::from("Last read 10 min ago")),
                recorded: Some(String::from("Recorded in a session 1 h ago")),
                failure: Some(String::from("Could not read again: y")),
                log_in_hint: Some(i18n::t(Key::AgentsLimitsLogInAgain)),
            }),
            can_log_in: true,
            can_remove: true,
        }
    }

    fn pressed(vm: ProfileDetailsViewModel, label: &str) -> Vec<AgentsIntent> {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(vm, themes.current().ui()));

        ui.click(label).expect("the button should show");

        ui.into_messages().collect()
    }

    #[test]
    fn the_details_show_the_profile() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(details(), themes.current().ui()));

        for shown in [
            "Claude Code · Work",
            "Claude Code",
            "not logged in",
            "user@example.com",
            "Max",
            "/home/user/.claude-work",
            "/opt/claude",
            "files: no folder",
            i18n::t(Key::AgentsNotLoggedInHint),
        ] {
            ui.find(shown).expect("the detail should show");
        }
    }

    #[test]
    fn the_details_show_each_limit_and_where_it_came_from() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(details(), themes.current().ui()));

        for shown in [
            i18n::t(Key::AgentsLimits),
            "5 hours",
            "42%",
            "resets in 3 h",
            "7 days",
            "17%",
            "12.5",
            "usage request",
            "Earlier: x",
            "Last read 10 min ago",
            "Recorded in a session 1 h ago",
            "Could not read again: y",
            i18n::t(Key::AgentsLimitsLogInAgain),
        ] {
            ui.find(shown).expect("the limit detail should show");
        }
    }

    #[test]
    fn a_profile_with_no_limits_has_no_limits_block() {
        let vm = ProfileDetailsViewModel {
            limits: None,
            ..details()
        };
        let themes = ThemeManager::new();
        let mut ui = simulator(view(vm, themes.current().ui()));

        assert!(ui.find(i18n::t(Key::AgentsLimits)).is_err());
    }

    #[test]
    fn each_button_sends_its_intent_with_the_profile() {
        let cases = [
            (Key::AgentsLogIn, AgentsIntent::LogIn(work())),
            (Key::AgentsEdit, AgentsIntent::EditProfile(work())),
            (Key::AgentsHide, AgentsIntent::HideProfile(work())),
            (Key::ButtonRemove, AgentsIntent::RemoveProfile(work())),
        ];

        for (label, intent) in cases {
            assert_eq!(pressed(details(), i18n::t(label)), vec![intent]);
        }
    }

    #[test]
    fn a_default_profile_that_is_logged_in_has_no_log_in_and_no_remove() {
        let vm = ProfileDetailsViewModel {
            can_log_in: false,
            can_remove: false,
            ..details()
        };
        let themes = ThemeManager::new();
        let mut ui = simulator(view(vm, themes.current().ui()));

        assert!(ui.find(i18n::t(Key::AgentsLogIn)).is_err());
        assert!(ui.find(i18n::t(Key::ButtonRemove)).is_err());
        ui.find(i18n::t(Key::AgentsEdit)).expect("Edit should show");
    }
}

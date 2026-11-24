//! The Limits or Credits card of a subscription account: each usage
//! window with its bar, the extra usage, or the credits, with when they
//! were read. An older answer is drawn greyed (spec Decisions 5 and 10).
//! Each figure is its own row (issue 24 of spec
//! 2026-11-23-accounts-apart-from-profiles).

use iced::Element;
use iced::widget::Column;
use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::spacing;

use super::super::event::AgentsIntent;
use super::super::model::{CreditsViewModel, LimitsViewModel};
use super::blocks;

/// Render the card named `title`, "Limits" or "Credits" on the page of an
/// account: a figure for each window and for the credits, split by thin
/// rules, then what is old and what failed.
pub(super) fn view<'a>(
    vm: LimitsViewModel,
    title: &'static str,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let is_stale = vm.is_stale;

    let mut figures: Vec<Element<'a, AgentsIntent>> = vm
        .windows
        .into_iter()
        .map(|window| blocks::usage_figure(window, is_stale, palette))
        .collect();
    if let Some(credits) = vm.credits {
        figures.push(credits_figure(credits, is_stale, palette));
    }

    let mut body = Column::new()
        .spacing(spacing::SPACE_4)
        .push(blocks::figures(figures, palette));
    if vm.recorded.is_some() || vm.failure.is_some() {
        let muted = vm.recorded.into_iter();
        body = body.push(blocks::notes(muted, vm.failure, palette));
    }
    blocks::card(title, vm.updated, body, palette)
}

/// The credits: their value, the line of what is left and when the pool
/// renews and the note of credits with no number under the name, and the
/// bar when the pool is known.
fn credits_figure<'a>(
    credits: CreditsViewModel,
    is_stale: bool,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let name = i18n::t(Key::AgentsFieldCredits).to_string();
    let explain = credits
        .line
        .into_iter()
        .chain(credits.note.map(String::from))
        .collect();
    let value = blocks::figure_value(credits.value, is_stale, palette);

    blocks::figure(name, explain, value, credits.percent, is_stale, palette)
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use otty_app_theme::theme::ThemeManager;

    use super::*;
    use crate::model::UsageRowViewModel;

    fn limits() -> LimitsViewModel {
        LimitsViewModel {
            updated: Some(String::from("Updated 2 min ago")),
            windows: vec![
                UsageRowViewModel {
                    name: String::from("5 hours"),
                    used: String::from("42 % used"),
                    percent: Some(42),
                    resets: Some(String::from(
                        "Resets in 2 h 10 min, at 15:30",
                    )),
                },
                UsageRowViewModel {
                    name: String::from("Extra usage this month"),
                    used: String::from("$4.20"),
                    percent: None,
                    resets: None,
                },
            ],
            credits: Some(CreditsViewModel {
                value: String::from("1 240 of 5 000 used"),
                percent: Some(25),
                line: Some(String::from(
                    "3 760 left, Resets on Thursday at 09:00",
                )),
                note: None,
            }),
            is_stale: true,
            recorded: Some(String::from("Recorded in a session 1 h ago")),
            failure: Some(String::from("Could not read again: y")),
        }
    }

    #[test]
    fn the_block_shows_each_window_the_credits_and_what_failed() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(
            limits(),
            i18n::t(Key::AgentsUsage),
            themes.current().ui(),
        ));

        for shown in [
            i18n::t(Key::AgentsUsage),
            "Updated 2 min ago",
            "5 hours",
            "42 % used",
            "Resets in 2 h 10 min, at 15:30",
            "Extra usage this month",
            "$4.20",
            i18n::t(Key::AgentsFieldCredits),
            "1 240 of 5 000 used",
            "3 760 left, Resets on Thursday at 09:00",
            "Recorded in a session 1 h ago",
            "Could not read again: y",
        ] {
            ui.find(shown).expect("the limit detail should show");
        }
    }

    #[test]
    fn the_usage_title_has_only_when_it_was_updated_under_it() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(
            limits(),
            i18n::t(Key::AgentsUsage),
            themes.current().ui(),
        ));

        let title = ui
            .find(i18n::t(Key::AgentsUsage))
            .expect("the title shows")
            .bounds();
        let updated = ui
            .find("Updated 2 min ago")
            .expect("the update shows")
            .bounds();

        assert!(
            updated.y >= title.y + title.height,
            "{updated:?} should be under {title:?}"
        );
    }

    #[test]
    fn credits_with_only_a_balance_show_what_is_left() {
        let vm = LimitsViewModel {
            windows: Vec::new(),
            credits: Some(CreditsViewModel {
                value: String::from("3 760 left"),
                percent: None,
                line: None,
                note: None,
            }),
            is_stale: false,
            ..limits()
        };
        let themes = ThemeManager::new();
        let mut ui = simulator(view(
            vm,
            i18n::t(Key::AgentsUsage),
            themes.current().ui(),
        ));

        ui.find("3 760 left").expect("the balance should show");
        assert!(ui.find("1 240 of 5 000 used").is_err());
    }

    #[test]
    fn credits_with_no_number_say_available_and_why() {
        let vm = LimitsViewModel {
            windows: Vec::new(),
            credits: Some(CreditsViewModel {
                value: String::from("Available"),
                percent: None,
                line: None,
                note: Some("This plan does not report a balance."),
            }),
            is_stale: false,
            recorded: None,
            failure: None,
            ..limits()
        };
        let themes = ThemeManager::new();
        let mut ui = simulator(view(
            vm,
            i18n::t(Key::AgentsUsage),
            themes.current().ui(),
        ));

        ui.find(i18n::t(Key::AgentsFieldCredits))
            .expect("the credits line should show");
        ui.find("Available")
            .expect("the credits should be available");
        ui.find("This plan does not report a balance.")
            .expect("the note should say why there is no number");
    }
}

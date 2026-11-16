//! The limits block of a subscription or credits profile: each usage
//! window with its bar, the extra usage, or the credits, with where they
//! came from. An older answer is drawn greyed (spec Decision 5).

use iced::widget::{Column, column};
use iced::{Element, Length};
use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::spacing;

use super::super::event::AgentsIntent;
use super::super::model::{CreditsViewModel, LimitsViewModel, WindowViewModel};
use super::blocks;

/// Gap between the windows of the block (`gap: 14px`).
const WINDOW_GAP: f32 = 14.0;

/// Render the limits block.
pub(super) fn view<'a>(
    vm: LimitsViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let is_stale = vm.is_stale;

    let mut block = Column::new().spacing(WINDOW_GAP).push(blocks::title(
        i18n::t(Key::AgentsLimits),
        vm.from,
        palette,
    ));
    for window in vm.windows {
        block = block.push(bar_row(window, is_stale, palette));
    }
    if let Some(credits) = vm.credits {
        block = block.push(credits_rows(credits, is_stale, palette));
    }

    let muted = vm
        .recorded
        .into_iter()
        .chain(vm.log_in_hint.map(String::from));
    block
        .push(blocks::notes(muted, vm.failure, palette))
        .width(Length::Fill)
        .into()
}

/// One window: its name and how much is used, the bar, and when it resets.
fn bar_row<'a>(
    window: WindowViewModel,
    is_stale: bool,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let mut rows = column![blocks::name_and_value(
        window.name,
        window.used,
        is_stale,
        palette
    )]
    .spacing(spacing::SPACE_2);
    if let Some(percent) = window.percent {
        rows = rows.push(blocks::bar(percent, is_stale, palette));
    }
    if let Some(resets) = window.resets {
        rows = rows.push(blocks::small_text(resets, palette.text_muted()));
    }

    rows.into()
}

/// The credits: used of the pool, the bar when the pool is known, what is
/// left and when the pool renews, then the note of credits with no number.
fn credits_rows<'a>(
    credits: CreditsViewModel,
    is_stale: bool,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let value = credits
        .used
        .or_else(|| credits.left.clone())
        .unwrap_or_default();
    let line = match (credits.left, credits.renews) {
        (Some(left), Some(renews)) => Some(format!("{left} · {renews}")),
        (Some(left), None) => Some(left),
        (None, renews) => renews,
    };

    let mut rows = column![blocks::name_and_value(
        i18n::t(Key::AgentsFieldCredits).to_string(),
        value.clone(),
        is_stale,
        palette,
    )]
    .spacing(spacing::SPACE_2);
    if let Some(percent) = credits.percent {
        rows = rows.push(blocks::bar(percent, is_stale, palette));
    }
    if let Some(line) = line.filter(|line| *line != value) {
        rows = rows.push(blocks::small_text(line, palette.text_muted()));
    }
    if let Some(note) = credits.note {
        rows = rows.push(blocks::small_text(note, palette.text_muted()));
    }

    rows.into()
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use otty_app_theme::theme::ThemeManager;

    use super::*;

    fn limits() -> LimitsViewModel {
        LimitsViewModel {
            from: Some(String::from("From usage request, 2 min ago")),
            windows: vec![
                WindowViewModel {
                    name: String::from("5 hours"),
                    used: String::from("42 % used"),
                    percent: Some(42),
                    resets: Some(String::from("resets in 3 h")),
                },
                WindowViewModel {
                    name: String::from("Extra usage this month"),
                    used: String::from("$4.20"),
                    percent: None,
                    resets: None,
                },
            ],
            credits: Some(CreditsViewModel {
                used: Some(String::from("1 240 of 5 000 used")),
                percent: Some(25),
                left: Some(String::from("3 760 left")),
                renews: Some(String::from("resets in 3 d")),
                note: None,
            }),
            is_stale: true,
            recorded: Some(String::from("Recorded in a session 1 h ago")),
            failure: Some(String::from("Could not read again: y")),
            log_in_hint: Some(i18n::t(Key::AgentsLimitsLogInAgain)),
        }
    }

    #[test]
    fn the_block_shows_each_window_the_credits_and_what_failed() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(limits(), themes.current().ui()));

        for shown in [
            i18n::t(Key::AgentsLimits),
            "From usage request, 2 min ago",
            "5 hours",
            "42 % used",
            "resets in 3 h",
            "Extra usage this month",
            "$4.20",
            i18n::t(Key::AgentsFieldCredits),
            "1 240 of 5 000 used",
            "3 760 left · resets in 3 d",
            "Recorded in a session 1 h ago",
            "Could not read again: y",
            i18n::t(Key::AgentsLimitsLogInAgain),
        ] {
            ui.find(shown).expect("the limit detail should show");
        }
    }

    #[test]
    fn credits_with_only_a_balance_show_what_is_left() {
        let vm = LimitsViewModel {
            windows: Vec::new(),
            credits: Some(CreditsViewModel {
                used: None,
                percent: None,
                left: Some(String::from("3 760 left")),
                renews: None,
                note: None,
            }),
            is_stale: false,
            ..limits()
        };
        let themes = ThemeManager::new();
        let mut ui = simulator(view(vm, themes.current().ui()));

        ui.find("3 760 left").expect("the balance should show");
        assert!(ui.find("1 240 of 5 000 used").is_err());
    }

    #[test]
    fn credits_with_no_number_say_available_and_why() {
        let vm = LimitsViewModel {
            windows: Vec::new(),
            credits: Some(CreditsViewModel {
                used: None,
                percent: None,
                left: Some(String::from("Available")),
                renews: None,
                note: Some("This plan does not report a balance."),
            }),
            is_stale: false,
            recorded: None,
            failure: None,
            log_in_hint: None,
            ..limits()
        };
        let themes = ThemeManager::new();
        let mut ui = simulator(view(vm, themes.current().ui()));

        ui.find(i18n::t(Key::AgentsFieldCredits))
            .expect("the credits line should show");
        ui.find("Available")
            .expect("the credits should be available");
        ui.find("This plan does not report a balance.")
            .expect("the note should say why there is no number");
    }
}

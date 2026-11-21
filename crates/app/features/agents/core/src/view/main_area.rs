//! The Agents mode main area: the project screen, the page of a
//! profile, the page of an account, the All accounts and All profiles
//! pages, a spinner while the profiles are read for the first time, or a
//! card while there is nothing else to show, such as no agent part of a
//! plugin.

use iced::widget::{Id, column, container, svg, text};
use iced::{Alignment, Element, Length};
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::icons;
use otty_ui_kit::components::button::{Button, ButtonKind};
use otty_ui_kit::components::empty_card::EmptyCard;
use otty_ui_kit::components::spinner::Spinner;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, spacing, typography};

use super::super::event::AgentsIntent;
use super::super::model::MainAreaViewModel;

/// Widget id of the spinner shown while the profiles are read for the
/// first time.
pub(crate) const LOADING_SPINNER_ID: &str = "agents_loading_spinner";

/// Render the main area.
pub(crate) fn view<'a>(
    vm: MainAreaViewModel<'a>,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    match vm {
        MainAreaViewModel::NoAgentPart => card(
            Key::AgentsNoAgentPartTitle,
            Key::AgentsNoAgentPartText,
            palette,
        )
        .action(manage_plugins_button(palette))
        .into(),
        MainAreaViewModel::Loading => loading(palette),
        MainAreaViewModel::NoAgent => {
            card(Key::AgentsNoAgentTitle, Key::AgentsNoAgentText, palette)
                .action(refresh_button(ButtonKind::Primary, palette))
                .into()
        },
        MainAreaViewModel::Start => {
            card(Key::AgentsStartTitle, Key::AgentsStartText, palette)
                .action(refresh_button(ButtonKind::Secondary, palette))
                .into()
        },
        MainAreaViewModel::Project(project) => {
            super::project::view(*project, palette)
        },
        MainAreaViewModel::Profile(page) => {
            super::profile_page::view(*page, palette)
        },
        MainAreaViewModel::Account(page) => {
            super::account_page::view(*page, palette)
        },
        MainAreaViewModel::AllAccounts(page) => {
            super::overview::all_accounts(*page, palette)
        },
        MainAreaViewModel::AllProfiles(page) => {
            super::overview::all_profiles(*page, palette)
        },
    }
}

/// The kit spinner with "Looking for agents…" under it, as Containers mode
/// shows its spinner while it looks for engines; the screen area puts it in
/// the middle.
fn loading<'a>(palette: &UiPalette) -> Element<'a, AgentsIntent> {
    let spinner =
        container(Spinner::new(palette)).id(Id::new(LOADING_SPINNER_ID));
    let body = typography::BODY;
    let note = text(i18n::t(Key::AgentsLoading))
        .size(body.size())
        .line_height(iced::Pixels(body.line_height()))
        .font(fonts::UI)
        .color(palette.text_muted());

    column![spinner, note]
        .spacing(spacing::SPACE_3)
        .align_x(Alignment::Center)
        .into()
}

/// A card with the mode icon, a title and a text.
fn card<'a>(
    title: Key,
    description: Key,
    palette: &UiPalette,
) -> EmptyCard<'a, AgentsIntent> {
    let icon_color = palette.text_muted();
    let icon = svg(svg::Handle::from_memory(icons::MODE_AGENTS))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_, _| svg::Style {
            color: Some(icon_color),
        });

    EmptyCard::new(i18n::t(title), i18n::t(description), icon, palette)
}

/// Manage plugins…, which opens the Plugins page, as in Settings.
fn manage_plugins_button<'a>(palette: &UiPalette) -> Element<'a, AgentsIntent> {
    Button::new(i18n::t(Key::ButtonManagePlugins), palette)
        .kind(ButtonKind::Primary)
        .on_press(AgentsIntent::OpenPluginsPage)
        .into()
}

fn refresh_button<'a>(
    kind: ButtonKind,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    Button::new(i18n::t(Key::Refresh), palette)
        .kind(kind)
        .on_press(AgentsIntent::Refresh)
        .into()
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use otty_app_theme::theme::ThemeManager;
    use otty_ui_kit::components::screen_area::ScreenArea;

    use super::*;

    /// While the profiles are read for the first time the kit spinner turns
    /// in the middle of the screen, with "Looking for agents…" under it.
    #[test]
    fn the_first_read_shows_the_spinner_in_the_middle() {
        let themes = ThemeManager::new();
        let palette = themes.current().ui();
        let area = iced::Size::new(900.0, 700.0);
        let mut ui = iced_test::Simulator::with_size(
            iced::Settings::default(),
            area,
            ScreenArea::new(
                0,
                view(MainAreaViewModel::Loading, palette),
                palette,
            ),
        );

        let spinner = ui
            .find(Id::new(LOADING_SPINNER_ID))
            .expect("the spinner should show")
            .bounds();
        let note = ui
            .find(i18n::t(Key::AgentsLoading))
            .expect("the text should show")
            .bounds();

        assert!((spinner.center_x() - area.width / 2.0).abs() < 1.0);
        assert!(note.y >= spinner.y + spinner.height, "the text is under it");
        assert!(ui.find(i18n::t(Key::AgentsStartTitle)).is_err());
    }

    #[test]
    fn the_cards_say_what_is_going_on() {
        let themes = ThemeManager::new();
        let palette = themes.current().ui();

        let mut no_agent = simulator(view(MainAreaViewModel::NoAgent, palette));
        no_agent
            .click(i18n::t(Key::Refresh))
            .expect("Refresh should show");
        let refresh: Vec<AgentsIntent> = no_agent.into_messages().collect();
        assert_eq!(refresh, vec![AgentsIntent::Refresh]);

        let mut no_part =
            simulator(view(MainAreaViewModel::NoAgentPart, palette));
        no_part
            .find(i18n::t(Key::AgentsNoAgentPartText))
            .expect("the line should show");
        no_part
            .click(i18n::t(Key::ButtonManagePlugins))
            .expect("the button should show");
        let open: Vec<AgentsIntent> = no_part.into_messages().collect();
        assert_eq!(open, vec![AgentsIntent::OpenPluginsPage]);

        let mut start = simulator(view(MainAreaViewModel::Start, palette));
        start
            .find(i18n::t(Key::AgentsStartText))
            .expect("the start card should show");
    }
}

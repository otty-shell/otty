//! The Agents mode main area: the project screen, the details of a
//! profile, or a card while there is nothing else to show.

use iced::widget::svg;
use iced::{Element, Length};
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::icons;
use otty_ui_kit::components::button::{Button, ButtonKind};
use otty_ui_kit::components::empty_card::EmptyCard;
use otty_ui_kit::components::spinner::Spinner;
use otty_ui_kit::palette::UiPalette;

use super::super::event::AgentsIntent;
use super::super::model::MainAreaViewModel;

/// Render the main area.
pub(crate) fn view<'a>(
    vm: MainAreaViewModel<'a>,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    match vm {
        MainAreaViewModel::Loading => card(Key::AgentsLoading, None, palette)
            .action(Spinner::new(palette))
            .into(),
        MainAreaViewModel::NoAgent => card(
            Key::AgentsNoAgentTitle,
            Some(Key::AgentsNoAgentText),
            palette,
        )
        .action(refresh_button(ButtonKind::Primary, palette))
        .into(),
        MainAreaViewModel::Start => {
            card(Key::AgentsStartTitle, Some(Key::AgentsStartText), palette)
                .action(refresh_button(ButtonKind::Secondary, palette))
                .into()
        },
        MainAreaViewModel::Project(project) => {
            super::project::view(*project, palette)
        },
        MainAreaViewModel::Profile(details) => {
            super::profile_details::view(*details, palette)
        },
    }
}

/// A card with the mode icon, a title and a text.
fn card<'a>(
    title: Key,
    description: Option<Key>,
    palette: &UiPalette,
) -> EmptyCard<'a, AgentsIntent> {
    let icon_color = palette.text_muted();
    let icon = svg(svg::Handle::from_memory(icons::MODE_AGENTS))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_, _| svg::Style {
            color: Some(icon_color),
        });
    let description = description.map_or("", i18n::t);

    EmptyCard::new(i18n::t(title), description, icon, palette)
}

fn refresh_button<'a>(
    kind: ButtonKind,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    Button::new(i18n::t(Key::AgentsRefresh), palette)
        .kind(kind)
        .on_press(AgentsIntent::Refresh)
        .into()
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use otty_app_theme::theme::ThemeManager;

    use super::*;

    #[test]
    fn the_cards_say_what_is_going_on() {
        let themes = ThemeManager::new();
        let palette = themes.current().ui();

        let mut loading = simulator(view(MainAreaViewModel::Loading, palette));
        loading
            .find(i18n::t(Key::AgentsLoading))
            .expect("the loading card should show");
        drop(loading);

        let mut no_agent = simulator(view(MainAreaViewModel::NoAgent, palette));
        no_agent
            .click(i18n::t(Key::AgentsRefresh))
            .expect("Refresh should show");
        let refresh: Vec<AgentsIntent> = no_agent.into_messages().collect();
        assert_eq!(refresh, vec![AgentsIntent::Refresh]);

        let mut start = simulator(view(MainAreaViewModel::Start, palette));
        start
            .find(i18n::t(Key::AgentsStartText))
            .expect("the start card should show");
    }
}

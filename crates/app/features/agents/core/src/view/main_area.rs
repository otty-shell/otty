//! The Agents mode main area: the sessions of the selected project with
//! Continue and New session, or a card while there is no project to show.

use iced::widget::{Column, Space, column, container, row, svg, text};
use iced::{Alignment, Color, Element, Length};
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::icons;
use otty_ui_kit::components::button::{Button, ButtonKind, ButtonSize};
use otty_ui_kit::components::divider;
use otty_ui_kit::components::empty_card::EmptyCard;
use otty_ui_kit::components::spinner::Spinner;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, spacing, typography};

use super::super::event::AgentsIntent;
use super::super::model::{
    MainAreaViewModel, ProfileChoiceViewModel, ProjectViewModel,
    SessionRowViewModel,
};

/// Separates the profile, the age and the branch under a session title.
const META_SEPARATOR: &str = " · ";
/// Edge of the agent icon before a session.
const ICON_SIZE: f32 = 16.0;

/// Render the main area.
pub(crate) fn view<'a>(
    vm: MainAreaViewModel,
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
        MainAreaViewModel::Project(project) => project_view(project, palette),
        MainAreaViewModel::Profile(details) => {
            super::profile_details::view(details, palette)
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

/// The project: its name and path with Refresh and New session, the notice
/// of a missing folder, the choice of a profile and the sessions.
fn project_view<'a>(
    vm: ProjectViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let title = typography::TITLE;
    let name = column![
        text(vm.name)
            .size(title.size())
            .line_height(iced::Pixels(title.line_height()))
            .font(fonts::UI_SEMIBOLD)
            .color(palette.text_strong()),
        small_text(vm.path, palette.text_muted()),
    ]
    .width(Length::Fill);
    let mut new_session = Button::new(i18n::t(Key::AgentsNewSession), palette)
        .kind(ButtonKind::Primary);
    if vm.can_start {
        new_session = new_session.on_press(AgentsIntent::NewSession);
    }
    let header = row![
        name,
        refresh_button(ButtonKind::Secondary, palette),
        new_session
    ]
    .spacing(spacing::SPACE_3)
    .align_y(Alignment::Center);

    let mut page = Column::new().spacing(spacing::SPACE_4).push(header);
    if vm.is_missing {
        page = page.push(small_text(
            i18n::t(Key::AgentsFolderMissing),
            palette.status_error(),
        ));
    }
    if let Some(choices) = vm.choices {
        page = page.push(profile_choices(choices, palette));
    }
    page = page.push(divider::horizontal(palette));

    let sessions = vm
        .sessions
        .into_iter()
        .map(|session| session_row(session, &vm.folder, palette));
    page = page.push(Column::with_children(sessions).spacing(spacing::SPACE_3));

    container(page).width(Length::Fill).into()
}

/// The profiles a new session can start with, and Cancel. The row wraps,
/// so long names stay inside the screen.
fn profile_choices<'a>(
    choices: Vec<ProfileChoiceViewModel>,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let mut buttons = row![small_text(
        i18n::t(Key::AgentsChooseProfile),
        palette.text_body()
    )]
    .spacing(spacing::SPACE_3)
    .align_y(Alignment::Center);
    for choice in choices {
        let name = choice.profile.name().to_string();
        buttons = buttons.push(
            Button::new(name, palette)
                .kind(ButtonKind::Secondary)
                .size(ButtonSize::Small)
                .on_press(AgentsIntent::NewSessionWith(choice.profile)),
        );
    }
    buttons = buttons.push(
        Button::new(i18n::t(Key::ButtonCancel), palette)
            .kind(ButtonKind::Ghost)
            .size(ButtonSize::Small)
            .on_press(AgentsIntent::CancelNewSession),
    );

    buttons.wrap().vertical_spacing(spacing::SPACE_2).into()
}

/// One session: the agent icon, the title, the profile, the age and the
/// branch, and Continue.
fn session_row<'a>(
    vm: SessionRowViewModel,
    folder: &std::path::Path,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let muted = palette.text_muted();
    let icon = svg(svg::Handle::from_memory(icons::agent_icon(vm.icon)))
        .width(ICON_SIZE)
        .height(ICON_SIZE)
        .style(move |_, _| svg::Style { color: Some(muted) });

    let mut meta = vec![vm.profile, vm.age];
    meta.extend(vm.branch);
    let body = typography::BODY;
    let texts = column![
        text(vm.title)
            .size(body.size())
            .line_height(iced::Pixels(body.line_height()))
            .font(fonts::UI)
            .color(palette.text_body()),
        small_text(meta.join(META_SEPARATOR), muted),
    ]
    .width(Length::Fill);

    let mut continue_button =
        Button::new(i18n::t(Key::AgentsContinue), palette)
            .kind(ButtonKind::Secondary)
            .size(ButtonSize::Small);
    if vm.can_continue {
        continue_button = continue_button.on_press(AgentsIntent::Continue {
            folder: folder.to_path_buf(),
            session: vm.id,
        });
    }

    row![
        icon,
        texts,
        Space::new().width(spacing::SPACE_2),
        continue_button
    ]
    .spacing(spacing::SPACE_3)
    .align_y(Alignment::Center)
    .into()
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
    use otty_agents::{AgentId, Profile, SessionId};
    use otty_app_theme::theme::ThemeManager;

    use super::*;

    fn codex() -> Profile {
        Profile::default_for(
            AgentId::new("codex"),
            "Codex",
            PathBuf::from("/home/user/.codex"),
        )
    }

    fn project(is_missing: bool) -> ProjectViewModel {
        ProjectViewModel {
            folder: PathBuf::from("/work/otty"),
            name: String::from("otty"),
            path: String::from("/work/otty"),
            is_missing,
            can_start: !is_missing,
            choices: None,
            sessions: vec![SessionRowViewModel {
                id: SessionId::new("x1"),
                title: String::from("Fix the tabs"),
                icon: "codex",
                profile: String::from("Codex"),
                age: String::from("5 min ago"),
                branch: Some(String::from("main")),
                can_continue: !is_missing,
            }],
        }
    }

    fn pressed(vm: MainAreaViewModel, label: &str) -> Vec<AgentsIntent> {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(vm, themes.current().ui()));

        ui.click(label).expect("the button should show");

        ui.into_messages().collect()
    }

    #[test]
    fn continue_sends_the_session_and_its_folder() {
        let intents = pressed(
            MainAreaViewModel::Project(project(false)),
            i18n::t(Key::AgentsContinue),
        );

        assert_eq!(
            intents,
            vec![AgentsIntent::Continue {
                folder: PathBuf::from("/work/otty"),
                session: SessionId::new("x1"),
            }]
        );
    }

    #[test]
    fn a_session_shows_its_title_profile_age_and_branch() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(
            MainAreaViewModel::Project(project(false)),
            themes.current().ui(),
        ));

        ui.find("Fix the tabs").expect("the title should show");
        ui.find("Codex · 5 min ago · main")
            .expect("the profile, age and branch should show");
        ui.find("/work/otty").expect("the path should show");
    }

    #[test]
    fn new_session_and_refresh_send_their_intents() {
        let new_session = pressed(
            MainAreaViewModel::Project(project(false)),
            i18n::t(Key::AgentsNewSession),
        );
        let refresh = pressed(
            MainAreaViewModel::Project(project(false)),
            i18n::t(Key::AgentsRefresh),
        );

        assert_eq!(new_session, vec![AgentsIntent::NewSession]);
        assert_eq!(refresh, vec![AgentsIntent::Refresh]);
    }

    #[test]
    fn a_missing_project_says_so_and_its_buttons_send_nothing() {
        let vm = MainAreaViewModel::Project(project(true));

        let continued = pressed(vm.clone(), i18n::t(Key::AgentsContinue));
        let started = pressed(vm.clone(), i18n::t(Key::AgentsNewSession));
        let themes = ThemeManager::new();
        let mut ui = simulator(view(vm, themes.current().ui()));

        assert!(continued.is_empty());
        assert!(started.is_empty());
        ui.find(i18n::t(Key::AgentsFolderMissing))
            .expect("the notice should show");
    }

    #[test]
    fn the_choice_of_a_profile_sends_it_or_cancels() {
        let mut vm = project(false);
        vm.choices = Some(vec![ProfileChoiceViewModel {
            icon: "codex",
            profile: codex(),
        }]);
        let vm = MainAreaViewModel::Project(vm);

        let chosen = pressed(vm.clone(), "Codex");
        let cancelled = pressed(vm, i18n::t(Key::ButtonCancel));

        assert_eq!(chosen, vec![AgentsIntent::NewSessionWith(codex())]);
        assert_eq!(cancelled, vec![AgentsIntent::CancelNewSession]);
    }

    #[test]
    fn the_cards_say_what_is_going_on() {
        let themes = ThemeManager::new();
        let palette = themes.current().ui();

        let mut loading = simulator(view(MainAreaViewModel::Loading, palette));
        loading
            .find(i18n::t(Key::AgentsLoading))
            .expect("the loading card should show");
        drop(loading);

        let refresh =
            pressed(MainAreaViewModel::NoAgent, i18n::t(Key::AgentsRefresh));
        assert_eq!(refresh, vec![AgentsIntent::Refresh]);

        let mut start = simulator(view(MainAreaViewModel::Start, palette));
        start
            .find(i18n::t(Key::AgentsStartText))
            .expect("the start card should show");
    }
}

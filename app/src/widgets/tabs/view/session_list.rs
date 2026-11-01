use iced::widget::{Column, container, scrollable, svg, text};
use iced::{Element, Length, Padding};
use otty_ui_kit::components::panel_section::PanelSection;
use otty_ui_kit::components::scrollbar;
use otty_ui_kit::components::session_group::SessionGroup;
use otty_ui_kit::components::session_row::{SessionRow, SessionTone};
use otty_ui_kit::components::split_button::SplitButton;
use otty_ui_kit::components::target_dot::TargetKind;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, size, spacing, typography};

use super::super::event::TabsIntent;
use super::super::model::{
    SessionGroupViewModel, SessionState, SessionViewModel, TabsViewModel,
};
use crate::i18n::{self, Key};
use crate::icons;
use crate::shortcuts::AppShortcut;
use crate::theme::ThemeProps;

/// Id of the session list scrollable, scrolled to its end when a tab opens.
pub(crate) const SESSION_LIST_SCROLL_ID: &str = "session_list_scroll";
/// Space above the empty hint.
const EMPTY_HINT_TOP: f32 = 2.0;

/// Props for the Sessions section of the left panel.
///
/// The section is generic over the message type because its header toggle
/// belongs to the sidebar, while the rows and the New session button send
/// tabs intents.
pub(crate) struct SessionListProps<'a, Message> {
    pub(crate) vm: TabsViewModel<'a>,
    pub(crate) theme: ThemeProps<'a>,
    /// Whether the section body is shown.
    pub(crate) is_open: bool,
    /// Title given to a terminal tab opened by the New session button.
    pub(crate) new_session_title: &'a str,
    /// Whether the New session menu is open, which frames the chevron.
    pub(crate) is_menu_open: bool,
    /// Message sent when the section title is pressed.
    pub(crate) on_toggle: Message,
    /// Message sent when the chevron next to the plus is pressed.
    pub(crate) on_menu: Message,
    /// Turns a tabs intent into the caller's message.
    pub(crate) on_intent: fn(TabsIntent) -> Message,
}

/// Render the Sessions section: header with the New session split button, and
/// one group per machine with a row per open tab.
pub(crate) fn view<'a, Message: Clone + 'a>(
    props: SessionListProps<'a, Message>,
) -> Element<'a, Message> {
    let palette = props.theme.theme.ui();
    let on_intent = props.on_intent;
    let session_count = props.vm.session_count;

    let new_session_label = i18n::t(Key::NewSession);
    let new_session = SplitButton::new(
        svg::Handle::from_memory(icons::ADD_TAB_HEADER),
        format!("{new_session_label} ({})", AppShortcut::NewTerminal.label()),
        i18n::t(Key::NewSessionMenu),
        palette,
    )
    .menu_open(props.is_menu_open)
    .on_press(on_intent(TabsIntent::OpenTerminalTab {
        title: props.new_session_title.to_string(),
    }))
    .on_menu(props.on_menu);

    let mut rows = Column::new().width(Length::Fill);
    for group in props.vm.groups {
        rows = rows.push(group_label(&group, palette));
        for session in group.sessions {
            rows = rows.push(session_row(session, palette, on_intent));
        }
    }
    if session_count == 0 {
        rows = rows.push(empty_hint(palette));
    }
    let body = scrollable::Scrollable::new(rows)
        .id(SESSION_LIST_SCROLL_ID)
        .width(Length::Fill)
        .direction(scrollbar::vertical())
        .style(scrollbar::style(palette));

    let mut section =
        PanelSection::new(i18n::t(Key::SessionsPanelTitle), palette)
            .open(props.is_open)
            .action(new_session)
            .body(body)
            .on_toggle(props.on_toggle);
    if !props.is_open {
        section = section.count(session_count);
    }

    section.into()
}

/// Height of the section body: group labels and rows, or the empty hint.
///
/// Counted from the rows because iced cannot measure a widget before the
/// left panel shares its height between the sections.
pub(crate) fn content_height(vm: &TabsViewModel<'_>) -> f32 {
    if vm.session_count == 0 {
        return EMPTY_HINT_TOP
            + typography::SMALL.line_height()
            + spacing::SPACE_2;
    }

    vm.groups.len() as f32 * size::GROUP + vm.session_count as f32 * size::ROW
}

/// Muted hint shown while no session is open.
fn empty_hint<'a, Message: 'a>(palette: &UiPalette) -> Element<'a, Message> {
    let small = typography::SMALL;

    container(
        text(i18n::t(Key::NoOpenSessions))
            .size(small.size())
            .line_height(iced::Pixels(small.line_height()))
            .font(fonts::UI)
            .color(palette.text_muted()),
    )
    .padding(Padding {
        top: EMPTY_HINT_TOP,
        right: spacing::SPACE_3,
        bottom: spacing::SPACE_2,
        left: spacing::SPACE_3,
    })
    .into()
}

/// Group label: machine dot, name, and "this computer" or "ssh".
fn group_label<'a, Message: 'a>(
    group: &SessionGroupViewModel<'_>,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let note = match group.target {
        TargetKind::Ssh => i18n::t(Key::SshMachineNote),
        _ => i18n::t(Key::ThisComputerNote),
    };

    SessionGroup::new(group.name, group.target, palette)
        .note(note)
        .into()
}

/// One session row: press activates the tab, the close cross (shown on hover) and a middle
/// click close it.
fn session_row<'a, Message: Clone + 'a>(
    session: SessionViewModel<'_>,
    palette: &UiPalette,
    on_intent: fn(TabsIntent) -> Message,
) -> Element<'a, Message> {
    let tab_id = session.tab_id;

    let mut row = SessionRow::new(session.title, palette)
        .selected(session.is_active)
        .on_press(on_intent(TabsIntent::ActivateTab { tab_id }))
        .on_middle_press(on_intent(TabsIntent::CloseTab { tab_id }))
        .on_close(on_intent(TabsIntent::CloseTab { tab_id }));
    if let Some(state) = session.state {
        let (word, tone) = state_word(state);
        row = row.state(word, tone);
    }

    row.into()
}

/// Word and color shown after the title of a non-terminal row.
fn state_word(state: SessionState) -> (&'static str, SessionTone) {
    match state {
        SessionState::Settings => {
            (i18n::t(Key::SessionStateSettings), SessionTone::Muted)
        },
        SessionState::Editor => {
            (i18n::t(Key::SessionStateEditor), SessionTone::Muted)
        },
        SessionState::Failed => {
            (i18n::t(Key::SessionStateFailed), SessionTone::Failed)
        },
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use iced::Point;
    use iced::widget::container;
    use iced_test::simulator;
    use otty_ui_kit::{size, spacing};

    use super::{SessionListProps, content_height, view};
    use crate::theme::{ThemeManager, ThemeProps};
    use crate::widgets::tabs::TabsIntent;
    use crate::widgets::tabs::model::TabsViewModel;
    use crate::widgets::tabs::types::{TabContent, TabItem, TabMachine};

    /// Width of the panel in the tests.
    const PANEL_WIDTH: f32 = 260.0;
    /// Section padding 8, header row 30, gap 1: the top of the rows.
    const BODY_TOP: f32 = 8.0 + 30.0 + 1.0;
    /// Section padding 8, header row 30 high: the middle of the header.
    const HEADER_Y: f32 = 8.0 + 15.0;
    /// Middle of the close cross: section padding 8, then the 24px button.
    const CLOSE_X: f32 = PANEL_WIDTH - 8.0 - 12.0;
    /// Middle of the chevron: section padding 8, then the chevron.
    const CHEVRON_X: f32 = PANEL_WIDTH - 8.0 - size::SPLIT_CHEVRON / 2.0;
    /// Middle of the plus: section padding 8, the chevron, then half the plus.
    const PLUS_X: f32 =
        PANEL_WIDTH - 8.0 - size::SPLIT_CHEVRON - size::CONTROL_SM / 2.0;

    #[derive(Debug, Clone)]
    enum Message {
        Toggle,
        OpenMenu,
        Intent(TabsIntent),
    }

    fn tabs() -> BTreeMap<u64, TabItem> {
        BTreeMap::from([
            (
                0,
                TabItem::new(
                    0,
                    String::from("zsh"),
                    TabContent::Terminal,
                    TabMachine::ThisComputer,
                ),
            ),
            (
                1,
                TabItem::new(
                    1,
                    String::from("htop"),
                    TabContent::Terminal,
                    TabMachine::Ssh(String::from("laptop")),
                ),
            ),
            (
                2,
                TabItem::new(
                    2,
                    String::from("Settings"),
                    TabContent::Settings,
                    TabMachine::ThisComputer,
                ),
            ),
        ])
    }

    fn section<'a>(
        tabs: &'a BTreeMap<u64, TabItem>,
        themes: &'a ThemeManager,
        is_open: bool,
    ) -> iced::Element<'a, Message> {
        container(view(SessionListProps {
            vm: TabsViewModel::new(tabs, Some(1), Some("fedora")),
            theme: ThemeProps::new(themes.current()),
            is_open,
            new_session_title: "zsh",
            is_menu_open: false,
            on_toggle: Message::Toggle,
            on_menu: Message::OpenMenu,
            on_intent: Message::Intent,
        }))
        .width(PANEL_WIDTH)
        .into()
    }

    fn middle_click() -> [iced::Event; 2] {
        [
            iced::Event::Mouse(iced::mouse::Event::ButtonPressed(
                iced::mouse::Button::Middle,
            )),
            iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
                iced::mouse::Button::Middle,
            )),
        ]
    }

    #[test]
    fn given_two_sessions_when_background_title_clicked_then_it_is_activated() {
        let tabs = tabs();
        let themes = ThemeManager::new();
        let mut ui = simulator(section(&tabs, &themes, true));

        ui.click("zsh").expect("row title should be visible");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert!(matches!(
            messages.as_slice(),
            [Message::Intent(TabsIntent::ActivateTab { tab_id: 0 })]
        ));
    }

    #[test]
    fn given_a_row_when_close_pressed_then_its_tab_is_closed() {
        let tabs = tabs();
        let themes = ThemeManager::new();
        let mut ui = simulator(section(&tabs, &themes, true));
        let title = ui.find("htop").expect("row title should be visible");

        ui.point_at(Point::new(CLOSE_X, title.bounds().center_y()));
        let _ = ui.simulate(iced_test::simulator::click());

        let messages: Vec<Message> = ui.into_messages().collect();
        assert!(matches!(
            messages.as_slice(),
            [Message::Intent(TabsIntent::CloseTab { tab_id: 1 })]
        ));
    }

    #[test]
    fn given_a_row_when_middle_clicked_then_its_tab_is_closed() {
        let tabs = tabs();
        let themes = ThemeManager::new();
        let mut ui = simulator(section(&tabs, &themes, true));
        let title = ui.find("zsh").expect("row title should be visible");

        ui.point_at(title.bounds().center());
        let _ = ui.simulate(middle_click());

        let messages: Vec<Message> = ui.into_messages().collect();
        assert!(matches!(
            messages.as_slice(),
            [Message::Intent(TabsIntent::CloseTab { tab_id: 0 })]
        ));
    }

    #[test]
    fn given_header_when_new_session_pressed_then_a_terminal_tab_opens() {
        let tabs = tabs();
        let themes = ThemeManager::new();
        let mut ui = simulator(section(&tabs, &themes, true));

        ui.point_at(Point::new(PLUS_X, HEADER_Y));
        let _ = ui.simulate(iced_test::simulator::click());

        let messages: Vec<Message> = ui.into_messages().collect();
        assert!(matches!(
            messages.as_slice(),
            [Message::Intent(TabsIntent::OpenTerminalTab { title })]
                if title == "zsh"
        ));
    }

    #[test]
    fn given_header_when_chevron_pressed_then_the_menu_opens() {
        let tabs = tabs();
        let themes = ThemeManager::new();
        let mut ui = simulator(section(&tabs, &themes, true));

        ui.point_at(Point::new(CHEVRON_X, HEADER_Y));
        let _ = ui.simulate(iced_test::simulator::click());

        let messages: Vec<Message> = ui.into_messages().collect();
        assert!(matches!(messages.as_slice(), [Message::OpenMenu]));
    }

    #[test]
    fn given_header_when_title_pressed_then_toggle_is_sent() {
        let tabs = tabs();
        let themes = ThemeManager::new();
        let mut ui = simulator(section(&tabs, &themes, true));

        ui.click("Sessions")
            .expect("section title should be visible");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert!(matches!(messages.as_slice(), [Message::Toggle]));
    }

    #[test]
    fn given_open_section_when_viewed_then_groups_notes_and_state_words_show() {
        let tabs = tabs();
        let themes = ThemeManager::new();
        let mut ui = simulator(section(&tabs, &themes, true));

        assert!(ui.find("fedora").is_ok());
        assert!(ui.find("this computer").is_ok());
        assert!(ui.find("laptop").is_ok());
        assert!(ui.find("ssh").is_ok());
        assert!(ui.find("settings").is_ok());
    }

    #[test]
    fn given_no_sessions_when_viewed_then_hint_shows() {
        let tabs = BTreeMap::new();
        let themes = ThemeManager::new();
        let mut ui = simulator(section(&tabs, &themes, true));

        assert!(ui.find("No open sessions.").is_ok());
    }

    #[test]
    fn given_sessions_when_viewed_then_hint_is_hidden() {
        let tabs = tabs();
        let themes = ThemeManager::new();
        let mut ui = simulator(section(&tabs, &themes, true));

        assert!(ui.find("No open sessions.").is_err());
    }

    #[test]
    fn given_closed_section_when_viewed_then_rows_hide_and_count_shows() {
        let tabs = tabs();
        let themes = ThemeManager::new();
        let mut ui = simulator(section(&tabs, &themes, false));

        assert!(ui.find("zsh").is_err());
        assert!(ui.find("3").is_ok());
    }

    #[test]
    fn given_sessions_when_content_height_read_then_it_ends_at_the_last_row() {
        let tabs = tabs();
        let themes = ThemeManager::new();
        let vm = TabsViewModel::new(&tabs, Some(1), Some("fedora"));
        let mut ui = simulator(section(&tabs, &themes, true));

        let content = content_height(&vm);
        let last = ui.find("htop").expect("last row should be visible");

        assert_eq!(content, 2.0 * size::GROUP + 3.0 * size::ROW);
        let row_middle = BODY_TOP + content - size::ROW / 2.0;
        assert!(
            (last.bounds().center_y() - row_middle).abs() < 1.0,
            "last row middle {} should be at {row_middle}",
            last.bounds().center_y()
        );
    }

    #[test]
    fn given_no_sessions_when_content_height_read_then_it_ends_under_the_hint()
    {
        let tabs = BTreeMap::new();
        let themes = ThemeManager::new();
        let vm = TabsViewModel::new(&tabs, None, Some("fedora"));
        let mut ui = simulator(section(&tabs, &themes, true));

        let content = content_height(&vm);
        let hint = ui.find("No open sessions.").expect("hint should show");

        let hint_bottom =
            hint.bounds().y + hint.bounds().height + spacing::SPACE_2;
        assert!(
            (hint_bottom - (BODY_TOP + content)).abs() < 1.0,
            "hint ends at {hint_bottom}, content at {}",
            BODY_TOP + content
        );
    }
}

use iced::Element;
use iced::widget::pane_grid;
use otty_ui_kit::components::menu::{self, Menu, MenuItem};
use otty_ui_kit::size;

use super::super::event::TabsIntent;
use crate::i18n::{self, Key};
use crate::shortcuts::AppShortcut;
use crate::theme::ThemeProps;
use crate::widgets::quick_launch::QuickLaunchIntent;
use crate::widgets::terminal_workspace::TerminalWorkspaceIntent;

/// Width of the New session menu, from the design.
pub(crate) const NEW_SESSION_MENU_WIDTH: f32 = 280.0;

/// Height of one separator: 4px margin, a 1px line, 4px margin.
const SEPARATOR_HEIGHT: f32 = 9.0;
/// Height of the "Launches" label: 6px above, 4px below, one text line.
const HEADING_HEIGHT: f32 = 26.0;
/// New terminal, Split current pane right and New launch.
const FIXED_ITEMS: f32 = 3.0;

/// The pane that "Split current pane right" splits.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SplitTarget {
    pub(crate) tab_id: u64,
    pub(crate) pane: pane_grid::Pane,
}

/// Props for the New session menu.
///
/// The menu is generic over the message type: each item sends an intent of
/// another widget, wrapped by the caller.
pub(crate) struct NewSessionMenuProps<'a, Message> {
    pub(crate) theme: ThemeProps<'a>,
    /// Shell name, shown next to "New terminal" and used as the tab title.
    pub(crate) shell_name: &'a str,
    /// Pane to split; `None` disables "Split current pane right".
    pub(crate) split_target: Option<SplitTarget>,
    /// Titles of the launches at the top level of the launch tree.
    pub(crate) launch_titles: Vec<&'a str>,
    /// Wraps a tabs intent.
    pub(crate) on_tabs: fn(TabsIntent) -> Message,
    /// Wraps a terminal workspace intent.
    pub(crate) on_terminal: fn(TerminalWorkspaceIntent) -> Message,
    /// Wraps a quick launch intent.
    pub(crate) on_launch: fn(QuickLaunchIntent) -> Message,
}

/// Height of the menu for `launch_count` launches, used to keep it inside
/// the window.
pub(crate) fn menu_height(launch_count: usize) -> f32 {
    let frame = 2.0 * menu::MENU_PADDING;

    if launch_count == 0 {
        return frame + size::ROW * FIXED_ITEMS + SEPARATOR_HEIGHT;
    }

    frame
        + size::ROW * (FIXED_ITEMS + launch_count as f32)
        + 2.0 * SEPARATOR_HEIGHT
        + HEADING_HEIGHT
}

/// Render the New session menu: New terminal, Split current pane right, the
/// top-level launches, and New launch.
pub(crate) fn view<'a, Message: Clone + 'a>(
    props: NewSessionMenuProps<'a, Message>,
) -> Element<'a, Message> {
    let palette = props.theme.theme.ui();
    let on_tabs = props.on_tabs;
    let on_terminal = props.on_terminal;
    let on_launch = props.on_launch;

    let new_terminal =
        format!("{} ({})", i18n::t(Key::MenuNewTerminal), props.shell_name);
    let new_terminal = MenuItem::new(new_terminal)
        .shortcut(AppShortcut::NewTerminal.label())
        .on_press(on_tabs(TabsIntent::OpenTerminalTab {
            title: props.shell_name.to_string(),
        }));

    let mut split = MenuItem::new(i18n::t(Key::MenuSplitCurrentPane))
        .shortcut(AppShortcut::SplitPane.label());
    if let Some(target) = props.split_target {
        split =
            split.on_press(on_terminal(TerminalWorkspaceIntent::SplitPane {
                tab_id: target.tab_id,
                pane: target.pane,
                axis: pane_grid::Axis::Vertical,
            }));
    }

    let mut menu = Menu::new(palette)
        .width(NEW_SESSION_MENU_WIDTH)
        .item(new_terminal)
        .item(split)
        .separator();

    if !props.launch_titles.is_empty() {
        menu = menu.heading(i18n::t(Key::LaunchesPanelTitle));
        for title in props.launch_titles {
            let run = QuickLaunchIntent::LaunchNode {
                path: vec![title.to_string()],
            };
            menu = menu.item(MenuItem::new(title).on_press(on_launch(run)));
        }
        menu = menu.separator();
    }

    menu.item(
        MenuItem::new(i18n::t(Key::MenuNewLaunch))
            .on_press(on_launch(QuickLaunchIntent::HeaderCreateCommand)),
    )
    .into()
}

#[cfg(test)]
mod tests {
    use iced::widget::{container, pane_grid};
    use iced_test::simulator;
    use otty_ui_kit::size;

    use super::{NewSessionMenuProps, SplitTarget, menu_height, view};
    use crate::theme::{ThemeManager, ThemeProps};
    use crate::widgets::quick_launch::QuickLaunchIntent;
    use crate::widgets::tabs::TabsIntent;
    use crate::widgets::terminal_workspace::TerminalWorkspaceIntent;

    #[derive(Debug, Clone)]
    enum Message {
        Tabs(TabsIntent),
        Terminal(TerminalWorkspaceIntent),
        Launch(QuickLaunchIntent),
    }

    fn menu<'a>(
        themes: &'a ThemeManager,
        split_target: Option<SplitTarget>,
        launch_titles: Vec<&'a str>,
    ) -> iced::Element<'a, Message> {
        container(view(NewSessionMenuProps {
            theme: ThemeProps::new(themes.current()),
            shell_name: "zsh",
            split_target,
            launch_titles,
            on_tabs: Message::Tabs,
            on_terminal: Message::Terminal,
            on_launch: Message::Launch,
        }))
        .into()
    }

    fn split_target() -> SplitTarget {
        let (_panes, pane) = pane_grid::State::new(());

        SplitTarget { tab_id: 4, pane }
    }

    fn click(
        split_target: Option<SplitTarget>,
        launch_titles: Vec<&str>,
        label: &str,
    ) -> Vec<Message> {
        let themes = ThemeManager::new();
        let mut ui = simulator(menu(&themes, split_target, launch_titles));

        ui.click(label).expect("menu item should be visible");

        ui.into_messages().collect()
    }

    #[test]
    fn given_new_terminal_item_when_pressed_then_a_terminal_tab_opens() {
        let messages = click(None, Vec::new(), "New terminal (zsh)");

        assert!(matches!(
            messages.as_slice(),
            [Message::Tabs(TabsIntent::OpenTerminalTab { title })]
                if title == "zsh"
        ));
    }

    #[test]
    fn given_active_terminal_when_split_item_pressed_then_pane_splits_right() {
        let messages =
            click(Some(split_target()), Vec::new(), "Split current pane right");

        assert!(matches!(
            messages.as_slice(),
            [Message::Terminal(TerminalWorkspaceIntent::SplitPane {
                tab_id: 4,
                axis: pane_grid::Axis::Vertical,
                ..
            })]
        ));
    }

    #[test]
    fn given_no_terminal_when_split_item_pressed_then_nothing_is_sent() {
        let messages = click(None, Vec::new(), "Split current pane right");

        assert!(messages.is_empty());
    }

    #[test]
    fn given_launch_item_when_pressed_then_that_launch_starts() {
        let messages = click(None, vec!["htop", "laptop"], "laptop");

        assert!(matches!(
            messages.as_slice(),
            [Message::Launch(QuickLaunchIntent::LaunchNode { path })]
                if path == &[String::from("laptop")]
        ));
    }

    #[test]
    fn given_new_launch_item_when_pressed_then_launch_creation_starts() {
        let messages = click(None, vec!["htop"], "New launch\u{2026}");

        assert!(matches!(
            messages.as_slice(),
            [Message::Launch(QuickLaunchIntent::HeaderCreateCommand)]
        ));
    }

    #[test]
    fn given_launches_when_viewed_then_the_launches_label_shows() {
        let themes = ThemeManager::new();
        let mut with = simulator(menu(&themes, None, vec!["htop"]));
        let mut without = simulator(menu(&themes, None, Vec::new()));

        assert!(with.find("Launches").is_ok());
        assert!(without.find("Launches").is_err());
    }

    #[test]
    fn given_launches_when_height_computed_then_each_adds_one_row() {
        assert_eq!(menu_height(2) - menu_height(1), size::ROW);
        assert!(menu_height(1) > menu_height(0));
    }
}

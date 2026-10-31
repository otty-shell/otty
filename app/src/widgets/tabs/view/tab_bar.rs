use iced::Element;
use otty_ui_kit::components::tab_strip::{Tab, TabStrip};

use super::super::event::TabsIntent;
use crate::theme::ThemeProps;

pub(crate) const TAB_BAR_SCROLL_ID: &str = "tab_bar_scroll";

/// Props for rendering the tab bar.
#[derive(Debug, Clone)]
pub(crate) struct TabBarProps<'a> {
    pub(crate) tabs: Vec<(u64, String)>,
    pub(crate) active_tab_id: Option<u64>,
    /// Title given to a terminal tab opened by the plus button.
    pub(crate) new_tab_title: String,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the tab bar as a horizontal scrollable strip of kit tabs.
pub(crate) fn view<'a>(props: TabBarProps<'a>) -> Element<'a, TabsIntent> {
    let mut strip = TabStrip::new(props.theme.theme.ui())
        .scroll_id(TAB_BAR_SCROLL_ID)
        .on_new(TabsIntent::OpenTerminalTab {
            title: props.new_tab_title,
        });

    for (tab_id, title) in props.tabs {
        strip = strip.tab(
            Tab::new(title)
                .active(props.active_tab_id == Some(tab_id))
                .on_press(TabsIntent::ActivateTab { tab_id })
                .on_close(TabsIntent::CloseTab { tab_id }),
        );
    }

    strip.into()
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;

    use super::{TabBarProps, view};
    use crate::theme::{ThemeManager, ThemeProps};
    use crate::widgets::tabs::TabsIntent;

    #[test]
    fn given_two_tabs_when_background_title_clicked_then_it_is_activated() {
        let themes = ThemeManager::new();
        let props = TabBarProps {
            tabs: vec![(0, String::from("zsh")), (1, String::from("htop"))],
            active_tab_id: Some(1),
            new_tab_title: String::from("zsh"),
            theme: ThemeProps::new(themes.current()),
        };
        let mut ui = simulator(view(props));

        ui.click("zsh").expect("tab title should be visible");

        let intents: Vec<TabsIntent> = ui.into_messages().collect();
        assert!(matches!(
            intents.as_slice(),
            [TabsIntent::ActivateTab { tab_id: 0 }]
        ));
    }
}

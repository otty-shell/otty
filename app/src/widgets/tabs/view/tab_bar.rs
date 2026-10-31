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

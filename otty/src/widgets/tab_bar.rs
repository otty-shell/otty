use iced::widget::{container, row, scrollable};
use iced::{Element, Length};

use crate::app::theme::ThemeProps;
use crate::components::tab_button::{
    TabButton, TabButtonEvent, TabButtonProps,
};

/// UI events emitted by the tab bar.
#[derive(Debug, Clone)]
pub(crate) enum TabBarEvent {
    TabButton(TabButtonEvent),
}

/// Summary data for rendering a tab.
#[derive(Debug, Clone)]
pub(crate) struct TabSummary {
    pub(crate) id: u64,
    pub(crate) title: String,
}

/// Layout metrics for the tab bar.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TabBarMetrics {
    pub(crate) height: f32,
}

impl Default for TabBarMetrics {
    fn default() -> Self {
        Self { height: 25.0 }
    }
}

/// Props for rendering the tab bar.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TabBarProps<'a> {
    pub(crate) tabs: &'a [TabSummary],
    pub(crate) active_tab_id: u64,
    pub(crate) theme: ThemeProps<'a>,
    pub(crate) metrics: TabBarMetrics,
}

/// A horizontally scrollable bar of tabs.
pub(crate) struct TabBar<'a> {
    props: TabBarProps<'a>,
}

impl<'a> TabBar<'a> {
    pub fn new(props: TabBarProps<'a>) -> Self {
        Self { props }
    }

    pub fn view(&self) -> Element<'a, TabBarEvent> {
        let mut tabs_row = row![].spacing(0);

        for tab in self.props.tabs {
            let tab_props = TabButtonProps {
                id: tab.id,
                title: tab.title.as_str(),
                is_active: self.props.active_tab_id == tab.id,
                theme: self.props.theme,
            };

            tabs_row = tabs_row.push(
                TabButton::new(tab_props).view().map(TabBarEvent::TabButton),
            );
        }

        let scroll = scrollable::Scrollable::with_direction(
            tabs_row,
            scrollable::Direction::Horizontal(
                scrollable::Scrollbar::new()
                    .width(0)
                    .scroller_width(0)
                    .margin(0),
            ),
        )
        .width(Length::Fill);

        let palette = self.props.theme.theme.iced_palette();

        container(scroll)
            .height(Length::Fixed(self.props.metrics.height))
            .width(Length::Fill)
            .style(move |_| iced::widget::container::Style {
                background: Some(palette.dim_black.into()),
                text_color: None,
                ..Default::default()
            })
            .into()
    }
}

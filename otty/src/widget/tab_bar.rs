use iced::{Element, Length, widget::{container, row, scrollable}};

use crate::{component::tab_button, tab::Tab, theme::{AppTheme, fallback_theme}};

#[derive(Debug, Clone)]
pub enum Event {
    TabButton(tab_button::Event)
}

#[derive(Clone, Copy)]
pub struct Metrics {
    pub height: f32,
}

impl Default for Metrics {
    fn default() -> Self {
        Self { height: 25.0 }
    }
}

pub struct TabBar<'a> {
    tabs: &'a [Tab],
    active_tab_index: usize,
    theme: Option<&'a AppTheme>,
    metrics: Metrics
}

impl<'a> TabBar<'a> {
    pub fn new(
        tabs: &'a [Tab],
        active_tab_index: usize,
    ) -> Self {
        Self {
            tabs,
            active_tab_index,
            theme: None,
            metrics: Metrics::default(),
        }
    }

    pub fn theme(mut self, theme: &'a AppTheme) -> Self {
        self.theme = Some(theme);
        self
    }

    pub fn view(&self) -> Element<'a, Event> {
        let theme = self.theme.unwrap_or(fallback_theme());
        let mut tabs_row = row![].spacing(0);
        let active_tab_id = self.tabs[self.active_tab_index].id;

        for tab in self.tabs {
            tabs_row = tabs_row.push(
                tab_button::TabButton::new(
                    tab.id,
                    &tab.title,
                )
                .theme(theme)
                .active(active_tab_id == tab.id)            
                .view()
                .map(Event::TabButton)
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

        container(scroll)
            .height(Length::Fixed(self.metrics.height))
            .width(Length::Fill)
            .style({
                let theme = theme.iced_palette();
                move |_| iced::widget::container::Style {
                    background: Some(theme.dim_black.into()),
                    text_color: None,
                    ..iced::widget::container::Style::default()
                }
            })
            .into()
    }
}

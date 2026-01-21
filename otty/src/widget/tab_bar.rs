use iced::{Length, advanced::graphics::core::Element, widget::{container, row, scrollable}};

use crate::{component::tab_button::{TabButton, TabButtonEvent}, tab::Tab, theme::AppTheme};

pub const TAB_BAR_HEIGHT: f32 = 25.0;

#[derive(Debug, Clone)]
pub enum TabBarEvent {
    TabButton(TabButtonEvent)
}

#[derive(Clone, Copy)]
struct TabBarState<'a> {
    tabs: &'a Vec<Tab>,
    theme: &'a AppTheme,
    active_tab_index: usize,
}

impl<'a> TabBarState<'a> {
    fn new(
        tabs: &'a Vec<Tab>,
        theme: &'a AppTheme,
        active_tab_index: usize,
    ) -> Self {
        Self {
            tabs,
            theme,
            active_tab_index,
        }
    }
}

pub struct TabBar<'a> {
    state: TabBarState<'a>,
}

impl<'a> TabBar<'a> {
    pub fn new(
        tabs: &'a Vec<Tab>,
        theme: &'a AppTheme,
        active_tab_index: usize,
    ) -> Self {
        Self {
            state: TabBarState::new(tabs, theme, active_tab_index)
        }
    }

    pub fn view(&self) -> Element<'a, TabBarEvent, iced::Theme, iced::Renderer> {
        let state = self.state;
        let mut tabs_row = row![].spacing(0);
        let active_tab_id = state.tabs[state.active_tab_index].id;

        for tab in state.tabs {
            tabs_row = tabs_row.push(TabButton::new(
                tab.id,
                &tab.title,
                active_tab_id == tab.id,
                state.theme,
            ).view().map(TabBarEvent::TabButton));
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
            .height(Length::Fixed(TAB_BAR_HEIGHT))
            .width(Length::Fill)
            .style({
                let theme = state.theme.iced_palette();
                move |_| iced::widget::container::Style {
                    background: Some(theme.dim_black.into()),
                    text_color: None,
                    ..iced::widget::container::Style::default()
                }
            })
            .into()
    }
}
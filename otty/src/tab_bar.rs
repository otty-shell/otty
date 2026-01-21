use iced::widget::{container, row, scrollable};
use iced::{Element, Length};

use crate::app::{App, Event};
use crate::tab_button::tab_button;

pub const TAB_BAR_HEIGHT: f32 = 25.0;

pub fn view_tab_bar<'a>(
    app: &'a App,
) -> Element<'a, Event, iced::Theme, iced::Renderer> {
    let mut tabs_row = row![].spacing(0);

    for tab in &app.tabs {
        let is_active = app.tabs[app.active_tab_index].id == tab.id;
        let tab_button = tab_button(
            &tab.title,
            is_active,
            tab.id,
            app.theme_manager.current().iced_palette(),
        );
        tabs_row = tabs_row.push(tab_button);
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
            let theme = app.theme_manager.current().iced_palette();
            move |_| iced::widget::container::Style {
                background: Some(theme.dim_black.into()),
                text_color: None,
                ..iced::widget::container::Style::default()
            }
        })
        .into()
}

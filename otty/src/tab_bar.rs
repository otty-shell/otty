use iced::widget::{button, container, row, scrollable, text};
use iced::{Element, Length};

use crate::main_window::{App, Event};
use crate::tab_button::tab_button;

pub fn view_tab_bar(app: &App) -> Element<Event, iced::Theme, iced::Renderer> {
    let mut tabs_row = row![].spacing(4);

    let tab_count = usize::max(app.tabs.len(), 1) as f32;
    const TAB_MAX_WIDTH: f32 = 152.0;
    const TAB_MIN_WIDTH: f32 = 72.0;
    const PLUS_BUTTON_WIDTH: f32 = 32.0;
    const TAB_BAR_HEIGHT: f32 = 25.0;

    let available_width =
        (app.window_size.width - PLUS_BUTTON_WIDTH).max(TAB_MIN_WIDTH);
    let per_tab_width =
        (available_width / tab_count).clamp(TAB_MIN_WIDTH, TAB_MAX_WIDTH);

    for tab in &app.tabs {
        let is_active = app.tabs[app.active_tab_index].id == tab.id;
        let tab_button = tab_button(
            &tab.title,
            per_tab_width,
            is_active,
            tab.id,
            app.theme_manager.current().font_size_ui,
        );
        tabs_row = tabs_row.push(tab_button);
    }

    let add_button = button(text("+"))
        .on_press(Event::NewTab)
        .padding([4, 10])
        .width(Length::Fixed(PLUS_BUTTON_WIDTH))
        .height(Length::Fill);

    tabs_row = tabs_row.push(add_button);

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
        // .padding([4, 8])
        .into()
}

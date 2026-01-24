use iced::widget::{column, container, text};
use iced::{Element, Length, Theme, alignment};

use crate::app::state::AppEvent;
use crate::app::tabs::{TabContent, WorkspaceState, map_tab_bar_event_to_app};
use crate::app::theme::ThemeProps;
use crate::widgets::tab::{TabProps, TabView};
use crate::widgets::tab_bar::{TabBar, TabBarMetrics, TabBarProps};

pub(crate) fn view<'a>(
    workspace: &'a WorkspaceState,
    theme: ThemeProps<'a>,
) -> Element<'a, AppEvent, Theme, iced::Renderer> {
    let tab_summaries = workspace.tab_summaries();
    let active_tab_id = workspace.active_tab_id.unwrap_or(0);

    let tab_bar = TabBar::new(TabBarProps {
        tabs: tab_summaries,
        active_tab_id,
        theme,
        metrics: TabBarMetrics::default(),
    })
    .view()
    .map(map_tab_bar_event_to_app);

    let main_content: Element<'a, AppEvent, Theme, iced::Renderer> =
        match workspace.active_tab() {
            Some(tab) => match &tab.content {
                TabContent::Terminal(terminal) => {
                    let selected_block_terminal =
                        terminal.selected_block_terminal();
                    TabView::new(TabProps {
                        tab_id: tab.id,
                        panes: terminal.panes(),
                        terminals: terminal.terminals(),
                        focus: terminal.focus(),
                        context_menu: terminal.context_menu(),
                        selected_block_terminal,
                        theme,
                    })
                    .view()
                    .map(AppEvent::Tab)
                },
            },
            None => container(text("No tabs"))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(alignment::Horizontal::Center)
                .align_y(alignment::Vertical::Center)
                .into(),
        };

    column![tab_bar, main_content]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

use iced::widget::{container, text};
use iced::{Element, Length, Theme, alignment};

use crate::app::Event as AppEvent;
use crate::features::tab::TabContent;
use crate::state::State;
use crate::theme::ThemeProps;
use crate::ui::widgets::quick_launches;
use crate::ui::widgets::settings;
use crate::ui::widgets::terminal;

pub(crate) fn view<'a>(
    state: &'a State,
    theme: ThemeProps<'a>,
) -> Element<'a, AppEvent, Theme, iced::Renderer> {
    let main_content: Element<'a, AppEvent, Theme, iced::Renderer> = match state
        .active_tab()
    {
        Some(tab) => match &tab.content {
            TabContent::Terminal(terminal) => {
                let tab_id = tab.id;
                terminal::view(terminal::Props {
                    panes: terminal.panes(),
                    terminals: terminal.terminals(),
                    focus: terminal.focus(),
                })
                .map(move |event| AppEvent::Terminal { tab_id, event })
            },
            TabContent::Settings => settings::view(settings::Props {
                state: &state.settings,
                theme,
            })
            .map(AppEvent::Settings),
            TabContent::QuickLaunchEditor(editor) => {
                let tab_id = tab.id;
                quick_launches::editor::view(quick_launches::editor::Props {
                    editor,
                    theme,
                })
                .map(move |event| AppEvent::QuickLaunchEditor { tab_id, event })
            },
            TabContent::QuickLaunchError(error) => {
                quick_launches::error::view(quick_launches::error::Props {
                    error,
                    theme,
                })
            },
        },
        None => container(text("No tabs"))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Center)
            .into(),
    };

    main_content
}

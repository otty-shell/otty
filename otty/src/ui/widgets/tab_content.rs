use iced::widget::{container, text};
use iced::{Element, Length, Theme, alignment};

use crate::features::settings::SettingsState;
use crate::features::tab::{TabContent, TabItem};
use crate::features::terminal::TerminalEvent;
use crate::theme::ThemeProps;
use crate::ui::widgets::{
    quick_launches_editor, quick_launches_error, settings, terminal_tab,
};

/// Props for rendering the active tab content.
#[derive(Clone, Copy)]
pub(crate) struct TabContentProps<'a> {
    pub(crate) active_tab: Option<&'a TabItem>,
    pub(crate) settings: &'a SettingsState,
    pub(crate) theme: ThemeProps<'a>,
}

/// Events emitted by tab content widget.
#[derive(Debug, Clone)]
pub(crate) enum TabContentEvent {
    Terminal(TerminalEvent),
    Settings(settings::SettingsEvent),
    QuickLaunchEditor {
        tab_id: u64,
        event: quick_launches_editor::QuickLaunchesEditorEvent,
    },
    QuickLaunchError(quick_launches_error::QuickLaunchesErrorEvent),
}

pub(crate) fn view<'a>(
    props: TabContentProps<'a>,
) -> Element<'a, TabContentEvent, Theme, iced::Renderer> {
    let theme = props.theme;

    let main_content: Element<'a, TabContentEvent, Theme, iced::Renderer> =
        match props.active_tab {
            Some(tab) => match &tab.content {
                TabContent::Terminal(terminal) => {
                    let tab_id = tab.id;
                    terminal_tab::view(terminal_tab::TerminalTabProps {
                        tab_id,
                        panes: terminal.panes(),
                        terminals: terminal.terminals(),
                        focus: terminal.focus(),
                    })
                    .map(TabContentEvent::Terminal)
                },
                TabContent::Settings => {
                    settings::view(settings::SettingsProps {
                        state: props.settings,
                        theme,
                    })
                    .map(TabContentEvent::Settings)
                },
                TabContent::QuickLaunchEditor(editor) => {
                    let tab_id = tab.id;
                    quick_launches_editor::view(
                        quick_launches_editor::QuickLaunchesEditorProps {
                            editor,
                            theme,
                        },
                    )
                    .map(move |event| {
                        TabContentEvent::QuickLaunchEditor { tab_id, event }
                    })
                },
                TabContent::QuickLaunchError(error) => {
                    quick_launches_error::view(
                        quick_launches_error::QuickLaunchesErrorProps {
                            error,
                            theme,
                        },
                    )
                    .map(TabContentEvent::QuickLaunchError)
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

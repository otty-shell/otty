use iced::widget::{container, text};
use iced::{Element, Length, Theme, alignment};

use crate::features::quick_launch::QuickLaunchState;
use crate::features::quick_launch_wizard::QuickLaunchWizardState;
use crate::features::settings::SettingsState;
use crate::features::terminal::{TerminalEvent, TerminalState};
use crate::tab::{TabContent, TabItem};
use crate::theme::ThemeProps;
use crate::ui::widgets::{
    quick_launches_error, quick_launches_wizard, settings, terminal_tab,
};

/// Props for rendering the active tab content.
#[derive(Clone, Copy)]
pub(crate) struct TabContentProps<'a> {
    pub(crate) active_tab: Option<&'a TabItem>,
    pub(crate) terminal: &'a TerminalState,
    pub(crate) quick_launch_wizard: &'a QuickLaunchWizardState,
    pub(crate) quick_launches: &'a QuickLaunchState,
    pub(crate) settings: &'a SettingsState,
    pub(crate) theme: ThemeProps<'a>,
}

/// Events emitted by tab content widget.
#[derive(Debug, Clone)]
pub(crate) enum TabContentEvent {
    Terminal(TerminalEvent),
    Settings(settings::SettingsEvent),
    QuickLaunchWizard {
        tab_id: u64,
        event: quick_launches_wizard::QuickLaunchesWizardEvent,
    },
    QuickLaunchError(quick_launches_error::QuickLaunchesErrorEvent),
}

pub(crate) fn view<'a>(
    props: TabContentProps<'a>,
) -> Element<'a, TabContentEvent, Theme, iced::Renderer> {
    let theme = props.theme;

    let main_content: Element<'a, TabContentEvent, Theme, iced::Renderer> =
        match props.active_tab {
            Some(tab) => match tab.content() {
                TabContent::Terminal => {
                    let tab_id = tab.id();
                    let Some(terminal) = props.terminal.tab(tab_id) else {
                        return missing_tab_state(
                            "Terminal tab is not initialized.",
                        );
                    };
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
                TabContent::QuickLaunchWizard => {
                    let tab_id = tab.id();
                    let Some(editor) = props.quick_launch_wizard.editor(tab_id)
                    else {
                        return missing_tab_state(
                            "Quick launch editor is not initialized.",
                        );
                    };
                    quick_launches_wizard::view(
                        quick_launches_wizard::QuickLaunchesWizardProps {
                            editor,
                            theme,
                        },
                    )
                    .map(move |event| {
                        TabContentEvent::QuickLaunchWizard { tab_id, event }
                    })
                },
                TabContent::QuickLaunchError => {
                    let tab_id = tab.id();
                    let Some(error) = props.quick_launches.error_tab(tab_id)
                    else {
                        return missing_tab_state(
                            "Quick launch error payload is missing.",
                        );
                    };
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

fn missing_tab_state<'a>(
    message: &'static str,
) -> Element<'a, TabContentEvent, Theme, iced::Renderer> {
    container(text(message))
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .into()
}

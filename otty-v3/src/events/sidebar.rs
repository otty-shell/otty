use iced::Task;

use crate::app::App;
use crate::widgets::sidebar::{
    SidebarCtx, SidebarEffect, SidebarEvent,
};
use crate::widgets::tabs::{TabsEvent, TabsUiEvent};
use super::AppEvent;

pub(crate) fn handle(app: &mut App, event: SidebarEvent) -> Task<AppEvent> {
    match event {
        SidebarEvent::Ui(event) => app.widgets
            .sidebar
            .reduce(event, &SidebarCtx)
            .map(AppEvent::Sidebar),
        SidebarEvent::Effect(effect) => handle_effect(effect),
    }
}

fn handle_effect(event: SidebarEffect) -> Task<AppEvent> {
    use SidebarEffect as E;

    match event {
        E::SyncTerminalGridSizes => {
            Task::done(AppEvent::SyncTerminalGridSizes)
        },
        E::OpenSettingsTab => {
            Task::done(AppEvent::Tabs(TabsEvent::Ui(TabsUiEvent::OpenSettingsTab)))
        },
        E::OpenTerminalTab => Task::done(AppEvent::OpenTerminalTab),
        E::QuickLaunchHeaderCreateCommand => {
            Task::done(AppEvent::QuickLaunch(
                crate::widgets::quick_launch::QuickLaunchEvent::Ui(
                    crate::widgets::quick_launch::QuickLaunchUiEvent::HeaderCreateCommand,
                ),
            ))
        },
        E::QuickLaunchHeaderCreateFolder => {
            Task::done(AppEvent::QuickLaunch(
                crate::widgets::quick_launch::QuickLaunchEvent::Ui(
                    crate::widgets::quick_launch::QuickLaunchUiEvent::HeaderCreateFolder,
                ),
            ))
        },
        E::QuickLaunchResetInteractionState => {
            Task::done(AppEvent::QuickLaunch(
                crate::widgets::quick_launch::QuickLaunchEvent::Ui(
                    crate::widgets::quick_launch::QuickLaunchUiEvent::ResetInteractionState,
                ),
            ))
        },
    }
}

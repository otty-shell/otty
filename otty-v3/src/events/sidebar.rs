use iced::Task;

use super::AppEvent;
use crate::app::App;
use crate::widgets::quick_launch::{QuickLaunchEvent, QuickLaunchIntent};
use crate::widgets::sidebar::{SidebarCtx, SidebarEffect, SidebarEvent};
use crate::widgets::tabs::{TabsEvent, TabsIntent};

pub(crate) fn handle(app: &mut App, event: SidebarEvent) -> Task<AppEvent> {
    match event {
        SidebarEvent::Intent(event) => app
            .widgets
            .sidebar
            .reduce(event, &SidebarCtx)
            .map(AppEvent::Sidebar),
        SidebarEvent::Effect(effect) => handle_effect(app, effect),
    }
}

fn handle_effect(app: &App, event: SidebarEffect) -> Task<AppEvent> {
    use SidebarEffect::*;

    match event {
        SyncTerminalGridSizes => Task::done(AppEvent::SyncTerminalGridSizes),
        OpenSettingsTab => Task::done(AppEvent::Tabs(TabsEvent::Intent(
            TabsIntent::OpenSettingsTab,
        ))),
        OpenTerminalTab => Task::done(AppEvent::Tabs(TabsEvent::Intent(
            TabsIntent::OpenTerminalTab {
                title: app.shell_session.name().to_string(),
            },
        ))),
        QuickLaunchHeaderCreateCommand => Task::done(AppEvent::QuickLaunch(
            QuickLaunchEvent::Intent(QuickLaunchIntent::HeaderCreateCommand),
        )),
        QuickLaunchHeaderCreateFolder => Task::done(AppEvent::QuickLaunch(
            QuickLaunchEvent::Intent(QuickLaunchIntent::HeaderCreateFolder),
        )),
        QuickLaunchResetInteractionState => Task::done(AppEvent::QuickLaunch(
            QuickLaunchEvent::Intent(QuickLaunchIntent::ResetInteractionState),
        )),
    }
}

use iced::Task;

use crate::app::{App, AppEvent};
use crate::widgets::quick_launch::{
    QuickLaunchCtx, QuickLaunchEffect, QuickLaunchEvent, QuickLaunchUiEvent,
};
use crate::widgets::sidebar::SidebarEvent;

/// Route a quick launch event through widget reduction or app orchestration.
pub(crate) fn route(app: &mut App, event: QuickLaunchEvent) -> Task<AppEvent> {
    match event {
        QuickLaunchEvent::Ui(event) => route_ui_event(app, event),
        QuickLaunchEvent::Effect(effect) => route_effect_event(effect),
    }
}

fn route_ui_event(app: &mut App, event: QuickLaunchUiEvent) -> Task<AppEvent> {
    // The add button lives in the quick launch panel but triggers the
    // sidebar add-menu overlay, so redirect instead of reducing.
    if matches!(event, QuickLaunchUiEvent::HeaderAddButtonPressed) {
        return Task::done(AppEvent::SidebarUi(SidebarEvent::AddMenuOpen));
    }

    let ctx = build_ctx_from_parts(
        &app.terminal_settings,
        app.widgets.sidebar.cursor(),
        app.widgets.sidebar.is_resizing(),
    );
    app.widgets
        .quick_launch
        .reduce(event, &ctx)
        .map(AppEvent::QuickLaunch)
}

fn route_effect_event(effect: QuickLaunchEffect) -> Task<AppEvent> {
    match effect {
        QuickLaunchEffect::OpenWizardCreateTab { parent_path } => {
            Task::done(AppEvent::OpenQuickLaunchWizardCreateTab { parent_path })
        },
        QuickLaunchEffect::OpenWizardEditTab { path, command } => {
            Task::done(AppEvent::OpenQuickLaunchWizardEditTab { path, command })
        },
        QuickLaunchEffect::OpenCommandTerminalTab {
            title,
            settings,
            command,
        } => Task::done(AppEvent::OpenQuickLaunchCommandTerminalTab {
            title,
            settings,
            command,
        }),
        QuickLaunchEffect::OpenErrorTab { title, message } => {
            Task::done(AppEvent::OpenQuickLaunchErrorTab { title, message })
        },
        QuickLaunchEffect::CloseTabRequested { tab_id } => {
            Task::done(AppEvent::CloseTab { tab_id })
        },
    }
}

fn build_ctx_from_parts<'a>(
    terminal_settings: &'a otty_ui_term::settings::Settings,
    sidebar_cursor: iced::Point,
    sidebar_is_resizing: bool,
) -> QuickLaunchCtx<'a> {
    QuickLaunchCtx {
        terminal_settings,
        sidebar_cursor,
        sidebar_is_resizing,
    }
}

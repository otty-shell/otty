use iced::Task;

use crate::app::{App, Event as AppEvent};
use crate::widgets::quick_launch::QuickLaunchEvent;
use crate::widgets::quick_launch_wizard::{
    QuickLaunchWizardCommand, QuickLaunchWizardCtx,
    QuickLaunchWizardEffectEvent, QuickLaunchWizardUiEvent,
};

/// Route quick-launch wizard UI event into reducer command path.
pub(crate) fn route_event(
    app: &mut App,
    tab_id: u64,
    event: QuickLaunchWizardUiEvent,
) -> Task<AppEvent> {
    let command = map_ui_event_to_command(event);
    app.widgets
        .quick_launch_wizard_mut()
        .reduce(command, &QuickLaunchWizardCtx { tab_id })
        .map(AppEvent::QuickLaunchWizardEffect)
}

/// Route quick-launch wizard side-effect event into app-level tasks.
pub(crate) fn route_effect(
    event: QuickLaunchWizardEffectEvent,
) -> Task<AppEvent> {
    use QuickLaunchWizardEffectEvent as E;

    match event {
        E::CloseTabRequested { tab_id } => {
            Task::done(AppEvent::CloseTabRequested { tab_id })
        },
        E::WizardSaveRequested(request) => Task::done(AppEvent::QuickLaunch(
            QuickLaunchEvent::WizardSaveRequested(request),
        )),
    }
}

fn map_ui_event_to_command(
    event: QuickLaunchWizardUiEvent,
) -> QuickLaunchWizardCommand {
    use {QuickLaunchWizardCommand as C, QuickLaunchWizardUiEvent as E};

    match event {
        E::InitializeCreate { parent_path } => {
            C::InitializeCreate { parent_path }
        },
        E::InitializeEdit { path, command } => {
            C::InitializeEdit { path, command }
        },
        E::TabClosed => C::TabClosed,
        E::Cancel => C::Cancel,
        E::Save => C::Save,
        E::SetError { message } => C::SetError { message },
        E::UpdateTitle(value) => C::UpdateTitle(value),
        E::UpdateProgram(value) => C::UpdateProgram(value),
        E::UpdateHost(value) => C::UpdateHost(value),
        E::UpdateUser(value) => C::UpdateUser(value),
        E::UpdatePort(value) => C::UpdatePort(value),
        E::UpdateIdentityFile(value) => C::UpdateIdentityFile(value),
        E::UpdateWorkingDirectory(value) => C::UpdateWorkingDirectory(value),
        E::AddArg => C::AddArg,
        E::RemoveArg(index) => C::RemoveArg(index),
        E::UpdateArg { index, value } => C::UpdateArg { index, value },
        E::AddEnv => C::AddEnv,
        E::RemoveEnv(index) => C::RemoveEnv(index),
        E::UpdateEnvKey { index, value } => C::UpdateEnvKey { index, value },
        E::UpdateEnvValue { index, value } => {
            C::UpdateEnvValue { index, value }
        },
        E::AddExtraArg => C::AddExtraArg,
        E::RemoveExtraArg(index) => C::RemoveExtraArg(index),
        E::UpdateExtraArg { index, value } => {
            C::UpdateExtraArg { index, value }
        },
        E::SelectCommandType(command_type) => {
            C::SelectCommandType(command_type)
        },
    }
}

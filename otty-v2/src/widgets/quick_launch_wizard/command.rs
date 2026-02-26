use crate::widgets::quick_launch::{QuickLaunch, QuickLaunchType};

/// Commands accepted by quick-launch wizard reducer.
#[derive(Debug, Clone)]
pub(crate) enum QuickLaunchWizardCommand {
    InitializeCreate {
        parent_path: Vec<String>,
    },
    InitializeEdit {
        path: Vec<String>,
        command: Box<QuickLaunch>,
    },
    TabClosed,
    Cancel,
    Save,
    SetError {
        message: String,
    },
    UpdateTitle(String),
    UpdateProgram(String),
    UpdateHost(String),
    UpdateUser(String),
    UpdatePort(String),
    UpdateIdentityFile(String),
    UpdateWorkingDirectory(String),
    AddArg,
    RemoveArg(usize),
    UpdateArg {
        index: usize,
        value: String,
    },
    AddEnv,
    RemoveEnv(usize),
    UpdateEnvKey {
        index: usize,
        value: String,
    },
    UpdateEnvValue {
        index: usize,
        value: String,
    },
    AddExtraArg,
    RemoveExtraArg(usize),
    UpdateExtraArg {
        index: usize,
        value: String,
    },
    SelectCommandType(QuickLaunchType),
}

use iced::Point;

use super::super::super::types::{ActionRequest, ContainersError};
use super::types::RowAction;

/// Intent events of the actions: the row menu, the confirmation dialog,
/// Prune and the engine calls that run.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ActionsIntent {
    /// An action button of the inspector or an item of the row menu was
    /// pressed: run it on the row of the open row menu, or else on the
    /// selected row.
    Act(RowAction),
    /// Open the menu of the row with the id `target` in the open list at
    /// `position` (window coordinates); the parent finds the row the press
    /// was on.
    OpenRowMenu { target: String, position: Point },
    /// A press outside the row menu closed it, or the list of its row went
    /// away.
    CloseRowMenu,
    /// The Prune button above the table was pressed.
    Prune,
    /// The switch of the confirmation dialog changed.
    ConfirmSwitchToggled(bool),
    /// The action button of the confirmation dialog was pressed.
    ConfirmAccepted,
    /// Cancel of the confirmation dialog was pressed.
    ConfirmCancelled,
    /// Run this engine call the way every action runs, so its list stays busy
    /// until the call ends: a sent Create volume or Create network dialog.
    Start(ActionRequest),
    /// The engine finished an action; the error is the engine's message.
    Finished {
        request: ActionRequest,
        result: Result<(), ContainersError>,
    },
}

/// Effect events of the actions, run by the containers route.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ActionsEffect {
    /// Run this action on its engine.
    RunAction(ActionRequest),
    /// Open a terminal tab named `title` running `program` with `args`
    /// (`docker --host <address> exec -it -- <id> sh`, or
    /// `podman --url <address> exec -it -- <id> sh`), so the shell opens on
    /// the chosen engine, then show Terminal mode.
    OpenShell {
        title: String,
        program: String,
        args: Vec<String>,
    },
}

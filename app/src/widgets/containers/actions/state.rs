//! The state of the actions (spec Decision 11): the engine calls that run,
//! the destructive action waiting for the user to confirm it and the open row
//! menu.

use iced::Point;
use otty_containers::EngineAddress;

use super::super::types::{ActionRequest, ResourceKind};
use super::offers::ImageRemoval;

/// The actions that run, the open confirmation and the open row menu.
#[derive(Debug, Default)]
pub(crate) struct ActionsState {
    /// The requests the engines run now; their buttons are disabled.
    running: Vec<ActionRequest>,
    /// The destructive action waiting for the user to confirm it.
    confirm: Option<Confirm>,
    /// The open row menu.
    row_menu: Option<RowMenu>,
}

impl ActionsState {
    /// Return the action that waits for the user to confirm it.
    pub(crate) fn confirm(&self) -> Option<&Confirm> {
        self.confirm.as_ref()
    }

    /// Return where the row menu is open, in window coordinates.
    pub(crate) fn row_menu(&self) -> Option<Point> {
        self.row_menu.as_ref().map(|menu| menu.position)
    }

    /// Return the id of the row the open row menu is for; an image row goes by
    /// its row key.
    pub(crate) fn row_menu_target(&self) -> Option<&str> {
        self.row_menu.as_ref().map(|menu| menu.target.as_str())
    }

    /// Return whether the row menu or the confirmation dialog covers the
    /// window.
    pub(crate) fn has_overlay(&self) -> bool {
        self.row_menu.is_some() || self.confirm.is_some()
    }

    /// Return whether an action runs on the row `target` (or, with `None`,
    /// a prune on the whole list) of the `resource` list of the engine at
    /// `address`.
    pub(crate) fn is_busy(
        &self,
        address: &EngineAddress,
        resource: ResourceKind,
        target: Option<&str>,
    ) -> bool {
        self.running
            .iter()
            .any(|request| request.acts_on(address, resource, target))
    }

    /// Remember that `request` runs, so its buttons are disabled.
    pub(super) fn start_action(&mut self, request: ActionRequest) {
        self.running.push(request);
    }

    /// Forget a finished `request`.
    pub(super) fn finish_action(&mut self, request: &ActionRequest) {
        if let Some(index) =
            self.running.iter().position(|running| running == request)
        {
            self.running.remove(index);
        }
    }

    /// Ask the user to confirm an action.
    pub(super) fn open_confirm(&mut self, confirm: Confirm) {
        self.confirm = Some(confirm);
    }

    /// Close the confirmation and return it.
    pub(super) fn take_confirm(&mut self) -> Option<Confirm> {
        self.confirm.take()
    }

    /// Turn the switch of the open confirmation on or off.
    pub(super) fn set_confirm_switch(&mut self, is_on: bool) {
        if let Some(confirm) = self.confirm.as_mut() {
            confirm.set_switch(is_on);
        }
    }

    /// Open the menu of the row with the id `target` at `position`.
    pub(super) fn open_row_menu(&mut self, position: Point, target: String) {
        self.row_menu = Some(RowMenu { position, target });
    }

    /// Close the row menu.
    pub(super) fn close_row_menu(&mut self) {
        self.row_menu = None;
    }
}

#[cfg(test)]
impl ActionsState {
    /// Return the actions the engines are running now.
    pub(crate) fn running(&self) -> &[ActionRequest] {
        &self.running
    }
}

/// A destructive action waiting for the user to confirm it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Confirm {
    request: ActionRequest,
    /// The dialog switch ("Stop it first"); `None` when the dialog has none.
    switch: Option<bool>,
    /// How many rows a prune deletes.
    count: usize,
    /// How many rows it deletes with the switch on, when that differs.
    count_if_on: usize,
    /// What the removal of an image row does; `None` for other actions.
    image_removal: Option<ImageRemoval>,
}

impl Confirm {
    /// The action that runs on confirm, before the switch is applied.
    pub(crate) fn request(&self) -> &ActionRequest {
        &self.request
    }

    /// Whether the switch is on; `None` when the dialog has no switch.
    pub(crate) fn switch(&self) -> Option<bool> {
        self.switch
    }

    /// How many rows a prune deletes, with the switch as it is now.
    pub(crate) fn count(&self) -> usize {
        if self.switch == Some(true) {
            self.count_if_on
        } else {
            self.count
        }
    }

    /// What the removal of an image row does; `None` for other actions.
    pub(crate) fn image_removal(&self) -> Option<ImageRemoval> {
        self.image_removal
    }

    /// Ask to confirm `request`, with a switch in its first state, or
    /// none.
    pub(super) fn new(
        request: ActionRequest,
        switch: Option<bool>,
        count: usize,
    ) -> Self {
        Self {
            request,
            switch,
            count,
            count_if_on: count,
            image_removal: None,
        }
    }

    /// Say what the removal of an image row does, for the text of the dialog.
    pub(super) fn with_image_removal(mut self, removal: ImageRemoval) -> Self {
        self.image_removal = Some(removal);
        self
    }

    /// Set how many rows the prune deletes with the switch on.
    pub(super) fn with_count_if_on(mut self, count_if_on: usize) -> Self {
        self.count_if_on = count_if_on;
        self
    }

    /// Turn the switch on or off; a dialog without a switch keeps none.
    pub(super) fn set_switch(&mut self, is_on: bool) {
        if self.switch.is_some() {
            self.switch = Some(is_on);
        }
    }

    /// The request to run, with the switch applied.
    pub(super) fn into_request(self) -> ActionRequest {
        let Some(is_on) = self.switch else {
            return self.request;
        };

        self.request.with_switch(is_on)
    }
}

/// The open row menu: where it opened and the row it is for. A right press
/// also selects that row; the row "⋯" button does not.
#[derive(Debug)]
struct RowMenu {
    /// Window coordinates.
    position: Point,
    /// The row's id in the open list: container or network id, image row key,
    /// or volume name.
    target: String,
}

#[cfg(test)]
mod tests {
    use otty_containers::{EngineKind, fixtures};

    use super::*;
    use crate::widgets::containers::types::EngineAction;

    fn request(action: EngineAction) -> ActionRequest {
        ActionRequest::new(
            fixtures::engine(EngineKind::Docker),
            action,
            String::from("otty-fx-running"),
        )
    }

    #[test]
    fn the_switch_of_a_remove_confirmation_forces_the_removal() {
        let remove = request(EngineAction::RemoveContainer {
            id: String::from("c1"),
            force: false,
        });
        let mut confirm = Confirm::new(remove, Some(true), 0);

        let forced = confirm.clone().into_request();
        confirm.set_switch(false);
        let gentle = confirm.into_request();

        assert_eq!(
            forced.action(),
            &EngineAction::RemoveContainer {
                id: String::from("c1"),
                force: true,
            }
        );
        assert_eq!(
            gentle.action(),
            &EngineAction::RemoveContainer {
                id: String::from("c1"),
                force: false,
            }
        );
    }

    #[test]
    fn a_confirmation_without_a_switch_keeps_it_hidden() {
        let mut confirm =
            Confirm::new(request(EngineAction::PruneContainers), None, 3);

        confirm.set_switch(true);

        assert_eq!(confirm.switch(), None);
        assert_eq!(confirm.count(), 3);
        assert_eq!(
            confirm.into_request().action(),
            &EngineAction::PruneContainers
        );
    }

    #[test]
    fn the_force_switch_forces_an_image_removal() {
        let remove = request(EngineAction::RemoveImage {
            reference: String::from("sha256:a"),
            force: false,
        });

        let forced = Confirm::new(remove, Some(true), 0).into_request();

        assert_eq!(
            forced.action(),
            &EngineAction::RemoveImage {
                reference: String::from("sha256:a"),
                force: true,
            }
        );
    }
}

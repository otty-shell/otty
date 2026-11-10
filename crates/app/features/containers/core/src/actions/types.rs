use otty_app_i18n::Key;

/// What a button in the inspector or an item of the row menu does with the
/// selected row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowAction {
    /// Opens a terminal tab with a shell in the container.
    Shell,
    Start,
    Stop,
    Restart,
    Pause,
    Unpause,
    /// Opens the Run container form with the image filled in.
    Run,
    /// Opens the pull dialog with the image's name and tag.
    PullAgain,
    /// Asks first.
    Remove,
}

impl RowAction {
    /// Translation key of the button and the menu item.
    pub(crate) fn label_key(self) -> Key {
        match self {
            Self::Shell => Key::ContainersActionShell,
            Self::Start => Key::ContainersActionStart,
            Self::Stop => Key::ContainersActionStop,
            Self::Restart => Key::ContainersActionRestart,
            Self::Pause => Key::ContainersActionPause,
            Self::Unpause => Key::ContainersActionUnpause,
            Self::Run => Key::ContainersActionRun,
            Self::PullAgain => Key::ContainersActionPullAgain,
            Self::Remove => Key::ContainersActionRemove,
        }
    }

    /// Whether the action deletes something; it is drawn in the danger
    /// color.
    pub(crate) fn is_danger(self) -> bool {
        self == Self::Remove
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_remove_is_destructive() {
        use RowAction::*;

        for action in [Start, Stop, Restart, Pause, Unpause] {
            assert!(!action.is_danger(), "{action:?}");
        }
        assert!(Remove.is_danger());
    }
}

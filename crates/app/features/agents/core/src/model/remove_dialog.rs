use otty_app_i18n::{self as i18n, Key};

use super::super::state::AgentsState;

/// The dialog that asks before an added profile is removed: it says that
/// the config folder and its sessions stay on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RemoveDialogViewModel {
    pub(crate) title: String,
    pub(crate) text: String,
    pub(crate) confirm_label: &'static str,
}

impl RemoveDialogViewModel {
    /// Build the dialog; `None` while no removal is asked about.
    pub(crate) fn from_state(state: &AgentsState) -> Option<Self> {
        let profile = state.removing()?;
        let folder = profile.config_dir().display().to_string();

        Some(Self {
            title: i18n::filled(
                Key::TplAgentsRemoveTitle,
                &[("name", profile.name())],
            ),
            text: i18n::filled(Key::TplAgentsRemoveText, &[("path", &folder)]),
            confirm_label: i18n::t(Key::ButtonRemove),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::AgentsIntent;
    use crate::reducer::reduce;
    use crate::testing::{claude_work, connected, with_profiles};

    #[test]
    fn the_dialog_names_the_profile_and_says_its_folder_stays() {
        let mut state = with_profiles(&[(claude_work(), connected())]);
        let closed = RemoveDialogViewModel::from_state(&state);

        let _ = reduce(&mut state, AgentsIntent::RemoveProfile(claude_work()));
        let vm = RemoveDialogViewModel::from_state(&state);

        assert_eq!(closed, None);
        assert_eq!(
            vm,
            Some(RemoveDialogViewModel {
                title: i18n::filled(
                    Key::TplAgentsRemoveTitle,
                    &[("name", "Claude Code · Work")]
                ),
                text: i18n::filled(
                    Key::TplAgentsRemoveText,
                    &[("path", "/home/user/.claude-work")]
                ),
                confirm_label: i18n::t(Key::ButtonRemove),
            })
        );
    }
}

use otty_agents::{Account, FoundIn};
use otty_app_domain::folder_path::tilde_path;
use otty_app_i18n::{self as i18n, Key};

use super::super::state::AgentsState;

/// The dialog that asks before an added profile is removed: it says that
/// the config folder and its sessions stay on disk. For OTTY's own login
/// of an account it says that OTTY deletes its private folder, and that
/// the user's agents and their own logins stay as they are (issue 09 of
/// spec 2026-11-23-accounts-apart-from-profiles).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RemoveDialogViewModel {
    pub(crate) title: String,
    pub(crate) text: String,
    pub(crate) confirm_label: &'static str,
}

impl RemoveDialogViewModel {
    /// Build the dialog; `None` while no removal is asked about.
    pub(crate) fn from_state(state: &AgentsState) -> Option<Self> {
        if let Some(account) = state.removing_own_login() {
            return Some(Self::of_own_login(state, account));
        }

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

    /// The dialog that asks before the own login of `account` is removed,
    /// naming its private folders, written from `~`.
    fn of_own_login(state: &AgentsState, account: &Account) -> Self {
        let home = state.env().home();
        let folders: Vec<String> = account
            .found_in()
            .iter()
            .filter(|place| matches!(place, FoundIn::OwnLogin { .. }))
            .map(|place| tilde_path(place.folder(), home))
            .collect();

        Self {
            title: i18n::t(Key::AgentsRemoveAccountTitle).to_string(),
            text: i18n::filled(
                Key::TplAgentsRemoveAccountText,
                &[("path", &folders.join(", "))],
            ),
            confirm_label: i18n::t(Key::ButtonRemove),
        }
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
                    &[("name", "Claude Code (Work)")]
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

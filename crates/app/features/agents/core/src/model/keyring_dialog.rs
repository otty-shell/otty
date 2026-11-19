use otty_agents::KeyringEntry;
use otty_app_i18n::{self as i18n, Key};

use super::super::state::AgentsState;
use super::needs::keyring_agents;

/// The dialog that asks the user to allow OTTY to read the system keyring
/// (spec 2026-11-19, Decision 8): which agents' entries it reads, that it
/// never changes them, and that the system may ask to unlock the keyring.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KeyringDialogViewModel {
    pub(crate) title: &'static str,
    pub(crate) text: String,
    pub(crate) services: String,
}

impl KeyringDialogViewModel {
    /// Build the dialog; `None` while it is closed.
    pub(crate) fn from_state(state: &AgentsState) -> Option<Self> {
        if !state.is_keyring_dialog_open() {
            return None;
        }

        let entries = state.profiles().keyring().entries();
        let agents = keyring_agents(entries);
        let services: Vec<&str> =
            entries.iter().map(KeyringEntry::service).collect();

        Some(Self {
            title: i18n::t(Key::AgentsKeyringDialogTitle),
            text: i18n::filled(
                Key::TplAgentsKeyringDialogText,
                &[("agents", &agents)],
            ),
            services: i18n::filled(
                Key::TplAgentsKeyringService,
                &[("services", &services.join(", "))],
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use otty_agents::SavedProfiles;
    use otty_agents::fixtures::goose_keyring_entry;

    use super::*;
    use crate::event::AgentsIntent;
    use crate::need::NeedAction;
    use crate::reducer::reduce;
    use crate::testing::{
        agents, claude, connected, env, profiles_loaded_from,
    };
    use crate::types::LoadedProfiles;

    #[test]
    fn the_dialog_names_the_agents_and_the_services_it_reads() {
        let mut state = AgentsState::new(agents(), env());
        let AgentsIntent::ProfilesLoaded(loaded) = profiles_loaded_from(
            &[(claude(), connected())],
            SavedProfiles::default().with_found_report_shown(),
        ) else {
            unreachable!("a profiles read is made");
        };
        let _ = reduce(
            &mut state,
            AgentsIntent::ProfilesLoaded(LoadedProfiles {
                keyring_entries: vec![goose_keyring_entry()],
                ..loaded
            }),
        );
        let closed = KeyringDialogViewModel::from_state(&state);

        let _ =
            reduce(&mut state, AgentsIntent::FixNeed(NeedAction::AllowKeyring));
        let vm = KeyringDialogViewModel::from_state(&state);

        assert_eq!(closed, None);
        assert_eq!(
            vm,
            Some(KeyringDialogViewModel {
                title: i18n::t(Key::AgentsKeyringDialogTitle),
                text: i18n::filled(
                    Key::TplAgentsKeyringDialogText,
                    &[("agents", "Goose")]
                ),
                services: i18n::filled(
                    Key::TplAgentsKeyringService,
                    &[("services", "goose")]
                ),
            })
        );
    }
}

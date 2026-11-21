use otty_app_i18n::{self as i18n, Key};

use super::super::keyring::{keyring_part_names, keyring_services};
use super::super::state::AgentsState;

/// The dialog that asks the user to allow the plugin parts to read the
/// system keyring (spec 2026-11-19, Decision 8): which parts read which
/// entries, that they never change them, and that the system may ask to
/// unlock the keyring.
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

        let parts = keyring_part_names(state.registry()).join(", ");
        let services = keyring_services(state.registry()).join(", ");

        Some(Self {
            title: i18n::t(Key::AgentsKeyringDialogTitle),
            text: i18n::filled(
                Key::TplAgentsKeyringDialogText,
                &[("agents", &parts)],
            ),
            services: i18n::filled(
                Key::TplAgentsKeyringService,
                &[("services", &services)],
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use otty_agents::{AgentEntry, AgentId, Registry, SavedProfiles};
    use otty_plugin_host::PluginPart;

    use super::*;
    use crate::event::AgentsIntent;
    use crate::need::NeedAction;
    use crate::reducer::reduce;
    use crate::testing::{
        agents, claude, connected, env, profiles_loaded_from,
    };

    #[test]
    fn the_dialog_names_the_parts_and_the_services_they_read() {
        let part = PluginPart::new("example", "Example", "0.1.0", "", "keeper");
        let keeper = AgentEntry::new(
            AgentId::new("keeper"),
            "Keeper",
            "keeper",
            "Example",
            part,
        )
        .with_keyring_services(vec![String::from("example-keyring")]);
        let registry = Arc::new(Registry::new(vec![keeper], Vec::new()));
        let mut state = AgentsState::new(agents(), registry, env());
        let _ = reduce(
            &mut state,
            profiles_loaded_from(
                &[(claude(), connected())],
                SavedProfiles::default().with_found_report_shown(),
            ),
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
                    &[("agents", "Keeper")]
                ),
                services: i18n::filled(
                    Key::TplAgentsKeyringService,
                    &[("services", "example-keyring")]
                ),
            })
        );
    }
}

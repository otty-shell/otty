//! "Read by" on the page of an account or a profile (spec
//! 2026-11-19-agent-plugins, "Links in the interface"): the plugin and
//! its version that reads the provider or the agent, linking to its row
//! in Plugins mode; the plugin with "(off)" when its part is off; or
//! "no plugin".

use otty_agents::{Account, AgentId, ProviderRef};
use otty_app_domain::plugins::{OffPart, PartSide};
use otty_app_i18n::{self as i18n, Key};
use otty_plugin_host::PluginPart;

use super::super::state::AgentsState;
use super::FactViewModel;

/// "Read by" of an account: the provider part that found it, else a part
/// that would read it but is off, else no plugin.
pub(crate) fn account_read_by(
    state: &AgentsState,
    account: &Account,
) -> FactViewModel {
    match state.registry().reader_of(account) {
        Some(entry) => read_by(entry.part()),
        None => match off_provider_part(
            state,
            &ProviderRef::Id(account.provider().clone()),
        ) {
            Some(off) => off_read_by(off),
            None => no_plugin(),
        },
    }
}

/// "Read by" of a profile: the reader of its agent.
pub(crate) fn profile_read_by(
    state: &AgentsState,
    agent: &AgentId,
) -> FactViewModel {
    if let Some(entry) = state.registry().agent(agent) {
        return read_by(entry.part());
    }

    let off =
        state.plugin_notes().off_parts.iter().find(|off| {
            off.side == PartSide::Agent && off.part == agent.as_str()
        });
    match off {
        Some(off) => off_read_by(off),
        None => no_plugin(),
    }
}

/// The part of an installed plugin that is off and would serve
/// `provider`: by its id, or by a host it serves.
pub(crate) fn off_provider_part<'a>(
    state: &'a AgentsState,
    provider: &ProviderRef,
) -> Option<&'a OffPart> {
    state.plugin_notes().off_parts.iter().find(|off| {
        off.side == PartSide::Provider
            && match provider {
                ProviderRef::Id(id) => off.part == id.as_str(),
                ProviderRef::Host(host) => off.hosts.iter().any(|served| {
                    host == served || host.ends_with(&format!(".{served}"))
                }),
            }
    })
}

/// The plugin of `part`, as "Example 0.4.0".
pub(crate) fn plugin_name(part: &PluginPart) -> String {
    format!("{} {}", part.name(), part.version())
}

fn read_by(part: &PluginPart) -> FactViewModel {
    FactViewModel::linked(
        i18n::t(Key::AgentsFieldReadBy),
        plugin_name(part),
        part.plugin().to_string(),
    )
}

fn off_read_by(off: &OffPart) -> FactViewModel {
    let name = format!("{} {}", off.plugin_name, off.version);

    FactViewModel::linked(
        i18n::t(Key::AgentsFieldReadBy),
        i18n::filled(Key::TplAgentsReadByOff, &[("plugin", &name)]),
        off.plugin.clone(),
    )
}

fn no_plugin() -> FactViewModel {
    FactViewModel::new(
        i18n::t(Key::AgentsFieldReadBy),
        i18n::t(Key::AgentsReadByNoPlugin).to_string(),
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use otty_agents::{AgentEntry, ProviderEntry, ProviderId, Registry};
    use otty_app_domain::plugins::PluginNotes;
    use otty_plugin_host::PluginPart;

    use super::*;
    use crate::event::AgentsIntent;
    use crate::reducer::reduce;
    use crate::testing::{
        claude, connected, registry_without_tealcloud, with_profiles,
    };
    use crate::types::AgentReaders;

    fn tealcloud_part() -> OffPart {
        OffPart {
            plugin: String::from("tealcloud"),
            plugin_name: String::from("Tealcloud"),
            version: String::from("0.2.0"),
            side: PartSide::Provider,
            part: String::from("tealcloud"),
            part_name: String::from("Tealcloud"),
            hosts: vec![String::from("tealcloud.example")],
        }
    }

    /// The registry of the tests with a Tealcloud provider read by the
    /// Tealcloud plugin.
    fn registry_with_plugin() -> Arc<Registry> {
        let part = PluginPart::new(
            "tealcloud",
            "Tealcloud",
            "0.2.0",
            "https://github.com/otty-shell/otty-plugin-tealcloud",
            "tealcloud",
        );
        let entry =
            ProviderEntry::new(ProviderId::new("tealcloud"), "Tealcloud", part)
                .with_hosts(["tealcloud.example"]);
        let without = registry_without_tealcloud();
        let agents: Vec<AgentEntry> = without.agents().to_vec();
        let mut providers = vec![entry];
        providers.extend(without.providers().iter().cloned());

        Arc::new(Registry::new(agents, providers))
    }

    fn state_with(registry: Arc<Registry>, notes: PluginNotes) -> AgentsState {
        let mut state = with_profiles(&[(claude(), connected())]);
        let readers = AgentReaders {
            agents: state.agents().clone(),
            registry,
            env: state.env().clone(),
        };
        let _ = reduce(&mut state, AgentsIntent::ReadersChanged(readers));
        let _ = reduce(&mut state, AgentsIntent::PluginNotesChanged(notes));

        state
    }

    fn tealcloud_host() -> ProviderRef {
        ProviderRef::Host(String::from("api.tealcloud.example"))
    }

    /// A Tealcloud key the Tealcloud plugin found: read by it while its part
    /// is in the registry.
    fn tealcloud_key() -> Account {
        crate::testing::found_key("tealcloud")
    }

    /// Issue 09: "Read by" for a plugin, no plugin and off.
    #[test]
    fn read_by_names_the_plugin_no_plugin_or_off() {
        let plugin = state_with(registry_with_plugin(), PluginNotes::default());
        let none =
            state_with(registry_without_tealcloud(), PluginNotes::default());
        let off = state_with(
            registry_without_tealcloud(),
            PluginNotes {
                off_parts: vec![tealcloud_part()],
                waiting_updates: Vec::new(),
                broken_parts: Vec::new(),
            },
        );
        let label = i18n::t(Key::AgentsFieldReadBy);

        assert_eq!(
            account_read_by(&plugin, &tealcloud_key()),
            FactViewModel::linked(
                label,
                String::from("Tealcloud 0.2.0"),
                String::from("tealcloud")
            )
        );
        assert_eq!(
            account_read_by(&none, &tealcloud_key()),
            FactViewModel::new(
                label,
                i18n::t(Key::AgentsReadByNoPlugin).to_string()
            )
        );
        assert_eq!(
            account_read_by(&off, &tealcloud_key()),
            FactViewModel::linked(
                label,
                i18n::filled(
                    Key::TplAgentsReadByOff,
                    &[("plugin", "Tealcloud 0.2.0")]
                ),
                String::from("tealcloud")
            )
        );
    }

    #[test]
    fn a_profile_is_read_by_the_part_of_its_agent() {
        let state =
            state_with(crate::testing::registry(), PluginNotes::default());

        let fact = profile_read_by(&state, &crate::testing::claude_id());

        assert_eq!(fact.value, "Anthropic 0.1.0");
    }

    #[test]
    fn an_off_part_serves_a_host_under_its_own() {
        let state = state_with(
            registry_without_tealcloud(),
            PluginNotes {
                off_parts: vec![tealcloud_part()],
                waiting_updates: Vec::new(),
                broken_parts: Vec::new(),
            },
        );

        assert!(off_provider_part(&state, &tealcloud_host()).is_some());
        assert!(
            off_provider_part(
                &state,
                &ProviderRef::Host(String::from("nottealcloud.example"))
            )
            .is_none()
        );
    }
}

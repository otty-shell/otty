//! The registry (spec "Words", "Links in the interface"): the one list of
//! the agents and providers OTTY knows right now. The app takes names,
//! icons, usage pages and hosts only from it. The parts of the installed
//! plugins that are on fill it. Two plugins may serve one provider: each
//! part finds its own accounts (spec
//! 2026-11-23-accounts-apart-from-profiles, Decision 1).

use crate::account::Account;
use crate::agent::{Agent, AgentEntry, AgentId};
use crate::plugin_provider::PluginProvider;
use crate::provider::{ProviderEntry, ProviderId, ProviderRef};

/// The agents and providers OTTY knows right now: one entry per agent id,
/// and one per provider part, so a provider two plugins serve has two
/// entries (issue 29).
///
/// ```
/// use otty_agents::{ProviderEntry, ProviderId, ProviderRef, Registry};
/// use otty_plugin_host::PluginPart;
///
/// let part = PluginPart::new("example", "Example", "0.1.0", "", "cloud");
/// let cloud = ProviderEntry::new(ProviderId::new("cloud"), "Cloud", part)
///     .with_hosts(["example.com"]);
/// let registry = Registry::new(Vec::new(), vec![cloud]);
///
/// let host = ProviderRef::Host("api.example.com".into());
///
/// assert!(registry.is_same_provider(&host, &ProviderRef::of_id("cloud")));
/// assert_eq!(
///     registry.provider_of(&host).map(|entry| entry.name()),
///     Some("Cloud")
/// );
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Registry {
    agents: Vec<AgentEntry>,
    providers: Vec<ProviderEntry>,
}

impl Registry {
    /// The agents, in the order they are shown.
    pub fn agents(&self) -> &[AgentEntry] {
        &self.agents
    }

    /// The providers, in the order they are shown.
    pub fn providers(&self) -> &[ProviderEntry] {
        &self.providers
    }

    /// The agent with the id `id`.
    pub fn agent(&self, id: &AgentId) -> Option<&AgentEntry> {
        self.agents.iter().find(|entry| entry.id() == id)
    }

    /// The provider with the id `id`: the first part that serves it.
    pub fn provider(&self, id: &ProviderId) -> Option<&ProviderEntry> {
        self.providers.iter().find(|entry| entry.id() == id)
    }

    /// Every provider part that serves `reference`, in the order of the
    /// registry: the parts with its id, or every part that serves its
    /// host, whatever its id (issue 31).
    pub fn readers_of(&self, reference: &ProviderRef) -> Vec<&ProviderEntry> {
        let host = reference
            .as_host()
            .map(|host| host.trim().to_ascii_lowercase());

        self.providers
            .iter()
            .filter(|entry| match &host {
                Some(host) => entry.serves_host(host),
                None => reference.as_id() == Some(entry.id()),
            })
            .collect()
    }

    /// The provider part that reads `account`: the part that found it, of
    /// its plugin and its provider id; `None` when that part is not in the
    /// registry now.
    pub fn reader_of(&self, account: &Account) -> Option<&ProviderEntry> {
        self.providers.iter().find(|entry| {
            entry.id() == account.provider()
                && entry.part().plugin() == account.plugin()
        })
    }

    /// The first provider whose servers are at `host`: `host` is one of its
    /// domains or ends with `.` and one. It names and draws a host; the
    /// accounts known by a host are read by every part that serves it
    /// ([`Registry::readers_of`]).
    pub fn provider_of_host(&self, host: &str) -> Option<&ProviderEntry> {
        let host = host.trim().to_ascii_lowercase();

        self.providers.iter().find(|entry| entry.serves_host(&host))
    }

    /// The provider `reference` names: by its id, or the one that serves
    /// its host.
    pub fn provider_of(
        &self,
        reference: &ProviderRef,
    ) -> Option<&ProviderEntry> {
        match reference {
            ProviderRef::Id(id) => self.provider(id),
            ProviderRef::Host(host) => self.provider_of_host(host),
        }
    }

    /// The name the price table knows the provider `reference` by (issue
    /// 23): the name its provider part gives, else the id the agent
    /// wrote, even when no part serves it. `None` for the host of a
    /// server no part serves.
    pub fn price_name_of<'a>(
        &'a self,
        reference: &'a ProviderRef,
    ) -> Option<&'a str> {
        match self.provider_of(reference) {
            Some(entry) => Some(entry.price_name()),
            None => reference.as_id().map(ProviderId::as_str),
        }
    }

    /// Whether `one` and `other` name the same provider: the same
    /// reference, or a part serves both, as a host is the provider of each
    /// id whose part serves it (issue 31). Two ids are two providers.
    pub fn is_same_provider(
        &self,
        one: &ProviderRef,
        other: &ProviderRef,
    ) -> bool {
        if one == other {
            return true;
        }

        let ids_of_one: Vec<&ProviderId> = self
            .readers_of(one)
            .into_iter()
            .map(ProviderEntry::id)
            .collect();
        self.readers_of(other)
            .into_iter()
            .any(|entry| ids_of_one.contains(&entry.id()))
    }

    /// The page where the user sees the use of `account`, from the entry
    /// of the part that reads it; `None` when that part is not in the
    /// registry or names no page.
    pub fn usage_page(&self, account: &Account) -> Option<&str> {
        self.reader_of(account)?.usage_page(account.kind())
    }

    /// The readers of `agents` whose entry is in the registry, in their
    /// order: the profiles of any other agent leave the lists, while what
    /// `agents.json` keeps for them stays.
    pub fn listed_agents(
        &self,
        agents: Vec<Box<dyn Agent>>,
    ) -> Vec<Box<dyn Agent>> {
        agents
            .into_iter()
            .filter(|agent| {
                self.agent(agent.entry().id()) == Some(agent.entry())
            })
            .collect()
    }

    /// A registry of `agents` and `providers`. An agent whose id an
    /// earlier agent has is left out, so each agent is read once; every
    /// provider part stays, except one the same plugin lists twice.
    pub fn new(agents: Vec<AgentEntry>, providers: Vec<ProviderEntry>) -> Self {
        let mut registry = Self::default();

        for agent in agents {
            if registry.agent(agent.id()).is_none() {
                registry.agents.push(agent);
            }
        }

        for provider in providers {
            let is_listed = registry.providers.iter().any(|kept| {
                kept.id() == provider.id()
                    && kept.part().plugin() == provider.part().plugin()
            });
            if !is_listed {
                registry.providers.push(provider);
            }
        }

        registry
    }

    /// The registry of the agent parts `agents` and the provider parts
    /// `providers` that are on (spec Decision 4): each serves its ids and
    /// hosts.
    pub fn of_readers(
        agents: &[Box<dyn Agent>],
        providers: &[PluginProvider],
    ) -> Self {
        let agents = agents.iter().map(|agent| agent.entry().clone());
        let providers =
            providers.iter().map(|provider| provider.entry().clone());

        Self::new(agents.collect(), providers.collect())
    }

    /// The same registry without the agent `id`, as when its part is off.
    pub fn without_agent(mut self, id: &AgentId) -> Self {
        self.agents.retain(|entry| entry.id() != id);
        self
    }

    /// The same registry without the provider `id`, as when its parts
    /// are off.
    pub fn without_provider(mut self, id: &ProviderId) -> Self {
        self.providers.retain(|entry| entry.id() != id);
        self
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use otty_plugin_host::PluginPart;

    use super::*;
    use crate::env::AgentEnv;
    use crate::profile::Profile;
    use crate::profiles;
    use crate::saved_profiles::SavedProfiles;
    use crate::testing::{made_up_part, test_agents};

    fn provider(id: &str, hosts: &[&str]) -> ProviderEntry {
        ProviderEntry::new(ProviderId::new(id), id, made_up_part(id))
            .with_hosts(hosts.iter().copied())
    }

    fn agent(id: &str, name: &str) -> AgentEntry {
        AgentEntry::new(AgentId::new(id), name, id, "Maker", made_up_part(id))
    }

    /// The registry of the agents of the tests and two made-up providers.
    fn registry() -> Registry {
        let agents = test_agents()
            .iter()
            .map(|agent| agent.entry().clone())
            .collect();

        Registry::new(
            agents,
            vec![
                provider("example-cloud", &["example.com", "example.net"]),
                provider("other-cloud", &["other.example.org"]),
            ],
        )
    }

    #[test]
    fn an_agent_and_a_provider_are_found_by_their_ids() {
        let registry = registry();

        let agent = registry.agent(&AgentId::new("coder"));
        let cloud = registry.provider(&ProviderId::new("example-cloud"));

        assert_eq!(agent.map(AgentEntry::name), Some("Coder"));
        assert_eq!(cloud.map(ProviderEntry::name), Some("example-cloud"));
        assert!(registry.agent(&AgentId::new("cursor")).is_none());
        assert!(registry.provider(&ProviderId::new("mistral")).is_none());
    }

    /// Issue 23: the price table knows a provider by its id, by the name
    /// its part gives, or, with no part, by the name the agent wrote; a
    /// server no part serves has no name there.
    #[test]
    fn a_provider_is_priced_by_its_id_the_name_its_part_gives_or_as_written() {
        let registry = Registry::new(
            Vec::new(),
            vec![
                provider("example-cloud", &["example.com"]),
                provider("example-google", &["example.org"])
                    .with_priced_as("example-gemini"),
            ],
        );
        let cases = [
            (ProviderRef::of_id("example-cloud"), Some("example-cloud")),
            (
                ProviderRef::Host("api.example.com".to_string()),
                Some("example-cloud"),
            ),
            (ProviderRef::of_id("example-google"), Some("example-gemini")),
            (
                ProviderRef::Host("api.example.org".to_string()),
                Some("example-gemini"),
            ),
            (ProviderRef::of_id("hopway"), Some("hopway")),
            (ProviderRef::Host("llm.example.net".to_string()), None),
        ];

        for (provider, name) in cases {
            assert_eq!(registry.price_name_of(&provider), name, "{provider:?}");
        }
    }

    /// A host is a provider's when it is one of its hosts or ends with `.`
    /// and one.
    #[test]
    fn a_provider_is_found_by_the_host_of_its_server() {
        let registry = registry();
        let cases = [
            ("api.example.com", "example-cloud"),
            ("example.net", "example-cloud"),
            ("API.Example.net", "example-cloud"),
            ("other.example.org", "other-cloud"),
        ];

        for (host, id) in cases {
            let found = registry.provider_of_host(host);

            assert_eq!(
                found.map(|entry| entry.id().as_str()),
                Some(id),
                "{host}"
            );
        }
        for host in ["gateway.example.org", "example.com.example.org", ""] {
            assert!(registry.provider_of_host(host).is_none(), "{host}");
        }
    }

    #[test]
    fn a_host_is_the_same_provider_as_the_id_of_an_entry_that_serves_it() {
        let registry = registry();
        let cloud = ProviderRef::Host(String::from("api.example.com"));
        let gateway = ProviderRef::Host(String::from("gateway.example.org"));
        let by_id = ProviderRef::of_id("example-cloud");

        assert!(registry.is_same_provider(&cloud, &by_id));
        assert!(registry.is_same_provider(&by_id, &cloud));
        assert!(registry.is_same_provider(&gateway, &gateway));
        assert!(!registry.is_same_provider(&gateway, &by_id));
        assert!(
            !registry
                .is_same_provider(&cloud, &ProviderRef::of_id("other-cloud"))
        );

        let without =
            registry.without_provider(&ProviderId::new("example-cloud"));
        assert!(!without.is_same_provider(&cloud, &by_id));
    }

    /// Issue 31: a host two plugins serve under two provider ids is the
    /// same provider as each id, while the two ids stay two providers.
    #[test]
    fn a_host_two_plugins_serve_under_two_ids_is_each_of_them() {
        let fork = ProviderEntry::new(
            ProviderId::new("cloud-fork"),
            "Cloud Fork",
            PluginPart::new("fork", "Fork", "0.1.0", "", "cloud-fork"),
        )
        .with_hosts(["api.example.com"]);
        let registry = Registry::new(
            Vec::new(),
            vec![
                provider("example-cloud", &["example.com"]),
                fork,
                provider("other-cloud", &["example.net"]),
            ],
        );
        let host = ProviderRef::Host(String::from("api.example.com"));
        let cloud = ProviderRef::of_id("example-cloud");
        let fork = ProviderRef::of_id("cloud-fork");

        let readers: Vec<_> = registry
            .readers_of(&host)
            .iter()
            .map(|entry| entry.id().as_str())
            .collect();
        assert_eq!(readers, ["example-cloud", "cloud-fork"]);
        assert!(registry.is_same_provider(&host, &cloud));
        assert!(registry.is_same_provider(&host, &fork));
        assert!(!registry.is_same_provider(&cloud, &fork));
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decision 1: an
    /// account is read by the part that found it, of its plugin and its
    /// provider id; with that part gone it has no reader, so nothing is
    /// read for it.
    #[test]
    fn each_account_is_read_by_the_part_that_found_it() {
        let fork = ProviderEntry::new(
            ProviderId::new("anthropic"),
            "Anthropic fork",
            PluginPart::new("fork", "Fork", "0.1.0", "", "anthropic"),
        );
        let registry = Registry::new(
            Vec::new(),
            vec![
                fork,
                provider("anthropic", &["anthropic.com"]),
                provider("openai", &["openai.com"]),
            ],
        );
        let without = registry
            .clone()
            .without_provider(&ProviderId::new("openai"));
        let reader = |account: &Account, registry: &Registry| {
            registry
                .reader_of(account)
                .map(|entry| entry.part().plugin().to_string())
        };
        let login = crate::testing::chatgpt_login("made-up");
        let organization = crate::testing::organization("made-up-org");

        assert_eq!(reader(&login, &registry).as_deref(), Some("example"));
        assert_eq!(
            reader(&organization, &registry).as_deref(),
            Some("example")
        );
        assert_eq!(reader(&login, &without), None);
    }

    /// One agent entry per agent id, the first one; every provider part
    /// stays, so two plugins that serve one provider each find their own
    /// accounts.
    #[test]
    fn an_agent_is_read_once_and_every_provider_part_stays() {
        let fork = |id: &str, hosts: &[&str]| {
            let part = PluginPart::new("fork", "Fork", "0.1.0", "", id);
            ProviderEntry::new(ProviderId::new(id), id, part)
                .with_hosts(hosts.iter().copied())
        };
        let registry = Registry::new(
            vec![agent("coder", "Coder"), agent("coder", "Other Coder")],
            vec![
                provider("example-cloud", &["example.com"]),
                provider("example-cloud", &["example.com"]),
                fork("example-cloud", &["example.com"]),
                provider("other-cloud", &["example.net"]),
            ],
        );

        let names: Vec<_> =
            registry.agents().iter().map(AgentEntry::name).collect();
        let readers: Vec<_> = registry
            .readers_of(&ProviderRef::Host("api.example.com".to_string()))
            .iter()
            .map(|entry| entry.part().plugin())
            .collect();
        assert_eq!(names, ["Coder"]);
        assert_eq!(registry.providers().len(), 3);
        assert_eq!(readers, ["example", "fork"]);
    }

    /// Spec "What the UI shows": the profiles of an agent with no entry
    /// leave the lists; its reader is not used.
    #[test]
    fn only_the_agents_with_an_entry_are_listed() {
        let made_up = || -> Box<dyn Agent> {
            let mut agent = crate::testing::MockAgent::new();
            agent
                .expect_entry()
                .return_const(crate::testing::mock_entry("test-agent", "test"));
            Box::new(agent)
        };
        let mut agents = test_agents();
        agents.push(made_up());
        let registry = Registry::of_readers(&agents, &[])
            .without_agent(&AgentId::new("test-agent"));
        let mut listed_agents = test_agents();
        listed_agents.push(made_up());

        let listed: Vec<_> = registry
            .listed_agents(listed_agents)
            .iter()
            .map(|agent| agent.entry().id().as_str().to_string())
            .collect();

        assert_eq!(listed, ["coder"]);
    }

    /// Spec "What the UI shows": with no entry for an agent its profiles
    /// leave the lists, while its saved name and hidden profile stay in
    /// `agents.json` and come back with the entry.
    #[test]
    fn the_saved_profiles_of_an_agent_come_back_with_its_entry() {
        let agent = AgentId::new("coder");
        let work = Profile::added(
            agent.clone(),
            "Coder (Work)",
            PathBuf::from("/no/such/home/.coder-work"),
        );
        let old = Profile::added(
            agent.clone(),
            "Coder (Old)",
            PathBuf::from("/no/such/home/.coder-old"),
        );
        let saved = SavedProfiles::default()
            .with_added(work)
            .with_added(old.clone())
            .with_profile_hidden(&old, true);
        let env = AgentEnv::new(PathBuf::from("/no/such/home"));
        let names = |registry: &Registry| {
            let agents = registry.listed_agents(test_agents());
            profiles(&agents, &env, &saved, |_| true)
                .iter()
                .filter(|profile| *profile.agent() == agent)
                .map(|profile| profile.name().to_string())
                .collect::<Vec<_>>()
        };

        let without = registry().without_agent(&agent);

        assert!(names(&without).is_empty());
        assert!(saved.is_profile_hidden(&old));
        assert_eq!(
            names(&registry()),
            ["Coder", "Coder (Work)", "Coder (Old)"]
        );
    }
}

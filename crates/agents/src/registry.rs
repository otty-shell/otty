//! The registry (spec "Words", "Links in the interface"): the one list of
//! the agents and providers OTTY knows right now. The app takes names,
//! icons, usage pages and hosts only from it, and an account whose
//! provider no entry serves stays known by the host of its key. The parts
//! of the installed plugins that are on fill it.

use otty_plugin_host::Manifest;

use crate::account::Account;
use crate::agent::{Agent, AgentEntry, AgentId};
use crate::plugin_provider::PluginProvider;
use crate::provider::{ProviderEntry, ProviderId, ProviderRef};

/// One part of a plugin that reads an agent or a provider: the plugin's
/// id, name, version and repository, and the id of the part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginPart {
    plugin: String,
    name: String,
    version: String,
    repository: String,
    part: String,
}

impl PluginPart {
    /// The id of the plugin, such as `example`.
    pub fn plugin(&self) -> &str {
        &self.plugin
    }

    /// The name of the plugin, such as "Example".
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The version of the plugin, such as `0.4.0`.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// The git repository of the plugin.
    pub fn repository(&self) -> &str {
        &self.repository
    }

    /// The id of the part in the plugin, such as `coder`.
    pub fn part(&self) -> &str {
        &self.part
    }

    /// The part `part` of the plugin `plugin`, named `name`, at `version`
    /// from `repository`.
    ///
    /// ```
    /// use otty_agents::PluginPart;
    ///
    /// let coder = PluginPart::new(
    ///     "example",
    ///     "Example",
    ///     "0.4.0",
    ///     "https://github.com/otty-shell/otty-plugin-example",
    ///     "coder",
    /// );
    ///
    /// assert_eq!(coder.part(), "coder");
    /// ```
    pub fn new(
        plugin: impl Into<String>,
        name: impl Into<String>,
        version: impl Into<String>,
        repository: impl Into<String>,
        part: impl Into<String>,
    ) -> Self {
        Self {
            plugin: plugin.into(),
            name: name.into(),
            version: version.into(),
            repository: repository.into(),
            part: part.into(),
        }
    }

    /// The part `part` of the plugin of `manifest`; a plugin with no
    /// repository, such as a local one, has an empty one.
    pub fn of_manifest(manifest: &Manifest, part: &str) -> Self {
        let plugin = manifest.plugin();

        Self::new(
            plugin.id(),
            plugin.name(),
            plugin.version().to_string(),
            plugin.repository().unwrap_or_default(),
            part,
        )
    }

    /// The lines a report names the part with: the plugin, its version,
    /// its repository and the part. Never anything the part read.
    pub(crate) fn report_lines(&self) -> [String; 3] {
        [
            format!("Plugin: {} {}", self.plugin, self.version),
            format!("Repository: {}", self.repository),
            format!("Part: {}", self.part),
        ]
    }
}

/// The agents and providers OTTY knows right now, with one entry per agent
/// id, provider id and provider host (spec Decision 4).
///
/// ```
/// use otty_agents::{
///     PluginPart, ProviderEntry, ProviderId, ProviderRef, Registry,
/// };
///
/// let part = PluginPart::new("example", "Example", "0.1.0", "", "cloud");
/// let cloud = ProviderEntry::new(ProviderId::new("cloud"), "Cloud", part)
///     .with_hosts(["example.com"]);
/// let registry = Registry::new(Vec::new(), vec![cloud]);
///
/// let host = ProviderRef::Host("api.example.com".into());
///
/// assert_eq!(registry.resolve(&host), ProviderRef::of_id("cloud"));
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

    /// The provider with the id `id`.
    pub fn provider(&self, id: &ProviderId) -> Option<&ProviderEntry> {
        self.providers.iter().find(|entry| entry.id() == id)
    }

    /// The provider whose servers are at `host`: `host` is one of its
    /// domains or ends with `.` and one.
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

    /// `reference` with a host turned into the id of the provider that
    /// serves it; any other reference as it is.
    pub fn resolve(&self, reference: &ProviderRef) -> ProviderRef {
        match reference {
            ProviderRef::Host(host) => match self.provider_of_host(host) {
                Some(entry) => ProviderRef::Id(entry.id().clone()),
                None => reference.clone(),
            },
            ProviderRef::Id(_) => reference.clone(),
        }
    }

    /// Whether `one` and `other` name the same provider, once each host is
    /// turned into the id of the provider that serves it.
    pub fn is_same_provider(
        &self,
        one: &ProviderRef,
        other: &ProviderRef,
    ) -> bool {
        one == other || self.resolve(one) == self.resolve(other)
    }

    /// The providers Add account offers, in the order of the registry.
    pub fn addable_providers(&self) -> Vec<&ProviderEntry> {
        self.providers
            .iter()
            .filter(|entry| entry.can_add_by_hand())
            .collect()
    }

    /// The page where the user sees the use of `account`, from the entry
    /// of its provider; `None` when no entry serves the provider or names
    /// a page.
    pub fn usage_page(&self, account: &Account) -> Option<&str> {
        self.provider_of(account.provider())?
            .usage_page(account.kind())
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

    /// A registry of `agents` and `providers`. An entry whose agent id,
    /// provider id or one of whose hosts an earlier entry already has is
    /// left out, so each is served once.
    pub fn new(agents: Vec<AgentEntry>, providers: Vec<ProviderEntry>) -> Self {
        let mut registry = Self::default();

        for agent in agents {
            if registry.agent(agent.id()).is_none() {
                registry.agents.push(agent);
            }
        }

        for provider in providers {
            let is_taken = registry.provider(provider.id()).is_some()
                || provider
                    .hosts()
                    .iter()
                    .any(|host| registry.provider_of_host(host).is_some());
            if !is_taken {
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

    /// The same registry without the provider `id`, as when its part is
    /// off.
    pub fn without_provider(mut self, id: &ProviderId) -> Self {
        self.providers.retain(|entry| entry.id() != id);
        self
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

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
    fn a_host_some_entry_serves_is_its_id_and_any_other_host_stays() {
        let registry = registry();
        let cloud = ProviderRef::Host(String::from("api.example.com"));
        let gateway = ProviderRef::Host(String::from("gateway.example.org"));

        assert_eq!(
            registry.resolve(&cloud),
            ProviderRef::of_id("example-cloud")
        );
        assert_eq!(registry.resolve(&gateway), gateway);
        assert!(
            registry
                .is_same_provider(&cloud, &ProviderRef::of_id("example-cloud"))
        );

        let without =
            registry.without_provider(&ProviderId::new("example-cloud"));
        assert_eq!(without.resolve(&cloud), cloud);
        assert!(
            !without
                .is_same_provider(&cloud, &ProviderRef::of_id("example-cloud"))
        );
    }

    /// Spec Decision 4: one entry per agent id, provider id and provider
    /// host; the first one stays.
    #[test]
    fn each_id_and_host_is_served_once_by_the_first_entry() {
        let registry = Registry::new(
            vec![agent("coder", "Coder"), agent("coder", "Other Coder")],
            vec![
                provider("example-cloud", &["example.com"]),
                provider("example-cloud", &["other.example.org"]),
                provider("example-mirror", &["example.com"]),
                provider("other-cloud", &["example.net"]),
            ],
        );

        let names: Vec<_> =
            registry.agents().iter().map(AgentEntry::name).collect();
        let ids: Vec<_> = registry
            .providers()
            .iter()
            .map(|entry| entry.id().as_str())
            .collect();
        assert_eq!(names, ["Coder"]);
        assert_eq!(ids, ["example-cloud", "other-cloud"]);
    }

    /// Spec 2026-11-19, Decision 7: Add account lists the providers whose
    /// part offers it, in their order.
    #[test]
    fn add_account_offers_the_providers_whose_part_offers_it() {
        let registry = Registry::new(
            Vec::new(),
            vec![
                provider("org-cloud", &["org.example.com"]).with_add_by_hand(),
                provider("plain-cloud", &["plain.example.com"]),
                provider("key-cloud", &["key.example.com"]).with_add_by_hand(),
            ],
        );

        let offered: Vec<_> = registry
            .addable_providers()
            .iter()
            .map(|entry| entry.id().as_str())
            .collect();

        assert_eq!(offered, ["org-cloud", "key-cloud"]);
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
            "Coder · Work",
            PathBuf::from("/no/such/home/.coder-work"),
        );
        let old = Profile::added(
            agent.clone(),
            "Coder · Old",
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
            ["Coder", "Coder · Work", "Coder · Old"]
        );
    }
}

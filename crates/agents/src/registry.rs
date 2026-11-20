//! The registry (spec "Words", "Links in the interface"): the one list of
//! the agents and providers OTTY knows right now. The app takes names,
//! icons, usage pages and hosts only from it, and an account whose
//! provider no entry serves stays known by the host of its key.
//!
//! Until the plugin engine fills it, the readers of this crate put their
//! entries in it as "built in" ones ([`Registry::built_in`]).

use crate::account::Account;
use crate::agent::{Agent, AgentEntry, AgentId};
use crate::built_in;
use crate::provider::{AddAccountKind, ProviderEntry, ProviderId, ProviderRef};

/// What reads an agent or a provider: the code of this crate, or one part
/// of an installed plugin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reader {
    /// The reader of this crate, which runs until its plugin replaces it.
    BuiltIn,
    /// A part of a plugin.
    Plugin(PluginPart),
}

/// One part of a plugin that reads an agent or a provider: the plugin's
/// id, version and repository, and the id of the part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginPart {
    plugin: String,
    version: String,
    repository: String,
    part: String,
}

impl PluginPart {
    /// The id of the plugin, such as `openai`.
    pub fn plugin(&self) -> &str {
        &self.plugin
    }

    /// The version of the plugin, such as `0.4.0`.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// The git repository of the plugin.
    pub fn repository(&self) -> &str {
        &self.repository
    }

    /// The id of the part in the plugin, such as `codex`.
    pub fn part(&self) -> &str {
        &self.part
    }

    /// The part `part` of the plugin `plugin` at `version` from
    /// `repository`.
    pub fn new(
        plugin: impl Into<String>,
        version: impl Into<String>,
        repository: impl Into<String>,
        part: impl Into<String>,
    ) -> Self {
        Self {
            plugin: plugin.into(),
            version: version.into(),
            repository: repository.into(),
            part: part.into(),
        }
    }
}

/// The agents and providers OTTY knows right now, with one entry per agent
/// id, provider id and provider host (spec Decision 4).
///
/// ```
/// use otty_agents::{ProviderRef, Registry, known_agents};
///
/// let registry = Registry::built_in(&known_agents());
///
/// let deepseek = ProviderRef::Host("api.deepseek.com".into());
///
/// assert_eq!(registry.resolve(&deepseek), ProviderRef::of_id("deepseek"));
/// assert_eq!(
///     registry.provider_of(&deepseek).map(|entry| entry.name()),
///     Some("DeepSeek")
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

    /// The providers Add account offers, in the order the dialog lists
    /// them: the keys with money left first, then the organizations whose
    /// bill an admin key reads.
    pub fn addable_providers(&self) -> Vec<&ProviderEntry> {
        let of_kind = |kind: AddAccountKind| {
            self.providers
                .iter()
                .filter(move |entry| entry.add_account() == Some(kind))
        };

        of_kind(AddAccountKind::Key)
            .chain(of_kind(AddAccountKind::AdminKey))
            .collect()
    }

    /// The page where the user sees the use of `account`, from the entry
    /// of its provider; `None` when no entry serves the provider or names
    /// a page.
    pub fn usage_page(&self, account: &Account) -> Option<&str> {
        self.provider_of(account.provider())?
            .usage_page(account.kind())
    }

    /// The readers of `agents` whose agent has an entry, in their order:
    /// the profiles of any other agent leave the lists, while what
    /// `agents.json` keeps for them stays.
    pub fn listed_agents(
        &self,
        agents: Vec<Box<dyn Agent>>,
    ) -> Vec<Box<dyn Agent>> {
        agents
            .into_iter()
            .filter(|agent| self.agent(agent.entry().id()).is_some())
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

    /// The registry of the readers of this crate: the entries of `agents`
    /// and of the providers it reads, all built in.
    pub fn built_in(agents: &[Box<dyn Agent>]) -> Self {
        let entries = agents.iter().map(|agent| agent.entry().clone());

        Self::new(entries.collect(), built_in::providers())
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
    use crate::saved_profiles::SavedProfiles;
    use crate::{known_agents, profiles};

    fn provider(id: &str, hosts: &[&str]) -> ProviderEntry {
        ProviderEntry::new(ProviderId::new(id), id, Reader::BuiltIn)
            .with_hosts(hosts.iter().copied())
    }

    fn agent(id: &str, name: &str) -> AgentEntry {
        AgentEntry::new(AgentId::new(id), name, id, "Maker", Reader::BuiltIn)
    }

    fn built_in() -> Registry {
        Registry::built_in(&known_agents())
    }

    #[test]
    fn an_agent_and_a_provider_are_found_by_their_ids() {
        let registry = built_in();

        let codex = registry.agent(&AgentId::new("codex"));
        let openai = registry.provider(&ProviderId::new("openai"));

        assert_eq!(codex.map(AgentEntry::name), Some("Codex"));
        assert_eq!(openai.map(ProviderEntry::name), Some("OpenAI"));
        assert!(registry.agent(&AgentId::new("cursor")).is_none());
        assert!(registry.provider(&ProviderId::new("mistral")).is_none());
    }

    /// Today's `PROVIDER_DOMAINS` rule: a host is a provider's when it is
    /// one of its hosts or ends with `.` and one.
    #[test]
    fn a_provider_is_found_by_the_host_of_its_server() {
        let registry = built_in();
        let cases = [
            ("api.anthropic.com", "anthropic"),
            ("api.deepseek.com", "deepseek"),
            ("openrouter.ai", "openrouter"),
            ("api.moonshot.ai", "moonshot"),
            ("api.moonshot.cn", "moonshot"),
            ("api.kilo.ai", "kilo"),
            ("api.openai.com", "openai"),
            ("API.DeepSeek.com", "deepseek"),
        ];

        for (host, id) in cases {
            let found = registry.provider_of_host(host);

            assert_eq!(
                found.map(|entry| entry.id().as_str()),
                Some(id),
                "{host}"
            );
        }
        for host in ["gateway.example.com", "deepseek.com.example.com", ""] {
            assert!(registry.provider_of_host(host).is_none(), "{host}");
        }
    }

    #[test]
    fn a_host_some_entry_serves_is_its_id_and_any_other_host_stays() {
        let registry = built_in();
        let deepseek = ProviderRef::Host(String::from("api.deepseek.com"));
        let gateway = ProviderRef::Host(String::from("gateway.example.com"));

        assert_eq!(registry.resolve(&deepseek), ProviderRef::of_id("deepseek"));
        assert_eq!(registry.resolve(&gateway), gateway);
        assert!(
            registry
                .is_same_provider(&deepseek, &ProviderRef::of_id("deepseek"))
        );

        let without = registry.without_provider(&ProviderId::new("deepseek"));
        assert_eq!(without.resolve(&deepseek), deepseek);
        assert!(
            !without
                .is_same_provider(&deepseek, &ProviderRef::of_id("deepseek"))
        );
    }

    /// Spec Decision 4: one entry per agent id, provider id and provider
    /// host; the first one stays.
    #[test]
    fn each_id_and_host_is_served_once_by_the_first_entry() {
        let registry = Registry::new(
            vec![agent("codex", "Codex"), agent("codex", "Other Codex")],
            vec![
                provider("deepseek", &["deepseek.com"]),
                provider("deepseek", &["other.example.com"]),
                provider("deepseek-mirror", &["deepseek.com"]),
                provider("openrouter", &["openrouter.ai"]),
            ],
        );

        let names: Vec<_> =
            registry.agents().iter().map(AgentEntry::name).collect();
        let ids: Vec<_> = registry
            .providers()
            .iter()
            .map(|entry| entry.id().as_str())
            .collect();
        assert_eq!(names, ["Codex"]);
        assert_eq!(ids, ["deepseek", "openrouter"]);
    }

    /// Spec 2026-11-19, Decision 7: Add account lists the keys with money
    /// left, then the organizations an admin key reads the bill of.
    #[test]
    fn add_account_offers_the_keys_then_the_organizations() {
        let registry = built_in();

        let offered: Vec<_> = registry
            .addable_providers()
            .iter()
            .map(|entry| (entry.id().as_str(), entry.add_account()))
            .collect();

        assert_eq!(
            offered,
            [
                ("deepseek", Some(AddAccountKind::Key)),
                ("openrouter", Some(AddAccountKind::Key)),
                ("moonshot", Some(AddAccountKind::Key)),
                ("anthropic", Some(AddAccountKind::AdminKey)),
                ("openai", Some(AddAccountKind::AdminKey)),
            ]
        );
    }

    #[test]
    fn the_built_in_entries_have_todays_names_and_icons() {
        let registry = built_in();

        let agents: Vec<_> = registry
            .agents()
            .iter()
            .map(|entry| {
                (entry.id().as_str(), entry.name(), entry.icon().is_some())
            })
            .collect();
        let providers: Vec<_> = registry
            .providers()
            .iter()
            .map(|entry| {
                (entry.id().as_str(), entry.name(), entry.icon().is_some())
            })
            .collect();

        assert_eq!(
            agents,
            [
                ("claude-code", "Claude Code", true),
                ("codex", "Codex", true),
                ("kilo", "Kilo", true),
                ("opencode", "OpenCode", true),
            ]
        );
        assert_eq!(
            providers,
            [
                ("anthropic", "Anthropic", true),
                ("openai", "OpenAI", true),
                ("deepseek", "DeepSeek", true),
                ("openrouter", "OpenRouter", true),
                ("moonshot", "Moonshot", true),
                ("kilo", "Kilo", true),
            ]
        );
        assert!(
            registry
                .agents()
                .iter()
                .all(|entry| entry.reader() == &Reader::BuiltIn)
        );
        assert!(
            registry
                .providers()
                .iter()
                .all(|entry| entry.reader() == &Reader::BuiltIn)
        );
    }

    /// Spec "What the UI shows": the profiles of an agent with no entry
    /// leave the lists; its reader is not used.
    #[test]
    fn only_the_agents_with_an_entry_are_listed() {
        let registry = built_in().without_agent(&AgentId::new("kilo"));

        let listed: Vec<_> = registry
            .listed_agents(known_agents())
            .iter()
            .map(|agent| agent.entry().id().as_str().to_string())
            .collect();

        assert_eq!(listed, ["claude-code", "codex", "opencode"]);
    }

    /// Spec "What the UI shows": with no Codex entry its profiles leave
    /// the lists, while its saved name and hidden profile stay in
    /// `agents.json` and come back with the entry.
    #[test]
    fn the_saved_profiles_of_an_agent_come_back_with_its_entry() {
        let codex = AgentId::new("codex");
        let work = Profile::added(
            codex.clone(),
            "Codex · Work",
            PathBuf::from("/no/such/home/.codex-work"),
        );
        let old = Profile::added(
            codex.clone(),
            "Codex · Old",
            PathBuf::from("/no/such/home/.codex-old"),
        );
        let saved = SavedProfiles::default()
            .with_added(work)
            .with_added(old.clone())
            .with_profile_hidden(&old, true);
        let env = AgentEnv::new(PathBuf::from("/no/such/home"));
        let names = |registry: &Registry| {
            let agents = registry.listed_agents(known_agents());
            profiles(&agents, &env, &saved, |_| true)
                .iter()
                .filter(|profile| *profile.agent() == codex)
                .map(|profile| profile.name().to_string())
                .collect::<Vec<_>>()
        };

        let without = built_in().without_agent(&codex);

        assert!(names(&without).is_empty());
        assert!(saved.is_profile_hidden(&old));
        assert_eq!(
            names(&built_in()),
            ["Codex", "Codex · Work", "Codex · Old"]
        );
    }
}

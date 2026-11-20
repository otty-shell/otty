//! An agent read by an agent part of a plugin (spec
//! 2026-11-19-agent-plugins, Decisions 4 and 6): the part finds the
//! profiles, the logins, the sessions and the answers, and OTTY keeps the
//! generic work, from walking the spend places to joining accounts.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use otty_plugin_host::agent::{
    Billing as PartBilling, Command as PartCommand, Login, LoginKind,
    Profile as PartProfile, ProfilePlace, ProfileStatus as PartStatus,
};
use otty_plugin_host::{LoadedPlugin, PartRef};

use crate::account::{
    Account, AccountIdentity, AccountKind, AccountLogin, FoundIn,
};
use crate::account_key::AccountKey;
use crate::admin_key::AdminKey;
use crate::agent::{Agent, AgentEntry, AgentId, ConfigFolder};
use crate::called_part::CalledPart;
use crate::command::AgentCommand;
use crate::env::AgentEnv;
use crate::hidden_terminal::StartTerminal;
use crate::key_fingerprint::KeyFingerprint;
use crate::limits::{BillingKind, LimitSource};
use crate::part_health::PartHealth;
use crate::plugin_sources::{PluginLimits, PluginSessions, PluginStats};
use crate::plugin_spend::PluginSpend;
use crate::plugin_values::{part_profile, provider_of};
use crate::profile::{Profile, ProfileStatus};
use crate::program::StartProgram;
use crate::provider::ProviderRef;
use crate::registry::Reader;
use crate::session::{SessionId, SessionSource};
use crate::source_id::{SourceId, SourceName};
use crate::spend::SpendCounter;
use crate::stats::StatsSource;

/// The ids of the one source of sessions a plugin agent has: its part.
const PLUGIN_SOURCES: [SourceId; 1] = [SourceId::Plugin];

/// One agent part of a loaded plugin as an [`Agent`]. Its sources, spend
/// counter and statistics call the same part; the app keeps one for the
/// whole run, since its spend counter remembers what it read.
pub struct PluginAgent {
    part: Arc<CalledPart>,
    entry: AgentEntry,
    tested_version: Option<String>,
    default_folder: PathBuf,
    folder_variable: String,
    limit_sources: Vec<String>,
    spend: Arc<PluginSpend>,
    writes_cost: Arc<AtomicBool>,
}

impl PluginAgent {
    /// How the part's reads went: the Plugins page pauses and starts it
    /// with this.
    pub fn health(&self) -> &PartHealth {
        self.part.health()
    }

    /// The newest version of the agent the part was checked with, as its
    /// manifest names it.
    pub fn tested_version(&self) -> Option<&str> {
        self.tested_version.as_deref()
    }

    /// The agent part `id` of `plugin`; `None` when the plugin has no such
    /// part. It asks the part for its default folder and the names of its
    /// sources of limits once, now.
    pub fn new(plugin: Arc<LoadedPlugin>, id: &str) -> Option<Self> {
        let manifest_part = plugin
            .manifest()
            .agents()
            .iter()
            .find(|part| part.id() == id)?
            .clone();
        let part = Arc::new(CalledPart::new(plugin, PartRef::agent(id)));

        let reader = Reader::Plugin(part.named().clone());
        let program = manifest_part.program().unwrap_or(id);
        let entry = AgentEntry::new(
            AgentId::new(id),
            manifest_part.name(),
            program,
            part.named().name(),
            reader,
        );
        let folder_variable = manifest_part
            .permissions()
            .env()
            .first()
            .cloned()
            .unwrap_or_default();

        let (default_folder, limit_sources) = first_look(&part, id);
        let writes_cost = Arc::new(AtomicBool::new(false));
        let tested_version = manifest_part.tested_version().map(String::from);
        let spend = Arc::new(PluginSpend::new(
            Arc::clone(&part),
            tested_version.clone(),
            Arc::clone(&writes_cost),
        ));

        Some(Self {
            part,
            entry,
            tested_version,
            default_folder,
            folder_variable,
            limit_sources,
            spend,
            writes_cost,
        })
    }

    /// The same agent with the SVG `icon` from the plugin's package.
    pub fn with_icon(mut self, icon: Vec<u8>) -> Self {
        self.entry = self.entry.with_icon(icon);
        self
    }

    /// The logins and keys the part finds in `profile`; none when the read
    /// failed.
    fn logins(&self, profile: &Profile) -> Vec<Login> {
        let asked = part_profile(profile);

        self.part
            .read("logins", |batch| batch.agent()?.logins(&asked))
            .unwrap_or_default()
    }

    /// The login of `logins` that is `account`.
    fn login_of<'a>(
        &self,
        logins: &'a [Login],
        profile: &Profile,
        account: &Account,
    ) -> Option<&'a Login> {
        logins.iter().find(|login| {
            account_of(&self.part, login, profile).is_some_and(|found| {
                found.identity() == account.identity()
                    && is_same_provider(found.provider(), account)
            })
        })
    }
}

impl Agent for PluginAgent {
    fn entry(&self) -> &AgentEntry {
        &self.entry
    }

    fn config_folder(&self) -> ConfigFolder {
        let name = self
            .default_folder
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();

        ConfigFolder::of_plugin(name, self.folder_variable.clone())
    }

    fn writes_cost(&self) -> bool {
        self.writes_cost.load(Ordering::Relaxed)
    }

    fn default_config_dir(&self, _env: &AgentEnv) -> PathBuf {
        self.default_folder.clone()
    }

    /// The state the part tells. A read that failed has no state of its
    /// own to tell, so the profile counts as not logged in until the part
    /// answers again.
    fn detect(&self, profile: &Profile) -> ProfileStatus {
        let asked = part_profile(profile);

        let status = self
            .part
            .read("detect", |batch| batch.agent()?.detect(&asked));

        match status {
            Ok(PartStatus::Connected(connection)) => ProfileStatus::Connected {
                account: connection.account,
                plan: connection.plan,
                billing: connection.billing.map(billing_kind),
            },
            Ok(PartStatus::ConfigFolderNotFound) => {
                ProfileStatus::ConfigDirNotFound
            },
            _ => ProfileStatus::NotLoggedIn,
        }
    }

    fn accounts(&self, profile: &Profile) -> Vec<Account> {
        self.logins(profile)
            .iter()
            .filter_map(|login| account_of(&self.part, login, profile))
            .collect()
    }

    fn account_key(
        &self,
        profile: &Profile,
        account: &Account,
    ) -> Option<AccountKey> {
        let logins = self.logins(profile);
        let login = self.login_of(&logins, profile, account)?;

        let key = self
            .part
            .plugin()
            .read_key(self.part.part(), &login.place)
            .ok()?;

        AdminKey::of(key.as_str()).map(AccountKey::new)
    }

    fn found_login(
        &self,
        profile: &Profile,
        account: &Account,
    ) -> Option<Login> {
        let logins = self.logins(profile);

        self.login_of(&logins, profile, account).cloned()
    }

    fn has_sessions(&self, profile: &Profile) -> bool {
        let asked = part_profile(profile);

        let sessions = self
            .part
            .read("sessions", |batch| batch.agent()?.sessions(&asked));

        matches!(sessions, Ok(Ok(sessions)) if !sessions.is_empty())
    }

    fn version(&self, profile: &Profile) -> Option<String> {
        let asked = part_profile(profile);

        self.part
            .read("version", |batch| batch.agent()?.version(&asked))
            .ok()
            .flatten()
    }

    fn launch(
        &self,
        profile: &Profile,
        session: Option<&SessionId>,
    ) -> AgentCommand {
        let asked = part_profile(profile);
        let session = session.map(SessionId::as_str);

        let command = self
            .part
            .read("launch", |batch| batch.agent()?.launch(&asked, session));

        match command {
            Ok(command) => agent_command(command),
            Err(_) => AgentCommand::new(self.entry.program()),
        }
    }

    fn login(&self, profile: &Profile) -> AgentCommand {
        let asked = part_profile(profile);

        let command = self.part.read("login-command", |batch| {
            batch.agent()?.login_command(&asked)
        });

        match command {
            Ok(command) => agent_command(command),
            Err(_) => AgentCommand::new(self.entry.program()),
        }
    }

    fn session_sources(&self) -> Vec<Box<dyn SessionSource>> {
        vec![Box::new(PluginSessions::new(Arc::clone(&self.part)))]
    }

    fn session_source_ids(&self) -> &'static [SourceId] {
        &PLUGIN_SOURCES
    }

    /// The part's sources, each named as the part names it and read in a
    /// read of its own. The plugin engine starts the programs and hidden
    /// terminals of every part, so `programs` only tells whether any may
    /// start now: while Agents mode is away none does.
    fn limit_sources(
        &self,
        programs: Arc<dyn StartProgram>,
        _terminals: Arc<dyn StartTerminal>,
    ) -> Vec<Box<dyn LimitSource>> {
        let starts_programs = programs.starts_now();

        self.limit_sources
            .iter()
            .map(|source| {
                let limits = PluginLimits::new(
                    Arc::clone(&self.part),
                    source,
                    starts_programs,
                );
                Box::new(limits) as Box<dyn LimitSource>
            })
            .collect()
    }

    fn limit_source_ids(&self) -> Vec<SourceId> {
        self.limit_sources
            .iter()
            .map(|source| SourceId::Named(SourceName::new(source)))
            .collect()
    }

    fn spend_counter(&self) -> Option<Box<dyn SpendCounter>> {
        Some(Box::new(Arc::clone(&self.spend)))
    }

    fn stats_source(&self) -> Option<Box<dyn StatsSource>> {
        Some(Box::new(PluginStats::new(Arc::clone(&self.part))))
    }

    fn found_config_dirs(&self, _env: &AgentEnv) -> Vec<PathBuf> {
        let places = self
            .part
            .read("profiles", |batch| batch.agent()?.profiles())
            .unwrap_or_default();

        places
            .into_iter()
            .filter(|place| !place.is_default)
            .map(|place| PathBuf::from(place.folder))
            .filter(|folder| *folder != self.default_folder)
            .collect()
    }
}

/// The account `login` of `profile` is: known by its account id,
/// organization id or email when the part tells one, else by the
/// fingerprint of its key, which the host reads at the login's place.
/// `None` when the key can not be read either.
pub(crate) fn account_of(
    part: &CalledPart,
    login: &Login,
    profile: &Profile,
) -> Option<Account> {
    let found = &login.identity;
    let identity =
        match (&found.account_id, &found.organization_id, &found.email) {
            (Some(id), _, _) => AccountIdentity::AccountId(id.clone()),
            (None, Some(id), _) => AccountIdentity::OrganizationId(id.clone()),
            (None, None, Some(email)) => AccountIdentity::Email(email.clone()),
            (None, None, None) => {
                let key =
                    part.plugin().read_key(part.part(), &login.place).ok()?;
                AccountIdentity::KeyFingerprint(KeyFingerprint::of(
                    key.as_str(),
                ))
            },
        };

    let shown = match (&found.email, login.kind) {
        (Some(email), _) => AccountLogin::Email(email.clone()),
        (None, LoginKind::Subscription) => AccountLogin::Unnamed,
        (None, _) => AccountLogin::Key(None),
    };
    let kind = match login.kind {
        LoginKind::Subscription => AccountKind::Limits,
        _ => AccountKind::Money,
    };
    let found_in = FoundIn::ProfileFolder(profile.config_dir().to_path_buf());

    let account = Account::new(
        provider_of(&login.for_whom),
        identity,
        shown,
        kind,
        found_in,
    );
    Some(account.with_plan(login.plan.clone()))
}

/// The default folder of the part `id` and the names of the sources of
/// limits the part reads for it, asked once in one read; the folder
/// `~/.<id>` and no sources when the read fails.
fn first_look(part: &CalledPart, id: &str) -> (PathBuf, Vec<String>) {
    let home = part.plugin().env().home().to_path_buf();
    let fallback = home.join(format!(".{id}"));

    let look = part.read("profiles", |batch| {
        let mut calls = batch.agent()?;
        let places = calls.profiles()?;
        let Some(default) = default_place(&places) else {
            return Ok(None);
        };

        let asked = PartProfile {
            agent: id.to_string(),
            folder: default.folder.clone(),
            is_default: true,
        };
        let sources = calls.limit_sources(&asked)?;
        Ok(Some((PathBuf::from(&default.folder), sources)))
    });

    match look {
        Ok(Some(found)) => found,
        _ => (fallback, Vec::new()),
    }
}

/// The default place of `places`: the one marked default, else the first.
fn default_place(places: &[ProfilePlace]) -> Option<&ProfilePlace> {
    places
        .iter()
        .find(|place| place.is_default)
        .or_else(|| places.first())
}

/// Whether `found`, the provider of a login, is the provider of
/// `account`: the same reference, or a host and an id, which only the
/// registry joins.
fn is_same_provider(found: &ProviderRef, account: &Account) -> bool {
    found == account.provider()
        || found.as_host().is_some()
        || account.provider().as_host().is_some()
}

/// How a part says a login is paid for.
fn billing_kind(billing: PartBilling) -> BillingKind {
    match billing {
        PartBilling::UsageWindows => BillingKind::UsageWindows,
        PartBilling::Credits => BillingKind::Credits,
        PartBilling::ApiKey => BillingKind::ApiKey,
    }
}

/// A part's command as the app runs it.
fn agent_command(command: PartCommand) -> AgentCommand {
    let mut agent_command = AgentCommand::new(command.program);
    for arg in command.args {
        agent_command = agent_command.with_arg(arg);
    }
    for (name, value) in command.env {
        agent_command = agent_command.with_env(name, value);
    }

    agent_command
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile_list::profiles;
    use crate::saved_profiles::SavedProfiles;
    use crate::testing::{NOW, TemplateHome, answer_line, meta_line};

    const AGENT: &str = "example-agent";

    fn agent(home: &TemplateHome) -> PluginAgent {
        PluginAgent::new(home.plugin(), AGENT).expect("the agent part")
    }

    #[test]
    fn the_entry_comes_from_the_manifest_and_names_the_plugin() {
        let home = TemplateHome::new();

        let agent = agent(&home);

        let entry = agent.entry();
        assert_eq!(entry.id().as_str(), AGENT);
        assert_eq!(entry.name(), "Example Agent");
        assert_eq!(entry.program(), "example-agent");
        assert_eq!(entry.provider_name(), "Example");
        let Reader::Plugin(part) = entry.reader() else {
            panic!("read by a plugin: {entry:?}");
        };
        assert_eq!((part.plugin(), part.part()), ("example", AGENT));
        assert_eq!(agent.tested_version(), Some("1.4.2"));
        assert!(agent.limit_source_ids().is_empty());
    }

    /// The default profile is the part's default folder; another folder
    /// the part finds is a profile named after its end.
    #[test]
    fn the_profiles_are_the_folders_the_part_finds() {
        let home = TemplateHome::new();
        home.write_session(
            ".example-agent-work",
            "w1.jsonl",
            &[meta_line("/p"), answer_line("w1", NOW, 1)],
            NOW,
        );
        let agents: Vec<Box<dyn Agent>> = vec![Box::new(agent(&home))];
        let env = AgentEnv::new(home.home().to_path_buf());

        let list = profiles(&agents, &env, &SavedProfiles::default(), |_| true);

        let listed: Vec<_> = list
            .iter()
            .map(|profile| (profile.name(), profile.config_dir().to_path_buf()))
            .collect();
        assert_eq!(
            listed,
            [
                ("Example Agent", home.folder()),
                ("Work", home.home().join(".example-agent-work")),
            ]
        );
        assert_eq!(agents[0].config_folder().name(), ".example-agent");
        assert_eq!(agents[0].config_folder().variable(), "EXAMPLE_AGENT_HOME");
    }

    #[test]
    fn a_profile_with_a_key_is_connected_and_tells_its_version() {
        let home = TemplateHome::new();
        let agent = agent(&home);

        let status = agent.detect(&home.profile());
        let version = agent.version(&home.profile());

        assert_eq!(
            status,
            ProfileStatus::Connected {
                account: Some("dev@example.com".to_string()),
                plan: None,
                billing: Some(BillingKind::ApiKey),
            }
        );
        assert_eq!(version.as_deref(), Some("1.4.2"));
    }

    /// Spec Decision 4: the logins of a profile are accounts of the
    /// provider the part names; OTTY reads the key only for itself.
    #[test]
    fn a_login_is_an_account_whose_key_otty_reads_for_itself() {
        let home = TemplateHome::new();
        let agent = agent(&home);

        let accounts = agent.accounts(&home.profile());
        let login = agent.found_login(&home.profile(), &accounts[0]);
        let key = agent.account_key(&home.profile(), &accounts[0]);

        assert_eq!(
            accounts,
            [Account::new(
                ProviderRef::of_id("example-cloud"),
                AccountIdentity::Email("dev@example.com".to_string()),
                AccountLogin::Email("dev@example.com".to_string()),
                AccountKind::Money,
                FoundIn::ProfileFolder(home.folder()),
            )]
        );
        assert!(login.is_some_and(|login| login.kind == LoginKind::ApiKey));
        let key = key.expect("the key");
        assert_eq!(key.key().secret(), "ex-made-up-key-0001");
        assert_eq!(format!("{key:?}"), "AccountKey(hidden)");
    }

    #[test]
    fn a_found_profile_launches_with_its_folder() {
        let home = TemplateHome::new();
        let agent = agent(&home);
        let work = Profile::found(
            AgentId::new(AGENT),
            "Work",
            home.home().join(".example-agent-work"),
        );

        let command = agent.launch(&work, Some(&SessionId::new("w1")));

        assert_eq!(command.program(), "example-agent");
        assert_eq!(command.args(), ["--resume", "w1"]);
        assert_eq!(
            command.env(),
            [(
                "EXAMPLE_AGENT_HOME".to_string(),
                work.config_dir().display().to_string()
            )]
        );
    }

    #[test]
    fn the_sessions_come_from_the_part() {
        let home = TemplateHome::new();
        home.write_session(
            ".example-agent",
            "s1.jsonl",
            &[meta_line("/home/dev/shop"), answer_line("a1", NOW, 1)],
            NOW,
        );
        let agent = agent(&home);

        let sources = agent.session_sources();
        let sessions = sources[0].sessions(&home.profile()).expect("listed");

        assert_eq!(sources[0].id(), SourceId::Plugin);
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id().as_str(), "s1");
        assert_eq!(
            sessions[0].folder(),
            std::path::Path::new("/home/dev/shop")
        );
    }
}

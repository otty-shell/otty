//! An agent read by an agent part of a plugin (spec
//! 2026-11-19-agent-plugins, Decisions 4 and 6): the part finds the
//! profiles, the logins, the sessions and the answers, and OTTY keeps the
//! generic work, from walking the spend places to joining accounts.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use otty_plugin_host::agents::agent::{
    Billing as PartBilling, Login, LoginKind, ProfilePlace,
    ProfileStatus as PartStatus,
};
use otty_plugin_host::{CalledPart, LoadedPlugin, PartHealth, PartRef};

use crate::account::{
    Account, AccountIdentity, AccountKind, AccountLogin, FoundIn,
};
use crate::agent::{Agent, AgentEntry, AgentId, ConfigFolder};
use crate::command::AgentCommand;
use crate::env::AgentEnv;
use crate::key_fingerprint::KeyFingerprint;
use crate::limits::BillingKind;
use crate::part_need::{PartNeed, agent_command};
use crate::plugin_sources::PluginSessions;
use crate::plugin_spend::PluginSpend;
use crate::plugin_values::{part_profile, provider_of};
use crate::profile::{Profile, ProfileStatus};
use crate::provider::ProviderRef;
use crate::session::{SessionId, SessionSource};
use crate::source_id::SourceId;
use crate::spend::SpendCounter;
use crate::stats::StatsSource;

/// The ids of the one source of sessions a plugin agent has: its part.
const PLUGIN_SOURCES: [SourceId; 1] = [SourceId::AgentPart];

/// One agent part of a loaded plugin as an [`Agent`]. Its sources, spend
/// counter and statistics call the same part; the app keeps one for the
/// whole run, since its sessions and its spend counter remember what they
/// read.
pub struct PluginAgent {
    part: Arc<CalledPart>,
    entry: AgentEntry,
    tested_version: Option<String>,
    default_folder: PathBuf,
    folder_variable: String,
    sessions: Arc<PluginSessions>,
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
    /// part. It asks the part for its default folder once, now.
    pub fn new(plugin: Arc<LoadedPlugin>, id: &str) -> Option<Self> {
        let manifest_part = plugin
            .manifest()
            .agents()
            .iter()
            .find(|part| part.id() == id)?
            .clone();
        let part = Arc::new(CalledPart::new(plugin, PartRef::agent(id)));

        let program = manifest_part.program().unwrap_or(id);
        let keyring_services = manifest_part
            .permissions()
            .keyring()
            .iter()
            .map(|rule| rule.service().to_string())
            .collect();
        let entry = AgentEntry::new(
            AgentId::new(id),
            manifest_part.name(),
            program,
            part.named().name(),
            part.named().clone(),
        )
        .with_keyring_services(keyring_services);
        let folder_variable = manifest_part
            .permissions()
            .env()
            .first()
            .cloned()
            .unwrap_or_default();

        let default_folder = default_folder(&part, id);
        let writes_cost = Arc::new(AtomicBool::new(false));
        let tested_version = manifest_part.tested_version().map(String::from);
        let sessions = Arc::new(PluginSessions::new(Arc::clone(&part)));
        let spend = Arc::new(PluginSpend::new(
            Arc::clone(&part),
            Arc::clone(&sessions),
            tested_version.clone(),
            Arc::clone(&writes_cost),
        ));

        Some(Self {
            part,
            entry,
            tested_version,
            default_folder,
            folder_variable,
            sessions,
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

    /// The state the part tells, with the need of a profile nobody is
    /// logged in with. A read that failed has no state of its own to tell,
    /// so the profile counts as not logged in, with no need, until the
    /// part answers again.
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
            Ok(PartStatus::NotLoggedIn(need)) => ProfileStatus::NotLoggedIn(
                PartNeed::of_part(self.part.named(), &need)
                    .map(|need| need.about_profile(profile.config_dir()))
                    .map(Box::new),
            ),
            Err(_) => ProfileStatus::NotLoggedIn(None),
        }
    }

    fn accounts(&self, profile: &Profile) -> Vec<Account> {
        self.logins(profile)
            .iter()
            .filter_map(|login| account_of(&self.part, login, profile))
            .collect()
    }

    fn found_login(
        &self,
        profile: &Profile,
        account: &Account,
    ) -> Option<Login> {
        let logins = self.logins(profile);

        self.login_of(&logins, profile, account).cloned()
    }

    /// The need the part gives for `login` of `profile` once a provider
    /// refused it; a read that failed gives none, and OTTY says the login
    /// was refused itself.
    fn login_refused(
        &self,
        profile: &Profile,
        login: &Login,
    ) -> Option<PartNeed> {
        let asked = part_profile(profile);

        let need = self.part.read("login-refused", |batch| {
            batch.agent()?.login_refused(&asked, &login.place)
        });

        let need = need.ok().flatten()?;
        PartNeed::of_part(self.part.named(), &need)
            .map(|need| need.about_profile(profile.config_dir()))
    }

    /// Read as the session list reads, so what it read is kept for it.
    fn has_sessions(&self, profile: &Profile) -> bool {
        let sessions = self.sessions.sessions(profile);

        matches!(sessions, Ok(sessions) if !sessions.is_empty())
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
            Ok(command) => agent_command(&command),
            Err(_) => AgentCommand::new(self.entry.program()),
        }
    }

    fn session_sources(&self) -> Vec<Box<dyn SessionSource>> {
        vec![Box::new(Arc::clone(&self.sessions))]
    }

    fn session_source_ids(&self) -> &'static [SourceId] {
        &PLUGIN_SOURCES
    }

    fn spend_counter(&self) -> Option<Box<dyn SpendCounter>> {
        Some(Box::new(Arc::clone(&self.spend)))
    }

    fn stats_source(&self) -> Option<Box<dyn StatsSource>> {
        Some(Box::new(Arc::clone(&self.spend)))
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

/// The default folder of the part `id`, asked once; the folder `~/.<id>`
/// when the read fails.
fn default_folder(part: &CalledPart, id: &str) -> PathBuf {
    let home = part.plugin().env().home().to_path_buf();
    let fallback = home.join(format!(".{id}"));

    let places = part.read("profiles", |batch| batch.agent()?.profiles());

    match places {
        Ok(places) => default_place(&places)
            .map(|place| PathBuf::from(&place.folder))
            .unwrap_or(fallback),
        Err(_) => fallback,
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
        let part = entry.part();
        assert!(entry.keyring_services().is_empty());
        assert_eq!((part.plugin(), part.part()), ("example", AGENT));
        assert_eq!(agent.tested_version(), Some("1.4.2"));
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
    /// provider the part names, with the place of their key for a
    /// provider part.
    #[test]
    fn a_login_is_an_account_whose_key_place_the_part_names() {
        let home = TemplateHome::new();
        let agent = agent(&home);

        let accounts = agent.accounts(&home.profile());
        let login = agent.found_login(&home.profile(), &accounts[0]);

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

        assert_eq!(sources[0].id(), SourceId::AgentPart);
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id().as_str(), "s1");
        assert_eq!(
            sessions[0].folder(),
            std::path::Path::new("/home/dev/shop")
        );
    }

    /// Issue 25: a file whose size and change time did not change is not
    /// read again, a changed one is, and a deleted one drops its session.
    #[test]
    fn the_sessions_of_unchanged_files_are_kept_and_of_deleted_ones_dropped() {
        let home = TemplateHome::new();
        let lines = |cwd: &str| [meta_line(cwd), answer_line("a1", NOW, 1)];
        home.write_session(".example-agent", "s1.jsonl", &lines("/p/one"), NOW);
        home.write_session(".example-agent", "s2.jsonl", &lines("/p/two"), NOW);
        let agent = agent(&home);
        let sources = agent.session_sources();
        let folders = || {
            let sessions =
                sources[0].sessions(&home.profile()).expect("listed");
            sessions
                .iter()
                .map(|session| session.folder().display().to_string())
                .collect::<Vec<_>>()
        };

        let first = folders();
        home.write_session(".example-agent", "s1.jsonl", &lines("/p/new"), NOW);
        let same_stamp = folders();
        home.write_session(
            ".example-agent",
            "s1.jsonl",
            &lines("/p/new"),
            NOW + 1,
        );
        std::fs::remove_file(home.folder().join("sessions/s2.jsonl"))
            .expect("remove s2");
        let changed = folders();

        assert_eq!(first, ["/p/one", "/p/two"]);
        assert_eq!(same_stamp, ["/p/one", "/p/two"]);
        assert_eq!(changed, ["/p/new"]);
    }
}

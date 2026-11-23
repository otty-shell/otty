//! An agent read by an agent part of a plugin (spec
//! 2026-11-19-agent-plugins, Decisions 4 and 6): the part names the
//! agent's default folder and finds the sessions and the answers, and OTTY
//! keeps the generic work, such as walking the spend places. Other folders
//! are profiles only when the user adds them (spec 2026-11-23, Decision
//! 4). The accounts are the provider part's (Decision 5): OTTY no longer
//! asks an agent part for the logins of a profile.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use otty_plugin_host::agents::agent::{
    ProfilePlace, ProfileStatus as PartStatus,
};
use otty_plugin_host::{
    AgentPart, CalledPart, LoadedPlugin, PartHealth, PartRef, ProgramInstall,
};

use crate::agent::{Agent, AgentEntry, AgentId, AgentInstall, ConfigFolder};
use crate::command::AgentCommand;
use crate::env::AgentEnv;
use crate::part_need::{PartNeed, started_command};
use crate::plugin_sources::PluginSessions;
use crate::plugin_spend::PluginSpend;
use crate::plugin_values::part_profile;
use crate::profile::{Profile, ProfileStatus};
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
    /// How the part's reads went: Plugins mode pauses and starts it
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
        .with_keyring_services(keyring_services)
        .with_program_places(program_places(&manifest_part, part.plugin()));
        let entry = match manifest_part.install() {
            Some(install) => entry.with_install(agent_install(install)),
            None => entry,
        };
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
            Ok(PartStatus::Found) => ProfileStatus::Connected,
            Ok(PartStatus::NotLoggedIn(need)) => ProfileStatus::NotLoggedIn(
                PartNeed::of_called_part(&self.part, &need)
                    .map(|need| need.about_profile(profile.config_dir()))
                    .map(Box::new),
            ),
            Err(_) => ProfileStatus::NotLoggedIn(None),
        }
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
            Ok(command) => started_command(self.part.plugin(), &command),
            Err(_) => AgentCommand::new(self.program_to_start()),
        }
    }

    /// Calls of every part of the plugin that start the program now start
    /// it from `place`.
    fn start_program_from(&self, place: Option<PathBuf>) {
        self.part
            .plugin()
            .set_program_path(self.entry.program(), place);
    }

    fn program_to_start(&self) -> String {
        self.part.plugin().program_to_start(self.entry.program())
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
}

/// The default folder of the part `id`, asked once; the folder `~/.<id>`
/// when the read fails or the part marks no place as the default one.
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

/// The default place of `places`: the one marked default. Any other place
/// the part gives is left out (spec 2026-11-23, Decision 4).
fn default_place(places: &[ProfilePlace]) -> Option<&ProfilePlace> {
    places.iter().find(|place| place.is_default)
}

/// The program places of `part` on the computer `plugin` sees.
fn program_places(part: &AgentPart, plugin: &LoadedPlugin) -> Vec<PathBuf> {
    part.program_places()
        .iter()
        .filter_map(|place| place.path_on(plugin.env()))
        .collect()
}

/// How the manifest says the agent's program is installed.
fn agent_install(install: &ProgramInstall) -> AgentInstall {
    let command = install.command().split_first().map(|(program, args)| {
        args.iter()
            .fold(AgentCommand::new(program.as_str()), |command, arg| {
                command.with_arg(arg.as_str())
            })
    });

    AgentInstall::new(command, install.page().map(String::from))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::part_need::PartNeedAction;
    use crate::profile_list::{profile_status, profiles};
    use crate::saved_profiles::SavedProfiles;
    use crate::testing::{NOW, TemplateHome, answer_line, meta_line};
    #[cfg(unix)]
    use crate::testing::{write_file, write_program};

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

    /// Spec 2026-11-23, Decision 4: the default profile is the part's
    /// default folder, and another folder of the agent with a session is
    /// no profile.
    #[test]
    fn the_only_profile_is_the_default_folder_of_the_part() {
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
        assert_eq!(listed, [("Example Agent", home.folder())]);
        assert_eq!(agents[0].config_folder().name(), ".example-agent");
        assert_eq!(agents[0].config_folder().variable(), "EXAMPLE_AGENT_HOME");
    }

    fn place(folder: &str, is_default: bool) -> ProfilePlace {
        ProfilePlace {
            folder: folder.to_string(),
            is_default,
        }
    }

    /// Spec 2026-11-23, Decision 4: OTTY takes the place the part marks
    /// as the default one, wherever it is in the list, and no other.
    #[test]
    fn the_default_place_is_the_one_marked_default_only() {
        let marked = [
            place("/home/dev/.example-agent-work", false),
            place("/home/dev/.example-agent", true),
        ];
        let unmarked = [place("/home/dev/.example-agent-work", false)];

        let default = default_place(&marked);
        let none = default_place(&unmarked);

        assert_eq!(
            default.map(|place| place.folder.as_str()),
            Some("/home/dev/.example-agent")
        );
        assert!(none.is_none());
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decision 2: a
    /// profile with a key is connected, with no word on whose key it is.
    #[test]
    fn a_profile_with_a_key_is_connected_and_tells_its_version() {
        let home = TemplateHome::new();
        let agent = agent(&home);

        let status = agent.detect(&home.profile());
        let version = agent.version(&home.profile());

        assert_eq!(status, ProfileStatus::Connected);
        assert_eq!(version.as_deref(), Some("1.4.2"));
    }

    /// Issue 32: the places and the install come from the manifest, the
    /// places made concrete for this home.
    #[test]
    fn the_entry_has_the_program_places_and_the_install_of_the_manifest() {
        let home = TemplateHome::new();

        let agent = agent(&home);

        let entry = agent.entry();
        assert_eq!(
            entry.program_places(),
            [home.home().join(".example/bin/example-agent")]
        );
        let install = entry.install().expect("an install");
        assert_eq!(
            install.command(),
            Some(
                &AgentCommand::new("sh")
                    .with_arg("-c")
                    .with_arg("curl -fsSL https://example.com/install.sh | sh")
            )
        );
        assert_eq!(install.page(), Some("https://example.com/docs/install"));
    }

    /// Issue 32: the login shell does not find `example-agent`, but the
    /// installer put it at `~/.example/bin/example-agent`. OTTY finds it
    /// there and starts it by that path for a session, for the login a
    /// need runs and for the version.
    #[cfg(unix)]
    #[test]
    fn a_program_at_a_program_place_is_started_by_its_full_path() {
        let home = TemplateHome::new();
        let place = home.home().join(".example/bin/example-agent");
        write_program(&place);
        let logged_out = home.home().join(".example-agent-work");
        write_file(&logged_out.join("state.json"), "{}");
        let work = Profile::added(AgentId::new(AGENT), "Work", logged_out);
        let agent = agent(&home);
        let full_path = place.display().to_string();

        let status = profile_status(&agent, &home.profile(), |_| false);
        let launch = agent.launch(&home.profile(), None);
        let need = agent.detect(&work);

        assert!(matches!(status, ProfileStatus::Connected));
        assert_eq!(launch.program(), full_path);
        assert_eq!(agent.program_to_start(), full_path);
        let ProfileStatus::NotLoggedIn(Some(need)) = need else {
            panic!("a login need, not {need:?}");
        };
        let PartNeedAction::Run(login) = need.action() else {
            panic!("a command to run, not {:?}", need.action());
        };
        assert_eq!(login.program(), full_path);
        assert_eq!(login.args(), ["login"]);
    }

    /// Issue 32: the name comes first: a program the login shell finds is
    /// started by its name, even with a copy at a program place.
    #[cfg(unix)]
    #[test]
    fn a_program_the_login_shell_finds_is_started_by_its_name() {
        let home = TemplateHome::new();
        write_program(&home.home().join(".example/bin/example-agent"));
        let agent = agent(&home);
        let _ = profile_status(&agent, &home.profile(), |_| false);

        let status = profile_status(&agent, &home.profile(), |program| {
            program == "example-agent"
        });

        assert!(matches!(status, ProfileStatus::Connected));
        assert_eq!(agent.launch(&home.profile(), None).program(), AGENT);
        assert_eq!(agent.program_to_start(), AGENT);
    }

    /// Issue 32: with no program at its places either, the profile says
    /// its program is not found, and nothing starts by a path.
    #[test]
    fn a_program_found_nowhere_is_not_found() {
        let home = TemplateHome::new();
        let agent = agent(&home);

        let status = profile_status(&agent, &home.profile(), |_| false);

        assert_eq!(status, ProfileStatus::ProgramNotFound);
        assert_eq!(agent.program_to_start(), AGENT);
    }

    #[test]
    fn an_added_profile_launches_with_its_folder() {
        let home = TemplateHome::new();
        let agent = agent(&home);
        let work = Profile::added(
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

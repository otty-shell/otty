//! The agent part `example-agent`: a made-up coding agent that keeps its
//! login in `auth.json`, its version in `state.json` and its sessions in
//! `sessions/*.jsonl` of its config folder, `~/.example-agent` by default.

use std::path::Path;

use otty_plugin_sdk::agents::{
    Agent, Answers, Billing, Connection, Identity, Login, LoginKind, LoginRef,
    Place, PlaceItem, PlaceKind, Profile, ProfilePlace, ProfileStatus,
    ProviderRef, ReadError, Session,
};
use otty_plugin_sdk::{
    Command, HostError, JsonField, KeyPlace, Need, env, files, log,
};
use serde::Deserialize;

use crate::session_file::SessionFile;
use crate::{Example, needs};

/// The variable that names another config folder, as the agent reads it.
const HOME_VARIABLE: &str = "EXAMPLE_AGENT_HOME";

/// The agent's default config folder in the home folder.
const DEFAULT_FOLDER: &str = ".example-agent";

/// The agent's program.
const PROGRAM: &str = "example-agent";

/// The provider whose keys the agent uses.
const PROVIDER: &str = "example-cloud";

/// The JSON field of `auth.json` that holds the key.
const KEY_FIELD: &str = "/api_key";

/// The most bytes of a small file the part reads at once.
const SMALL_FILE: u32 = 64 * 1024;

/// `auth.json`, its key replaced by a handle.
#[derive(Debug, Deserialize)]
struct AuthFile {
    api_key: Option<KeyMarker>,
    email: Option<String>,
}

/// `{"$key": n}`: where the host put a key.
#[derive(Debug, Deserialize)]
struct KeyMarker {
    #[serde(rename = "$key")]
    _index: usize,
}

/// `state.json`.
#[derive(Debug, Deserialize)]
struct StateFile {
    last_version: Option<String>,
}

impl Agent for Example {
    /// The default config folder only: OTTY ignores any other place, and
    /// the user adds other folders as profiles.
    fn profiles(_agent: &str) -> Vec<ProfilePlace> {
        let folder = env::var(HOME_VARIABLE)
            .unwrap_or_else(|| format!("{}/{DEFAULT_FOLDER}", env::home()));

        vec![ProfilePlace {
            folder,
            is_default: true,
        }]
    }

    fn detect(profile: &Profile) -> ProfileStatus {
        match read_auth(profile) {
            Ok(AuthFile {
                api_key: Some(_),
                email,
            }) => ProfileStatus::Connected(Connection {
                account: email,
                plan: None,
                billing: Some(Billing::ApiKey),
            }),
            Err(HostError::NotFound(_)) if !folder_exists(profile) => {
                ProfileStatus::ConfigFolderNotFound
            },
            _ => ProfileStatus::NotLoggedIn(needs::not_logged_in(login_run(
                profile,
            ))),
        }
    }

    fn version(profile: &Profile) -> Option<String> {
        let path = format!("{}/state.json", profile.folder);
        let bytes = files::read(&path, SMALL_FILE).ok()?;
        let Ok(state) = serde_json::from_slice::<StateFile>(&bytes) else {
            log::warn("state.json is not in the shape this plugin reads");
            return None;
        };

        state.last_version
    }

    fn logins(profile: &Profile) -> Vec<Login> {
        let Ok(AuthFile {
            api_key: Some(_),
            email,
        }) = read_auth(profile)
        else {
            return Vec::new();
        };

        vec![Login {
            kind: LoginKind::ApiKey,
            for_whom: ProviderRef::Id(PROVIDER.to_string()),
            place: KeyPlace::JsonField(JsonField {
                file: auth_path(profile),
                pointer: KEY_FIELD.to_string(),
            }),
            identity: Identity {
                email,
                ..Identity::default()
            },
            plan: None,
        }]
    }

    /// Each session file is one session; OTTY reads again only the files
    /// that changed.
    fn session_places(profile: &Profile) -> Vec<Place> {
        vec![Place {
            glob: sessions_glob(profile),
            kind: PlaceKind::Files,
        }]
    }

    fn read_sessions(
        _profile: &Profile,
        item: &PlaceItem,
    ) -> Result<Vec<Session>, ReadError> {
        let session = SessionFile::read(&item.file.path)?;

        Ok(vec![Session {
            id: session_id(&item.file.path),
            folder: session.project().map(str::to_string),
            title: session.title().map(str::to_string),
            changed_ms: session.last_at_ms().unwrap_or(item.file.changed_ms),
        }])
    }

    fn spend_places(profile: &Profile) -> Vec<Place> {
        vec![Place {
            glob: sessions_glob(profile),
            kind: PlaceKind::Files,
        }]
    }

    /// Each answer names its session, the file's: OTTY sums the panel of
    /// one session from the answers that name it.
    fn read_answers(
        _profile: &Profile,
        item: &PlaceItem,
    ) -> Result<Answers, ReadError> {
        let session = session_id(&item.file.path);

        SessionFile::read(&item.file.path)
            .map(|file| file.into_answers(&session))
    }

    fn launch(profile: &Profile, session: Option<&str>) -> Command {
        let args = match session {
            Some(session) => vec!["--resume".to_string(), session.to_string()],
            None => Vec::new(),
        };

        Command {
            program: PROGRAM.to_string(),
            args,
            env: folder_env(profile),
        }
    }

    /// The provider refused the key of the profile, or it expired: the
    /// provider part does not know where the agent keeps it, so it asks
    /// this part, which logs the profile in again.
    fn login_refused(profile: &Profile, _login: &LoginRef) -> Option<Need> {
        Some(needs::login_refused(login_run(profile)))
    }
}

/// The command that logs the profile in: `example-agent login`.
fn login_run(profile: &Profile) -> Command {
    Command {
        program: PROGRAM.to_string(),
        args: vec!["login".to_string()],
        env: folder_env(profile),
    }
}

/// `auth.json` of the profile, its key replaced by a handle.
fn read_auth(profile: &Profile) -> Result<AuthFile, HostError> {
    let document = files::json_with_keys(&auth_path(profile))?;

    serde_json::from_str(document.json()).map_err(|_| {
        HostError::Failed("auth.json is not an auth file".to_string())
    })
}

/// The id of the session kept in the session file at `path`: its name
/// without `.jsonl`.
fn session_id(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// The session files of the profile: `sessions/*.jsonl`.
fn sessions_glob(profile: &Profile) -> String {
    format!("{}/sessions/*.jsonl", profile.folder)
}

fn auth_path(profile: &Profile) -> String {
    format!("{}/auth.json", profile.folder)
}

/// Whether the profile's config folder holds any file.
fn folder_exists(profile: &Profile) -> bool {
    files::list(&format!("{}/**", profile.folder))
        .is_ok_and(|found| !found.is_empty())
}

/// The variable a profile in another folder than the default needs.
fn folder_env(profile: &Profile) -> Vec<(String, String)> {
    if profile.is_default {
        return Vec::new();
    }

    vec![(HOME_VARIABLE.to_string(), profile.folder.clone())]
}

#[cfg(test)]
mod tests {
    use otty_plugin_sdk::NeedAction;
    use otty_plugin_sdk::test_host::TestHost;

    use super::*;

    const AGENT: &str = "example-agent";

    fn host() -> TestHost {
        TestHost::new(env!("CARGO_MANIFEST_DIR"))
    }

    fn profile(host: &TestHost, folder: &str, is_default: bool) -> Profile {
        Profile {
            agent: AGENT.to_string(),
            folder: host.home().join(folder).display().to_string(),
            is_default,
        }
    }

    /// The fixture home holds `~/.example-agent-work` with a session, and
    /// still the part gives only the default folder.
    #[test]
    fn the_only_profile_is_the_default_folder() {
        let host = host();

        let places = host.as_agent(AGENT, || Example::profiles(AGENT));

        assert_eq!(
            places,
            [ProfilePlace {
                folder: profile(&host, ".example-agent", true).folder,
                is_default: true,
            }]
        );
    }

    #[test]
    fn the_variable_names_the_default_folder() {
        let host = host();
        let work = profile(&host, ".example-agent-work", true).folder;
        let host = host.with_var(HOME_VARIABLE, &work);

        let places = host.as_agent(AGENT, || Example::profiles(AGENT));

        assert_eq!(places.len(), 1);
        assert_eq!(places[0].folder, work);
        assert!(places[0].is_default);
    }

    #[test]
    fn a_profile_with_a_key_is_connected_as_its_email() {
        let host = host();
        let profile = profile(&host, ".example-agent", true);

        let status = host.as_agent(AGENT, || Example::detect(&profile));

        assert_eq!(
            status,
            ProfileStatus::Connected(Connection {
                account: Some("dev@example.com".to_string()),
                plan: None,
                billing: Some(Billing::ApiKey),
            })
        );
    }

    /// A need of a part is in the language OTTY names, and its button
    /// runs the agent's login with the profile's folder.
    #[test]
    fn a_profile_with_no_key_needs_a_login_in_the_users_language() {
        let host = host().with_language("ru");
        let profile = profile(&host, ".example-agent-nobody", false);
        let folder = profile.folder.clone();

        let status = host.as_agent(AGENT, || Example::detect(&profile));

        let ProfileStatus::NotLoggedIn(need) = status else {
            panic!("not logged in: {status:?}");
        };
        assert_eq!(need.id, "not-logged-in");
        assert_eq!(need.title, "Нужен вход");
        assert_eq!(
            need.action,
            NeedAction::Run(Command {
                program: PROGRAM.to_string(),
                args: vec!["login".to_string()],
                env: vec![(HOME_VARIABLE.to_string(), folder)],
            })
        );
    }

    /// Issue 20: a login the provider refused is logged in again in the
    /// profile's own folder, in the language OTTY shows now.
    #[test]
    fn a_refused_login_logs_the_profile_in_again() {
        let host = host();
        let profile = profile(&host, ".example-agent-work", false);
        let folder = profile.folder.clone();
        let login = LoginRef::EnvVar("EXAMPLE_CLOUD_KEY".to_string());

        host.set_language("de");
        let need =
            host.as_agent(AGENT, || Example::login_refused(&profile, &login));

        let need = need.expect("a need");
        assert_eq!(need.id, "login-refused");
        assert_eq!(need.title, "Erneut anmelden");
        assert_eq!(
            need.action,
            NeedAction::Run(Command {
                program: PROGRAM.to_string(),
                args: vec!["login".to_string()],
                env: vec![(HOME_VARIABLE.to_string(), folder)],
            })
        );
    }

    #[test]
    fn a_missing_folder_is_told_apart() {
        let host = host();
        let profile = profile(&host, ".example-agent-gone", false);

        let status = host.as_agent(AGENT, || Example::detect(&profile));

        assert_eq!(status, ProfileStatus::ConfigFolderNotFound);
    }

    #[test]
    fn the_version_comes_from_the_state_file() {
        let host = host();
        let profile = profile(&host, ".example-agent", true);

        let version = host.as_agent(AGENT, || Example::version(&profile));

        assert_eq!(version.as_deref(), Some("1.4.2"));
    }

    #[test]
    fn a_state_file_of_another_shape_is_logged() {
        let host = host();
        let profile = profile(&host, ".example-agent-work", false);

        let version = host.as_agent(AGENT, || Example::version(&profile));

        assert_eq!(version, None);
        assert_eq!(
            host.log(),
            ["[warn] state.json is not in the shape this plugin reads"]
        );
    }

    #[test]
    fn a_login_names_where_its_key_is_never_the_key() {
        let host = host();
        let profile = profile(&host, ".example-agent", true);

        let logins = host.as_agent(AGENT, || Example::logins(&profile));

        assert_eq!(logins.len(), 1);
        assert_eq!(logins[0].kind, LoginKind::ApiKey);
        assert_eq!(logins[0].for_whom, ProviderRef::Id(PROVIDER.to_string()));
        assert_eq!(
            logins[0].place,
            KeyPlace::JsonField(JsonField {
                file: format!("{}/auth.json", profile.folder),
                pointer: KEY_FIELD.to_string(),
            })
        );
        assert_eq!(
            logins[0].identity.email.as_deref(),
            Some("dev@example.com")
        );
        assert!(!format!("{logins:?}").contains("ex-made-up-key"));
    }

    #[test]
    fn the_key_of_a_login_opens_as_a_handle() {
        let host = host();
        let profile = profile(&host, ".example-agent", true);
        let login = &host.as_agent(AGENT, || Example::logins(&profile))[0];

        let fingerprint = host.as_agent(AGENT, || {
            otty_plugin_sdk::keys::open(&login.place)?.fingerprint()
        });

        assert_eq!(
            fingerprint.map(|fingerprint| fingerprint.len()),
            Ok(64),
            "SHA-256 in hex"
        );
    }

    /// Issue 25: each session file tells its one session, as OTTY reads
    /// the files of the session places.
    #[test]
    fn each_session_file_tells_its_session() {
        let host = host();
        let profile = profile(&host, ".example-agent", true);
        let places = Example::session_places(&profile);
        let items = host.place_items(AGENT, &places);

        let sessions: Vec<Session> = items
            .iter()
            .flat_map(|item| {
                host.as_agent(AGENT, || Example::read_sessions(&profile, item))
                    .expect("the sessions")
            })
            .collect();

        let listed: Vec<(&str, Option<&str>, Option<&str>)> = sessions
            .iter()
            .map(|session| {
                (
                    session.id.as_str(),
                    session.title.as_deref(),
                    session.folder.as_deref(),
                )
            })
            .collect();
        assert_eq!(
            listed,
            [
                ("s1", Some("Fix the cart total"), Some("/home/dev/shop")),
                (
                    "s2",
                    Some("Write the release notes"),
                    Some("/home/dev/blog")
                ),
            ]
        );
    }

    #[test]
    fn answers_are_read_with_the_records_seen_and_understood() {
        let host = host();
        let profile = profile(&host, ".example-agent", true);
        let places = Example::spend_places(&profile);
        let items = host.place_items(AGENT, &places);
        let item = items
            .iter()
            .find(|item| item.file.path.ends_with("/s2.jsonl"))
            .expect("the file of s2");

        let answers = host
            .as_agent(AGENT, || Example::read_answers(&profile, item))
            .expect("the answers");

        assert_eq!(answers.answers.len(), 1);
        assert_eq!(answers.answers[0].identity.as_deref(), Some("b1"));
        assert_eq!(answers.answers[0].session.as_deref(), Some("s2"));
        assert_eq!(answers.answers[0].tokens.output, 900);
        assert_eq!(
            answers.answers[0].project.as_deref(),
            Some("/home/dev/blog")
        );
        assert_eq!(answers.records.seen, 3);
        assert_eq!(answers.records.understood, 1);
        assert_eq!(answers.records.version.as_deref(), Some("1.4.2"));
        assert_eq!(answers.records.missing, ["tokens", "record"]);
        assert!(answers.records.fields.contains(&"usage".to_string()));
    }

    #[test]
    fn a_found_profile_launches_with_its_folder() {
        let host = host();
        let work = profile(&host, ".example-agent-work", false);

        let command = Example::launch(&work, Some("w1"));

        assert_eq!(command.program, PROGRAM);
        assert_eq!(command.args, ["--resume", "w1"]);
        assert_eq!(command.env, [(HOME_VARIABLE.to_string(), work.folder)]);
    }
}

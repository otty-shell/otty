//! The agent part `example-agent`: a made-up coding agent that keeps its
//! login in `auth.json`, its version in `state.json` and its sessions in
//! `sessions/*.jsonl` of its config folder, `~/.example-agent` by default.

use std::path::Path;

use otty_plugin_sdk::{
    Agent, Answers, Billing, Command, Connection, HostError, Identity,
    JsonField, KeyPlace, Login, LoginKind, Profile, ProfilePlace,
    ProfileStatus, ProviderRef, ReadError, Session, SpendItem, SpendKind,
    SpendPlace, env, files, log,
};
use serde::Deserialize;

use crate::Example;
use crate::session_file::SessionFile;

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
    fn profiles(_agent: &str) -> Vec<ProfilePlace> {
        let home = env::home();
        let default = env::var(HOME_VARIABLE)
            .unwrap_or_else(|| format!("{home}/{DEFAULT_FOLDER}"));
        let mut places = vec![ProfilePlace {
            folder: default,
            is_default: true,
        }];

        let sessions = format!("{home}/{DEFAULT_FOLDER}-*/sessions/*.jsonl");
        for file in files::list(&sessions).unwrap_or_default() {
            let Some(folder) = profile_folder(&file.path) else {
                continue;
            };
            if places.iter().all(|place| place.folder != folder) {
                places.push(ProfilePlace {
                    folder,
                    is_default: false,
                });
            }
        }

        places
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
            _ => ProfileStatus::NotLoggedIn,
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

    fn sessions(profile: &Profile) -> Result<Vec<Session>, ReadError> {
        let glob = format!("{}/sessions/*.jsonl", profile.folder);
        let found = files::list(&glob).map_err(ReadError::Host)?;

        let mut sessions = Vec::new();
        for file in found {
            let session = SessionFile::read(&file.path)?;
            let id = Path::new(&file.path)
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
                .unwrap_or_default();

            sessions.push(Session {
                id,
                folder: session.project().unwrap_or_default().to_string(),
                title: session.title().unwrap_or("Untitled").to_string(),
                changed_ms: session.last_at_ms().unwrap_or(file.changed_ms),
            });
        }
        sessions.sort_by_key(|session| std::cmp::Reverse(session.changed_ms));

        Ok(sessions)
    }

    fn spend_places(profile: &Profile) -> Vec<SpendPlace> {
        vec![SpendPlace {
            glob: format!("{}/sessions/*.jsonl", profile.folder),
            kind: SpendKind::Files,
        }]
    }

    fn read_answers(
        _profile: &Profile,
        item: &SpendItem,
    ) -> Result<Answers, ReadError> {
        SessionFile::read(&item.path).map(SessionFile::into_answers)
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

    fn login_command(profile: &Profile) -> Command {
        Command {
            program: PROGRAM.to_string(),
            args: vec!["login".to_string()],
            env: folder_env(profile),
        }
    }
}

/// `auth.json` of the profile, its key replaced by a handle.
fn read_auth(profile: &Profile) -> Result<AuthFile, HostError> {
    let document = files::json_with_keys(&auth_path(profile))?;

    serde_json::from_str(document.json()).map_err(|_| {
        HostError::Failed("auth.json is not an auth file".to_string())
    })
}

fn auth_path(profile: &Profile) -> String {
    format!("{}/auth.json", profile.folder)
}

/// Whether the profile's config folder holds any file.
fn folder_exists(profile: &Profile) -> bool {
    files::list(&format!("{}/**", profile.folder))
        .is_ok_and(|found| !found.is_empty())
}

/// The config folder of a session file `<folder>/sessions/<id>.jsonl`.
fn profile_folder(session_file: &str) -> Option<String> {
    let folder = Path::new(session_file).parent()?.parent()?;

    Some(folder.to_string_lossy().into_owned())
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

    #[test]
    fn profiles_are_the_default_folder_and_the_found_ones() {
        let host = host();

        let places = host.as_agent(AGENT, || Example::profiles(AGENT));

        assert_eq!(
            places,
            [
                ProfilePlace {
                    folder: profile(&host, ".example-agent", true).folder,
                    is_default: true,
                },
                ProfilePlace {
                    folder: profile(&host, ".example-agent-work", false).folder,
                    is_default: false,
                },
            ]
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

    #[test]
    fn sessions_are_newest_first_with_their_titles() {
        let host = host();
        let profile = profile(&host, ".example-agent", true);

        let sessions = host
            .as_agent(AGENT, || Example::sessions(&profile))
            .expect("the sessions");

        let listed: Vec<(&str, &str, &str)> = sessions
            .iter()
            .map(|session| {
                (
                    session.id.as_str(),
                    session.title.as_str(),
                    session.folder.as_str(),
                )
            })
            .collect();
        assert_eq!(
            listed,
            [
                ("s2", "Write the release notes", "/home/dev/blog"),
                ("s1", "Fix the cart total", "/home/dev/shop"),
            ]
        );
    }

    #[test]
    fn answers_are_read_with_the_records_seen_and_understood() {
        let host = host();
        let profile = profile(&host, ".example-agent", true);
        let places = Example::spend_places(&profile);
        let item = SpendItem {
            path: format!("{}/sessions/s2.jsonl", profile.folder),
            kind: places[0].kind,
        };

        let answers = host
            .as_agent(AGENT, || Example::read_answers(&profile, &item))
            .expect("the answers");

        assert_eq!(answers.answers.len(), 1);
        assert_eq!(answers.answers[0].identity.as_deref(), Some("b1"));
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

//! OTTY's own login on disk (spec 2026-11-23-accounts-apart-from-profiles,
//! Decision 6): a new own login with a fresh id and the login command its
//! provider part gives for its folder, the empty folder
//! `<data dir>/accounts/<id>/` made on Continue, the question to the part
//! whether the folder holds an account once the login tab closed, and the
//! removal of that folder when it holds none or the user removed its
//! account. Log in again asks the part for the login command of a folder
//! already made. They touch files or call a plugin part, so each runs on
//! a thread of its own. Only the folder of an own login is ever removed,
//! never a profile folder.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::path::Path;
use std::sync::Arc;
use std::time::SystemTime;

use iced::Task;
use otty_agents::{AgentId, OwnLogin, PluginProvider, ProviderId, SourceError};
use otty_app_agents::{AgentsIntent, CheckedOwnLogin, PreparedOwnLogin};
use otty_app_i18n::{self as i18n, Key};

use crate::background::{on_own_thread, read_stopped};
use crate::plugin_parts::ProviderList;

/// Make a new own login of the provider part `provider` of the plugin
/// `plugin`, logged in with its agent part `agent`, with a fresh id, and
/// ask the part in the background for the agent's login command pointed
/// at its folder in OTTY's data folder `data_dir`. Nothing is made on
/// disk. The answer comes back as [`AgentsIntent::OwnLoginPrepared`].
///
/// # Example
///
/// ```
/// use std::path::Path;
/// use std::sync::Arc;
///
/// use otty_agents::{AgentId, ProviderId};
///
/// let providers: otty_app_agents_io::ProviderList = Arc::from(Vec::new());
/// let asked = otty_app_agents_io::prepare_own_login(
///     &providers,
///     Path::new("/home/user/.local/share/otty"),
///     String::from("example"),
///     ProviderId::new("example-cloud"),
///     AgentId::new("coder"),
/// );
///
/// assert_eq!(asked.units(), 1);
/// ```
pub fn prepare_own_login(
    providers: &ProviderList,
    data_dir: &Path,
    plugin: String,
    provider: ProviderId,
    agent: AgentId,
) -> Task<AgentsIntent> {
    let login = OwnLogin::new(new_login_id(), plugin, provider, agent);

    ask_login_command(
        providers,
        data_dir,
        login,
        AgentsIntent::OwnLoginPrepared,
    )
}

/// Ask the provider part of `providers` that made `login`, in the
/// background, for the agent's login command pointed at its folder in
/// `data_dir`, for Log in again (issue 09 of spec
/// 2026-11-23-accounts-apart-from-profiles). The answer comes back as
/// [`AgentsIntent::LogInAgainPrepared`].
///
/// # Example
///
/// ```
/// use std::path::Path;
/// use std::sync::Arc;
///
/// use otty_agents::{AgentId, OwnLogin, ProviderId};
///
/// let providers: otty_app_agents_io::ProviderList = Arc::from(Vec::new());
/// let login = OwnLogin::new(
///     "6f0c1e2a",
///     "example",
///     ProviderId::new("example-cloud"),
///     AgentId::new("coder"),
/// );
/// let asked = otty_app_agents_io::log_in_again(
///     &providers,
///     Path::new("/home/user/.local/share/otty"),
///     login,
/// );
///
/// assert_eq!(asked.units(), 1);
/// ```
pub fn log_in_again(
    providers: &ProviderList,
    data_dir: &Path,
    login: OwnLogin,
) -> Task<AgentsIntent> {
    ask_login_command(
        providers,
        data_dir,
        login,
        AgentsIntent::LogInAgainPrepared,
    )
}

/// Make the empty folder of `login` in OTTY's data folder `data_dir` in
/// the background. The answer comes back as
/// [`AgentsIntent::OwnLoginFolderMade`] with the error ready to show, in
/// the user's language.
pub fn make_own_login_folder(
    data_dir: &Path,
    login: OwnLogin,
) -> Task<AgentsIntent> {
    let data_dir = data_dir.to_path_buf();
    let answer = login.clone();

    on_own_thread(
        String::from("making the folder of an own login"),
        move || make_folder(&data_dir, &login),
        || Err(i18n::t(Key::AgentsSaveStopped).to_string()),
        move |made| AgentsIntent::OwnLoginFolderMade {
            login: answer,
            made,
        },
    )
}

/// Ask the provider part of `providers` that made `login`, in the
/// background, whether its folder in `data_dir` holds an account now. The
/// answer comes back as [`AgentsIntent::OwnLoginChecked`].
pub fn check_own_login(
    providers: &ProviderList,
    data_dir: &Path,
    login: OwnLogin,
) -> Task<AgentsIntent> {
    let providers = Arc::clone(providers);
    let data_dir = data_dir.to_path_buf();
    let fallback = login.clone();

    on_own_thread(
        String::from("asking for the accounts of an own login"),
        move || {
            let found = has_account(&providers, &data_dir, &login);
            CheckedOwnLogin { login, found }
        },
        move || CheckedOwnLogin {
            login: fallback,
            found: Err(read_stopped()),
        },
        AgentsIntent::OwnLoginChecked,
    )
}

/// Remove the folder of `login` in `data_dir` in the background, with
/// everything in it, and nothing else. The answer comes back as
/// [`AgentsIntent::OwnLoginFolderRemoved`] with the error ready to show.
pub fn remove_own_login_folder(
    data_dir: &Path,
    login: OwnLogin,
) -> Task<AgentsIntent> {
    let data_dir = data_dir.to_path_buf();

    on_own_thread(
        String::from("removing the folder of an own login"),
        move || remove_folder(&data_dir, &login),
        || Err(i18n::t(Key::AgentsSaveStopped).to_string()),
        AgentsIntent::OwnLoginFolderRemoved,
    )
}

/// Ask the part that made `login` for its login command in the
/// background; `answer` makes the intent of the answer.
fn ask_login_command(
    providers: &ProviderList,
    data_dir: &Path,
    login: OwnLogin,
    answer: fn(PreparedOwnLogin) -> AgentsIntent,
) -> Task<AgentsIntent> {
    let providers = Arc::clone(providers);
    let data_dir = data_dir.to_path_buf();
    let fallback = login.clone();

    on_own_thread(
        format!("asking {} for its login", login.provider().as_str()),
        move || {
            let command =
                otty_agents::log_in_command(&providers, &login, &data_dir);
            PreparedOwnLogin { login, command }
        },
        move || PreparedOwnLogin {
            login: fallback,
            command: Err(read_stopped()),
        },
        answer,
    )
}

/// A new id of an own login: 16 hex digits from the system's random seed
/// of a hasher and the time, a plain folder name.
fn new_login_id() -> String {
    let mut hasher = RandomState::new().build_hasher();
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos());
    hasher.write_u128(nanos);
    hasher.write_u32(std::process::id());

    format!("{:016x}", hasher.finish())
}

/// Make the empty folder of `login` in `data_dir`, and `accounts/` before
/// it. A folder that is there already is not taken.
fn make_folder(data_dir: &Path, login: &OwnLogin) -> Result<(), String> {
    let folder = login.folder_in(data_dir);
    let not_made = |error: std::io::Error| {
        folder_text(Key::TplAgentsOwnFolderNotMade, &folder, &error)
    };

    if let Some(accounts) = folder.parent() {
        std::fs::create_dir_all(accounts).map_err(not_made)?;
    }
    std::fs::create_dir(&folder).map_err(not_made)
}

/// Whether the part of `providers` that made `login` finds an account in
/// its folder in `data_dir`.
fn has_account(
    providers: &[PluginProvider],
    data_dir: &Path,
    login: &OwnLogin,
) -> Result<bool, SourceError> {
    let accounts = otty_agents::own_login_accounts(providers, login, data_dir)?;

    Ok(!accounts.is_empty())
}

/// Remove the folder of `login` in `data_dir` with what it holds. A
/// folder that is gone already is removed.
fn remove_folder(data_dir: &Path, login: &OwnLogin) -> Result<(), String> {
    let folder = login.folder_in(data_dir);

    match std::fs::remove_dir_all(&folder) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(folder_text(
            Key::TplAgentsOwnFolderNotRemoved,
            &folder,
            &error,
        )),
    }
}

/// The text `key` of `folder` and the system's reason `error`, in the
/// user's language.
fn folder_text(key: Key, folder: &Path, error: &std::io::Error) -> String {
    i18n::filled(
        key,
        &[
            ("path", &folder.display().to_string()),
            ("error", &format!("{error}")),
        ],
    )
}

#[cfg(test)]
mod tests {
    use otty_agents::{AgentId, OwnLogin, ProviderId};
    use otty_app_testing::temp_dir::TempDir;

    use super::*;
    use crate::testing::{providers_with_plugins, write};

    fn claude_login(id: &str) -> OwnLogin {
        OwnLogin::new(
            id,
            "anthropic",
            ProviderId::new("anthropic"),
            AgentId::new("claude-code"),
        )
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decision 6: Continue
    /// makes an empty folder `<data dir>/accounts/<id>/`.
    #[test]
    fn the_folder_of_a_new_own_login_is_made_empty_under_accounts() {
        let data = TempDir::new("own-login-make");
        let login = claude_login("6f0c1e2a");

        let made = make_folder(data.path(), &login);

        let folder = data.path().join("accounts").join("6f0c1e2a");
        assert_eq!(made, Ok(()));
        assert!(folder.is_dir());
        assert_eq!(std::fs::read_dir(&folder).expect("a folder").count(), 0);
    }

    /// A folder that is there already is not taken: it may hold another
    /// login.
    #[test]
    fn a_folder_that_is_there_already_is_not_made_again() {
        let data = TempDir::new("own-login-there");
        let login = claude_login("6f0c1e2a");
        write(
            &data.path().join("accounts").join("6f0c1e2a").join("a.json"),
            "{}",
        );

        let made = make_folder(data.path(), &login);

        assert!(made.is_err(), "{made:?}");
    }

    /// Each new own login gets an id of its own, a plain name.
    #[test]
    fn each_new_own_login_gets_a_new_plain_id() {
        let ids: Vec<String> = (0..20).map(|_| new_login_id()).collect();

        let mut unique = ids.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), ids.len(), "{ids:?}");
        assert!(ids.iter().all(|id| {
            id.len() == 16 && id.chars().all(|char| char.is_ascii_hexdigit())
        }));
    }

    /// Decision 6: once the login tab closes, the provider part is asked
    /// about the folder: nothing there is no login, the Claude login there
    /// is an account.
    #[test]
    fn the_part_tells_whether_the_folder_holds_an_account() {
        let home = TempDir::new("own-login-check");
        let data = home.path().join("data");
        let providers = providers_with_plugins(home.path());
        let empty = claude_login("a1");
        let logged_in = claude_login("b2");
        std::fs::create_dir_all(empty.folder_in(&data)).expect("a folder");
        let folder = logged_in.folder_in(&data);
        write(
            &folder.join(".claude.json"),
            r#"{"oauthAccount":{"accountUuid":"00000000-0000-4000-8000-00000000000a","emailAddress":"own@example.com"}}"#,
        );
        write(
            &folder.join(".credentials.json"),
            r#"{"claudeAiOauth":{"accessToken":"made-up","refreshToken":"made-up","expiresAt":4102444800000}}"#,
        );

        let nothing = has_account(&providers, &data, &empty);
        let found = has_account(&providers, &data, &logged_in);

        assert_eq!(nothing, Ok(false));
        assert_eq!(found, Ok(true));
    }

    /// Only the folder of the own login goes: the other own logins and the
    /// data folder stay.
    #[test]
    fn removing_the_folder_of_an_own_login_removes_it_only() {
        let data = TempDir::new("own-login-remove");
        let gone = claude_login("a1");
        let kept = claude_login("b2");
        write(&gone.folder_in(data.path()).join("x.json"), "{}");
        write(&kept.folder_in(data.path()).join("y.json"), "{}");

        let removed = remove_folder(data.path(), &gone);

        assert_eq!(removed, Ok(()));
        assert!(!gone.folder_in(data.path()).exists());
        assert!(kept.folder_in(data.path()).join("y.json").exists());
    }
}

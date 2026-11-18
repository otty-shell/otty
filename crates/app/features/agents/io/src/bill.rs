//! The read of the real bill of one API key profile (spec Decision 9). It
//! reads the admin key from its file or its environment variable, asks the
//! provider's server with it and drops it, so it runs on a thread of its
//! own.

use std::time::SystemTime;

use iced::Task;
use otty_agents::{Agent, Profile, SourceError};
use otty_app_agents::{AgentList, AgentsIntent, LoadedBill};

use crate::load::on_thread;
use crate::shell_variable::shell_variable;

/// Read the bill of `profile` in the background, with the bill source of
/// the agent of `agents` it belongs to and the profile's admin key. The key
/// is read from its file, or from its variable as the user's login shell
/// at `shell` sees it, so a variable set in `~/.zshrc` is found when OTTY
/// was started from the desktop. The key is used for this read only and
/// never logged. The answer comes back as [`AgentsIntent::BillLoaded`].
///
/// # Example
///
/// ```
/// use std::path::PathBuf;
/// use std::sync::Arc;
///
/// use otty_agents::{AgentId, Profile};
///
/// let profile = Profile::default_for(
///     AgentId::new("codex"),
///     "Codex",
///     PathBuf::from("/home/user/.codex"),
/// );
/// let read = otty_app_agents_io::load_bill(
///     Arc::from(otty_agents::known_agents()),
///     profile,
///     "/bin/zsh",
/// );
///
/// assert_eq!(read.units(), 1);
/// ```
pub fn load_bill(
    agents: AgentList,
    profile: Profile,
    shell: &str,
) -> Task<AgentsIntent> {
    let fallback = profile.clone();
    let shell = shell.to_string();

    Task::perform(
        on_thread(move || {
            read_bill(&agents, profile, SystemTime::now(), |name| {
                shell_variable(&shell, name)
            })
        }),
        move |loaded| {
            let loaded = loaded.unwrap_or_else(|| {
                log::warn!("reading the bill of {} stopped", fallback.name());
                LoadedBill {
                    profile: fallback,
                    bill: Err(SourceError::NotAvailable(String::from(
                        "the read stopped",
                    ))),
                    read_at: SystemTime::now(),
                }
            });
            AgentsIntent::BillLoaded(loaded)
        },
    )
}

/// The bill of `profile` up to `now`, read with its admin key; `env_var`
/// gives the value of an environment variable.
fn read_bill(
    agents: &[Box<dyn Agent>],
    profile: Profile,
    now: SystemTime,
    env_var: impl Fn(&str) -> Option<String>,
) -> LoadedBill {
    let bill = otty_agents::read_bill(agents, &profile, now, env_var);

    LoadedBill {
        profile,
        bill,
        read_at: SystemTime::now(),
    }
}

#[cfg(all(test, unix))]
mod tests {
    use std::path::Path;
    use std::sync::Arc;

    use otty_agents::{AdminKeyRef, AgentId, known_agents};
    use otty_app_testing::temp_dir::TempDir;

    use super::*;
    use crate::load::tests::run;
    use crate::testing::fake_shell;

    /// No environment variable is set: no test reads the real environment.
    fn no_variables(_: &str) -> Option<String> {
        None
    }

    /// A Claude Code API key profile in a made-up folder, with its admin
    /// key at `admin_key`.
    fn claude_api(home: &Path, admin_key: Option<AdminKeyRef>) -> Profile {
        Profile::added(
            AgentId::new("claude-code"),
            "Claude Code · API",
            home.join(".claude-api"),
        )
        .with_admin_key(admin_key)
    }

    #[test]
    fn a_key_file_that_is_missing_is_not_available_and_nothing_is_sent() {
        let home = TempDir::new("agents-bill-no-file");
        let key_file = home.path().join("no-such-admin-key");
        let profile =
            claude_api(home.path(), Some(AdminKeyRef::File(key_file.clone())));

        let before = SystemTime::now();

        let loaded = read_bill(
            &known_agents(),
            profile.clone(),
            SystemTime::now(),
            no_variables,
        );

        assert_eq!(loaded.profile, profile);
        assert!(loaded.read_at >= before, "the read tells when it ended");
        assert_eq!(
            loaded.bill,
            Err(SourceError::NotAvailable(format!(
                "the admin key file {} is missing",
                key_file.display()
            )))
        );
    }

    #[test]
    fn an_unset_variable_is_not_available() {
        let home = TempDir::new("agents-bill-no-variable");
        let profile = claude_api(
            home.path(),
            Some(AdminKeyRef::EnvVar(String::from("OTTY_TEST_ADMIN_KEY"))),
        );

        let loaded = read_bill(
            &known_agents(),
            profile,
            SystemTime::now(),
            no_variables,
        );

        assert_eq!(
            loaded.bill,
            Err(SourceError::NotAvailable(String::from(
                "OTTY_TEST_ADMIN_KEY is not set"
            )))
        );
    }

    #[test]
    fn the_variable_is_read_through_the_login_shell() {
        let home = TempDir::new("agents-bill-shell-variable");
        let shell = fake_shell(&home);
        let profile = claude_api(
            home.path(),
            Some(AdminKeyRef::EnvVar(String::from(
                "OTTY_TEST_NO_SUCH_ADMIN_KEY",
            ))),
        );
        let agents: AgentList = Arc::from(known_agents());

        let answers = run(load_bill(agents, profile, &shell));

        let starts = std::fs::read_to_string(home.path().join("starts"))
            .map_or(0, |text| text.lines().count());
        assert_eq!(starts, 1, "the shell was asked for the variable");
        assert!(matches!(
            answers.as_slice(),
            [AgentsIntent::BillLoaded(loaded)]
                if loaded.bill == Err(SourceError::NotAvailable(String::from(
                    "OTTY_TEST_NO_SUCH_ADMIN_KEY is not set"
                )))
        ));
    }

    #[test]
    fn the_read_runs_in_the_background_and_answers_with_an_intent() {
        let home = TempDir::new("agents-bill-task");
        let key_file = home.path().join("no-such-admin-key");
        let profile =
            claude_api(home.path(), Some(AdminKeyRef::File(key_file)));
        let agents: AgentList = Arc::from(known_agents());

        let answers = run(load_bill(agents, profile.clone(), "/no/such/shell"));

        assert!(matches!(
            answers.as_slice(),
            [AgentsIntent::BillLoaded(loaded)]
                if loaded.profile == profile && loaded.bill.is_err()
        ));
    }
}

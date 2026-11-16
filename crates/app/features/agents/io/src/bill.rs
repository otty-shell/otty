//! The read of the real bill of one API key profile (spec Decision 9). It
//! reads the admin key from its file or its environment variable, asks the
//! provider's server with it and drops it, so it runs on a thread of its
//! own.

use std::time::SystemTime;

use iced::Task;
use otty_agents::{Agent, Profile, SourceError};
use otty_app_agents::{AgentList, AgentsIntent, LoadedBill};

use crate::load::on_thread;

/// Read the bill of `profile` in the background, with the bill source of
/// the agent of `agents` it belongs to and the profile's admin key. The key
/// is read from its file or from OTTY's environment, used for this read
/// only and never logged. The answer comes back as
/// [`AgentsIntent::BillLoaded`].
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
/// );
///
/// assert_eq!(read.units(), 1);
/// ```
pub fn load_bill(agents: AgentList, profile: Profile) -> Task<AgentsIntent> {
    let fallback = profile.clone();

    Task::perform(
        on_thread(move || {
            read_bill(&agents, profile, SystemTime::now(), |name| {
                std::env::var(name).ok()
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
                }
            });
            AgentsIntent::BillLoaded(loaded)
        },
    )
}

/// The bill of `profile` up to `now`, read with its admin key; `env_var`
/// gives the value of an environment variable. An agent with no bill and a
/// profile with no admin key are "not available".
fn read_bill(
    agents: &[Box<dyn Agent>],
    profile: Profile,
    now: SystemTime,
    env_var: impl Fn(&str) -> Option<String>,
) -> LoadedBill {
    let source = agents
        .iter()
        .find(|agent| agent.descriptor().id() == profile.agent())
        .and_then(|agent| agent.bill_source());

    let bill = match (source, profile.admin_key()) {
        (None, _) => Err(SourceError::NotAvailable(String::from(
            "the agent has no bill",
        ))),
        (Some(_), None) => {
            Err(SourceError::NotAvailable(String::from("no admin key")))
        },
        (Some(source), Some(admin_key)) => admin_key
            .read(env_var)
            .and_then(|key| source.bill(&key, now)),
    };

    LoadedBill { profile, bill }
}

#[cfg(all(test, unix))]
mod tests {
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    use otty_agents::{AdminKeyRef, AgentId, known_agents};
    use otty_app_testing::temp_dir::TempDir;

    use super::*;
    use crate::load::tests::run;

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

        let loaded = read_bill(
            &known_agents(),
            profile.clone(),
            SystemTime::now(),
            no_variables,
        );

        assert_eq!(loaded.profile, profile);
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
    fn no_admin_key_or_an_agent_with_no_bill_is_not_available() {
        let home = TempDir::new("agents-bill-no-key");
        let codex = Profile::added(
            AgentId::new("codex"),
            "Codex · API",
            PathBuf::from("/home/user/.codex-api"),
        )
        .with_admin_key(Some(AdminKeyRef::EnvVar(String::from("KEY"))));

        let no_key = read_bill(
            &known_agents(),
            claude_api(home.path(), None),
            SystemTime::now(),
            no_variables,
        );
        let no_bill =
            read_bill(&known_agents(), codex, SystemTime::now(), no_variables);

        assert_eq!(
            no_key.bill,
            Err(SourceError::NotAvailable(String::from("no admin key")))
        );
        assert_eq!(
            no_bill.bill,
            Err(SourceError::NotAvailable(String::from(
                "the agent has no bill"
            )))
        );
    }

    #[test]
    fn the_read_runs_in_the_background_and_answers_with_an_intent() {
        let home = TempDir::new("agents-bill-task");
        let key_file = home.path().join("no-such-admin-key");
        let profile =
            claude_api(home.path(), Some(AdminKeyRef::File(key_file)));
        let agents: AgentList = Arc::from(known_agents());

        let answers = run(load_bill(agents, profile.clone()));

        assert!(matches!(
            answers.as_slice(),
            [AgentsIntent::BillLoaded(loaded)]
                if loaded.profile == profile && loaded.bill.is_err()
        ));
    }
}

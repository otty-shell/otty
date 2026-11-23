//! The read of the limits of one profile: the provider part of the plugin
//! that reads the account whose limits the profile shows reads them, any
//! way it can (spec 2026-11-19-agent-plugins, issue 29). It may ask a
//! server over the network or start the agent's program, so it runs on a
//! thread of its own.

use std::sync::Arc;
use std::time::SystemTime;

use iced::Task;
use otty_agents::{
    Account, AccountNumbers, Agent, PluginProvider, Profile, SourceError,
    SourceFailure, SourceId, Unavailable,
};
use otty_app_agents::{AgentList, AgentsIntent, LimitsRead, LoadedLimits};

use crate::background::{on_own_thread, read_stopped};
use crate::plugin_parts::ProviderList;

/// The folder in OTTY's cache folder that an agent's program started in a
/// hidden terminal runs in, so it never shows up as a session of a real
/// project.
pub(crate) const PROBE_FOLDER: &str = "agent-probe";

/// What one read of limits reads: the profile it is read through, the
/// account whose limits it shows, and every profile the account was found
/// in, the profile first.
struct LimitsToRead {
    profile: Profile,
    account: Option<Account>,
    found_in: Vec<Profile>,
}

/// Read the limits of `profile` in the background: the provider part of
/// `providers` that reads `account`, the account whose limits the profile
/// shows, reads them, told of every profile of `found_in` it was found in,
/// with the agent parts of `agents` that found it. It may start the
/// agent's program, as is or in a hidden terminal. With no account there
/// is nothing to read. The answer comes back as
/// [`AgentsIntent::LimitsLoaded`].
///
/// # Example
///
/// ```
/// use std::path::PathBuf;
/// use std::sync::Arc;
///
/// use otty_agents::{AgentId, Profile};
/// use otty_app_agents::AgentList;
///
/// let profile = Profile::default_for(
///     AgentId::new("coder"),
///     "Coder",
///     PathBuf::from("/home/user/.coder"),
/// );
/// let agents: AgentList = Arc::from(Vec::new());
/// let providers: otty_app_agents_io::ProviderList = Arc::from(Vec::new());
/// let read = otty_app_agents_io::load_limits(
///     &agents,
///     &providers,
///     profile.clone(),
///     None,
///     vec![profile],
/// );
///
/// assert_eq!(read.units(), 1);
/// ```
pub fn load_limits(
    agents: &AgentList,
    providers: &ProviderList,
    profile: Profile,
    account: Option<Account>,
    found_in: Vec<Profile>,
) -> Task<AgentsIntent> {
    let asked = LimitsToRead {
        profile,
        account,
        found_in,
    };

    read_task(agents, providers, asked, true)
}

/// Read the limits of `profile` in the background while Agents mode is not
/// on screen, as [`load_limits`] does, except that the provider part
/// starts no program, neither `codex app-server` nor `claude` in a hidden
/// terminal, and a read on screen starts it (spec Decision 10).
pub fn load_limits_in_background(
    agents: &AgentList,
    providers: &ProviderList,
    profile: Profile,
    account: Option<Account>,
    found_in: Vec<Profile>,
) -> Task<AgentsIntent> {
    let asked = LimitsToRead {
        profile,
        account,
        found_in,
    };

    read_task(agents, providers, asked, false)
}

/// The read of `asked` with the agents and the provider parts on a thread
/// of its own, which may start the agent's program when
/// `starts_programs`.
fn read_task(
    agents: &AgentList,
    providers: &ProviderList,
    asked: LimitsToRead,
    starts_programs: bool,
) -> Task<AgentsIntent> {
    let task = format!("reading the limits of {}", asked.profile.name());
    let agents = Arc::clone(agents);
    let providers = Arc::clone(providers);
    let fallback = asked.profile.clone();
    let has_account = asked.account.is_some();

    on_own_thread(
        task,
        move || read_limits(&agents, &providers, asked, starts_programs),
        move || stopped_read(fallback, has_account),
        AgentsIntent::LimitsLoaded,
    )
}

/// The limits of `asked`, with the time the read ended: what the provider
/// part that reads its account answered. The part's programs start only
/// when `starts_programs`.
fn read_limits(
    agents: &[Box<dyn Agent>],
    providers: &[PluginProvider],
    asked: LimitsToRead,
    starts_programs: bool,
) -> LoadedLimits {
    let LimitsToRead {
        profile,
        account,
        found_in,
    } = asked;

    let limits = match account {
        Some(account) => {
            let numbers = otty_agents::read_account_numbers(
                agents,
                providers,
                &account,
                &found_in,
                &[],
                starts_programs,
                |_| None,
            );
            limits_read(numbers)
        },
        None => LimitsRead::NoSources,
    };

    LoadedLimits {
        profile,
        limits,
        read_at: SystemTime::now(),
    }
}

/// What a read of the numbers of a subscription login tells of its
/// limits. Nothing in them, or no part that reads the account, is
/// nothing to show; any other failure is the failure of the provider part.
fn limits_read(numbers: Result<AccountNumbers, SourceError>) -> LimitsRead {
    match numbers {
        Ok(AccountNumbers::Limits(answer)) if !answer.value().is_nothing() => {
            LimitsRead::Answered(answer)
        },
        Ok(_) => LimitsRead::NoSources,
        Err(SourceError::NotAvailable(Unavailable::NoAccountSource)) => {
            LimitsRead::NoSources
        },
        Err(error) => LimitsRead::Failed(vec![SourceFailure::new(
            SourceId::ProviderPart,
            error,
        )]),
    }
}

/// What a read of the limits of `profile` that stopped before it answered
/// gives: the provider part failed because the read stopped, so the last
/// good answer stays, greyed, and the user sees why. A profile with no
/// account, `has_account` false, has nothing to read.
fn stopped_read(profile: Profile, has_account: bool) -> LoadedLimits {
    let limits = match has_account {
        true => LimitsRead::Failed(vec![SourceFailure::new(
            SourceId::ProviderPart,
            read_stopped(),
        )]),
        false => LimitsRead::NoSources,
    };

    LoadedLimits {
        profile,
        limits,
        read_at: SystemTime::now(),
    }
}

#[cfg(all(test, unix))]
mod tests {
    use std::path::PathBuf;

    use otty_agents::{
        AccountIdentity, AccountKind, AccountLogin, AgentId, FoundIn,
        ProviderRef,
    };
    use otty_app_testing::temp_dir::TempDir;

    use super::*;
    use crate::background::read_stopped;
    use crate::testing::{
        agents_and_providers_with_shell, executable_script, run, write,
    };

    /// The made-up id of the Claude account of the tests.
    const ACCOUNT_ID: &str = "00000000-0000-4000-8000-0000000000a1";

    /// A Claude Code profile in a made-up folder whose `.credentials.json`
    /// holds no token while `.claude.json` names its account: Claude Code
    /// still works with it (the owner's Semrush profile, issue 29).
    fn claude_without_token(home: &TempDir) -> Profile {
        let config_dir = home.path().join(".claude-test");
        write(&config_dir.join(".credentials.json"), "{}");
        write(
            &config_dir.join(".claude.json"),
            &format!(
                r#"{{"oauthAccount":{{"accountUuid":"{ACCOUNT_ID}","emailAddress":"dev@example.com"}}}}"#
            ),
        );

        Profile::added(
            AgentId::new("claude-code"),
            "Claude Code · Test",
            config_dir,
        )
    }

    /// The Claude login of [`claude_without_token`], found in `profile`
    /// and read by the Anthropic plugin.
    fn claude_login(profile: &Profile) -> Account {
        Account::new(
            ProviderRef::of_id("anthropic"),
            AccountIdentity::AccountId(ACCOUNT_ID.to_string()),
            AccountLogin::Email(String::from("dev@example.com")),
            AccountKind::Limits,
            FoundIn::ProfileFolder(profile.config_dir().to_path_buf()),
        )
        .with_read_by(Some(String::from("anthropic")))
    }

    /// A shell in `home` that only counts its starts, so no real `claude`
    /// ever runs.
    fn counting_shell(home: &TempDir) -> String {
        let starts = home.path().join("starts");

        executable_script(
            home.path(),
            "counting-shell",
            &format!("echo started >> '{}'\nexit 1", starts.display()),
        )
        .display()
        .to_string()
    }

    fn started(home: &TempDir) -> bool {
        home.path().join("starts").exists()
    }

    /// Issue 29, the owner's case: no token in the profile's files, so
    /// the Anthropic provider part reads the limits with `claude /usage`
    /// in the Claude Code profile the login was found in. The shell here
    /// only counts its starts, so the made-up `claude` never shows its
    /// panel; the plugin's own tests read the panel.
    #[test]
    fn a_login_with_no_token_is_read_with_usage_in_its_profile() {
        let home = TempDir::new("agents-limits-semrush");
        let profile = claude_without_token(&home);
        let shell = counting_shell(&home);
        let (agents, providers) =
            agents_and_providers_with_shell(home.path(), &shell);
        let asked = LimitsToRead {
            account: Some(claude_login(&profile)),
            found_in: vec![profile.clone()],
            profile: profile.clone(),
        };

        let loaded = read_limits(&agents, &providers, asked, true);

        assert!(started(&home), "`claude /usage` was not started");
        assert_eq!(loaded.profile, profile);
    }

    /// While Agents mode is away the provider part starts no hidden
    /// terminal: the read gives nothing, and the next read on screen
    /// starts it.
    #[test]
    fn a_read_in_the_background_starts_no_hidden_terminal() {
        let home = TempDir::new("agents-limits-background");
        let profile = claude_without_token(&home);
        let shell = counting_shell(&home);
        let (agents, providers) =
            agents_and_providers_with_shell(home.path(), &shell);
        let asked = LimitsToRead {
            account: Some(claude_login(&profile)),
            found_in: vec![profile.clone()],
            profile,
        };

        let loaded = read_limits(&agents, &providers, asked, false);

        assert!(!started(&home), "the shell was started");
        assert!(
            !matches!(loaded.limits, LimitsRead::Answered(_)),
            "{:?}",
            loaded.limits
        );
    }

    /// A profile with no subscription login has no limits to read.
    #[test]
    fn a_profile_with_no_account_has_nothing_to_read() {
        let profile = Profile::added(
            AgentId::new("stranger"),
            "Stranger",
            PathBuf::from("/home/user/.stranger"),
        );
        let asked = LimitsToRead {
            profile: profile.clone(),
            account: None,
            found_in: vec![profile],
        };

        let loaded = read_limits(&[], &[], asked, true);

        assert_eq!(loaded.limits, LimitsRead::NoSources);
    }

    /// An account no installed plugin reads has nothing to show.
    #[test]
    fn an_account_no_plugin_reads_has_nothing_to_read() {
        let home = TempDir::new("agents-limits-no-plugin");
        let profile = claude_without_token(&home);
        let asked = LimitsToRead {
            account: Some(claude_login(&profile)),
            found_in: vec![profile.clone()],
            profile,
        };

        let loaded = read_limits(&[], &[], asked, true);

        assert_eq!(loaded.limits, LimitsRead::NoSources);
    }

    #[test]
    fn the_reads_run_in_the_background_and_answer_with_an_intent() {
        let profile = Profile::added(
            AgentId::new("stranger"),
            "Stranger",
            PathBuf::from("/home/user/.stranger"),
        );
        let agents: AgentList = Arc::from(Vec::new());
        let providers: ProviderList = Arc::from(Vec::new());

        let on_screen = run(load_limits(
            &agents,
            &providers,
            profile.clone(),
            None,
            vec![profile.clone()],
        ));
        let away = run(load_limits_in_background(
            &agents,
            &providers,
            profile.clone(),
            None,
            vec![profile.clone()],
        ));

        for answers in [on_screen, away] {
            assert!(matches!(
                answers.as_slice(),
                [AgentsIntent::LimitsLoaded(loaded)]
                    if loaded.profile == profile
                        && loaded.limits == LimitsRead::NoSources
            ));
        }
    }

    /// A read that stopped before it answered fails at the provider part
    /// with why, so the last good limits stay, greyed, and the user is
    /// told; it never says nothing reads the limits.
    #[test]
    fn a_read_that_stopped_fails_at_the_provider_part_because_it_stopped() {
        let home = TempDir::new("agents-limits-stopped");
        let profile = claude_without_token(&home);

        let loaded = stopped_read(profile.clone(), true);
        let no_account = stopped_read(profile.clone(), false);

        assert_eq!(loaded.profile, profile);
        assert_eq!(
            loaded.limits,
            LimitsRead::Failed(vec![SourceFailure::new(
                SourceId::ProviderPart,
                read_stopped()
            )])
        );
        assert_eq!(no_account.limits, LimitsRead::NoSources);
    }
}

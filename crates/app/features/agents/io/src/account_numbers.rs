//! The read of the numbers of one account (spec
//! 2026-11-23-accounts-apart-from-profiles, Decision 5): the money left, a
//! key limit, or the limits of a subscription. The provider part that
//! found the account reads it in the folders it found it in, any way it
//! can (spec 2026-11-19-agent-plugins, issue 29), with handles to the keys
//! the user gave for it, which OTTY reads again and drops. It may ask a
//! server over the network or start the agent's program, so it runs on a
//! thread of its own.

use std::sync::Arc;
use std::time::SystemTime;

use iced::Task;
use otty_agents::{Account, GivenKey, PluginProvider};
use otty_app_agents::{AgentsIntent, LoadedAccountNumbers};

use crate::background::{on_own_thread, read_stopped};
use crate::plugin_parts::ProviderList;
use crate::shell_variable::shell_variable;

/// Read the numbers of `account` in the background by the provider part
/// of `providers` that found it, with the keys the user gave for it
/// (`given`), read from their files or variables, a variable as the
/// user's login shell at `shell` sees it. The part reads with a handle to
/// each key, any way it can, in the folders the account was found in; its
/// programs start only when `starts_programs`. A key is used for this read
/// only and never logged. The answer comes back as
/// [`AgentsIntent::AccountNumbersLoaded`].
///
/// # Example
///
/// ```
/// use std::sync::Arc;
///
/// use otty_agents::{Account, AccountKind, ProviderId};
///
/// let account = Account::new(
///     "example",
///     ProviderId::new("example-cloud"),
///     "chatgpt:made-up-account",
///     "you@example.com",
///     AccountKind::Subscription,
///     Vec::new(),
/// );
/// let providers: otty_app_agents_io::ProviderList = Arc::from(Vec::new());
/// let read = otty_app_agents_io::load_account_numbers(
///     &providers,
///     account,
///     Vec::new(),
///     true,
///     "/bin/zsh",
/// );
///
/// assert_eq!(read.units(), 1);
/// ```
pub fn load_account_numbers(
    providers: &ProviderList,
    account: Account,
    given: Vec<GivenKey>,
    starts_programs: bool,
    shell: &str,
) -> Task<AgentsIntent> {
    let task = format!(
        "reading the numbers of a {} account",
        account.provider().as_str()
    );
    let providers = Arc::clone(providers);
    let fallback = account.clone();
    let shell = shell.to_string();

    on_own_thread(
        task,
        move || {
            read_account_numbers(
                &providers,
                account,
                &given,
                starts_programs,
                |name| shell_variable(&shell, name),
            )
        },
        move || LoadedAccountNumbers {
            account: fallback,
            numbers: Err(read_stopped()),
            read_at: SystemTime::now(),
        },
        AgentsIntent::AccountNumbersLoaded,
    )
}

/// The numbers of `account`, read with the keys the user gave for it
/// (`given`), starting programs only when `starts_programs`; `env_var`
/// gives the value of an environment variable.
fn read_account_numbers(
    providers: &[PluginProvider],
    account: Account,
    given: &[GivenKey],
    starts_programs: bool,
    env_var: impl Fn(&str) -> Option<String>,
) -> LoadedAccountNumbers {
    let numbers = otty_agents::read_account_numbers(
        providers,
        &account,
        given,
        starts_programs,
        env_var,
    );

    LoadedAccountNumbers {
        account,
        numbers,
        read_at: SystemTime::now(),
    }
}

#[cfg(all(test, unix))]
mod tests {
    use std::path::Path;

    use otty_agents::{
        AccountKind, AccountNumbers, AgentId, FoundIn, ProviderId, SourceError,
        Unavailable,
    };
    use otty_app_testing::temp_dir::TempDir;

    use super::*;
    use crate::testing::{
        agents_and_providers_with_shell, executable_script,
        providers_with_plugins, run, write,
    };

    /// The made-up id of the Claude account of the tests.
    const ACCOUNT_ID: &str = "00000000-0000-4000-8000-0000000000a1";

    /// The Claude login the Anthropic plugin found in a made-up Claude
    /// Code folder under `home`, whose `.credentials.json` holds no token
    /// while `.claude.json` names its account: Claude Code still works
    /// with it (the owner's Semrush profile, issue 29).
    fn claude_login_without_token(home: &TempDir) -> Account {
        let config_dir = home.path().join(".claude-test");
        write(&config_dir.join(".credentials.json"), "{}");
        write(
            &config_dir.join(".claude.json"),
            &format!(
                r#"{{"oauthAccount":{{"accountUuid":"{ACCOUNT_ID}","emailAddress":"dev@example.com"}}}}"#
            ),
        );

        Account::new(
            "anthropic",
            ProviderId::new("anthropic"),
            format!("claude:{ACCOUNT_ID}"),
            "dev@example.com",
            AccountKind::Subscription,
            vec![FoundIn::profile(AgentId::new("claude-code"), config_dir)],
        )
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

    /// Issue 29, the owner's case: no token in the login's files, so the
    /// Anthropic provider part reads the limits with `claude /usage` in
    /// the Claude Code folder the account was found in. The shell here
    /// only counts its starts, so the made-up `claude` never shows its
    /// panel; the plugin's own tests read the panel.
    #[test]
    fn a_login_with_no_token_is_read_with_usage_in_its_folder() {
        let home = TempDir::new("agents-numbers-semrush");
        let account = claude_login_without_token(&home);
        let shell = counting_shell(&home);
        let (_, providers) =
            agents_and_providers_with_shell(home.path(), &shell);

        let loaded = read_account_numbers(
            &providers,
            account.clone(),
            &[],
            true,
            |_| None,
        );

        assert!(started(&home), "`claude /usage` was not started");
        assert_eq!(loaded.account, account);
    }

    /// While Agents mode is away the provider part starts no hidden
    /// terminal: the read gives nothing, and the next read on screen
    /// starts it.
    #[test]
    fn a_read_in_the_background_starts_no_hidden_terminal() {
        let home = TempDir::new("agents-numbers-background");
        let account = claude_login_without_token(&home);
        let shell = counting_shell(&home);
        let (_, providers) =
            agents_and_providers_with_shell(home.path(), &shell);

        let loaded =
            read_account_numbers(&providers, account, &[], false, |_| None);

        assert!(!started(&home), "the shell was started");
        assert!(
            !matches!(&loaded.numbers, Ok(AccountNumbers::Limits(answer))
                if !answer.value().is_nothing()),
            "{:?}",
            loaded.numbers
        );
    }

    /// An Anthropic organization the Anthropic plugin found in the Claude
    /// Code folder under `home`, which holds no login now.
    fn organization_in(home: &Path) -> Account {
        Account::new(
            "anthropic",
            ProviderId::new("anthropic"),
            "org:made-up-org",
            "Made-up org",
            AccountKind::Organization,
            vec![FoundIn::profile(
                AgentId::new("claude-code"),
                home.join(".claude"),
            )],
        )
    }

    /// An account of a plugin no part of which is installed is not read:
    /// nothing is sent and no test reaches the network.
    #[test]
    fn an_account_no_part_reads_is_not_available_and_nothing_is_sent() {
        let folder = TempDir::new("agents-numbers-no-part");
        let account = Account::new(
            "no-such-plugin",
            ProviderId::new("anthropic"),
            "org:made-up-org",
            "Made-up org",
            AccountKind::Organization,
            Vec::new(),
        );
        let before = SystemTime::now();

        let loaded = read_account_numbers(
            &providers_with_plugins(folder.path()),
            account.clone(),
            &[],
            true,
            |_| None,
        );

        assert_eq!(loaded.account, account);
        assert!(loaded.read_at >= before, "the read tells when it ended");
        assert_eq!(
            loaded.numbers,
            Err(SourceError::NotAvailable(Unavailable::NoAccountSource))
        );
    }

    #[test]
    fn the_read_runs_in_the_background_and_answers_with_an_intent() {
        let folder = TempDir::new("agents-numbers-task");
        let account = organization_in(folder.path());

        let providers = providers_with_plugins(folder.path());
        let answers = run(load_account_numbers(
            &providers,
            account.clone(),
            Vec::new(),
            false,
            "/no/such/shell",
        ));

        assert!(matches!(
            answers.as_slice(),
            [AgentsIntent::AccountNumbersLoaded(loaded)]
                if loaded.account == account
        ));
    }
}

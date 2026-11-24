//! What the user is asked to do (spec "Words" and Decision 6): each need
//! is a title, a line and one action, collected per account and per
//! profile and never mixed. The Found by OTTY report, the All accounts
//! and All profiles pages and the page of an account list them.
//!
//! Who asks (spec 2026-11-19-agent-plugins, issue 20): everything the user
//! must do for an agent or a provider comes from its plugin part as a
//! [`PartNeed`], shown as it is with the plugin's name. A login that
//! expired is said by each part for its own page: the agent part on the
//! profile, the provider part on the account it found there (spec
//! 2026-11-23-accounts-apart-from-profiles, Decision 9). OTTY writes only
//! the needs about plugins and its own permissions: a plugin update, a
//! plugin that failed, a changed format and the system keyring, and a
//! profile whose program or folder is missing.

use std::path::Path;

use otty_agents::{
    Account, AgentCommand, AgentEntry, AgentInstall, FormatChange, PartNeed,
    PartNeedAction, ProfileStatus, ProviderId, SourceError, SourceFailure,
};
use otty_app_domain::folder_path::tilde_path;
use otty_app_i18n::{self as i18n, Key};
use otty_plugin_host::PluginFailure;

use super::super::failure_text::source_error_text;
use super::super::keyring::keyring_part_names;
use super::super::need::NeedAction;
use super::super::profile_state::ProfileState;
use super::super::state::AgentsState;
use super::account_name::account_name;
use super::reset_time::date_time_text;
use super::user_need::UserNeed;

/// One thing the user is asked to do: where, who asks, what, why, and its
/// one button when OTTY can do it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NeedViewModel {
    /// The account or the profile it is about.
    pub(crate) place: String,
    /// The plugin that asks, such as "OpenAI plugin"; `None` for a need of
    /// OTTY itself.
    pub(crate) plugin: Option<String>,
    /// What is needed, in a few words.
    pub(crate) title: String,
    /// Why, and what the button does.
    pub(crate) line: String,
    /// The button, when there is one.
    pub(crate) fix: Option<NeedFixViewModel>,
    /// A second button, such as Open plugin repository.
    pub(crate) extra: Option<NeedFixViewModel>,
}

/// The one button of a need.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NeedFixViewModel {
    /// The label of the button, such as "Run codex login".
    pub(crate) label: String,
    /// What it does; `None` for a button shown disabled.
    pub(crate) action: Option<NeedAction>,
}

/// What the accounts need from the user, in the order of the accounts,
/// each as [`needs_of_account`] lists them, then what the provider parts
/// ask that belongs to no account, then the need of the system keyring,
/// which hides accounts until it is read.
pub(crate) fn account_needs(state: &AgentsState) -> Vec<NeedViewModel> {
    state
        .profiles()
        .accounts()
        .iter()
        .flat_map(|account| needs_of_account(state, account))
        .chain(found_needs(state))
        .chain(keyring_need(state))
        .chain(plugin_update_needs(state))
        .collect()
}

/// What `account` needs from the user: what its provider part asks, each
/// once, then the need of a plugin that failed.
pub(crate) fn needs_of_account(
    state: &AgentsState,
    account: &Account,
) -> Vec<NeedViewModel> {
    account_part_needs(state, account)
        .into_iter()
        .chain(account_plugin_failed_need(state, account))
        .collect()
}

/// What the profiles need from the user, in the order of the profiles,
/// each as [`needs_of_profile`] lists them.
pub(crate) fn profile_needs(state: &AgentsState) -> Vec<NeedViewModel> {
    state
        .profiles()
        .shown()
        .iter()
        .flat_map(|shown| needs_of_profile(state, shown))
        .collect()
}

/// What the profile `shown` needs from the user: the login its agent part
/// asks for, the program or the folder it misses, and a report to copy
/// when its agent changed the format of its session files.
pub(crate) fn needs_of_profile(
    state: &AgentsState,
    shown: &ProfileState,
) -> Vec<NeedViewModel> {
    profile_part_need(shown)
        .into_iter()
        .chain(profile_need(state, shown))
        .chain(format_changed_need(state, shown))
        .chain(profile_plugin_failed_needs(shown))
        .collect()
}

/// The needs the provider part gives for `account`, each once: the ones
/// it gave when it found the account, such as an expired login, and the
/// ones of the latest read of its numbers and of the bill of the
/// organization it is. A key a part asks for is given for the account.
fn account_part_needs(
    state: &AgentsState,
    account: &Account,
) -> Vec<NeedViewModel> {
    let profiles = state.profiles();
    let read = profiles.account_reads().of(account);
    let numbers = read.and_then(|read| need_in(read.failure()));
    let bill = read.and_then(|read| need_in(read.bill().failure()));

    let mut needs: Vec<&PartNeed> = Vec::new();
    for need in account.needs().iter().chain(numbers).chain(bill) {
        if !needs.iter().any(|shown| shown.is_same_need(need)) {
            needs.push(need);
        }
    }

    let place = account_name(state.registry(), profiles.saved(), account);
    needs
        .into_iter()
        .map(|need| part_need_view(need, &place, Some(account)))
        .collect()
}

/// The need of the agent part of a profile nobody is logged in with.
fn profile_part_need(shown: &ProfileState) -> Option<NeedViewModel> {
    let otty_agents::ProfileStatus::NotLoggedIn(Some(need)) = shown.status()
    else {
        return None;
    };

    Some(part_need_view(need, shown.profile().name(), None))
}

/// `need` of a plugin part as it is, about `place`, with the name of its
/// plugin and the button of its action. A key it asks for is given for
/// `account`; with no account the button is shown disabled.
fn part_need_view(
    need: &PartNeed,
    place: &str,
    account: Option<&Account>,
) -> NeedViewModel {
    let plugin = need.part().name();
    let line = match need.action() {
        PartNeedAction::Run(command) => run_line(need.text(), command),
        _ => need.text().to_string(),
    };

    let fix = match need.action() {
        PartNeedAction::Run(command) => NeedFixViewModel {
            label: run_label(command),
            action: Some(NeedAction::Run {
                title: place.to_string(),
                command: command.clone(),
            }),
        },
        PartNeedAction::AskKey(ask) => NeedFixViewModel {
            label: i18n::filled(
                Key::TplAgentsNeedAskKey,
                &[("label", ask.label())],
            ),
            action: account.map(|account| NeedAction::AskKey {
                account: Box::new(account.clone()),
                plugin: plugin.to_string(),
                ask: ask.clone(),
            }),
        },
        PartNeedAction::OpenPage(page) => NeedFixViewModel {
            label: open_page_label(page),
            action: Some(NeedAction::OpenPage(page.clone())),
        },
        PartNeedAction::CopyReport(report) => NeedFixViewModel {
            label: i18n::t(Key::AgentsCopyReport).to_string(),
            action: Some(NeedAction::CopyReport(report.clone())),
        },
    };

    NeedViewModel {
        place: place.to_string(),
        plugin: Some(i18n::filled(
            Key::TplAgentsNeedPluginLabel,
            &[("plugin", plugin)],
        )),
        title: need.title().to_string(),
        line,
        fix: Some(fix),
        extra: None,
    }
}

/// The label of a button that runs `command`, kept short: "Run" and the
/// command with only the name of its program, such as "Run opencode auth
/// login" for a program OTTY found at a full path.
fn run_label(command: &AgentCommand) -> String {
    let program = command.program();
    let name = Path::new(program)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(program);
    let short = std::iter::once(name)
        .chain(command.args().iter().map(String::as_str))
        .map(typed_word)
        .collect::<Vec<_>>()
        .join(" ");

    i18n::filled(Key::TplAgentsNeedRun, &[("command", &short)])
}

/// The line of a need whose button runs `command`: the part's `text`,
/// and, when the program is started by a full path, the whole command,
/// since the button names only the program.
fn run_line(text: &str, command: &AgentCommand) -> String {
    if !command.program().contains('/') {
        return text.to_string();
    }

    let runs = i18n::filled(
        Key::TplAgentsNeedRunsCommand,
        &[("command", &command_text(command))],
    );
    format!("{text} {runs}")
}

/// The label of a button that opens `page`: "Open" and its host, such as
/// "Open console.example.com".
pub(crate) fn open_page_label(page: &str) -> String {
    let host = page
        .strip_prefix("https://")
        .and_then(|rest| rest.split(['/', '?', '#']).next())
        .unwrap_or(page);

    i18n::filled(Key::TplAgentsNeedOpenPage, &[("host", host)])
}

/// `command` as the user would type it, without its variables, such as
/// `codex login` or `sh -c 'curl -fsSL https://example.com/install | sh'`.
fn command_text(command: &AgentCommand) -> String {
    std::iter::once(command.program())
        .chain(command.args().iter().map(String::as_str))
        .map(typed_word)
        .collect::<Vec<_>>()
        .join(" ")
}

/// `word` as the user would type it in a shell: as it is when it holds
/// only letters, digits and `-_./:=@%+,~`, else in single quotes.
fn typed_word(word: &str) -> String {
    let is_plain = !word.is_empty()
        && word
            .chars()
            .all(|char| char.is_alphanumeric() || "-_./:=@%+,~".contains(char));
    if is_plain {
        return word.to_string();
    }

    let quoted = word.replace('\'', r"'\''");
    format!("'{quoted}'")
}

/// The need of a part in `failure`, when the read waits for the user.
fn need_in(failure: Option<&SourceError>) -> Option<&PartNeed> {
    match failure? {
        SourceError::NeedsUser(need) => Some(need.as_ref()),
        _ => None,
    }
}

/// What the provider parts ask that belongs to no account, such as a
/// login file a part does not understand, about the part's provider.
fn found_needs(state: &AgentsState) -> Vec<NeedViewModel> {
    state
        .profiles()
        .found_needs()
        .iter()
        .map(|need| {
            let id = ProviderId::new(need.part().part());
            let place = state
                .registry()
                .provider(&id)
                .map_or(need.part().name(), |entry| entry.name());

            part_need_view(need, place, None)
        })
        .collect()
}

/// The needs of the plugin updates that ask for new permissions and wait
/// for the user's OK: "Example 0.4.0 → 0.5.0 asks for new permissions
/// [Review]".
fn plugin_update_needs(state: &AgentsState) -> Vec<NeedViewModel> {
    state
        .plugin_notes()
        .waiting_updates
        .iter()
        .map(|update| NeedViewModel {
            place: i18n::t(Key::AgentsPluginsPlace).to_string(),
            plugin: None,
            title: i18n::t(Key::AgentsNeedPluginUpdateTitle).to_string(),
            line: i18n::filled(
                Key::TplAgentsNeedPluginUpdateText,
                &[
                    ("plugin", &update.name),
                    ("from", &update.from),
                    ("to", &update.to),
                ],
            ),
            fix: Some(NeedFixViewModel {
                label: i18n::t(Key::AgentsReview).to_string(),
                action: Some(NeedAction::ReviewPluginUpdate(
                    update.plugin.clone(),
                )),
            }),
            extra: None,
        })
        .collect()
}

/// The need of an account whose numbers a provider part could not read.
fn account_plugin_failed_need(
    state: &AgentsState,
    account: &Account,
) -> Option<NeedViewModel> {
    let own = state.profiles().account_reads().of(account)?;
    let SourceError::PluginFailed(failure) = own.failure()? else {
        return None;
    };

    let place =
        account_name(state.registry(), state.profiles().saved(), account);
    Some(plugin_failed_need(
        place,
        failure,
        Key::AgentsPluginReadNumbers,
    ))
}

/// The needs of a profile whose sessions or tokens a plugin part could
/// not read, one per kind.
fn profile_plugin_failed_needs(shown: &ProfileState) -> Vec<NeedViewModel> {
    let plugin_failure = |error: &SourceError| match error {
        SourceError::PluginFailed(failure) => Some(failure.as_ref().clone()),
        _ => None,
    };
    let in_failures = |failures: Option<&[SourceFailure]>| {
        failures?
            .iter()
            .find_map(|failure| plugin_failure(failure.error()))
    };

    let found = [
        (in_failures(shown.failures()), Key::AgentsPluginReadSessions),
        (
            shown.spend().failure().and_then(plugin_failure),
            Key::AgentsPluginReadTokens,
        ),
    ];

    let place = shown.profile().name().to_string();
    found
        .into_iter()
        .filter_map(|(failure, what)| {
            Some(plugin_failed_need(place.clone(), &failure?, what))
        })
        .collect()
}

/// "<Plugin> <version> could not read <what> [Copy report]".
fn plugin_failed_need(
    place: String,
    failure: &PluginFailure,
    what: Key,
) -> NeedViewModel {
    let part = failure.part();

    NeedViewModel {
        place,
        plugin: None,
        title: i18n::filled(
            Key::TplAgentsNeedPluginFailedTitle,
            &[
                ("plugin", part.name()),
                ("version", part.version()),
                ("what", i18n::t(what)),
            ],
        ),
        line: i18n::filled(
            Key::TplAgentsNeedPluginFailedText,
            &[(
                "reason",
                &source_error_text(&SourceError::PluginFailed(Box::new(
                    failure.clone(),
                ))),
            )],
        ),
        fix: Some(NeedFixViewModel {
            label: i18n::t(Key::AgentsCopyReport).to_string(),
            action: Some(NeedAction::CopyReport(failure.report())),
        }),
        extra: None,
    }
}

/// The need of the system keyring (spec 2026-11-19, Decision 8), while a
/// plugin part that is on declares an entry of it, the user has not
/// allowed the read and `agents.json` can keep the answer: Allow keyring.
fn keyring_need(state: &AgentsState) -> Option<NeedViewModel> {
    let saved = state.profiles().saved()?;
    let parts = keyring_part_names(state.registry());
    if parts.is_empty() || saved.is_keyring_allowed() {
        return None;
    }

    Some(NeedViewModel {
        place: i18n::t(Key::AgentsKeyringPlace).to_string(),
        plugin: None,
        title: i18n::t(Key::AgentsNeedKeyringTitle).to_string(),
        line: i18n::filled(
            Key::TplAgentsNeedKeyringText,
            &[("agents", &parts.join(", "))],
        ),
        fix: Some(NeedFixViewModel {
            label: i18n::t(Key::AgentsAllowKeyring).to_string(),
            action: Some(NeedAction::AllowKeyring),
        }),
        extra: None,
    })
}

/// The need of a profile whose agent's program or config folder OTTY can
/// not find. A missing folder has no button; a missing program offers the
/// install its agent part gives.
fn profile_need(
    state: &AgentsState,
    shown: &ProfileState,
) -> Option<NeedViewModel> {
    if *shown.status() == ProfileStatus::ProgramNotFound {
        return Some(program_not_found_need(state, shown));
    }

    let need = UserNeed::of(shown.status())?;
    let hint = need.hint()?;

    Some(NeedViewModel {
        place: shown.profile().name().to_string(),
        plugin: None,
        title: need.state().to_string(),
        line: hint.to_string(),
        fix: None,
        extra: None,
    })
}

/// The need of a profile whose agent's program OTTY found neither on the
/// `PATH` of the login shell nor at the program places of its agent part
/// (issue 32): "OTTY did not find codex on the PATH of your login shell or
/// at ~/.codex/bin/codex. [Install Codex] [Open example.com]". The install
/// command runs in a terminal tab, and the line shows it; with only a page
/// the button opens the page; with neither there is no button.
fn program_not_found_need(
    state: &AgentsState,
    shown: &ProfileState,
) -> NeedViewModel {
    let agent = shown.profile().agent();
    let entry = state.registry().agent(agent);
    let name = state.agent_name(agent);
    let program = entry.map_or(agent.as_str(), AgentEntry::program);
    let places: Vec<String> = entry
        .map(AgentEntry::program_places)
        .unwrap_or_default()
        .iter()
        .map(|place| tilde_path(place, state.env().home()))
        .collect();
    let install = entry.and_then(AgentEntry::install);
    let command = install.and_then(AgentInstall::command);
    let page = install.and_then(AgentInstall::page);

    let not_found = match places.is_empty() {
        true => i18n::filled(
            Key::TplAgentsNeedProgramNotFoundText,
            &[("program", program)],
        ),
        false => i18n::filled(
            Key::TplAgentsNeedProgramNotFoundAtPlacesText,
            &[("program", program), ("places", &places.join(", "))],
        ),
    };
    let line = match command {
        Some(command) => {
            let runs = i18n::filled(
                Key::TplAgentsNeedInstallRuns,
                &[("agent", name), ("command", &command_text(command))],
            );
            format!("{not_found} {runs}")
        },
        None => not_found,
    };

    let place = shown.profile().name().to_string();
    let page_button = |page: &str| NeedFixViewModel {
        label: open_page_label(page),
        action: Some(NeedAction::OpenPage(page.to_string())),
    };
    let (fix, extra) = match command {
        Some(command) => {
            let install = NeedFixViewModel {
                label: i18n::filled(
                    Key::TplAgentsInstallAgent,
                    &[("agent", name)],
                ),
                action: Some(NeedAction::Run {
                    title: place.clone(),
                    command: command.clone(),
                }),
            };
            (Some(install), page.map(page_button))
        },
        None => (page.map(page_button), None),
    };

    NeedViewModel {
        place,
        plugin: None,
        title: i18n::t(Key::AgentsStateProgramNotFound).to_string(),
        line,
        fix,
        extra,
    }
}

/// The need of a profile whose agent changed the format of its session
/// files (spec 2026-11-19, Decision 9): it names the version and, when a
/// plugin part reads the files, the plugin (spec 2026-11-19-agent-plugins,
/// "What the UI shows"), says since when the tokens are missing while the
/// last good numbers stay, and copies a report for the makers of OTTY or
/// of the plugin.
fn format_changed_need(
    state: &AgentsState,
    shown: &ProfileState,
) -> Option<NeedViewModel> {
    let change = shown.spend().format_change()?;
    let profile = shown.profile();
    let agent = state.agent_name(profile.agent());
    let folder = tilde_path(profile.config_dir(), state.env().home());
    let tested = change.tested_version();

    let time = shown.spend().last_good().map(|(_, counted_at)| {
        date_time_text(counted_at, state.profiles().time_zone())
    });
    let line = match (change.plugin(), time) {
        (None, Some(time)) => i18n::filled(
            Key::TplAgentsNeedFormatChangedText,
            &[("agent", agent), ("tested", tested), ("time", &time)],
        ),
        (None, None) => i18n::filled(
            Key::TplAgentsNeedFormatChangedNoNumbers,
            &[("agent", agent), ("tested", tested)],
        ),
        (Some(part), Some(time)) => i18n::filled(
            Key::TplAgentsNeedFormatChangedPluginText,
            &[
                ("plugin", part.name()),
                ("version", part.version()),
                ("agent", agent),
                ("tested", tested),
                ("time", &time),
            ],
        ),
        (Some(part), None) => i18n::filled(
            Key::TplAgentsNeedFormatChangedPluginNoNumbers,
            &[
                ("plugin", part.name()),
                ("version", part.version()),
                ("agent", agent),
                ("tested", tested),
            ],
        ),
    };

    Some(NeedViewModel {
        place: profile.name().to_string(),
        plugin: None,
        title: format_changed_title(agent, change),
        line,
        fix: Some(NeedFixViewModel {
            label: i18n::t(Key::AgentsCopyReport).to_string(),
            action: Some(NeedAction::CopyReport(change.report(agent, &folder))),
        }),
        extra: plugin_repository(change),
    })
}

/// Open plugin repository, for a changed format a plugin part saw whose
/// plugin names its repository.
fn plugin_repository(change: &FormatChange) -> Option<NeedFixViewModel> {
    let repository = change.plugin()?.repository();
    if repository.is_empty() {
        return None;
    }

    Some(NeedFixViewModel {
        label: i18n::t(Key::AgentsOpenPluginRepository).to_string(),
        action: Some(NeedAction::OpenPage(repository.to_string())),
    })
}

/// The title of the need of `change` of the files of `agent`, such as
/// "Coder 2.2.0 changed its session files".
fn format_changed_title(agent: &str, change: &FormatChange) -> String {
    match change.version() {
        Some(version) => i18n::filled(
            Key::TplAgentsNeedFormatChangedTitle,
            &[("agent", agent), ("version", version)],
        ),
        None => i18n::filled(
            Key::TplAgentsNeedFormatChangedNoVersion,
            &[("agent", agent)],
        ),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use otty_agents::fixtures::format_change;
    use otty_agents::{
        AgentInstall, FormatChange, ProfileStatus, SavedProfiles, SourceError,
    };
    use otty_app_domain::plugins::{PluginNotes, WaitingUpdate};
    use otty_plugin_host::{PluginFailureKind, PluginPart};

    use super::*;
    use crate::event::AgentsIntent;
    use crate::model::reset_time::date_time_text;
    use crate::reducer::reduce;
    use crate::testing::{
        at, bill_loaded, chatgpt_login, claude, claude_work, codex, codex_work,
        connected, numbers_loaded, organization, registry_with_codex,
        saved_with_given_key, spend_loaded, spend_of, with_accounts,
        with_accounts_in, with_profiles, with_registry_and_profiles,
    };
    use crate::types::SpendCount;

    /// The label of a need of the plugin named `plugin`.
    fn plugin_label(plugin: &str) -> Option<String> {
        Some(i18n::filled(
            Key::TplAgentsNeedPluginLabel,
            &[("plugin", plugin)],
        ))
    }

    /// Owner, 2026-11-22: a program found at a full path (issue 32) keeps
    /// the button short, with only the program's name; the line says the
    /// whole command it runs.
    #[test]
    fn a_run_button_names_the_program_and_the_line_has_its_path() {
        let by_path = AgentCommand::new("/home/user/.opencode/bin/opencode")
            .with_arg("auth")
            .with_arg("login");
        let by_name = AgentCommand::new("codex").with_arg("login");

        let label = run_label(&by_path);
        let line = run_line("Nobody is logged in.", &by_path);
        let named_line = run_line("Nobody is logged in.", &by_name);

        assert_eq!(
            label,
            i18n::filled(
                Key::TplAgentsNeedRun,
                &[("command", "opencode auth login")]
            )
        );
        assert_eq!(
            line,
            format!(
                "Nobody is logged in. {}",
                i18n::filled(
                    Key::TplAgentsNeedRunsCommand,
                    &[(
                        "command",
                        "/home/user/.opencode/bin/opencode auth login"
                    )]
                )
            )
        );
        assert_eq!(named_line, "Nobody is logged in.");
    }

    /// Spec Decision 6: the needs of the accounts and of the profiles are
    /// collected apart and never mixed. Issue 20: a part's need shows as
    /// the part gave it, with the plugin that asks and the button of its
    /// action; OTTY's own need names no plugin.
    #[test]
    fn the_needs_of_the_accounts_and_of_the_profiles_are_apart() {
        let mut state = with_accounts(
            &[
                (claude(), connected(), vec![organization()]),
                (codex(), crate::testing::not_logged_in(&codex()), Vec::new()),
                (codex_work(), ProfileStatus::ProgramNotFound, Vec::new()),
            ],
            saved_with_given_key(),
        );
        let _ = reduce(
            &mut state,
            bill_loaded(Err(crate::testing::bill_key_needed())),
        );

        let accounts = account_needs(&state);
        let profiles = profile_needs(&state);

        assert_eq!(
            accounts,
            vec![NeedViewModel {
                place: String::from("Anthropic (dev@example.com)"),
                plugin: plugin_label("Anthropic"),
                title: String::from("The real bill needs a key"),
                line: String::from("OTTY shows an estimate from the tokens."),
                fix: Some(NeedFixViewModel {
                    label: i18n::filled(
                        Key::TplAgentsNeedAskKey,
                        &[("label", "Admin key")]
                    ),
                    action: Some(NeedAction::AskKey {
                        account: Box::new(organization()),
                        plugin: String::from("Anthropic"),
                        ask: crate::testing::key_ask(),
                    }),
                }),
                extra: None,
            }]
        );
        assert_eq!(
            profiles,
            vec![
                NeedViewModel {
                    place: String::from("Codex"),
                    plugin: plugin_label("OpenAI"),
                    title: String::from("Needs login"),
                    line: String::from(
                        "Nobody is logged in with this config folder."
                    ),
                    fix: Some(NeedFixViewModel {
                        label: i18n::filled(
                            Key::TplAgentsNeedRun,
                            &[("command", "codex login")]
                        ),
                        action: Some(NeedAction::Run {
                            title: String::from("Codex"),
                            command: AgentCommand::new("codex")
                                .with_arg("login"),
                        }),
                    }),
                    extra: None,
                },
                NeedViewModel {
                    place: String::from("Codex (Work)"),
                    plugin: None,
                    title: i18n::t(Key::AgentsStateProgramNotFound).to_string(),
                    line: i18n::filled(
                        Key::TplAgentsNeedProgramNotFoundText,
                        &[("program", "codex")]
                    ),
                    fix: None,
                    extra: None,
                },
            ]
        );
    }

    /// The install the Codex part gives in the tests of issue 32.
    fn install_command() -> AgentCommand {
        AgentCommand::new("sh")
            .with_arg("-c")
            .with_arg("curl -fsSL https://example.com/install | sh")
    }

    /// The need of the default Codex profile, whose program OTTY did not
    /// find, with the Codex entry `change` makes.
    fn program_need(
        change: impl Fn(otty_agents::AgentEntry) -> otty_agents::AgentEntry,
    ) -> Vec<NeedViewModel> {
        let state = with_registry_and_profiles(
            registry_with_codex(change),
            &[(codex(), ProfileStatus::ProgramNotFound)],
        );

        profile_needs(&state)
    }

    /// Issue 32: OTTY found the program nowhere, neither on the `PATH` of
    /// the login shell nor at the places of its part. The need says so,
    /// and its button installs the program with the command the part
    /// gives, run in a terminal tab like any need's command, which the
    /// line shows; the install page is a second button.
    #[test]
    fn a_program_found_nowhere_offers_the_install_its_part_gives() {
        let needs = program_need(|entry| {
            entry
                .with_program_places(vec![
                    PathBuf::from("/home/user/.codex/bin/codex"),
                    PathBuf::from("/opt/codex/codex"),
                ])
                .with_install(AgentInstall::new(
                    Some(install_command()),
                    Some(String::from("https://example.com/docs")),
                ))
        });

        let not_found = i18n::filled(
            Key::TplAgentsNeedProgramNotFoundAtPlacesText,
            &[
                ("program", "codex"),
                ("places", "~/.codex/bin/codex, /opt/codex/codex"),
            ],
        );
        let runs = i18n::filled(
            Key::TplAgentsNeedInstallRuns,
            &[
                ("agent", "Codex"),
                (
                    "command",
                    "sh -c 'curl -fsSL https://example.com/install | sh'",
                ),
            ],
        );
        assert_eq!(
            needs,
            vec![NeedViewModel {
                place: String::from("Codex"),
                plugin: None,
                title: i18n::t(Key::AgentsStateProgramNotFound).to_string(),
                line: format!("{not_found} {runs}"),
                fix: Some(NeedFixViewModel {
                    label: i18n::filled(
                        Key::TplAgentsInstallAgent,
                        &[("agent", "Codex")]
                    ),
                    action: Some(NeedAction::Run {
                        title: String::from("Codex"),
                        command: install_command(),
                    }),
                }),
                extra: Some(NeedFixViewModel {
                    label: open_page_label("https://example.com/docs"),
                    action: Some(NeedAction::OpenPage(String::from(
                        "https://example.com/docs"
                    ))),
                }),
            }]
        );
    }

    /// Issue 32: a part that gives only the install page: the button opens
    /// it.
    #[test]
    fn a_program_found_nowhere_with_only_an_install_page_opens_it() {
        let needs = program_need(|entry| {
            entry.with_install(AgentInstall::new(
                None,
                Some(String::from("https://example.com/docs")),
            ))
        });

        assert_eq!(needs.len(), 1);
        assert_eq!(
            needs[0].line,
            i18n::filled(
                Key::TplAgentsNeedProgramNotFoundText,
                &[("program", "codex")]
            )
        );
        assert_eq!(
            needs[0].fix,
            Some(NeedFixViewModel {
                label: open_page_label("https://example.com/docs"),
                action: Some(NeedAction::OpenPage(String::from(
                    "https://example.com/docs"
                ))),
            })
        );
        assert_eq!(needs[0].extra, None);
    }

    /// Issue 32: a part that says nothing about installing its program:
    /// the need has no button.
    #[test]
    fn a_program_found_nowhere_with_no_install_has_no_button() {
        let needs = program_need(|entry| entry);

        assert_eq!(needs.len(), 1);
        assert_eq!(needs[0].fix, None);
        assert_eq!(needs[0].extra, None);
    }

    /// Issue 20: the need a part gives when no part could read the
    /// limits is a need of the account, not of the profile; its button
    /// runs the part's command in a tab named after the account.
    #[test]
    fn an_account_whose_login_expired_shows_the_need_of_its_part() {
        let mut state = with_accounts(
            &[(codex(), connected(), vec![chatgpt_login()])],
            SavedProfiles::default(),
        );
        let _ = reduce(
            &mut state,
            numbers_loaded(
                &chatgpt_login(),
                Err(crate::testing::login_expired()),
                at(10),
            ),
        );

        assert_eq!(
            account_needs(&state),
            vec![NeedViewModel {
                place: String::from("OpenAI (you@example.com)"),
                plugin: plugin_label("Openai"),
                title: String::from("The login expired"),
                line: String::from("Open Codex: it renews the login."),
                fix: Some(NeedFixViewModel {
                    label: i18n::filled(
                        Key::TplAgentsNeedRun,
                        &[("command", "codex")]
                    ),
                    action: Some(NeedAction::Run {
                        title: String::from("OpenAI (you@example.com)"),
                        command: AgentCommand::new("codex"),
                    }),
                }),
                extra: None,
            }]
        );
        assert!(profile_needs(&state).is_empty());
    }

    /// Issue 20: the same need of the same plugin, given when the part
    /// found the account and again when it read its numbers, shows once.
    #[test]
    fn the_same_need_from_the_find_and_the_read_shows_once() {
        let expired = chatgpt_login().with_needs(vec![expired_need()]);
        let mut state = with_accounts(
            &[(codex(), connected(), vec![expired])],
            SavedProfiles::default(),
        );
        let _ = reduce(
            &mut state,
            crate::testing::numbers_loaded(
                &chatgpt_login(),
                Err(crate::testing::login_expired()),
                at(10),
            ),
        );

        let needs = account_needs(&state);

        assert_eq!(needs.len(), 1, "{needs:?}");
    }

    /// The need of the OpenAI part whose login expired, as the part gives
    /// it with the account it found.
    fn expired_need() -> otty_agents::PartNeed {
        let SourceError::NeedsUser(need) = crate::testing::login_expired()
        else {
            panic!("a need");
        };

        *need
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decision 9: the need
    /// a provider part gives with an account it found, such as an expired
    /// login, shows as a need of the account, with the part's button.
    #[test]
    fn the_need_a_part_gives_with_an_account_shows_on_it() {
        let expired =
            crate::testing::chatgpt_login().with_needs(vec![expired_need()]);
        let state = with_accounts(
            &[(codex(), connected(), vec![expired.clone()])],
            SavedProfiles::default(),
        );

        let needs = needs_of_account(&state, &expired);

        assert_eq!(needs.len(), 1, "{needs:?}");
        assert_eq!(needs[0].title, "The login expired");
        assert_eq!(needs[0].plugin, plugin_label("Openai"));
        assert!(matches!(
            needs[0].fix.as_ref().and_then(|fix| fix.action.as_ref()),
            Some(NeedAction::Run { .. })
        ));
        assert!(profile_needs(&state).is_empty());
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decision 5: an
    /// account whose part is not in the registry now asks for no provider
    /// plugin: the part found it, so no other plugin would read it.
    #[test]
    fn an_account_whose_part_is_gone_asks_for_no_provider_plugin() {
        let registry = std::sync::Arc::new(
            crate::testing::registry()
                .as_ref()
                .clone()
                .without_provider(&otty_agents::ProviderId::new("openai")),
        );
        let state = with_accounts_in(
            registry,
            &[(codex(), connected(), vec![chatgpt_login()])],
            SavedProfiles::default(),
        );

        assert!(account_needs(&state).is_empty());
    }

    /// The needs a provider part gives that belong to no account show with
    /// the other needs of the accounts, about its provider.
    #[test]
    fn a_need_of_no_account_shows_with_the_needs_of_the_accounts() {
        let mut state = with_profiles(&[(codex(), connected())]);
        let AgentsIntent::ProfilesLoaded(mut loaded) =
            crate::testing::profiles_loaded(&[(codex(), connected())])
        else {
            panic!("a profiles read");
        };
        loaded.accounts =
            otty_agents::FoundAccounts::new(Vec::new(), vec![expired_need()]);
        let _ = reduce(&mut state, AgentsIntent::ProfilesLoaded(loaded));

        let needs = account_needs(&state);

        assert_eq!(needs.len(), 1, "{needs:?}");
        assert_eq!(needs[0].title, "The login expired");
    }

    #[test]
    fn each_action_of_a_part_need_gets_its_button() {
        let part = otty_agents::fixtures::plugin_part("example", "example");
        let page = PartNeed::new(
            part.clone(),
            "new-key",
            "The key was refused",
            "Make a new key.",
            PartNeedAction::OpenPage(String::from(
                "https://console.example.com/keys",
            )),
        );
        let report = PartNeed::new(
            part,
            "report",
            "Something to tell",
            "Copy it.",
            PartNeedAction::CopyReport(String::from("the report")),
        );
        let key =
            otty_agents::fixtures::key_need("example", "example", "admin");
        let found_key = crate::testing::key_account();

        let page = part_need_view(&page, "Place", None);
        let report = part_need_view(&report, "Place", None);
        let key = part_need_view(&key, "Place", Some(&found_key));

        assert_eq!(
            page.fix,
            Some(NeedFixViewModel {
                label: i18n::filled(
                    Key::TplAgentsNeedOpenPage,
                    &[("host", "console.example.com")]
                ),
                action: Some(NeedAction::OpenPage(String::from(
                    "https://console.example.com/keys"
                ))),
            })
        );
        assert_eq!(
            report.fix,
            Some(NeedFixViewModel {
                label: i18n::t(Key::AgentsCopyReport).to_string(),
                action: Some(NeedAction::CopyReport(String::from(
                    "the report"
                ))),
            })
        );
        assert!(matches!(
            key.fix.and_then(|fix| fix.action),
            Some(NeedAction::AskKey { account, .. }) if *account == found_key
        ));
    }

    /// Claude Code, whose last good count ended at 10 and whose next count
    /// found its session files changed by `change`.
    fn with_changed_format(change: &FormatChange) -> AgentsState {
        let mut state = with_profiles(&[(claude(), connected())]);
        let error = SourceError::FormatChanged(Box::new(change.clone()));
        for (spend, counted_at) in [
            (SpendCount::Counted(spend_of(1, 2)), at(10)),
            (SpendCount::Failed(error), at(20)),
        ] {
            let _ =
                reduce(&mut state, spend_loaded(&claude(), spend, counted_at));
        }

        state
    }

    /// Spec 2026-11-19, Decision 9: a profile whose session files OTTY no
    /// longer understands names the version, says the last good numbers
    /// are kept, and copies a report.
    #[test]
    fn a_profile_whose_files_changed_asks_to_copy_a_report() {
        let change = format_change(Some("2.2.0"), 40, 0);
        let state = with_changed_format(&change);

        let needs = profile_needs(&state);

        let since = date_time_text(at(10), state.profiles().time_zone());
        assert_eq!(
            needs,
            vec![NeedViewModel {
                place: String::from("Claude Code"),
                plugin: None,
                title: i18n::filled(
                    Key::TplAgentsNeedFormatChangedTitle,
                    &[("agent", "Claude Code"), ("version", "2.2.0")]
                ),
                line: i18n::filled(
                    Key::TplAgentsNeedFormatChangedText,
                    &[
                        ("agent", "Claude Code"),
                        ("tested", "2.1.0"),
                        ("time", &since)
                    ]
                ),
                fix: Some(NeedFixViewModel {
                    label: i18n::t(Key::AgentsCopyReport).to_string(),
                    action: Some(NeedAction::CopyReport(
                        change.report("Claude Code", "~/.claude")
                    )),
                }),
                extra: None,
            }]
        );
        assert!(account_needs(&state).is_empty());
    }

    /// With no version in the files and no good count yet, the need names
    /// the agent only and says no tokens are shown.
    #[test]
    fn a_changed_format_with_no_version_and_no_numbers_says_so() {
        let mut state = with_profiles(&[(claude(), connected())]);
        let error =
            SourceError::FormatChanged(Box::new(format_change(None, 40, 0)));
        let _ = reduce(
            &mut state,
            spend_loaded(&claude(), SpendCount::Failed(error), at(20)),
        );

        let needs = profile_needs(&state);

        assert_eq!(
            needs[0].title,
            i18n::filled(
                Key::TplAgentsNeedFormatChangedNoVersion,
                &[("agent", "Claude Code")]
            )
        );
        assert_eq!(
            needs[0].line,
            i18n::filled(
                Key::TplAgentsNeedFormatChangedNoNumbers,
                &[("agent", "Claude Code"), ("tested", "2.1.0")]
            )
        );
    }

    #[test]
    fn an_organization_whose_bill_was_read_needs_nothing() {
        let mut state = with_accounts(
            &[(claude(), connected(), vec![organization()])],
            saved_with_given_key(),
        );
        let _ =
            reduce(&mut state, bill_loaded(Ok(crate::testing::bill_of(1, 2))));

        assert!(account_needs(&state).is_empty());
        assert!(profile_needs(&state).is_empty());
    }

    /// An organization found in two profiles asks for its key once: its
    /// bill is read once for both.
    #[test]
    fn an_organization_of_two_profiles_needs_its_key_once() {
        let mut state = with_accounts(
            &[
                (claude(), connected(), vec![organization()]),
                (claude_work(), connected(), vec![organization()]),
            ],
            saved_with_given_key(),
        );
        let _ = reduce(
            &mut state,
            bill_loaded(Err(crate::testing::bill_key_needed())),
        );

        assert_eq!(account_needs(&state).len(), 1);
    }
    /// Spec 2026-11-19-agent-plugins, "What the UI shows": the need of a
    /// changed format a plugin part saw names the plugin and its version,
    /// and asks to update the plugin.
    #[test]
    fn a_changed_format_a_plugin_saw_names_the_plugin() {
        let change = otty_agents::fixtures::plugin_format_change("1.5.0", 6);
        let state = with_changed_format(&change);

        let needs = profile_needs(&state);

        let since = date_time_text(at(10), state.profiles().time_zone());
        assert_eq!(
            needs[0].line,
            i18n::filled(
                Key::TplAgentsNeedFormatChangedPluginText,
                &[
                    ("plugin", "Example"),
                    ("version", "0.1.0"),
                    ("agent", "Claude Code"),
                    ("tested", "1.4.2"),
                    ("time", &since)
                ]
            )
        );
        let Some(NeedFixViewModel {
            action: Some(NeedAction::CopyReport(report)),
            ..
        }) = &needs[0].fix
        else {
            panic!("a report to copy: {needs:?}");
        };
        assert!(report.contains("Plugin: example 0.1.0"), "{report}");
        assert_eq!(
            needs[0].extra,
            Some(NeedFixViewModel {
                label: i18n::t(Key::AgentsOpenPluginRepository).to_string(),
                action: Some(NeedAction::OpenPage(String::from(
                    "https://github.com/otty-shell/otty-plugin-example"
                ))),
            })
        );
    }

    /// Spec "What the UI shows": an update that asks for new permissions
    /// waits for the user's OK; Review opens its dialog.
    #[test]
    fn a_plugin_update_that_waits_needs_the_users_ok() {
        let mut state = with_profiles(&[(claude(), connected())]);
        let _ = reduce(
            &mut state,
            AgentsIntent::PluginNotesChanged(PluginNotes {
                off_parts: Vec::new(),
                waiting_updates: vec![WaitingUpdate {
                    plugin: String::from("openai"),
                    name: String::from("OpenAI"),
                    from: String::from("0.4.0"),
                    to: String::from("0.5.0"),
                }],
            }),
        );

        let needs = account_needs(&state);
        let _ = reduce(
            &mut state,
            AgentsIntent::PluginNotesChanged(PluginNotes::default()),
        );

        assert_eq!(
            needs,
            vec![NeedViewModel {
                place: i18n::t(Key::AgentsPluginsPlace).to_string(),
                plugin: None,
                title: i18n::t(Key::AgentsNeedPluginUpdateTitle).to_string(),
                line: i18n::filled(
                    Key::TplAgentsNeedPluginUpdateText,
                    &[("plugin", "OpenAI"), ("from", "0.4.0"), ("to", "0.5.0")]
                ),
                fix: Some(NeedFixViewModel {
                    label: i18n::t(Key::AgentsReview).to_string(),
                    action: Some(NeedAction::ReviewPluginUpdate(String::from(
                        "openai"
                    ))),
                }),
                extra: None,
            }]
        );
        assert!(account_needs(&state).is_empty(), "it leaves once updated");
    }

    /// Issue 11 of spec 2026-11-23-accounts-apart-from-profiles: a
    /// provider part that could not read the numbers of an account found
    /// in a profile asks on the account, never on the profile.
    #[test]
    fn a_part_that_could_not_read_an_account_asks_on_the_account_only() {
        let mut state = with_accounts(
            &[(codex(), connected(), vec![chatgpt_login()])],
            SavedProfiles::default(),
        );
        let part = PluginPart::new("openai", "OpenAI", "0.1.0", "", "openai");
        let failure =
            PluginFailure::new(part, "numbers", PluginFailureKind::TooSlow);
        let error = SourceError::PluginFailed(Box::new(failure));
        let _ = reduce(
            &mut state,
            crate::testing::numbers_loaded(
                &chatgpt_login(),
                Err(error),
                at(20),
            ),
        );

        assert!(profile_needs(&state).is_empty());
        assert_eq!(needs_of_account(&state, &chatgpt_login()).len(), 1);
    }

    /// Spec "What the UI shows": a part that could not read says which
    /// plugin and what, and copies a report with no value in it.
    #[test]
    fn a_plugin_that_could_not_read_the_tokens_asks_to_copy_a_report() {
        let mut state = with_profiles(&[(claude(), connected())]);
        let part = PluginPart::new(
            "example",
            "Example",
            "0.1.0",
            "https://github.com/otty-shell/otty-plugin-example",
            "example-agent",
        );
        let failure = PluginFailure::new(
            part,
            "read-answers",
            PluginFailureKind::TooSlow,
        );
        let error = SourceError::PluginFailed(Box::new(failure.clone()));
        let _ = reduce(
            &mut state,
            spend_loaded(&claude(), SpendCount::Failed(error), at(20)),
        );

        let needs = profile_needs(&state);

        assert_eq!(needs.len(), 1);
        assert_eq!(
            needs[0].title,
            i18n::filled(
                Key::TplAgentsNeedPluginFailedTitle,
                &[
                    ("plugin", "Example"),
                    ("version", "0.1.0"),
                    ("what", i18n::t(Key::AgentsPluginReadTokens))
                ]
            )
        );
        assert_eq!(
            needs[0].fix,
            Some(NeedFixViewModel {
                label: i18n::t(Key::AgentsCopyReport).to_string(),
                action: Some(NeedAction::CopyReport(failure.report())),
            })
        );

        let _ = reduce(
            &mut state,
            spend_loaded(
                &claude(),
                SpendCount::Counted(spend_of(1, 2)),
                at(30),
            ),
        );
        assert!(profile_needs(&state).is_empty(), "it leaves once read");
    }
}

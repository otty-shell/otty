//! What the user is asked to do (spec "Words" and Decision 6): each need
//! is a title, a line and one action, collected per account and per
//! profile and never mixed. The Found by OTTY report, the All accounts
//! and All profiles pages and the page of an account list them.

use otty_agents::{
    Account, FormatChange, KeyringEntry, KeyringError, PluginFailure,
    SourceError, SourceFailure,
};
use otty_app_domain::folder_path::tilde_path;
use otty_app_i18n::{self as i18n, Key};

use super::super::failure_text::source_error_text;
use super::super::need::NeedAction;
use super::super::profile_state::ProfileState;
use super::super::state::AgentsState;
use super::account_name::account_name;
use super::found_in::found_in_text;
use super::read_by::off_provider_part;
use super::reset_time::date_time_text;
use super::user_need::UserNeed;

/// One thing the user is asked to do: where, what, why, and its one
/// button when OTTY can do it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NeedViewModel {
    /// The account or the profile it is about.
    pub(crate) place: String,
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
    /// The label of the button, such as "Log in".
    pub(crate) label: String,
    /// What it does; `None` for a button shown disabled.
    pub(crate) action: Option<NeedAction>,
}

/// What the accounts need from the user, in the order of the accounts,
/// each as [`needs_of_account`] lists them, then the need of the system
/// keyring, which hides accounts until it is read.
pub(crate) fn account_needs(state: &AgentsState) -> Vec<NeedViewModel> {
    state
        .profiles()
        .accounts()
        .iter()
        .flat_map(|account| needs_of_account(state, account))
        .chain(keyring_need(state))
        .chain(plugin_update_needs(state))
        .collect()
}

/// What `account` needs from the user: an expired login, renewed by
/// opening the agent it was found in, and an organization whose bill OTTY
/// could read with an admin key it has not got.
pub(crate) fn needs_of_account(
    state: &AgentsState,
    account: &Account,
) -> Vec<NeedViewModel> {
    login_expired_need(state, account)
        .into_iter()
        .chain(admin_key_need(state, account))
        .chain(no_provider_need(state, account))
        .chain(part_off_need(state, account))
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

/// What the profile `shown` needs from the user: Log in or the program to
/// install, when it can not be used, and a report to copy when its agent
/// changed the format of its session files.
pub(crate) fn needs_of_profile(
    state: &AgentsState,
    shown: &ProfileState,
) -> Vec<NeedViewModel> {
    profile_need(state, shown)
        .into_iter()
        .chain(format_changed_need(state, shown))
        .chain(profile_plugin_failed_needs(shown))
        .collect()
}

/// The need of an organization whose bill OTTY could read but has no admin
/// key for: Add admin key opens the profile dialog of its first profile,
/// which has the field.
fn admin_key_need(
    state: &AgentsState,
    account: &Account,
) -> Option<NeedViewModel> {
    account.organization_id()?;
    let profiles = state.profiles();
    let shown = profiles.first_with(account)?;
    let profile = shown.profile();

    let can_add = shown.is_api_key() && profiles.has_bill(shown);
    if !can_add || profiles.admin_key_of(shown).is_some() {
        return None;
    }

    Some(NeedViewModel {
        place: account_name(
            state.registry(),
            state.profiles().saved(),
            account,
        ),
        title: i18n::t(Key::AgentsNeedAdminKeyTitle).to_string(),
        line: i18n::t(Key::AgentsNeedAdminKeyText).to_string(),
        fix: Some(NeedFixViewModel {
            label: i18n::t(Key::AgentsAddAdminKey).to_string(),
            action: Some(NeedAction::AddAdminKey(profile.clone())),
        }),
        extra: None,
    })
}

/// The need of an account whose provider no entry of the registry serves
/// (spec 2026-11-19-agent-plugins, "What the UI shows"): OTTY lists it by
/// its host with its tokens and estimate, but reads none of its own
/// numbers. Add plugin… opens the Plugins page with the URL field empty.
/// An installed part that is off and would serve it asks to be turned on
/// instead.
fn no_provider_need(
    state: &AgentsState,
    account: &Account,
) -> Option<NeedViewModel> {
    if state.registry().provider_of(account.provider()).is_some()
        || off_provider_part(state, account.provider()).is_some()
    {
        return None;
    }

    let found_in = match state.profiles().first_with(account) {
        Some(shown) => state.agent_name(shown.profile().agent()).to_string(),
        None => found_in_text(account.found_in().first()?, state.env().home()),
    };

    Some(NeedViewModel {
        place: account_name(
            state.registry(),
            state.profiles().saved(),
            account,
        ),
        title: i18n::t(Key::AgentsNeedNoProviderTitle).to_string(),
        line: i18n::filled(
            Key::TplAgentsNeedNoProviderText,
            &[("host", account.provider().as_str()), ("agent", &found_in)],
        ),
        fix: Some(NeedFixViewModel {
            label: i18n::t(Key::AgentsAddPlugin).to_string(),
            action: Some(NeedAction::AddPlugin),
        }),
        extra: None,
    })
}

/// The need of an account whose provider an installed part would serve,
/// but the part is off: "DeepSeek of the DeepSeek plugin is off [Turn
/// on]".
fn part_off_need(
    state: &AgentsState,
    account: &Account,
) -> Option<NeedViewModel> {
    if state.registry().provider_of(account.provider()).is_some() {
        return None;
    }
    let off = off_provider_part(state, account.provider())?;

    Some(NeedViewModel {
        place: account_name(
            state.registry(),
            state.profiles().saved(),
            account,
        ),
        title: i18n::t(Key::AgentsNeedPartOffTitle).to_string(),
        line: i18n::filled(
            Key::TplAgentsNeedPartOffText,
            &[("part", &off.part_name), ("plugin", &off.plugin_name)],
        ),
        fix: Some(NeedFixViewModel {
            label: i18n::t(Key::AgentsTurnOn).to_string(),
            action: Some(NeedAction::TurnOnPart(off.clone())),
        }),
        extra: None,
    })
}

/// The needs of the plugin updates that ask for new permissions and wait
/// for the user's OK: "OpenAI 0.4.0 → 0.5.0 asks for new permissions
/// [Review]".
fn plugin_update_needs(state: &AgentsState) -> Vec<NeedViewModel> {
    state
        .plugin_notes()
        .waiting_updates
        .iter()
        .map(|update| NeedViewModel {
            place: i18n::t(Key::AgentsPluginsPlace).to_string(),
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
    let own = state.profiles().own_reads().of(account)?;
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

/// The needs of a profile whose sessions, limits or tokens a plugin part
/// could not read, one per kind.
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
            in_failures(shown.limits().failures()),
            Key::AgentsPluginReadLimits,
        ),
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

/// The need of an account whose login expired at the latest read of its
/// limits (spec 2026-11-19, Decision 3): OTTY never renews it, so the
/// button opens the agent of the profile the limits are read through.
fn login_expired_need(
    state: &AgentsState,
    account: &Account,
) -> Option<NeedViewModel> {
    let profiles = state.profiles();
    let shown = profiles.with_limits_of(account)?;
    if !shown.limits().is_login_expired() {
        return None;
    }

    let profile = shown.profile();
    let agent = state.agent_name(profile.agent());
    let fix = profiles.can_start(profile).then(|| NeedFixViewModel {
        label: i18n::filled(Key::TplAgentsOpenAgent, &[("agent", agent)]),
        action: Some(NeedAction::OpenAgent(profile.clone())),
    });

    Some(NeedViewModel {
        place: account_name(
            state.registry(),
            state.profiles().saved(),
            account,
        ),
        title: i18n::t(Key::AgentsNeedLoginExpiredTitle).to_string(),
        line: i18n::filled(
            Key::TplAgentsNeedLoginExpiredText,
            &[("agent", agent)],
        ),
        fix,
        extra: None,
    })
}

/// The need of the system keyring (spec 2026-11-19, Decision 8), while an
/// agent that keeps keys there is on this computer and `agents.json` can
/// keep the user's answer: Allow keyring while the user has not allowed
/// the read, Try again after a read that failed.
fn keyring_need(state: &AgentsState) -> Option<NeedViewModel> {
    let profiles = state.profiles();
    let saved = profiles.saved()?;
    let keyring = profiles.keyring();
    if keyring.entries().is_empty() {
        return None;
    }

    let place = i18n::t(Key::AgentsKeyringPlace).to_string();
    if !saved.is_keyring_allowed() {
        let agents = keyring_agents(keyring.entries());
        return Some(NeedViewModel {
            place,
            title: i18n::t(Key::AgentsNeedKeyringTitle).to_string(),
            line: i18n::filled(
                Key::TplAgentsNeedKeyringText,
                &[("agents", &agents)],
            ),
            fix: Some(NeedFixViewModel {
                label: i18n::t(Key::AgentsAllowKeyring).to_string(),
                action: Some(NeedAction::AllowKeyring),
            }),
            extra: None,
        });
    }

    let line = match keyring.failure()? {
        KeyringError::NotAvailable => Key::AgentsKeyringNotAvailable,
        KeyringError::NotAllowed => Key::AgentsKeyringNotAllowed,
        _ => Key::AgentsKeyringUnreadable,
    };
    Some(NeedViewModel {
        place,
        title: i18n::t(Key::AgentsNeedKeyringFailedTitle).to_string(),
        line: i18n::t(line).to_string(),
        fix: Some(NeedFixViewModel {
            label: i18n::t(Key::AgentsTryAgain).to_string(),
            action: Some(NeedAction::ReadKeyring),
        }),
        extra: None,
    })
}

/// The names of the agents of `entries`, such as "Goose", joined.
pub(crate) fn keyring_agents(entries: &[KeyringEntry]) -> String {
    let names: Vec<&str> = entries.iter().map(KeyringEntry::agent).collect();

    names.join(", ")
}

/// The need of a profile that can not be used, with Log in while it can
/// log in.
fn profile_need(
    state: &AgentsState,
    shown: &ProfileState,
) -> Option<NeedViewModel> {
    let need = UserNeed::of(shown.status())?;
    let profile = shown.profile();

    let fix = state
        .profiles()
        .can_log_in(profile)
        .then(|| NeedFixViewModel {
            label: i18n::t(Key::AgentsLogIn).to_string(),
            action: Some(NeedAction::LogIn(profile.clone())),
        });

    Some(NeedViewModel {
        place: profile.name().to_string(),
        title: need.state().to_string(),
        line: need.hint().to_string(),
        fix,
        extra: None,
    })
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
/// "Claude Code 2.2.0 changed its session files".
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
    use otty_agents::fixtures::format_change;
    use otty_agents::{
        FormatChange, PluginFailureKind, PluginPart, ProfileStatus,
        ProviderRef, SavedProfiles, SourceError, SourceId,
    };
    use otty_app_domain::plugins::{
        OffPart, PartSide, PluginNotes, WaitingUpdate,
    };

    use super::*;
    use crate::event::AgentsIntent;
    use crate::model::reset_time::date_time_text;
    use crate::reducer::reduce;
    use crate::testing::{
        api_key, at, chatgpt_login, claude, claude_work, codex, codex_work,
        connected, deepseek_key_by_host, limits_answer, limits_loaded,
        organization, registry_without_deepseek, saved_with_admin_key,
        spend_loaded, spend_of, with_accounts, with_accounts_in, with_profiles,
    };
    use crate::types::SpendCount;

    /// Spec Decision 6: the needs of the accounts and of the profiles are
    /// collected apart and never mixed.
    #[test]
    fn the_needs_of_the_accounts_and_of_the_profiles_are_apart() {
        let state = with_accounts(
            &[
                (claude(), api_key(), vec![organization()]),
                (codex(), ProfileStatus::NotLoggedIn, Vec::new()),
                (codex_work(), ProfileStatus::ProgramNotFound, Vec::new()),
            ],
            SavedProfiles::default(),
        );

        let accounts = account_needs(&state);
        let profiles = profile_needs(&state);

        assert_eq!(
            accounts,
            vec![NeedViewModel {
                place: String::from("Anthropic · dev@example.com"),
                title: i18n::t(Key::AgentsNeedAdminKeyTitle).to_string(),
                line: i18n::t(Key::AgentsNeedAdminKeyText).to_string(),
                fix: Some(NeedFixViewModel {
                    label: i18n::t(Key::AgentsAddAdminKey).to_string(),
                    action: Some(NeedAction::AddAdminKey(claude())),
                }),
                extra: None,
            }]
        );
        assert_eq!(
            profiles,
            vec![
                NeedViewModel {
                    place: String::from("Codex"),
                    title: i18n::t(Key::AgentsStateNeedsLogin).to_string(),
                    line: i18n::t(Key::AgentsNotLoggedInHint).to_string(),
                    fix: Some(NeedFixViewModel {
                        label: i18n::t(Key::AgentsLogIn).to_string(),
                        action: Some(NeedAction::LogIn(codex())),
                    }),
                    extra: None,
                },
                NeedViewModel {
                    place: String::from("Codex · Work"),
                    title: i18n::t(Key::AgentsStateProgramNotFound).to_string(),
                    line: i18n::t(Key::AgentsProgramNotFoundHint).to_string(),
                    fix: None,
                    extra: None,
                },
            ]
        );
    }

    /// Spec 2026-11-19, Decision 3: an account whose login expired asks
    /// to open the agent it is read through, which renews the login or
    /// asks for a new one; it is a need of the account, not of the
    /// profile.
    #[test]
    fn an_account_whose_login_expired_asks_to_open_its_agent() {
        let mut state = with_accounts(
            &[(codex(), connected(), vec![chatgpt_login()])],
            SavedProfiles::default(),
        );
        let _ = reduce(
            &mut state,
            limits_loaded(
                &codex(),
                limits_answer(vec![(
                    SourceId::ChatGptUsageApi,
                    Err(SourceError::NotLoggedIn),
                )]),
                at(10),
            ),
        );

        assert_eq!(
            account_needs(&state),
            vec![NeedViewModel {
                place: String::from("OpenAI · you@example.com"),
                title: i18n::t(Key::AgentsNeedLoginExpiredTitle).to_string(),
                line: i18n::filled(
                    Key::TplAgentsNeedLoginExpiredText,
                    &[("agent", "Codex")]
                ),
                fix: Some(NeedFixViewModel {
                    label: i18n::filled(
                        Key::TplAgentsOpenAgent,
                        &[("agent", "Codex")]
                    ),
                    action: Some(NeedAction::OpenAgent(codex())),
                }),
                extra: None,
            }]
        );
        assert!(profile_needs(&state).is_empty());
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

    /// Spec 2026-11-19-agent-plugins, "What the UI shows": a key for a
    /// host no entry serves is an account by its host that asks for a
    /// provider plugin; Add plugin… opens the Plugins page.
    #[test]
    fn a_key_for_a_host_with_no_entry_asks_for_a_provider_plugin() {
        let state = with_accounts_in(
            registry_without_deepseek(),
            &[(claude(), api_key(), vec![deepseek_key_by_host()])],
            SavedProfiles::default(),
        );

        let needs = account_needs(&state);

        assert_eq!(
            needs,
            vec![NeedViewModel {
                place: format!(
                    "api.deepseek.com · {}",
                    i18n::filled(Key::TplAgentsAccountKey, &[("last", "seek")])
                ),
                title: i18n::t(Key::AgentsNeedNoProviderTitle).to_string(),
                line: i18n::filled(
                    Key::TplAgentsNeedNoProviderText,
                    &[("host", "api.deepseek.com"), ("agent", "Claude Code")]
                ),
                fix: Some(NeedFixViewModel {
                    label: i18n::t(Key::AgentsAddPlugin).to_string(),
                    action: Some(NeedAction::AddPlugin),
                }),
                extra: None,
            }]
        );
    }

    /// With the DeepSeek entry, the same key is a DeepSeek account and
    /// needs nothing.
    #[test]
    fn a_key_for_a_host_an_entry_serves_needs_no_plugin() {
        let state = with_accounts(
            &[(claude(), api_key(), vec![deepseek_key_by_host()])],
            SavedProfiles::default(),
        );

        let accounts = state.profiles().accounts();

        assert!(account_needs(&state).is_empty());
        assert_eq!(accounts[0].provider(), &ProviderRef::of_id("deepseek"));
    }

    #[test]
    fn an_organization_with_an_admin_key_needs_nothing() {
        let state = with_accounts(
            &[(claude(), api_key(), vec![organization()])],
            saved_with_admin_key(),
        );

        assert!(account_needs(&state).is_empty());
        assert!(profile_needs(&state).is_empty());
    }

    /// An organization found in two profiles asks for its admin key once.
    #[test]
    fn an_organization_of_two_profiles_needs_its_key_once() {
        let state = with_accounts(
            &[
                (claude(), api_key(), vec![organization()]),
                (claude_work(), api_key(), vec![organization()]),
            ],
            SavedProfiles::default(),
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

    fn deepseek_off() -> OffPart {
        OffPart {
            plugin: String::from("deepseek"),
            plugin_name: String::from("DeepSeek"),
            version: String::from("0.2.0"),
            side: PartSide::Provider,
            part: String::from("deepseek"),
            part_name: String::from("DeepSeek"),
            hosts: vec![String::from("deepseek.com")],
        }
    }

    /// Spec "What the UI shows": the provider part is off, so the account
    /// asks to turn it on, not for a plugin; Turn on leaves when the part
    /// serves again.
    #[test]
    fn an_account_whose_provider_part_is_off_asks_to_turn_it_on() {
        let mut state = with_accounts_in(
            registry_without_deepseek(),
            &[(claude(), api_key(), vec![deepseek_key_by_host()])],
            SavedProfiles::default(),
        );
        let _ = reduce(
            &mut state,
            AgentsIntent::PluginNotesChanged(PluginNotes {
                off_parts: vec![deepseek_off()],
                waiting_updates: Vec::new(),
            }),
        );

        let needs = account_needs(&state);

        assert_eq!(needs.len(), 1);
        assert_eq!(
            needs[0].title,
            i18n::t(Key::AgentsNeedPartOffTitle).to_string()
        );
        assert_eq!(
            needs[0].line,
            i18n::filled(
                Key::TplAgentsNeedPartOffText,
                &[("part", "DeepSeek"), ("plugin", "DeepSeek")]
            )
        );
        assert_eq!(
            needs[0].fix,
            Some(NeedFixViewModel {
                label: i18n::t(Key::AgentsTurnOn).to_string(),
                action: Some(NeedAction::TurnOnPart(deepseek_off())),
            })
        );

        let served = with_accounts(
            &[(claude(), api_key(), vec![deepseek_key_by_host()])],
            SavedProfiles::default(),
        );
        assert!(account_needs(&served).is_empty());
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

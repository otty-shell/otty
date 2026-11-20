//! The page of one profile in the main area (spec 2026-11-19, Decision
//! 6): what it needs from the user, a line with a plain fact, the usage
//! block, the accounts it pays through, its facts and where the numbers
//! come from. No line under the title.

use std::path::Path;
use std::time::SystemTime;

use otty_agents::{
    Account, AgentSession, Profile, ProfileStatus, ProviderRef, SourceFailure,
};
use otty_app_domain::folder_path::tilde_path;
use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::components::session_row::SessionTone;

use super::super::failure_text::failure_reasons;
use super::super::profile_state::ProfileState;
use super::super::state::AgentsState;
use super::super::usage::UsageTab;
use super::account_name::account_name;
use super::entry_icon::EntryIcon;
use super::kept_answer::KeptAnswer;
use super::needs::{NeedViewModel, needs_of_profile};
use super::read_by::profile_read_by;
use super::short_number::account_number;
use super::sources::{
    SourceRowViewModel, bill_state, money_sources, named_by, token_sources,
};
use super::usage_block::{UsageBlockViewModel, UsageSubject};
use super::usage_groups::UsageSource;
use super::used_by::untold_accounts;
use super::{FactViewModel, UNKNOWN};

/// The page of one profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProfilePageViewModel {
    /// The profile, sent with the buttons.
    pub(crate) profile: Profile,
    /// The icon of the agent.
    pub(crate) icon: EntryIcon,
    /// The profile name.
    pub(crate) name: String,
    /// What the profile needs from the user; the block shows only when
    /// there is any.
    pub(crate) needs: Vec<NeedViewModel>,
    /// A plain fact, not a task, such as "Codex has no sessions yet".
    pub(crate) info: Option<String>,
    /// Why the sessions could not be read, when the last read failed.
    pub(crate) failure: Option<String>,
    /// The usage block, once the tokens were counted.
    pub(crate) usage: Option<UsageBlockViewModel>,
    /// The accounts the profile pays through.
    pub(crate) pays_through: Vec<PaysThroughViewModel>,
    /// Profile info: Agent, Version, Folder, Projects and Sessions.
    pub(crate) facts: Vec<FactViewModel>,
    /// Where the numbers of the page come from.
    pub(crate) sources: Vec<SourceRowViewModel>,
}

impl ProfilePageViewModel {
    /// Build the page of the shown profile `shown` at `now`, the time the
    /// ages count from.
    pub(crate) fn of(
        state: &AgentsState,
        shown: &ProfileState,
        now: SystemTime,
    ) -> Self {
        let profile = shown.profile();
        let agent = state.agent_name(profile.agent());

        Self {
            profile: profile.clone(),
            icon: state.agent_icon(profile.agent()),
            name: profile.name().to_string(),
            needs: needs_of_profile(state, shown),
            info: info(agent, shown),
            failure: shown.failures().map(failure_text),
            usage: usage_block(state, shown, now),
            pays_through: pays_through(state, shown),
            facts: facts(state, shown),
            sources: sources(state, shown),
        }
    }

    /// The page of the profile the user hid: nothing of it is read, so
    /// under the title it says so, and Profile info has only its agent and
    /// its folder.
    pub(crate) fn hidden(state: &AgentsState, profile: &Profile) -> Self {
        let home = state.env().home();
        let facts = vec![
            FactViewModel::new(
                i18n::t(Key::AgentsAgent),
                state.agent_name(profile.agent()).to_string(),
            ),
            FactViewModel::new(
                i18n::t(Key::AgentsFieldFolder),
                tilde_path(profile.config_dir(), home),
            ),
        ];

        Self {
            profile: profile.clone(),
            icon: state.agent_icon(profile.agent()),
            name: profile.name().to_string(),
            needs: Vec::new(),
            info: Some(i18n::t(Key::AgentsHiddenProfileNote).to_string()),
            failure: None,
            usage: None,
            pays_through: Vec::new(),
            facts,
            sources: Vec::new(),
        }
    }
}

/// One chip of "Pays through"; a press opens the account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PaysThroughViewModel {
    /// The account, sent when the chip is pressed.
    pub(crate) account: Account,
    /// The icon of its provider.
    pub(crate) icon: EntryIcon,
    /// The provider and the login, such as "OpenAI · you@example.com".
    pub(crate) name: String,
    /// Its number and its color, as in the left panel ("42 % used",
    /// "login expired"); `None` while nothing is known.
    pub(crate) number: Option<(String, SessionTone)>,
}

/// Why the usage of `shown` can not be shown as money, for the Money
/// choice of its usage block; `None` when it can: the profile pays per
/// token and the list prices are known.
pub(crate) fn money_reason(
    state: &AgentsState,
    shown: &ProfileState,
) -> Option<String> {
    if !shown.is_api_key() {
        return Some(i18n::t(Key::AgentsMoneySubscription).to_string());
    }
    if state.profiles().prices().table().is_none() {
        return Some(i18n::t(Key::AgentsMoneyNoPrices).to_string());
    }

    None
}

/// Why the usage of `shown`, a profile of `agent`, can not be split by
/// account (spec Decision 4): it has several accounts of one provider and
/// its agent does not write which one served a message. `None` when it
/// can.
pub(super) fn account_reason(
    state: &AgentsState,
    agent: &str,
    shown: &ProfileState,
) -> Option<String> {
    let mut providers: Vec<&ProviderRef> =
        shown.accounts().iter().map(Account::provider).collect();
    providers.sort();
    providers.dedup();

    providers.into_iter().find_map(|provider| {
        let count = untold_accounts(state.registry(), shown, provider)?;

        Some(i18n::filled(
            Key::TplAgentsAccountsNotTold,
            &[
                ("agent", agent),
                ("count", &count.to_string()),
                ("provider", state.provider_name(provider)),
            ],
        ))
    })
}

/// The usage block of `shown` at `now`: `None` while its agent counts no
/// tokens or nothing was counted yet, only why the count failed while no
/// count ended well, and the last good numbers greyed after a failed
/// count.
fn usage_block(
    state: &AgentsState,
    shown: &ProfileState,
    now: SystemTime,
) -> Option<UsageBlockViewModel> {
    let spend = shown.spend();
    if !spend.has_counter() {
        return None;
    }

    let profiles = state.profiles();
    let agent = state.agent_name(shown.profile().agent());
    let money_reason = money_reason(state, shown);
    let account_reason = account_reason(state, agent, shown);
    let kept = KeptAnswer::new(spend.last_good(), spend.failure());
    let counted = match kept {
        KeptAnswer::Nothing => return None,
        KeptAnswer::OnlyFailure(_) => None,
        KeptAnswer::Answer { value, .. } => Some(value),
    };

    let sources: Vec<UsageSource> = counted
        .map(|counted| UsageSource {
            name: shown.profile().name(),
            spend: counted,
            accounts: shown.accounts(),
            is_priced: money_reason.is_none(),
        })
        .into_iter()
        .collect();
    let subject = UsageSubject {
        sources: &sources,
        project: None,
        tabs: &UsageTab::PROFILE_PAGE,
        prices: profiles.prices().table(),
        can_show_money: money_reason.is_none(),
        account_reason: account_reason.as_deref(),
        left_out: None,
        home: state.env().home(),
        saved: profiles.saved(),
        registry: state.registry(),
        time_zone: profiles.time_zone(),
    };
    let block = UsageBlockViewModel::of(&subject, state.usage(), now);

    let block = match kept {
        KeptAnswer::OnlyFailure(error) => block.only_failure(error),
        KeptAnswer::Answer {
            read_at,
            failure: Some(error),
            ..
        } => block.with_stale(read_at, error, profiles.time_zone()),
        _ => block,
    };
    Some(block)
}

/// The line of a plain fact once the sessions were read and there are
/// none: the agent is installed but not used yet when no one logged in,
/// else it has no sessions yet.
fn info(agent: &str, shown: &ProfileState) -> Option<String> {
    if !shown.sessions().is_some_and(<[_]>::is_empty) {
        return None;
    }

    let is_unused = shown.accounts().is_empty()
        && *shown.status() == ProfileStatus::NotLoggedIn;
    let key = if is_unused {
        Key::TplAgentsInstalledNotUsed
    } else {
        Key::TplAgentsNoSessionsYet
    };

    Some(i18n::filled(key, &[("agent", agent)]))
}

/// The chips of the accounts found in `shown`, each with its number. An
/// account the user hid leaves them.
fn pays_through(
    state: &AgentsState,
    shown: &ProfileState,
) -> Vec<PaysThroughViewModel> {
    shown
        .accounts()
        .iter()
        .filter(|account| !state.profiles().is_account_hidden(account))
        .map(|account| PaysThroughViewModel {
            account: account.clone(),
            icon: state.provider_icon(account.provider()),
            name: account_name(
                state.registry(),
                state.profiles().saved(),
                account,
            ),
            number: account_number(state, account),
        })
        .collect()
}

/// Profile info: the agent, its version, the folder, once the sessions
/// were read how many projects and sessions there are ("—" before), and
/// the plugin that reads the agent.
fn facts(state: &AgentsState, shown: &ProfileState) -> Vec<FactViewModel> {
    let profile = shown.profile();
    let home = state.env().home();
    let version = state.profiles().version_of(shown).unwrap_or(UNKNOWN);
    let sessions = shown.sessions();

    let projects = sessions.map_or_else(
        || String::from(UNKNOWN),
        |sessions| {
            let mut folders: Vec<&Path> =
                sessions.iter().map(AgentSession::folder).collect();
            folders.sort();
            folders.dedup();
            folders.len().to_string()
        },
    );
    let count = sessions.map_or_else(
        || String::from(UNKNOWN),
        |sessions| sessions.len().to_string(),
    );

    vec![
        FactViewModel::new(
            i18n::t(Key::AgentsAgent),
            state.agent_name(profile.agent()).to_string(),
        ),
        FactViewModel::new(
            i18n::t(Key::AgentsColumnVersion),
            version.to_string(),
        ),
        FactViewModel::new(
            i18n::t(Key::AgentsFieldFolder),
            tilde_path(profile.config_dir(), home),
        ),
        FactViewModel::new(i18n::t(Key::AgentsProjects), projects),
        FactViewModel::new(i18n::t(Key::AgentsSessions), count),
        profile_read_by(state, profile.agent()),
    ]
}

/// Where the numbers of the page come from: the money, when the profile
/// pays per token, then the tokens.
fn sources(
    state: &AgentsState,
    shown: &ProfileState,
) -> Vec<SourceRowViewModel> {
    let profiles = state.profiles();
    let time_zone = profiles.time_zone();

    let agent = shown.profile().agent();
    let mut rows = if shown.is_api_key() {
        money_sources(
            shown.spend(),
            shown.money(),
            profiles.prices(),
            bill_state(state, shown),
            state.writes_cost(agent),
            time_zone,
        )
    } else {
        Vec::new()
    };
    rows.extend(token_sources(
        shown.spend(),
        state.token_source(agent),
        time_zone,
    ));

    let reader = state.registry().agent(agent).map(|entry| entry.reader());
    named_by(rows, reader)
}

/// The note of a failed read of the sessions: each way of reading them
/// and why it failed, such as "files: not available: no session files".
fn failure_text(failures: &[SourceFailure]) -> String {
    i18n::with_error(Key::TplAgentsSessionsFailed, &failure_reasons(failures))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use otty_agents::{
        AccountIdentity, AccountKind, AccountLogin, DaySpend, FoundIn,
        ModelTokens, ProjectModelTokens, SavedProfiles, SourceId, Spend,
        TokenCount,
    };

    use super::*;
    use crate::event::AgentsIntent;
    use crate::failure_text::source_name;
    use crate::model::NeedFixViewModel;
    use crate::need::NeedAction;
    use crate::reducer::reduce;
    use crate::testing::{
        SESSION_VERSION, api_key, chatgpt_login, codex, connected, now,
        price_table, read, session, set_prices, spend_loaded, with_accounts,
    };
    use crate::types::SpendCount;
    use crate::usage::{UsageMeasure, UsagePeriod, UsageTab};

    /// A second made-up ChatGPT login.
    fn second_login() -> Account {
        Account::new(
            ProviderRef::of_id("openai"),
            AccountIdentity::AccountId(String::from("made-up-second")),
            AccountLogin::Email(String::from("two@example.com")),
            AccountKind::Limits,
            FoundIn::ProfileFolder(PathBuf::from("/home/user/.codex")),
        )
    }

    /// 1 000 tokens of gpt-5 in otty today, 2026-11-16, through OpenAI,
    /// paid by `account` when the agent wrote it.
    fn spend(account: Option<AccountIdentity>) -> Spend {
        let tokens =
            ModelTokens::new(Some("gpt-5"), TokenCount::new(1_000, 0, 0), 1)
                .with_provider(Some(ProviderRef::of_id("openai")))
                .with_account(account);
        let part = ProjectModelTokens::new(
            Some(PathBuf::from("/work/otty")),
            tokens.clone(),
        );

        Spend::new(
            vec![
                DaySpend::new(jiff::civil::date(2026, 11, 16), vec![tokens])
                    .with_parts(vec![part]),
            ],
            Vec::new(),
        )
    }

    /// Codex of `status` with `accounts`, its sessions read and its
    /// tokens of [`spend`] counted.
    fn codex_with(
        status: ProfileStatus,
        accounts: Vec<Account>,
        spent: Spend,
    ) -> AgentsState {
        let mut state = with_accounts(
            &[(codex(), status, accounts)],
            SavedProfiles::default(),
        );
        read(
            &mut state,
            &codex(),
            vec![
                session("c1", "/work/otty", 999_820),
                session("c2", "/work/site", 500),
                session("c3", "/work/otty", 400),
            ],
        );
        let _ = reduce(
            &mut state,
            spend_loaded(&codex(), SpendCount::Counted(spent), now()),
        );
        let _ = reduce(&mut state, AgentsIntent::SelectProfile(codex()));

        state
    }

    fn page(state: &AgentsState) -> ProfilePageViewModel {
        let shown = state.selected_profile().expect("a profile is selected");

        ProfilePageViewModel::of(state, shown, now())
    }

    fn fact(key: Key, value: &str) -> FactViewModel {
        FactViewModel::new(i18n::t(key), String::from(value))
    }

    #[test]
    fn a_subscription_shows_its_usage_its_account_and_its_info() {
        let state = codex_with(connected(), vec![chatgpt_login()], spend(None));

        let vm = page(&state);

        assert_eq!(vm.name, "Codex");
        assert_eq!(vm.icon, crate::testing::agent_icon("codex"));
        assert!(vm.needs.is_empty());
        assert_eq!(vm.info, None);
        let usage = vm.usage.expect("the usage shows");
        assert_eq!(usage.period, UsagePeriod::SevenDays);
        assert_eq!(usage.tab, UsageTab::ByModel);
        assert_eq!(usage.measure, UsageMeasure::Tokens);
        assert_eq!(usage.account_reason, None);
        assert_eq!(
            vm.pays_through,
            [PaysThroughViewModel {
                account: chatgpt_login(),
                icon: crate::testing::provider_icon("openai"),
                name: String::from("OpenAI · you@example.com"),
                number: None,
            }]
        );
        assert_eq!(
            vm.facts,
            [
                fact(Key::AgentsAgent, "Codex"),
                fact(Key::AgentsColumnVersion, SESSION_VERSION),
                fact(Key::AgentsFieldFolder, "~/.codex"),
                fact(Key::AgentsProjects, "2"),
                fact(Key::AgentsSessions, "3"),
                fact(Key::AgentsFieldReadBy, i18n::t(Key::AgentsReadByBuiltIn)),
            ]
        );
        let sources: Vec<_> = vm.sources.iter().map(|row| row.name).collect();
        assert_eq!(sources, [source_name(SourceId::SessionFiles)]);
    }

    /// The last good numbers stay greyed after a failed count; a count
    /// that never ended well shows only its failure.
    #[test]
    fn a_failed_count_keeps_the_last_good_usage_or_shows_the_failure() {
        let mut state =
            codex_with(connected(), vec![chatgpt_login()], spend(None));
        let down = || {
            SpendCount::Failed(otty_agents::SourceError::Network(String::from(
                "down",
            )))
        };
        let _ = reduce(&mut state, spend_loaded(&codex(), down(), now()));
        let stale = page(&state).usage.expect("the last good usage stays");
        let mut never = with_accounts(
            &[(codex(), connected(), vec![chatgpt_login()])],
            SavedProfiles::default(),
        );
        let _ = reduce(&mut never, spend_loaded(&codex(), down(), now()));
        let _ = reduce(&mut never, AgentsIntent::SelectProfile(codex()));
        let failed = page(&never).usage.expect("the failure shows");

        assert!(stale.is_stale);
        assert_eq!(stale.rows.len(), 1);
        assert!(stale.failure.is_some());
        assert!(failed.rows.is_empty());
        assert_eq!(failed.line, None);
        assert!(failed.failure.is_some());
    }

    #[test]
    fn money_is_off_for_a_subscription_and_until_the_prices_are_known() {
        let subscription =
            codex_with(connected(), vec![chatgpt_login()], spend(None));
        let no_prices = codex_with(api_key(), Vec::new(), spend(None));
        let mut priced = codex_with(api_key(), Vec::new(), spend(None));
        set_prices(&mut priced, price_table());
        let reason = |state: &AgentsState| {
            let shown = state.selected_profile().expect("selected");
            money_reason(state, shown)
        };

        assert_eq!(
            reason(&subscription),
            Some(i18n::t(Key::AgentsMoneySubscription).to_string())
        );
        assert_eq!(
            reason(&no_prices),
            Some(i18n::t(Key::AgentsMoneyNoPrices).to_string())
        );
        assert_eq!(reason(&priced), None);
        let sources: Vec<_> =
            page(&priced).sources.iter().map(|row| row.name).collect();
        assert_eq!(
            sources.last(),
            Some(&source_name(SourceId::SessionFiles)),
            "the money sources, then the tokens"
        );
        assert!(sources.len() > 1, "{sources:?}");
    }

    #[test]
    fn by_account_is_off_when_the_agent_does_not_tell_two_accounts_apart() {
        let untold = codex_with(
            connected(),
            vec![chatgpt_login(), second_login()],
            spend(None),
        );
        let told = codex_with(
            connected(),
            vec![chatgpt_login(), second_login()],
            spend(Some(second_login().identity().clone())),
        );

        let off = page(&untold).usage.expect("usage").account_reason;
        let on = page(&told).usage.expect("usage").account_reason;

        assert_eq!(
            off,
            Some(i18n::filled(
                Key::TplAgentsAccountsNotTold,
                &[("agent", "Codex"), ("count", "2"), ("provider", "OpenAI")]
            ))
        );
        assert_eq!(on, None);
    }

    #[test]
    fn a_profile_with_no_sessions_says_so_in_its_info_line() {
        let mut used = with_accounts(
            &[(codex(), connected(), vec![chatgpt_login()])],
            SavedProfiles::default(),
        );
        read(&mut used, &codex(), Vec::new());
        let _ = reduce(&mut used, AgentsIntent::SelectProfile(codex()));
        let mut unused = with_accounts(
            &[(codex(), ProfileStatus::NotLoggedIn, Vec::new())],
            SavedProfiles::default(),
        );
        read(&mut unused, &codex(), Vec::new());
        let _ = reduce(&mut unused, AgentsIntent::SelectProfile(codex()));

        let used = page(&used);
        let unused = page(&unused);

        assert_eq!(
            used.info,
            Some(i18n::filled(
                Key::TplAgentsNoSessionsYet,
                &[("agent", "Codex")]
            ))
        );
        assert_eq!(
            unused.info,
            Some(i18n::filled(
                Key::TplAgentsInstalledNotUsed,
                &[("agent", "Codex")]
            ))
        );
        assert_eq!(unused.needs.len(), 1);
        assert_eq!(
            unused.needs[0].fix,
            Some(NeedFixViewModel {
                label: i18n::t(Key::AgentsLogIn).to_string(),
                action: Some(NeedAction::LogIn(codex())),
            })
        );
        assert!(unused.pays_through.is_empty());
        assert!(unused.facts.contains(&fact(Key::AgentsSessions, "0")));
    }

    #[test]
    fn before_the_sessions_are_read_the_counts_are_dashes() {
        let mut state = with_accounts(
            &[(codex(), connected(), Vec::new())],
            SavedProfiles::default(),
        );
        let _ = reduce(&mut state, AgentsIntent::SelectProfile(codex()));

        let vm = page(&state);

        assert!(vm.facts.contains(&fact(Key::AgentsProjects, UNKNOWN)));
        assert!(vm.facts.contains(&fact(Key::AgentsSessions, UNKNOWN)));
        assert_eq!(vm.info, None);
        assert_eq!(vm.usage, None, "nothing was counted yet");
    }

    #[test]
    fn a_failed_read_of_the_sessions_says_why() {
        let mut state = with_accounts(
            &[(codex(), connected(), Vec::new())],
            SavedProfiles::default(),
        );
        let _ = reduce(
            &mut state,
            AgentsIntent::SessionsLoaded(crate::types::LoadedSessions {
                profile: codex(),
                sessions: crate::types::SessionsRead::Failed(vec![
                    crate::testing::no_files(),
                ]),
                missing_folders: Vec::new(),
            }),
        );
        let _ = reduce(&mut state, AgentsIntent::SelectProfile(codex()));

        let vm = page(&state);

        assert_eq!(
            vm.failure,
            Some(i18n::with_error(
                Key::TplAgentsSessionsFailed,
                &format!(
                    "{}: not available: no session files",
                    i18n::t(Key::AgentsSourceSessionFiles)
                )
            ))
        );
    }
}

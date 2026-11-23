//! The page of one profile in the main area (spec 2026-11-19, Decision
//! 6): what it needs from the user, a line with a plain fact, the usage
//! block and its facts. No line under the title, and no accounts: nothing
//! links a profile to an account (spec
//! 2026-11-23-accounts-apart-from-profiles, Decision 1). Where a number
//! comes from is the plugin's business (issue 19 of spec
//! 2026-11-19-agent-plugins).

use std::path::Path;
use std::time::SystemTime;

use otty_agents::{AgentSession, Profile, ProfileStatus, SourceFailure};
use otty_app_domain::folder_path::tilde_path;
use otty_app_i18n::{self as i18n, Key};

use super::super::failure_text::failure_reasons;
use super::super::profile_state::ProfileState;
use super::super::state::AgentsState;
use super::super::usage::UsageTab;
use super::entry_icon::EntryIcon;
use super::kept_answer::KeptAnswer;
use super::needs::{NeedViewModel, needs_of_profile};
use super::read_by::profile_read_by;
use super::usage_block::{UsageBlockViewModel, UsageSubject};
use super::usage_groups::UsageSource;
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
    /// A plain fact, not a task, such as "Coder has no sessions yet".
    pub(crate) info: Option<String>,
    /// Why the sessions could not be read, when the last read failed.
    pub(crate) failure: Option<String>,
    /// The usage block, once the tokens were counted.
    pub(crate) usage: Option<UsageBlockViewModel>,
    /// Profile info: Agent, Version, Folder, Projects, Sessions and Read
    /// by.
    pub(crate) facts: Vec<FactViewModel>,
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
            facts: facts(state, shown),
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
            facts,
        }
    }
}

/// Why the usage can not be shown as money, for the Money choice of a
/// usage block: the list prices are not known yet. `None` when they are:
/// the money of every profile is its tokens at API prices, on a
/// subscription too (spec 2026-11-23-accounts-apart-from-profiles,
/// Decision 3).
pub(crate) fn money_reason(state: &AgentsState) -> Option<String> {
    if state.profiles().prices().table().is_none() {
        return Some(i18n::t(Key::AgentsMoneyNoPrices).to_string());
    }

    None
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
    let money_reason = money_reason(state);
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
        })
        .into_iter()
        .collect();
    let subject = UsageSubject {
        sources: &sources,
        project: None,
        tabs: &UsageTab::PROFILE_PAGE,
        prices: profiles.prices().last_good(),
        can_show_money: money_reason.is_none(),
        home: state.env().home(),
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
/// none: the agent is installed but not used yet when its agent part says
/// no one logged in, else it has no sessions yet. Whatever accounts were
/// found in its folder change nothing (spec
/// 2026-11-23-accounts-apart-from-profiles, Decision 1).
fn info(agent: &str, shown: &ProfileState) -> Option<String> {
    if !shown.sessions().is_some_and(<[_]>::is_empty) {
        return None;
    }

    let is_unused = matches!(shown.status(), ProfileStatus::NotLoggedIn(_));
    let key = if is_unused {
        Key::TplAgentsInstalledNotUsed
    } else {
        Key::TplAgentsNoSessionsYet
    };

    Some(i18n::filled(key, &[("agent", agent)]))
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

/// The note of a failed read of the sessions: each way of reading them
/// and why it failed, such as "files: not available: no session files".
fn failure_text(failures: &[SourceFailure]) -> String {
    i18n::with_error(Key::TplAgentsSessionsFailed, &failure_reasons(failures))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use otty_agents::{
        Account, DaySpend, ModelTokens, ProjectModelTokens, ProviderRef,
        SavedProfiles, Spend, TokenCount,
    };

    use super::*;
    use crate::event::AgentsIntent;
    use crate::model::NeedFixViewModel;
    use crate::need::NeedAction;
    use crate::reducer::reduce;
    use crate::testing::{
        SESSION_VERSION, at_list_prices, chatgpt_login, claude, codex,
        connected, month_spend, not_logged_in, now, numbers_loaded,
        price_table, read, session, set_prices, spend_loaded, with_accounts,
    };
    use crate::types::SpendCount;
    use crate::usage::{UsageMeasure, UsagePeriod, UsageTab};

    /// 1 000 tokens of gpt-5 in otty today, 2026-11-16, through OpenAI.
    fn spend() -> Spend {
        let tokens =
            ModelTokens::new(Some("gpt-5"), TokenCount::new(1_000, 0, 0), 1)
                .with_provider(Some(ProviderRef::of_id("openai")))
                .with_money(at_list_prices(1_000));
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

    /// Spec 2026-11-23, Decision 1: the page of a profile logged in to an
    /// account shows its usage and its info, and no account: there is no
    /// Pays through, and the tabs have no By account.
    #[test]
    fn a_subscription_shows_its_usage_and_its_info_and_no_account() {
        let state = codex_with(connected(), vec![chatgpt_login()], spend());

        let vm = page(&state);

        assert_eq!(vm.name, "Codex");
        assert_eq!(vm.icon, crate::testing::agent_icon("codex"));
        assert!(vm.needs.is_empty());
        assert_eq!(vm.info, None);
        let usage = vm.usage.expect("the usage shows");
        assert_eq!(usage.period, UsagePeriod::SevenDays);
        assert_eq!(usage.tab, UsageTab::ByModel);
        assert_eq!(usage.tabs, [UsageTab::ByProject, UsageTab::ByModel]);
        assert_eq!(usage.measure, UsageMeasure::Tokens);
        assert_eq!(
            vm.facts,
            [
                fact(Key::AgentsAgent, "Codex"),
                fact(Key::AgentsColumnVersion, SESSION_VERSION),
                fact(Key::AgentsFieldFolder, "~/.codex"),
                fact(Key::AgentsProjects, "2"),
                fact(Key::AgentsSessions, "3"),
                FactViewModel::linked(
                    i18n::t(Key::AgentsFieldReadBy),
                    String::from("OpenAI 0.1.0"),
                    String::from("openai")
                ),
            ]
        );
    }

    /// Codex of `status` with no sessions, `accounts` found in its folder
    /// and the numbers of each of them read and failed, selected.
    fn unused_codex_with(
        status: ProfileStatus,
        accounts: Vec<Account>,
    ) -> AgentsState {
        let mut state = with_accounts(
            &[(codex(), status, accounts.clone())],
            SavedProfiles::default(),
        );
        read(&mut state, &codex(), Vec::new());
        for account in &accounts {
            let down = otty_agents::SourceError::Network(String::from("down"));
            let _ =
                reduce(&mut state, numbers_loaded(account, Err(down), now()));
        }
        let _ = reduce(&mut state, AgentsIntent::SelectProfile(codex()));

        state
    }

    /// Issue 11 of spec 2026-11-23-accounts-apart-from-profiles: nothing
    /// of a profile's page depends on the accounts found in its folder or
    /// on what was read of them, logged in or not.
    #[test]
    fn a_profile_page_is_the_same_with_and_without_accounts() {
        for status in [connected(), not_logged_in(&codex())] {
            let with = unused_codex_with(
                status.clone(),
                vec![chatgpt_login(), crate::testing::key_account()],
            );
            let without = unused_codex_with(status.clone(), Vec::new());

            assert_eq!(page(&with), page(&without), "{status:?}");
        }
    }

    /// The last good numbers stay greyed after a failed count; a count
    /// that never ended well shows only its failure.
    #[test]
    fn a_failed_count_keeps_the_last_good_usage_or_shows_the_failure() {
        let mut state = codex_with(connected(), vec![chatgpt_login()], spend());
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

    /// Spec 2026-11-23, Decision 3: money is off only until the prices are
    /// known, for a subscription as for a key.
    #[test]
    fn money_is_off_only_until_the_prices_are_known() {
        let no_prices = codex_with(connected(), vec![chatgpt_login()], spend());
        let mut subscription =
            codex_with(connected(), vec![chatgpt_login()], spend());
        set_prices(&mut subscription, price_table());
        let mut key = codex_with(connected(), Vec::new(), spend());
        set_prices(&mut key, price_table());

        assert_eq!(
            money_reason(&no_prices),
            Some(i18n::t(Key::AgentsMoneyNoPrices).to_string())
        );
        assert_eq!(money_reason(&subscription), None);
        assert_eq!(money_reason(&key), None);
    }

    /// Spec 2026-11-23, Decision 3: the money of a profile logged in with a
    /// subscription is its tokens at API prices, shown and marked "at API
    /// prices".
    #[test]
    fn money_of_a_subscription_is_shown_at_api_prices() {
        let mut state =
            codex_with(connected(), vec![chatgpt_login()], month_spend());
        set_prices(&mut state, price_table());

        let shown = usage_after(
            &mut state,
            &[AgentsIntent::PickMeasure(UsageMeasure::Money)],
        );

        let money = &shown[0];
        assert_eq!(money.measure, UsageMeasure::Money);
        assert_eq!(money.rows[0].value, "$0.88");
        let mark = money
            .money_kind
            .as_ref()
            .and_then(crate::model::MoneyKind::mark)
            .expect("the money is marked");
        assert_eq!(mark.label, i18n::t(Key::AgentsApiPricesMark));
    }

    /// The usage block of the page after each intent of `intents`.
    fn usage_after(
        state: &mut AgentsState,
        intents: &[AgentsIntent],
    ) -> Vec<UsageBlockViewModel> {
        intents
            .iter()
            .map(|intent| {
                let _ = reduce(state, intent.clone());
                page(state).usage.expect("the usage shows")
            })
            .collect()
    }

    /// Owner, 2026-11-23: the period switch of the page. Today has a bar
    /// per hour, 7 and 30 days a bar per day with today the last, and the
    /// total under the title follows.
    #[test]
    fn each_period_shows_its_own_bars_and_total() {
        let mut state =
            codex_with(connected(), vec![chatgpt_login()], month_spend());

        let shown = usage_after(
            &mut state,
            &[
                AgentsIntent::PickPeriod(UsagePeriod::Today),
                AgentsIntent::PickPeriod(UsagePeriod::ThirtyDays),
                AgentsIntent::PickPeriod(UsagePeriod::SevenDays),
            ],
        );

        let [today, month, week] = shown.as_slice() else {
            panic!("three blocks: {shown:?}");
        };
        let today_word = i18n::t(Key::AgentsToday).to_string();
        let in_days = |amount: &str, count: &str| {
            Some(i18n::filled(
                Key::TplAgentsAmountInDays,
                &[("amount", amount), ("count", count)],
            ))
        };
        assert_eq!(today.period, UsagePeriod::Today);
        assert_eq!(today.bars.len(), 24);
        assert_eq!(today.bars[9].value, 100_000);
        assert_eq!(
            today.line,
            Some(i18n::filled(
                Key::TplAgentsAmountToday,
                &[("amount", "100K")]
            ))
        );
        assert_eq!(
            today.axis,
            Some((String::from("00:00"), String::from("23:00")))
        );
        assert_eq!(month.period, UsagePeriod::ThirtyDays);
        assert_eq!(month.bars.len(), 30);
        assert_eq!(month.line, in_days("3M", "30"));
        assert_eq!(month.current, Some(29), "today is the last bar");
        assert_eq!(
            month.axis,
            Some((String::from("Oct 18"), today_word.clone()))
        );
        assert_eq!(week.period, UsagePeriod::SevenDays);
        assert_eq!(week.bars.len(), 7);
        assert_eq!(week.line, in_days("700K", "7"));
        assert_eq!(week.axis, Some((String::from("Nov 10"), today_word)));
    }

    /// Owner, 2026-11-23: the measure switch of the page. Money writes
    /// the total, the bars and the rows in dollars at API prices, with
    /// its mark; Tokens writes them in tokens again.
    #[test]
    fn money_and_tokens_switch_the_numbers_of_the_page() {
        let mut state = codex_with(connected(), Vec::new(), month_spend());
        set_prices(&mut state, price_table());

        let shown = usage_after(
            &mut state,
            &[
                AgentsIntent::PickMeasure(UsageMeasure::Money),
                AgentsIntent::PickMeasure(UsageMeasure::Tokens),
            ],
        );

        let [money, tokens] = shown.as_slice() else {
            panic!("two blocks: {shown:?}");
        };
        let in_week = |amount: &str| {
            Some(i18n::filled(
                Key::TplAgentsAmountInDays,
                &[("amount", amount), ("count", "7")],
            ))
        };
        assert_eq!(money.measure, UsageMeasure::Money);
        assert_eq!(money.line, in_week("$0.88"));
        assert_eq!(money.rows[0].value, "$0.88");
        assert_eq!(money.bars[6].value, 125_000, "micro-dollars");
        assert!(money.money_kind.is_some(), "the money is marked");
        assert_eq!(tokens.measure, UsageMeasure::Tokens);
        assert_eq!(tokens.line, in_week("700K"));
        assert_eq!(tokens.rows[0].value, "700K");
        assert_eq!(tokens.bars[6].value, 100_000);
        assert_eq!(tokens.money_kind, None);
    }

    /// The module doc of `usage.rs`: the choices of the profile pages are
    /// the same for every profile. A period and a measure picked on one
    /// page show on the next and on the first again.
    #[test]
    fn the_choices_stay_when_another_profile_shows() {
        let mut state = with_accounts(
            &[
                (codex(), connected(), Vec::new()),
                (claude(), connected(), Vec::new()),
            ],
            SavedProfiles::default(),
        );
        for profile in [codex(), claude()] {
            let _ = reduce(
                &mut state,
                spend_loaded(
                    &profile,
                    SpendCount::Counted(month_spend()),
                    now(),
                ),
            );
        }
        set_prices(&mut state, price_table());
        let _ = reduce(&mut state, AgentsIntent::SelectProfile(codex()));

        let shown = usage_after(
            &mut state,
            &[
                AgentsIntent::PickPeriod(UsagePeriod::Today),
                AgentsIntent::PickMeasure(UsageMeasure::Money),
                AgentsIntent::SelectProfile(claude()),
                AgentsIntent::SelectProfile(codex()),
            ],
        );

        for usage in &shown[2..] {
            assert_eq!(usage.period, UsagePeriod::Today);
            assert_eq!(usage.measure, UsageMeasure::Money);
            assert_eq!(usage.bars.len(), 24);
        }
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
            &[(codex(), crate::testing::not_logged_in(&codex()), Vec::new())],
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
                label: i18n::filled(
                    Key::TplAgentsNeedRun,
                    &[("command", "codex login")]
                ),
                action: Some(NeedAction::Run {
                    title: String::from("Codex"),
                    command: otty_agents::AgentCommand::new("codex")
                        .with_arg("login"),
                }),
            })
        );
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
                    i18n::t(Key::AgentsSourcePlugin)
                )
            ))
        );
    }
}

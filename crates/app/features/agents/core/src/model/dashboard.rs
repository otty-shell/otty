use std::path::Path;
use std::time::SystemTime;

use otty_agents::{
    AgentSession, BillingKind, Profile, ProfileStatus, SourceFailure,
};
use otty_app_i18n::{self as i18n, Key};

use super::super::age::age_text;
use super::super::state::{AgentsState, ProfileState};
use super::limits::LimitsViewModel;
use super::money::MoneyViewModel;
use super::sources::{
    BillSourceState, SourceRowViewModel, limit_sources, money_sources,
    token_sources,
};
use super::tokens::TokensViewModel;
use super::top_table::{TopViewModel, top};
use super::{failure_reasons, folder_name};

/// The dashboard of the selected profile in the main area (spec Decisions
/// 6, 9, 10, 12 and 13): the header with the name and its buttons, the
/// billing block, the tokens, the facts of the profile, its top section and
/// where the numbers come from. A block the profile's agent does not
/// support is left out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DashboardViewModel {
    /// The profile, sent with the buttons.
    pub(crate) profile: Profile,
    /// The name of the agent's icon.
    pub(crate) icon: &'static str,
    /// The profile name.
    pub(crate) name: String,
    /// Whether Log in shows under the hint: the profile is not logged in
    /// and its program was found.
    pub(crate) can_log_in: bool,
    /// What is missing, or what to do, when the profile can not be used.
    pub(crate) hint: Option<&'static str>,
    /// Why the sessions could not be read, when the last read failed.
    pub(crate) failure: Option<String>,
    /// The billing block: the limits, the credits or the money.
    pub(crate) billing: Option<BillingViewModel>,
    /// The tokens of the last 30 days.
    pub(crate) tokens: Option<TokensViewModel>,
    /// The facts of the Profile list: a label and its value.
    pub(crate) facts: Vec<(&'static str, String)>,
    /// The projects or the models that spent the most tokens in the last
    /// 30 days.
    pub(crate) top: Option<TopViewModel>,
    /// Where the numbers come from, in order.
    pub(crate) sources: Vec<SourceRowViewModel>,
}

impl DashboardViewModel {
    /// Build the dashboard of the shown profile `shown` at `now`, the time
    /// the ages and the reset times count from.
    pub(in super::super) fn from_profile(
        state: &AgentsState,
        shown: &ProfileState,
        now: SystemTime,
    ) -> Self {
        let profile = shown.profile();
        let agent = state.agent(profile.agent());
        let (account, plan) = match shown.status() {
            ProfileStatus::Connected { account, plan, .. } => {
                (account.clone(), plan.clone())
            },
            _ => (None, None),
        };
        let is_api_key = billing_kind(shown) == Some(BillingKind::ApiKey);
        let plan = if is_api_key {
            let provider =
                agent.map_or("", |agent| agent.descriptor().provider());
            Some(i18n::agents_template(
                Key::TplAgentsProviderApi,
                &[("provider", provider)],
            ))
        } else {
            plan.or_else(|| limits_plan(shown))
        };
        let hint = match shown.status() {
            ProfileStatus::NotLoggedIn => Some(Key::AgentsNotLoggedInHint),
            ProfileStatus::ProgramNotFound => {
                Some(Key::AgentsProgramNotFoundHint)
            },
            ProfileStatus::ConfigDirNotFound => {
                Some(Key::AgentsConfigDirNotFoundHint)
            },
            _ => None,
        };

        let (billing, sources) = if is_api_key {
            (
                spending(state, shown, now).map(BillingViewModel::Money),
                money_sources(
                    shown.spend(),
                    shown.money(),
                    state.prices(),
                    bill_state(state, shown),
                    state.utc_offset_seconds(),
                ),
            )
        } else {
            (
                LimitsViewModel::from_limits(
                    shown.limits(),
                    now,
                    state.utc_offset_seconds(),
                )
                .map(BillingViewModel::Limits),
                limit_sources(shown.limits(), state.utc_offset_seconds()),
            )
        };
        let mut sources = sources;
        sources
            .extend(token_sources(shown.spend(), state.utc_offset_seconds()));
        let top = shown.spend().last_good().and_then(|(spend, _)| {
            top(spend, shown.sessions().unwrap_or_default(), state.top_tab())
        });

        let facts = facts(Facts {
            account,
            plan,
            config_dir: home_relative(profile.config_dir(), state.env().home()),
            sessions: shown.sessions(),
            now,
        });

        Self {
            profile: profile.clone(),
            icon: state.icon_of(profile.agent()),
            name: profile.name().to_string(),
            can_log_in: state.can_log_in(profile),
            hint: hint.map(i18n::t),
            failure: shown.failures().map(failure_text),
            billing,
            tokens: TokensViewModel::from_spend(shown.spend(), now),
            facts,
            top,
            sources,
        }
    }
}

/// The billing block of a dashboard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BillingViewModel {
    /// The usage windows or the credits of a subscription.
    Limits(LimitsViewModel),
    /// The money of an API key.
    Money(MoneyViewModel),
}

/// What the Profile list of a dashboard is made of.
struct Facts<'a> {
    account: Option<String>,
    plan: Option<String>,
    config_dir: String,
    sessions: Option<&'a [AgentSession]>,
    now: SystemTime,
}

/// How the account of `shown` is paid for: an API key as its files tell,
/// else what the limits answer says, else what the files tell.
fn billing_kind(shown: &ProfileState) -> Option<BillingKind> {
    if shown.is_api_key() {
        return Some(BillingKind::ApiKey);
    }

    let from_limits = shown
        .limits()
        .last_good()
        .map(|(answer, _)| answer.value().billing().kind());
    let from_files = match shown.status() {
        ProfileStatus::Connected { billing, .. } => *billing,
        _ => None,
    };

    from_limits.or(from_files)
}

/// The Usage block of an API key profile: from the bill when its latest
/// read answered, else the estimate at the prices of `state` (spec
/// Decision 5).
fn spending(
    state: &AgentsState,
    shown: &ProfileState,
    now: SystemTime,
) -> Option<MoneyViewModel> {
    let budget = shown.profile().monthly_budget();
    let bill = shown.bill();

    match bill.answer().zip(bill.read_at()) {
        Some((bill, read_at)) => {
            Some(MoneyViewModel::from_bill(bill, read_at, budget, now))
        },
        None => MoneyViewModel::from_money(
            shown.money(),
            shown.spend(),
            state.prices(),
            budget,
            now,
        ),
    }
}

/// What is known of the bill of `shown`: whether its agent has one and
/// the profile an admin key to read it with.
fn bill_state<'a>(
    state: &AgentsState,
    shown: &'a ProfileState,
) -> BillSourceState<'a> {
    let profile = shown.profile();

    if !state.has_bill(profile.agent()) {
        BillSourceState::NoSource
    } else if profile.admin_key().is_none() {
        BillSourceState::NoAdminKey
    } else {
        BillSourceState::Read(shown.bill())
    }
}

/// The plan the last good limits named, such as Codex's "Plus".
fn limits_plan(shown: &ProfileState) -> Option<String> {
    let (answer, _) = shown.limits().last_good()?;

    answer.value().plan().map(String::from)
}

/// The Profile list: the account and the plan when known, the config
/// folder, and once the sessions were read, how many there are in how
/// many projects and when the profile was last used.
fn facts(facts: Facts<'_>) -> Vec<(&'static str, String)> {
    let mut list = Vec::new();
    if let Some(account) = facts.account {
        list.push((i18n::t(Key::AgentsFieldAccount), account));
    }
    if let Some(plan) = facts.plan {
        list.push((i18n::t(Key::AgentsFieldPlan), plan));
    }
    list.push((i18n::t(Key::AgentsFieldConfigDir), facts.config_dir));

    let Some(sessions) = facts.sessions else {
        return list;
    };
    let mut folders: Vec<&Path> =
        sessions.iter().map(AgentSession::folder).collect();
    folders.sort();
    folders.dedup();
    list.push((
        i18n::t(Key::AgentsFieldSessions),
        i18n::agents_template(
            Key::TplAgentsSessionsInProjects,
            &[
                ("sessions", &format!("{}", sessions.len())),
                ("projects", &format!("{}", folders.len())),
            ],
        ),
    ));

    let latest = sessions.iter().max_by_key(|session| session.changed_at());
    if let Some(latest) = latest {
        list.push((
            i18n::t(Key::AgentsFieldLastUsed),
            i18n::agents_template(
                Key::TplAgentsLastUsedIn,
                &[
                    ("time", &age_text(facts.now, latest.changed_at())),
                    ("project", &folder_name(latest.folder())),
                ],
            ),
        ));
    }

    list
}

/// `path` with the home folder `home` written as `~`, such as
/// `~/.claude`.
fn home_relative(path: &Path, home: &Path) -> String {
    match path.strip_prefix(home) {
        Ok(rest) if rest.as_os_str().is_empty() => String::from("~"),
        Ok(rest) => format!("~/{}", rest.display()),
        Err(_) => path.display().to_string(),
    }
}

/// The note of a failed read: each way of reading the sessions and why it
/// failed, such as "files: not available: no projects folder".
fn failure_text(failures: &[SourceFailure]) -> String {
    i18n::with_error(Key::TplAgentsSessionsFailed, &failure_reasons(failures))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Duration;

    use otty_agents::{
        Answer, Billing, CalendarDate, Credits, DaySpend, FetchedPrices,
        Limits, ModelTokens, ProjectTokens, SourceError, SourceId, Spend,
        TokenCount,
    };

    use super::super::reset_time::resets_text;
    use super::super::sources::source_name;
    use super::*;
    use crate::event::AgentsIntent;
    use crate::model::{TopModelRowViewModel, TopRowsViewModel, TopViewModel};
    use crate::reducer::reduce;
    use crate::testing::{
        api_key, at, bill_loaded, bill_of, claude, claude_with_admin_key,
        claude_work, codex, connected, dollars, env, fetched_prices,
        five_hour_limits, limits_answer, limits_loaded, limits_loaded_from,
        now, price_table, prices_answer, prices_loaded, read, session,
        spend_loaded, with_profiles,
    };
    use crate::top_tab::TopTab;

    /// The ids of all the limit sources of an agent, best first, and the
    /// answers of those its walk tried.
    type LimitsWalk =
        (Vec<SourceId>, Vec<(SourceId, Result<Limits, SourceError>)>);

    fn dashboard(state: &AgentsState) -> DashboardViewModel {
        let shown = state.selected_profile().expect("a profile is selected");

        DashboardViewModel::from_profile(state, shown, now())
    }

    fn subscription() -> ProfileStatus {
        ProfileStatus::Connected {
            account: Some(String::from("user@example.com")),
            plan: Some(String::from("Max")),
            billing: Some(BillingKind::UsageWindows),
        }
    }

    /// Tokens of one project, `/work/otty`, spent today.
    fn spend_in_otty() -> Spend {
        let tokens = TokenCount::new(9_000, 0, 1_000);
        let today = DaySpend::new(
            CalendarDate::new(2026, 11, 16),
            vec![ModelTokens::new(Some("claude-sonnet-4-5"), tokens, 2)],
        );

        Spend::new(
            vec![today],
            vec![ProjectTokens::new(PathBuf::from("/work/otty"), tokens)],
        )
    }

    #[test]
    fn a_subscription_shows_its_plan_account_limits_tokens_and_projects() {
        let mut state = with_profiles(&[(claude(), subscription())]);
        read(
            &mut state,
            &claude(),
            vec![
                session("c1", "/work/otty", 999_820),
                session("c2", "/work/site", 500),
            ],
        );
        let _ = reduce(
            &mut state,
            limits_loaded(
                &claude(),
                limits_answer(vec![(
                    SourceId::AnthropicUsageApi,
                    Ok(five_hour_limits(42.0)),
                )]),
                now(),
            ),
        );
        let _ = reduce(
            &mut state,
            spend_loaded(&claude(), Some(Ok(spend_in_otty())), now()),
        );
        let _ = reduce(&mut state, AgentsIntent::SelectProfile(claude()));

        let vm = dashboard(&state);

        assert_eq!(vm.name, "Claude Code");
        assert_eq!(vm.icon, "claude-code");
        assert!(matches!(vm.billing, Some(BillingViewModel::Limits(_))));
        assert!(vm.tokens.is_some());
        assert!(vm.top.is_some());
        assert_eq!(
            vm.sources.iter().map(|row| row.name).collect::<Vec<_>>(),
            [
                i18n::t(Key::AgentsSourceAnthropicUsageApi),
                i18n::t(Key::AgentsSourceSessionFiles)
            ]
        );
        assert!(!vm.can_log_in);
        let last_used = i18n::agents_template(
            Key::TplAgentsLastUsedIn,
            &[
                ("time", &i18n::with_count(Key::TplAgentsAgeMinutes, 3)),
                ("project", "otty"),
            ],
        );
        assert_eq!(
            vm.facts,
            vec![
                (
                    i18n::t(Key::AgentsFieldAccount),
                    String::from("user@example.com")
                ),
                (i18n::t(Key::AgentsFieldPlan), String::from("Max")),
                (
                    i18n::t(Key::AgentsFieldConfigDir),
                    String::from("~/.claude")
                ),
                (
                    i18n::t(Key::AgentsFieldSessions),
                    i18n::agents_template(
                        Key::TplAgentsSessionsInProjects,
                        &[("sessions", "2"), ("projects", "2")]
                    )
                ),
                (i18n::t(Key::AgentsFieldLastUsed), last_used),
            ]
        );
    }

    #[test]
    fn an_api_key_profile_pays_per_token_and_shows_its_money() {
        let mut state = with_profiles(&[(claude(), api_key())]);
        state.set_prices(price_table());
        let _ = reduce(
            &mut state,
            spend_loaded(&claude(), Some(Ok(spend_in_otty())), now()),
        );
        let _ = reduce(&mut state, AgentsIntent::SelectProfile(claude()));

        let vm = dashboard(&state);

        assert!(
            vm.facts.contains(&(
                i18n::t(Key::AgentsFieldPlan),
                String::from("Anthropic API")
            )),
            "the plan is the provider's API with no more words: {:?}",
            vm.facts
        );
        assert!(matches!(vm.billing, Some(BillingViewModel::Money(_))));
        assert!(vm.tokens.is_some());
        assert_eq!(
            vm.sources
                .iter()
                .take(2)
                .map(|row| row.name)
                .collect::<Vec<_>>(),
            [
                source_name(SourceId::AnthropicCostReport),
                source_name(SourceId::SessionFilesAtListPrices)
            ],
            "with no admin key the bill is listed first, as not available"
        );
    }

    #[test]
    fn a_bill_that_answered_shows_in_place_of_the_estimate() {
        let profile =
            claude_with_admin_key().with_monthly_budget(Some(dollars(300)));
        let mut state = with_profiles(&[(profile.clone(), api_key())]);
        let _ = reduce(
            &mut state,
            spend_loaded(&profile, Some(Ok(spend_in_otty())), now()),
        );
        let _ = reduce(&mut state, bill_loaded(&profile, Ok(bill_of(12, 3))));
        let _ = reduce(&mut state, AgentsIntent::SelectProfile(profile));

        let vm = dashboard(&state);

        let Some(BillingViewModel::Money(money)) = vm.billing else {
            panic!("an API key shows its money");
        };
        let used: Vec<_> =
            money.rows.iter().map(|row| row.used.as_str()).collect();
        assert_eq!(used[0], "$12.00", "today is the bill's");
        assert_eq!(money.rows.last().and_then(|row| row.percent), Some(5));
        let states: Vec<_> =
            vm.sources.iter().map(|row| (row.name, row.state)).collect();
        assert_eq!(
            states,
            [
                (
                    source_name(SourceId::AnthropicCostReport),
                    i18n::t(Key::AgentsSourceAnswered)
                ),
                (
                    source_name(SourceId::SessionFilesAtListPrices),
                    i18n::t(Key::AgentsSourceNotNeeded)
                ),
                (
                    source_name(SourceId::SessionFiles),
                    i18n::t(Key::AgentsSourceAnswered)
                ),
            ]
        );
        assert_eq!(
            money.updated,
            Some(i18n::agents_template(
                Key::TplAgentsUpdated,
                &[("time", i18n::t(Key::AgentsAgeJustNow))]
            ))
        );
    }

    /// Owner, issue 30: the blocks after the Usage block of an API key are
    /// a subscription's, with the top projects and no money by model.
    #[test]
    fn an_api_key_has_the_top_projects_of_a_subscription() {
        let shown_as = |status: ProfileStatus| {
            let mut state = with_profiles(&[(claude(), status)]);
            read(
                &mut state,
                &claude(),
                vec![session("c1", "/work/otty", 999_820)],
            );
            let _ = reduce(
                &mut state,
                spend_loaded(&claude(), Some(Ok(spend_in_otty())), now()),
            );
            let _ = reduce(&mut state, AgentsIntent::SelectProfile(claude()));
            dashboard(&state)
        };

        let api_key = shown_as(api_key());
        let subscription = shown_as(subscription());

        assert!(api_key.top.is_some(), "the top projects show");
        assert_eq!(api_key.top, subscription.top);
    }

    /// Owner, issue 30: what the money is, the estimate at the prices of a
    /// time or the bill, is said only by the sources, in their tooltips.
    #[test]
    fn the_sources_say_whether_the_money_is_an_estimate_or_the_bill() {
        // 2026-11-16T10:00:00Z.
        let fetched_at = at(1_794_823_200);
        let mut state = with_profiles(&[(claude(), api_key())]);
        let _ = reduce(
            &mut state,
            prices_loaded(
                prices_answer(&[], Some(fetched_prices(fetched_at))),
                fetched_at,
            ),
        );
        let _ = reduce(
            &mut state,
            spend_loaded(&claude(), Some(Ok(spend_in_otty())), now()),
        );
        let _ = reduce(&mut state, AgentsIntent::SelectProfile(claude()));
        let estimate = dashboard(&state);

        let billed = claude_with_admin_key();
        let mut state = with_profiles(&[(billed.clone(), api_key())]);
        let _ = reduce(&mut state, bill_loaded(&billed, Ok(bill_of(12, 3))));
        let _ = reduce(&mut state, AgentsIntent::SelectProfile(billed));
        let bill = dashboard(&state);

        let estimate_tip = i18n::agents_template(
            Key::TplAgentsMoneyEstimate,
            &[("time", "Nov 16, 2026, 10:00")],
        );
        assert_eq!(estimate.sources[1].tip, Some(estimate_tip));
        assert_eq!(
            bill.sources[0].tip,
            Some(i18n::t(Key::AgentsMoneyFromBill).to_string())
        );
    }

    /// Owner, issue 33: the top section lists the projects or the models,
    /// as chosen; the choice stays when another profile is selected.
    #[test]
    fn the_top_section_lists_the_chosen_tab_for_every_profile() {
        let mut state =
            with_profiles(&[(claude(), subscription()), (codex(), api_key())]);
        for profile in [claude(), codex()] {
            let _ = reduce(
                &mut state,
                spend_loaded(&profile, Some(Ok(spend_in_otty())), now()),
            );
        }
        let _ = reduce(&mut state, AgentsIntent::SelectProfile(claude()));
        let tab = |state: &AgentsState| dashboard(state).top.map(|top| top.tab);
        let at_start = tab(&state);

        let _ = reduce(&mut state, AgentsIntent::ShowTop(TopTab::ByModel));
        let by_model = dashboard(&state).top;
        let _ = reduce(&mut state, AgentsIntent::SelectProfile(codex()));
        let other_profile = tab(&state);
        let _ = reduce(&mut state, AgentsIntent::ShowTop(TopTab::ByProject));
        let back = tab(&state);

        assert_eq!(at_start, Some(TopTab::ByProject));
        assert_eq!(
            by_model,
            Some(TopViewModel {
                tab: TopTab::ByModel,
                rows: TopRowsViewModel::Models(vec![TopModelRowViewModel {
                    name: String::from("claude-sonnet-4-5"),
                    requests: String::from("2"),
                    tokens: String::from("10K"),
                }]),
            })
        );
        assert_eq!(other_profile, Some(TopTab::ByModel));
        assert_eq!(back, Some(TopTab::ByProject));
    }

    /// The table of the sources of `vm`: the number of each row, empty
    /// below the first row of a group, and the name.
    fn table(vm: &DashboardViewModel) -> Vec<(&'static str, &'static str)> {
        vm.sources
            .iter()
            .map(|row| (row.number.unwrap_or_default(), row.name))
            .collect()
    }

    /// The state with `profile` of `status` selected: the walk over the
    /// limit sources of `limits`, all of its agent with their answers until
    /// one answered, gave its limits, its tokens were counted, and the
    /// prices are what `prices` gave.
    fn shown_with(
        profile: &Profile,
        status: ProfileStatus,
        limits: Option<LimitsWalk>,
        prices: Option<Result<Answer<FetchedPrices>, Vec<SourceFailure>>>,
    ) -> AgentsState {
        let mut state = with_profiles(&[(profile.clone(), status)]);
        if let Some((sources, answers)) = limits {
            let _ = reduce(
                &mut state,
                limits_loaded_from(
                    profile,
                    sources,
                    limits_answer(answers),
                    now(),
                ),
            );
        }
        if let Some(prices) = prices {
            let _ = reduce(&mut state, prices_loaded(prices, now()));
        }
        let _ = reduce(
            &mut state,
            spend_loaded(profile, Some(Ok(spend_in_otty())), now()),
        );
        let _ =
            reduce(&mut state, AgentsIntent::SelectProfile(profile.clone()));
        state
    }

    fn good_prices() -> Result<Answer<FetchedPrices>, Vec<SourceFailure>> {
        prices_answer(&[], Some(fetched_prices(now())))
    }

    /// Owner, issue 42: every number of the dashboard has its rows, the
    /// rows of one number together, in the order their walk tries them,
    /// with the number on the first row of the group.
    #[test]
    fn the_table_groups_the_sources_of_each_profile_by_their_number() {
        let limits = i18n::t(Key::AgentsNumberLimits);
        let tokens = i18n::t(Key::AgentsNumberTokens);
        let money = i18n::t(Key::AgentsNumberMoney);
        let prices = i18n::t(Key::AgentsNumberPrices);
        let name = source_name;

        let claude_subscription = shown_with(
            &claude(),
            subscription(),
            Some((
                vec![SourceId::AnthropicUsageApi, SourceId::ClaudeUsageCommand],
                vec![
                    (
                        SourceId::AnthropicUsageApi,
                        Err(SourceError::NotLoggedIn),
                    ),
                    (SourceId::ClaudeUsageCommand, Ok(five_hour_limits(42.0))),
                ],
            )),
            None,
        );
        let codex_subscription = shown_with(
            &codex(),
            connected(),
            Some((
                vec![
                    SourceId::ChatGptUsageApi,
                    SourceId::CodexAppServer,
                    SourceId::LatestCodexSession,
                ],
                vec![(SourceId::ChatGptUsageApi, Ok(five_hour_limits(1.0)))],
            )),
            None,
        );
        let no_admin_key =
            shown_with(&claude(), api_key(), None, Some(good_prices()));
        let mut with_admin_key = shown_with(
            &claude_with_admin_key(),
            api_key(),
            None,
            Some(good_prices()),
        );
        let _ = reduce(
            &mut with_admin_key,
            bill_loaded(&claude_with_admin_key(), Ok(bill_of(12, 3))),
        );
        let down = (
            SourceId::LiteLlmPrices,
            SourceError::Network(String::from("down")),
        );
        let no_kept = (
            SourceId::KeptPrices,
            SourceError::NotAvailable(String::from("no kept price file")),
        );
        let no_prices = shown_with(
            &codex(),
            api_key(),
            None,
            Some(prices_answer(&[down, no_kept], None)),
        );

        assert_eq!(
            table(&dashboard(&claude_subscription)),
            [
                (limits, name(SourceId::AnthropicUsageApi)),
                ("", name(SourceId::ClaudeUsageCommand)),
                (tokens, name(SourceId::SessionFiles)),
            ]
        );
        assert_eq!(
            table(&dashboard(&codex_subscription)),
            [
                (limits, name(SourceId::ChatGptUsageApi)),
                ("", name(SourceId::CodexAppServer)),
                ("", name(SourceId::LatestCodexSession)),
                (tokens, name(SourceId::SessionFiles)),
            ]
        );
        assert_eq!(
            table(&dashboard(&no_admin_key)),
            [
                (money, name(SourceId::AnthropicCostReport)),
                ("", name(SourceId::SessionFilesAtListPrices)),
                (prices, name(SourceId::LiteLlmPrices)),
                ("", name(SourceId::KeptPrices)),
                (tokens, name(SourceId::SessionFiles)),
            ]
        );
        assert_eq!(
            table(&dashboard(&with_admin_key)),
            [
                (money, name(SourceId::AnthropicCostReport)),
                ("", name(SourceId::SessionFilesAtListPrices)),
                (tokens, name(SourceId::SessionFiles)),
            ]
        );
        assert_eq!(
            table(&dashboard(&no_prices)),
            [
                (money, name(SourceId::SessionFilesAtListPrices)),
                (prices, name(SourceId::LiteLlmPrices)),
                ("", name(SourceId::KeptPrices)),
                (tokens, name(SourceId::SessionFiles)),
            ]
        );
        let states: Vec<_> = dashboard(&no_prices)
            .sources
            .iter()
            .map(|row| row.state)
            .collect();
        assert_eq!(
            states,
            [
                i18n::t(Key::AgentsSourceNotAvailable),
                i18n::t(Key::AgentsSourceFailed),
                i18n::t(Key::AgentsSourceNotAvailable),
                i18n::t(Key::AgentsSourceAnswered),
            ]
        );
    }

    #[test]
    fn a_codex_api_key_says_openai_and_has_only_the_estimate() {
        let mut state = with_profiles(&[(codex(), api_key())]);
        state.set_prices(price_table());
        let _ = reduce(
            &mut state,
            spend_loaded(&codex(), Some(Ok(spend_in_otty())), now()),
        );
        let _ = reduce(&mut state, AgentsIntent::SelectProfile(codex()));

        let vm = dashboard(&state);

        let names: Vec<_> = vm.sources.iter().map(|row| row.name).collect();
        assert_eq!(
            names,
            [
                source_name(SourceId::SessionFilesAtListPrices),
                source_name(SourceId::LiteLlmPrices),
                source_name(SourceId::KeptPrices),
                source_name(SourceId::SessionFiles),
            ],
            "the estimate, the two sources of the prices, the tokens"
        );

        assert!(vm.facts.contains(&(
            i18n::t(Key::AgentsFieldPlan),
            String::from("OpenAI API")
        )));
    }

    #[test]
    fn the_limits_answer_tells_the_billing_when_the_files_do_not() {
        let mut state = with_profiles(&[(codex(), connected())]);
        let credits = Limits::new(
            Billing::Credits(Credits::new(Some(3760.0), None, None, None)),
            Some(String::from("Enterprise")),
        );
        let _ = reduce(
            &mut state,
            limits_loaded(
                &codex(),
                limits_answer(vec![(SourceId::CodexAppServer, Ok(credits))]),
                now(),
            ),
        );
        let _ = reduce(&mut state, AgentsIntent::SelectProfile(codex()));

        let vm = dashboard(&state);

        assert!(vm.facts.contains(&(
            i18n::t(Key::AgentsFieldPlan),
            String::from("Enterprise")
        )));
        let Some(BillingViewModel::Limits(limits)) = vm.billing else {
            panic!("the credits show in the limits block");
        };
        assert!(limits.credits.is_some());
    }

    #[test]
    fn a_block_the_agent_does_not_support_is_left_out() {
        let mut state = with_profiles(&[(codex(), connected())]);
        let _ =
            reduce(&mut state, limits_loaded(&codex(), Err(Vec::new()), now()));
        let _ = reduce(&mut state, spend_loaded(&codex(), None, now()));
        let _ = reduce(&mut state, AgentsIntent::SelectProfile(codex()));

        let vm = dashboard(&state);

        assert_eq!(vm.billing, None);
        assert_eq!(vm.tokens, None);
        assert_eq!(vm.top, None);
        assert!(vm.sources.is_empty());
    }

    #[test]
    fn the_last_good_limits_stay_greyed_when_a_read_fails() {
        let mut state = with_profiles(&[(claude(), subscription())]);
        let _ = reduce(
            &mut state,
            limits_loaded(
                &claude(),
                limits_answer(vec![(
                    SourceId::AnthropicUsageApi,
                    Ok(five_hour_limits(42.0)),
                )]),
                now() - Duration::from_secs(600),
            ),
        );
        let _ = reduce(
            &mut state,
            limits_loaded(
                &claude(),
                limits_answer(vec![(
                    SourceId::AnthropicUsageApi,
                    Err(SourceError::Network(String::from("down"))),
                )]),
                now(),
            ),
        );
        let _ = reduce(&mut state, AgentsIntent::SelectProfile(claude()));

        let vm = dashboard(&state);

        let Some(BillingViewModel::Limits(limits)) = vm.billing else {
            panic!("the limits show");
        };
        assert!(limits.is_stale);
        assert_eq!(limits.windows.len(), 1);
        assert_eq!(vm.sources[0].state, i18n::t(Key::AgentsSourceFailed));
    }

    #[test]
    fn the_reset_times_are_written_in_the_time_zone_the_profiles_read_told() {
        let mut state = AgentsState::new(crate::testing::agents(), env());
        let _ = reduce(&mut state, AgentsIntent::Enter);
        let _ = reduce(
            &mut state,
            AgentsIntent::ProfilesLoaded(crate::types::LoadedProfiles {
                profiles: vec![crate::types::LoadedProfile {
                    profile: claude(),
                    status: subscription(),
                }],
                saved: None,
                problem: None,
                utc_offset_seconds: None,
                read_at: std::time::SystemTime::UNIX_EPOCH,
            }),
        );
        let _ = reduce(
            &mut state,
            limits_loaded(
                &claude(),
                limits_answer(vec![(
                    SourceId::AnthropicUsageApi,
                    Ok(five_hour_limits(42.0)),
                )]),
                now(),
            ),
        );
        let _ = reduce(&mut state, AgentsIntent::SelectProfile(claude()));
        let resets = |state: &AgentsState| match dashboard(state).billing {
            Some(BillingViewModel::Limits(limits)) => {
                limits.windows[0].resets.clone()
            },
            _ => None,
        };
        let before = resets(&state);

        let _ = reduce(
            &mut state,
            AgentsIntent::ProfilesLoaded(crate::types::LoadedProfiles {
                profiles: vec![crate::types::LoadedProfile {
                    profile: claude(),
                    status: subscription(),
                }],
                saved: None,
                problem: None,
                utc_offset_seconds: Some(3 * 3600),
                read_at: std::time::SystemTime::UNIX_EPOCH,
            }),
        );

        let in_an_hour = now() + Duration::from_secs(3600);
        assert_eq!(before, Some(resets_text(now(), in_an_hour, None)));
        assert_eq!(
            resets(&state),
            Some(resets_text(now(), in_an_hour, Some(3 * 3600)))
        );
    }

    #[test]
    fn an_added_profile_that_is_not_logged_in_offers_log_in_and_says_why() {
        let work = claude_work();
        let mut state =
            with_profiles(&[(work.clone(), ProfileStatus::NotLoggedIn)]);
        read(&mut state, &work, Vec::new());
        let _ = reduce(&mut state, AgentsIntent::SelectProfile(work));

        let vm = dashboard(&state);

        assert!(vm.can_log_in);
        assert_eq!(vm.hint, Some(i18n::t(Key::AgentsNotLoggedInHint)));
        assert_eq!(vm.billing, None);
        assert_eq!(
            vm.facts,
            vec![
                (
                    i18n::t(Key::AgentsFieldConfigDir),
                    String::from("~/.claude-work")
                ),
                (
                    i18n::t(Key::AgentsFieldSessions),
                    i18n::agents_template(
                        Key::TplAgentsSessionsInProjects,
                        &[("sessions", "0"), ("projects", "0")]
                    )
                ),
            ]
        );
    }

    #[test]
    fn a_failed_read_of_the_sessions_says_why() {
        let mut state = with_profiles(&[(claude(), connected())]);
        let _ = reduce(
            &mut state,
            AgentsIntent::SessionsLoaded(crate::types::LoadedSessions {
                profile: claude(),
                sessions: Err(vec![crate::testing::no_files()]),
                missing_folders: Vec::new(),
            }),
        );
        let _ = reduce(&mut state, AgentsIntent::SelectProfile(claude()));

        let vm = dashboard(&state);

        assert_eq!(
            vm.failure,
            Some(i18n::with_error(
                Key::TplAgentsSessionsFailed,
                &format!(
                    "{}: not available: no projects folder",
                    i18n::t(Key::AgentsSourceSessionFiles)
                )
            ))
        );
    }

    #[test]
    fn a_folder_outside_the_home_folder_keeps_its_full_path() {
        let home = Path::new("/home/user");

        assert_eq!(
            home_relative(Path::new("/home/user/.codex"), home),
            "~/.codex"
        );
        assert_eq!(home_relative(Path::new("/home/user"), home), "~");
        assert_eq!(
            home_relative(Path::new("/data/.codex"), home),
            "/data/.codex"
        );
        assert_eq!(
            home_relative(Path::new("/home/username/.codex"), home),
            "/home/username/.codex"
        );
    }
}

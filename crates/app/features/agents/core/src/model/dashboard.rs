use std::path::Path;
use std::time::SystemTime;

use otty_agents::{
    AgentSession, BillingKind, Profile, ProfileStatus, SourceFailure,
};
use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::components::status_badge::StatusTone;

use super::super::age::age_text;
use super::super::state::{AgentsState, ProfileState};
use super::limits::LimitsViewModel;
use super::money::MoneyViewModel;
use super::sources::{
    BillSourceState, SourceRowViewModel, limit_sources, money_sources,
};
use super::tokens::TokensViewModel;
use super::top_table::{TopTableViewModel, by_model, top_projects};
use super::{failure_reasons, folder_name};

/// The dashboard of the selected profile in the main area (spec Decisions
/// 6 and 9): the header with its buttons, the billing block and the tokens
/// on the left, the facts of the profile, its top table and where the
/// numbers come from on the right. A block the profile's agent does not
/// support is left out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DashboardViewModel {
    /// The profile, sent with the buttons.
    pub(crate) profile: Profile,
    /// The name of the agent's icon.
    pub(crate) icon: &'static str,
    /// The profile name.
    pub(crate) name: String,
    /// The state word of the badge and its color.
    pub(crate) state: String,
    pub(crate) tone: StatusTone,
    /// The plan and the account, or how an API key pays, after the badge.
    pub(crate) subtitle: Option<String>,
    /// Whether Log in shows: the profile is not logged in and its program
    /// was found.
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
    /// Top projects, or the money by model of an API key.
    pub(crate) top: Option<TopTableViewModel>,
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
        let (word, tone) = state_word(shown);
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
                Key::TplAgentsPayPerToken,
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

        let (billing, top, sources) = if is_api_key {
            let money = shown.money();
            (
                spending(shown, now).map(BillingViewModel::Money),
                money.and_then(by_model),
                money_sources(shown.spend(), bill_state(state, shown)),
            )
        } else {
            let top = shown.spend().last_good().and_then(|(spend, _)| {
                top_projects(spend, shown.sessions().unwrap_or_default())
            });
            (
                LimitsViewModel::from_limits(shown.limits(), now)
                    .map(BillingViewModel::Limits),
                top,
                limit_sources(shown.limits()),
            )
        };

        let program = agent
            .map(|agent| agent.launch(profile, None).program().to_string());
        let facts = facts(Facts {
            account: account.clone(),
            plan: plan.clone(),
            config_dir: home_relative(profile.config_dir(), state.env().home()),
            program,
            sessions: shown.sessions(),
            now,
        });
        let subtitle =
            [plan, account].into_iter().flatten().collect::<Vec<_>>();

        Self {
            profile: profile.clone(),
            icon: state.icon_of(profile.agent()),
            name: profile.name().to_string(),
            state: i18n::t(word).to_string(),
            tone,
            subtitle: (!subtitle.is_empty()).then(|| subtitle.join(" · ")),
            can_log_in: *shown.status() == ProfileStatus::NotLoggedIn
                && state.can_start(profile),
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
    program: Option<String>,
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

/// The Spending block of an API key profile: from the bill when its
/// latest read answered, else the estimate (spec Decision 5).
fn spending(shown: &ProfileState, now: SystemTime) -> Option<MoneyViewModel> {
    let budget = shown.profile().monthly_budget();

    match shown.bill().answer() {
        Some(bill) => Some(MoneyViewModel::from_bill(bill, budget)),
        None => MoneyViewModel::from_money(
            shown.money(),
            shown.spend(),
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
/// folder, the program, and once the sessions were read, how many there
/// are in how many projects and when the profile was last used.
fn facts(facts: Facts<'_>) -> Vec<(&'static str, String)> {
    let mut list = Vec::new();
    if let Some(account) = facts.account {
        list.push((i18n::t(Key::AgentsFieldAccount), account));
    }
    if let Some(plan) = facts.plan {
        list.push((i18n::t(Key::AgentsFieldPlan), plan));
    }
    list.push((i18n::t(Key::AgentsFieldConfigDir), facts.config_dir));
    if let Some(program) = facts.program {
        list.push((i18n::t(Key::AgentsFieldProgram), program));
    }

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

/// The state word of a shown profile and its color: while its sessions
/// are read, when that failed, else its status.
fn state_word(shown: &ProfileState) -> (Key, StatusTone) {
    if shown.is_reading() {
        (Key::AgentsStateReading, StatusTone::Neutral)
    } else if shown.failures().is_some() {
        (Key::AgentsStateReadFailed, StatusTone::Error)
    } else {
        status_word(shown.status())
    }
}

/// The state word of a profile and its color.
fn status_word(status: &ProfileStatus) -> (Key, StatusTone) {
    match status {
        ProfileStatus::Connected { .. } => {
            (Key::AgentsStateConnected, StatusTone::Ok)
        },
        ProfileStatus::NotLoggedIn => {
            (Key::AgentsStateNotLoggedIn, StatusTone::Warn)
        },
        ProfileStatus::ProgramNotFound => {
            (Key::AgentsStateProgramNotFound, StatusTone::Error)
        },
        ProfileStatus::ConfigDirNotFound => {
            (Key::AgentsStateConfigDirNotFound, StatusTone::Error)
        },
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Duration;

    use otty_agents::{
        Billing, CalendarDate, Credits, DaySpend, Limits, ModelTokens,
        ProjectTokens, SourceError, Spend, TokenCount,
    };

    use super::*;
    use crate::event::AgentsIntent;
    use crate::reducer::reduce;
    use crate::testing::{
        api_key, bill_loaded, bill_of, claude, claude_with_admin_key,
        claude_work, codex, connected, dollars, five_hour_limits,
        limits_answer, limits_loaded, now, read, session, spend_loaded,
        with_profiles,
    };

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
                    "usage request",
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
        assert_eq!(
            (vm.state.as_str(), vm.tone),
            (i18n::t(Key::AgentsStateConnected), StatusTone::Ok)
        );
        assert_eq!(vm.subtitle.as_deref(), Some("Max · user@example.com"));
        assert!(matches!(vm.billing, Some(BillingViewModel::Limits(_))));
        assert!(vm.tokens.is_some());
        assert!(matches!(vm.top, Some(TopTableViewModel::Projects(_))));
        assert_eq!(vm.sources.len(), 1);
        assert_eq!(vm.sources[0].name, "usage request");
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
                (i18n::t(Key::AgentsFieldProgram), String::from("claude")),
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
    fn an_api_key_profile_pays_per_token_and_shows_its_money_by_model() {
        let mut state = with_profiles(&[(claude(), api_key())]);
        let _ = reduce(
            &mut state,
            spend_loaded(&claude(), Some(Ok(spend_in_otty())), now()),
        );
        let _ = reduce(&mut state, AgentsIntent::SelectProfile(claude()));

        let vm = dashboard(&state);

        let pay_per_token = i18n::agents_template(
            Key::TplAgentsPayPerToken,
            &[("provider", "Anthropic")],
        );
        assert_eq!(vm.subtitle, Some(pay_per_token.clone()));
        assert!(
            vm.facts
                .contains(&(i18n::t(Key::AgentsFieldPlan), pay_per_token))
        );
        assert!(matches!(vm.billing, Some(BillingViewModel::Money(_))));
        assert!(matches!(vm.top, Some(TopTableViewModel::Models(_))));
        assert_eq!(
            vm.sources
                .iter()
                .map(|row| row.name.as_str())
                .collect::<Vec<_>>(),
            [
                i18n::t(Key::AgentsBillSource),
                i18n::t(Key::AgentsMoneySource)
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
        assert_eq!(
            money.note,
            Some(i18n::t(Key::AgentsMoneyFromBill).to_string())
        );
        assert_eq!(money.budget.map(|budget| budget.percent), Some(5));
        let states: Vec<_> = vm
            .sources
            .iter()
            .map(|row| (row.name.as_str(), row.state))
            .collect();
        assert_eq!(
            states,
            [
                (
                    i18n::t(Key::AgentsBillSource),
                    i18n::t(Key::AgentsSourceAnswered)
                ),
                (
                    i18n::t(Key::AgentsMoneySource),
                    i18n::t(Key::AgentsSourceNotNeeded)
                ),
            ]
        );
    }

    #[test]
    fn a_codex_api_key_says_openai_and_has_only_the_estimate() {
        let mut state = with_profiles(&[(codex(), api_key())]);
        let _ = reduce(
            &mut state,
            spend_loaded(&codex(), Some(Ok(spend_in_otty())), now()),
        );
        let _ = reduce(&mut state, AgentsIntent::SelectProfile(codex()));

        let vm = dashboard(&state);

        let names: Vec<_> =
            vm.sources.iter().map(|row| row.name.as_str()).collect();
        assert_eq!(names, [i18n::t(Key::AgentsMoneySource)]);

        assert_eq!(
            vm.subtitle,
            Some(i18n::agents_template(
                Key::TplAgentsPayPerToken,
                &[("provider", "OpenAI")]
            ))
        );
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
                limits_answer(vec![("app-server", Ok(credits))]),
                now(),
            ),
        );
        let _ = reduce(&mut state, AgentsIntent::SelectProfile(codex()));

        let vm = dashboard(&state);

        assert_eq!(vm.subtitle.as_deref(), Some("Enterprise"));
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
                    "usage request",
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
                    "usage request",
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
    fn an_added_profile_that_is_not_logged_in_offers_log_in_and_says_why() {
        let work = claude_work().with_program(PathBuf::from("/opt/claude"));
        let mut state =
            with_profiles(&[(work.clone(), ProfileStatus::NotLoggedIn)]);
        read(&mut state, &work, Vec::new());
        let _ = reduce(&mut state, AgentsIntent::SelectProfile(work));

        let vm = dashboard(&state);

        assert!(vm.can_log_in);
        assert_eq!(
            (vm.state.as_str(), vm.tone),
            (i18n::t(Key::AgentsStateNotLoggedIn), StatusTone::Warn)
        );
        assert_eq!(vm.hint, Some(i18n::t(Key::AgentsNotLoggedInHint)));
        assert_eq!(vm.subtitle, None);
        assert_eq!(vm.billing, None);
        assert_eq!(
            vm.facts,
            vec![
                (
                    i18n::t(Key::AgentsFieldConfigDir),
                    String::from("~/.claude-work")
                ),
                (
                    i18n::t(Key::AgentsFieldProgram),
                    String::from("/opt/claude")
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
            (vm.state.as_str(), vm.tone),
            (i18n::t(Key::AgentsStateReadFailed), StatusTone::Error)
        );
        assert_eq!(
            vm.failure,
            Some(i18n::with_error(
                Key::TplAgentsSessionsFailed,
                "files: not available: no projects folder"
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

//! Helpers that the tests of several files share: made-up profiles and
//! sessions, and Agents mode with them read.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use jiff::tz::TimeZone;
use otty_agents::{
    Account, AccountIdentity, AccountKind, AccountLogin, AdminKeyRef,
    AgentCommand, AgentEnv, AgentId, AgentSession, Answer, Bill, Billing,
    DayCost, DaySpend, Dollars, FetchedPrices, FoundIn, LimitSource,
    LimitWindow, Limits, ModelPrice, ModelTokens, PriceTable, Profile,
    ProfileStatus, Provider, SavedProfiles, SessionId, SourceError,
    SourceFailure, SourceId, Spend, TokenCount, TokenPrices, Unavailable,
    UsageWindows, WindowId,
};
use otty_ui_kit::components::context_menu::MenuPlace;

use crate::event::{AgentsIntent, AgentsOutcome};
use crate::menus::AgentsMenu;
use crate::model::{
    UsageBarViewModel, UsageBlockViewModel, UsageTableRowViewModel,
};
use crate::reducer::reduce;
use crate::state::AgentsState;
use crate::types::{
    AgentList, FolderAsker, FolderQuery, LimitsRead, LoadedBill, LoadedFolder,
    LoadedLimits, LoadedPrices, LoadedProfile, LoadedProfiles, LoadedSessions,
    LoadedSpend, SessionsRead, SpendCount,
};
use crate::usage::{UsageMeasure, UsagePeriod, UsageTab};

pub(crate) const CLAUDE: AgentId = AgentId::new("claude-code");
pub(crate) const CODEX: AgentId = AgentId::new("codex");
/// Position of the Title column of the sessions table, which production
/// code reaches only as the fallback of the sort.
pub(crate) const TITLE_COLUMN: usize = 0;

/// A session source that never answers.
struct NoFiles;

impl otty_agents::SessionSource for NoFiles {
    fn id(&self) -> SourceId {
        SourceId::SessionFiles
    }

    fn sessions(
        &self,
        _profile: &Profile,
    ) -> Result<Vec<AgentSession>, SourceError> {
        Err(SourceError::NotAvailable(Unavailable::NoSessionFiles))
    }
}

/// A limit source that gives the same answer each time.
struct FixedLimits {
    id: SourceId,
    answer: Result<Limits, SourceError>,
}

impl LimitSource for FixedLimits {
    fn id(&self) -> SourceId {
        self.id
    }

    fn limits(&self, _profile: &Profile) -> Result<Limits, SourceError> {
        self.answer.clone()
    }
}

/// The version of the agent the session files of the made-up profiles
/// name, so no read asks a program for it.
pub(crate) const SESSION_VERSION: &str = "1.0.0";

/// The time the tests look at the sessions from.
pub(crate) fn now() -> SystemTime {
    at(1_000_000)
}

/// The time `seconds` after 1970.
pub(crate) fn at(seconds: u64) -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(seconds)
}

/// Moscow: three hours east of UTC, with no summer time.
pub(crate) fn moscow() -> TimeZone {
    TimeZone::fixed(jiff::tz::offset(3))
}

/// Berlin: UTC+1 in winter and UTC+2 in summer, from the last Sunday of
/// March to the last Sunday of October. It is made from its POSIX rule, so
/// no test depends on the time zone files of the machine.
pub(crate) fn berlin() -> TimeZone {
    TimeZone::posix("CET-1CEST,M3.5.0,M10.5.0/3").expect("the rule of Berlin")
}

/// The agents OTTY knows: Claude Code and Codex.
pub(crate) fn agents() -> AgentList {
    Arc::from(otty_agents::known_agents())
}

/// The environment of the tests: the home folder `/home/user`, no
/// variable of an agent.
pub(crate) fn env() -> AgentEnv {
    AgentEnv::new(PathBuf::from("/home/user"))
}

/// The default Claude Code profile in `/home/user/.claude`.
pub(crate) fn claude() -> Profile {
    Profile::default_for(
        CLAUDE,
        "Claude Code",
        PathBuf::from("/home/user/.claude"),
    )
}

/// A Claude Code profile the user added, in `/home/user/.claude-work`.
pub(crate) fn claude_work() -> Profile {
    Profile::added(
        CLAUDE,
        "Claude Code · Work",
        PathBuf::from("/home/user/.claude-work"),
    )
}

/// The default Codex profile in `/home/user/.codex`.
pub(crate) fn codex() -> Profile {
    Profile::default_for(CODEX, "Codex", PathBuf::from("/home/user/.codex"))
}

/// A Codex profile the user added, in `/home/user/.codex-work`.
pub(crate) fn codex_work() -> Profile {
    Profile::added(
        CODEX,
        "Codex · Work",
        PathBuf::from("/home/user/.codex-work"),
    )
}

/// The status of a profile that can be used.
pub(crate) fn connected() -> ProfileStatus {
    ProfileStatus::Connected {
        account: None,
        plan: None,
        billing: None,
    }
}

/// The status of a profile that pays per token with an API key.
pub(crate) fn api_key() -> ProfileStatus {
    ProfileStatus::Connected {
        account: None,
        plan: None,
        billing: Some(otty_agents::BillingKind::ApiKey),
    }
}

/// A session `id` titled after it, in `folder`, changed at `seconds`.
pub(crate) fn session(id: &str, folder: &str, seconds: u64) -> AgentSession {
    AgentSession::new(
        SessionId::new(id),
        PathBuf::from(folder),
        format!("Title of {id}"),
        at(seconds),
    )
}

/// A session `id` titled `title` in `/work/otty`, changed at `seconds`.
pub(crate) fn titled_session(
    id: &str,
    title: &str,
    seconds: u64,
) -> AgentSession {
    AgentSession::new(
        SessionId::new(id),
        PathBuf::from("/work/otty"),
        title,
        at(seconds),
    )
}

/// A failure of the session files of a profile.
pub(crate) fn no_files() -> SourceFailure {
    let sources: Vec<Box<dyn otty_agents::SessionSource>> =
        vec![Box::new(NoFiles)];
    let profile = claude();

    match otty_agents::read_sessions(&sources, &profile) {
        Err(failures) => failures.into_iter().next().expect("one failure"),
        Ok(_) => unreachable!("the source never answers"),
    }
}

/// Agents mode entered, with `profiles` read.
pub(crate) fn with_profiles(
    profiles: &[(Profile, ProfileStatus)],
) -> AgentsState {
    with_saved_profiles(profiles, SavedProfiles::default())
}

/// Agents mode entered, with `profiles` read from `saved`.
pub(crate) fn with_saved_profiles(
    profiles: &[(Profile, ProfileStatus)],
    saved: SavedProfiles,
) -> AgentsState {
    let mut state = AgentsState::new(agents(), env());
    let _ = reduce(&mut state, AgentsIntent::Enter);
    let _ = reduce(&mut state, profiles_loaded_from(profiles, saved));

    state
}

/// The answer of a profiles read that found `profiles`.
pub(crate) fn profiles_loaded(
    profiles: &[(Profile, ProfileStatus)],
) -> AgentsIntent {
    profiles_loaded_from(profiles, SavedProfiles::default())
}

/// The answer of a profiles read that found `profiles` and `saved` in
/// `agents.json`.
pub(crate) fn profiles_loaded_from(
    profiles: &[(Profile, ProfileStatus)],
    saved: SavedProfiles,
) -> AgentsIntent {
    AgentsIntent::ProfilesLoaded(loaded_profiles(profiles, saved))
}

/// The answer of a profiles read that found `profiles` just now.
pub(crate) fn fresh_profiles_loaded(
    profiles: &[(Profile, ProfileStatus)],
) -> AgentsIntent {
    AgentsIntent::ProfilesLoaded(LoadedProfiles {
        read_at: SystemTime::now(),
        ..loaded_profiles(profiles, SavedProfiles::default())
    })
}

/// The answer of a profiles read that found `profiles` and could not read
/// `agents.json` for the reason `problem`, when there is one.
pub(crate) fn profiles_loaded_with_problem(
    profiles: &[(Profile, ProfileStatus)],
    problem: Option<&str>,
) -> AgentsIntent {
    AgentsIntent::ProfilesLoaded(LoadedProfiles {
        problem: problem.map(str::to_string),
        ..loaded_profiles(profiles, SavedProfiles::default())
    })
}

/// The answer of a sessions read of `profile` that found `sessions`.
pub(crate) fn sessions_loaded(
    profile: &Profile,
    sessions: Vec<AgentSession>,
) -> AgentsIntent {
    AgentsIntent::SessionsLoaded(LoadedSessions {
        profile: profile.clone(),
        sessions: SessionsRead::Read(sessions),
        missing_folders: Vec::new(),
    })
}

/// Read `sessions` of `profile` into `state`.
pub(crate) fn read(
    state: &mut AgentsState,
    profile: &Profile,
    sessions: Vec<AgentSession>,
) {
    let _ = reduce(state, sessions_loaded(profile, sessions));
}

/// The default Claude Code and Codex profiles, connected, with sessions in
/// `/work/otty` from both and in `/work/site` from Claude Code.
pub(crate) fn two_agents_with_sessions() -> AgentsState {
    let mut state =
        with_profiles(&[(claude(), connected()), (codex(), connected())]);
    read(
        &mut state,
        &claude(),
        vec![
            session("c1", "/work/otty", 100),
            session("c2", "/work/site", 300),
        ],
    );
    read(&mut state, &codex(), vec![session("x1", "/work/otty", 200)]);

    state
}

/// The folders of the projects of `state`, in their order.
pub(crate) fn folders(state: &AgentsState) -> Vec<&Path> {
    state
        .profiles()
        .projects()
        .iter()
        .map(|project| project.folder())
        .collect()
}

/// Limits with one window, "5 hours", `percent` used, that resets an hour
/// after [`now`].
pub(crate) fn five_hour_limits(percent: f64) -> Limits {
    let resets_at = now() + Duration::from_secs(3600);

    let window = LimitWindow::new(WindowId::hours(5), percent, Some(resets_at));

    Limits::new(
        Billing::UsageWindows(UsageWindows::new(vec![window], None)),
        None,
    )
}

/// What the walk over `sources`, each an id and its answer, gives: the
/// answer of the first one that answers, or every failure.
pub(crate) fn limits_answer(
    sources: Vec<(SourceId, Result<Limits, SourceError>)>,
) -> LimitsRead {
    let sources: Vec<Box<dyn LimitSource>> = sources
        .into_iter()
        .map(|(id, answer)| {
            Box::new(FixedLimits { id, answer }) as Box<dyn LimitSource>
        })
        .collect();

    LimitsRead::from_walk(otty_agents::read_limits(&sources, &claude()))
}

/// The answer of a limits read of `profile` that ended at `read_at`, whose
/// agent has only the sources the walk asked.
pub(crate) fn limits_loaded(
    profile: &Profile,
    limits: LimitsRead,
    read_at: SystemTime,
) -> AgentsIntent {
    let answered = match &limits {
        LimitsRead::Answered(answer) => Some(answer.source()),
        _ => None,
    };
    let sources = limits
        .failures()
        .iter()
        .map(SourceFailure::source)
        .chain(answered)
        .collect();

    limits_loaded_from(profile, sources, limits, read_at)
}

/// The answer of a limits read of `profile` over `sources`, best first,
/// that ended at `read_at`.
pub(crate) fn limits_loaded_from(
    profile: &Profile,
    sources: Vec<SourceId>,
    limits: LimitsRead,
    read_at: SystemTime,
) -> AgentsIntent {
    AgentsIntent::LimitsLoaded(LoadedLimits {
        profile: profile.clone(),
        limits,
        sources,
        read_at,
    })
}

/// Tokens spent: `today` input tokens today, 2026-11-16, and `earlier`
/// input tokens five days before.
pub(crate) fn spend_of(today: u64, earlier: u64) -> Spend {
    let day = |day: i8, input: u64| {
        let tokens = TokenCount::new(input, 0, 0);
        DaySpend::new(
            jiff::civil::date(2026, 11, day),
            vec![ModelTokens::new(Some("gpt-5"), tokens, 1)],
        )
    };

    Spend::new(vec![day(11, earlier), day(16, today)], Vec::new())
}

/// The answer of a count of the tokens of `profile` that ended at
/// `counted_at`.
pub(crate) fn spend_loaded(
    profile: &Profile,
    spend: SpendCount,
    counted_at: SystemTime,
) -> AgentsIntent {
    AgentsIntent::SpendLoaded(LoadedSpend {
        profile: profile.clone(),
        spend,
        counted_at,
    })
}

/// A made-up Anthropic Console organization, paid per token, found in
/// the default Claude Code folder.
pub(crate) fn organization() -> Account {
    Account::new(
        Provider::Anthropic,
        AccountIdentity::OrganizationId(String::from("made-up-org")),
        AccountLogin::Email(String::from("dev@example.com")),
        AccountKind::Money,
        FoundIn::ProfileFolder(PathBuf::from("/home/user/.claude")),
    )
}

/// A made-up ChatGPT login with a subscription, found in the Codex
/// folders.
pub(crate) fn chatgpt_login() -> Account {
    Account::new(
        Provider::OpenAI,
        AccountIdentity::AccountId(String::from("made-up-account")),
        AccountLogin::Email(String::from("you@example.com")),
        AccountKind::Limits,
        FoundIn::ProfileFolder(PathBuf::from("/home/user/.codex")),
    )
}

/// The [`chatgpt_login`] as found in the Codex folder `folder`.
pub(crate) fn chatgpt_login_in(folder: &str) -> Account {
    let login = chatgpt_login();

    Account::new(
        login.provider(),
        login.identity().clone(),
        login.login().clone(),
        login.kind(),
        FoundIn::ProfileFolder(PathBuf::from(folder)),
    )
}

/// Where the admin key of [`organization`] is kept: the variable
/// `ANTHROPIC_ADMIN_KEY`.
pub(crate) fn admin_key() -> AdminKeyRef {
    AdminKeyRef::EnvVar(String::from("ANTHROPIC_ADMIN_KEY"))
}

/// What `agents.json` holds with the [`admin_key`] of the
/// [`organization`].
pub(crate) fn saved_with_admin_key() -> SavedProfiles {
    SavedProfiles::default()
        .with_admin_key_of(&organization(), Some(admin_key()))
}

/// Agents mode entered, with `profiles` read from `saved`, each with the
/// accounts found in it.
pub(crate) fn with_accounts(
    profiles: &[(Profile, ProfileStatus, Vec<Account>)],
    saved: SavedProfiles,
) -> AgentsState {
    let mut state = AgentsState::new(agents(), env());
    let _ = reduce(&mut state, AgentsIntent::Enter);
    let _ = reduce(&mut state, profiles_with_accounts_loaded(profiles, saved));

    state
}

/// The answer of a profiles read that found `profiles`, each with the
/// accounts found in it, and `saved` in `agents.json`.
pub(crate) fn profiles_with_accounts_loaded(
    profiles: &[(Profile, ProfileStatus, Vec<Account>)],
    saved: SavedProfiles,
) -> AgentsIntent {
    let shown: Vec<_> = profiles
        .iter()
        .map(|(profile, status, _)| (profile.clone(), status.clone()))
        .collect();
    let mut loaded = loaded_profiles(&shown, saved);
    for (entry, (_, _, accounts)) in loaded.profiles.iter_mut().zip(profiles) {
        entry.accounts = accounts.clone();
    }

    AgentsIntent::ProfilesLoaded(loaded)
}

/// The answer of a profiles read that found `profiles`, each with the
/// accounts found in it, the profiles the user hid, `hidden`, which were
/// not read, and `saved` in `agents.json`.
pub(crate) fn profiles_with_hidden_loaded(
    profiles: &[(Profile, ProfileStatus, Vec<Account>)],
    hidden: &[Profile],
    saved: SavedProfiles,
) -> AgentsIntent {
    let AgentsIntent::ProfilesLoaded(mut loaded) =
        profiles_with_accounts_loaded(profiles, saved)
    else {
        unreachable!("a profiles read is made");
    };
    loaded.hidden_profiles = hidden.to_vec();

    AgentsIntent::ProfilesLoaded(loaded)
}

/// `loaded`, the answer of a profiles read, with `versions` as the
/// versions of the agents of its profiles, in their order.
pub(crate) fn with_versions(
    loaded: AgentsIntent,
    versions: &[Option<&str>],
) -> AgentsIntent {
    let AgentsIntent::ProfilesLoaded(mut loaded) = loaded else {
        panic!("a profiles read is given");
    };
    for (entry, version) in loaded.profiles.iter_mut().zip(versions) {
        entry.version = version.map(String::from);
    }

    AgentsIntent::ProfilesLoaded(loaded)
}

/// The default Claude Code profile paying per token through the
/// [`organization`], with its [`admin_key`] saved, as a profiles read
/// found it.
pub(crate) fn billed_claude_loaded() -> AgentsIntent {
    profiles_with_accounts_loaded(
        &[(claude(), api_key(), vec![organization()])],
        saved_with_admin_key(),
    )
}

/// Agents mode entered, with [`billed_claude_loaded`] read.
pub(crate) fn with_billed_claude() -> AgentsState {
    let mut state = AgentsState::new(agents(), env());
    let _ = reduce(&mut state, AgentsIntent::Enter);
    let _ = reduce(&mut state, billed_claude_loaded());

    state
}

/// The read of the bill of the [`organization`] through `profile`, with
/// its [`admin_key`].
pub(crate) fn load_bill(profile: &Profile) -> AgentsOutcome {
    AgentsOutcome::LoadBill {
        profile: profile.clone(),
        admin_key: admin_key(),
    }
}

/// `amount` whole dollars.
pub(crate) fn dollars(amount: u64) -> Dollars {
    Dollars::from_micro_dollars(amount * 1_000_000)
}

/// A bill of `today` dollars today, 2026-11-16, and `earlier` dollars
/// five days before.
pub(crate) fn bill_of(today: u64, earlier: u64) -> Bill {
    Bill::new(vec![
        DayCost::new(jiff::civil::date(2026, 11, 11), dollars(earlier)),
        DayCost::new(jiff::civil::date(2026, 11, 16), dollars(today)),
    ])
}

/// The answer of a read of the bill through `profile` with the
/// [`admin_key`], that ended at [`now`].
pub(crate) fn bill_loaded(
    profile: &Profile,
    bill: Result<Bill, SourceError>,
) -> AgentsIntent {
    AgentsIntent::BillLoaded(LoadedBill {
        profile: profile.clone(),
        admin_key: admin_key(),
        bill,
        read_at: now(),
    })
}

/// Made-up list prices per million tokens: `gpt-5` costs $1.25 per
/// million input tokens and $10 per million output tokens,
/// `claude-sonnet-4-5` $3 and $15.
pub(crate) fn price_table() -> PriceTable {
    let gpt_5 = TokenPrices {
        input: Dollars::from_micro_dollars(1_250_000),
        cached: Dollars::from_micro_dollars(125_000),
        cache_written: Dollars::from_micro_dollars(1_250_000),
        output: Dollars::from_micro_dollars(10_000_000),
    };
    let sonnet = TokenPrices {
        input: Dollars::from_micro_dollars(3_000_000),
        cached: Dollars::from_micro_dollars(300_000),
        cache_written: Dollars::from_micro_dollars(3_750_000),
        output: Dollars::from_micro_dollars(15_000_000),
    };

    PriceTable::new(vec![
        ModelPrice::new("gpt-5", gpt_5),
        ModelPrice::new("claude-sonnet-4-5", sonnet),
    ])
}

/// The made-up prices of [`price_table`], fetched at `fetched_at`.
pub(crate) fn fetched_prices(fetched_at: SystemTime) -> FetchedPrices {
    FetchedPrices::new(price_table(), fetched_at)
}

/// What a walk over the price sources gave: the sources of `failures`
/// failed in order, then `prices` answered, from LiteLLM when nothing
/// failed before, else from the kept file.
pub(crate) fn prices_answer(
    failures: &[(SourceId, SourceError)],
    prices: Option<FetchedPrices>,
) -> Result<Answer<FetchedPrices>, Vec<SourceFailure>> {
    let answering = if failures.is_empty() {
        SourceId::LiteLlmPrices
    } else {
        SourceId::KeptPrices
    };
    let failed = failures
        .iter()
        .map(|(source, error)| (*source, Err(error.clone())));
    let answered = prices.map(|prices| (answering, Ok(prices)));

    otty_agents::fixtures::walked(failed.chain(answered))
}

/// The answer of a read of the prices that ended at `read_at`.
pub(crate) fn prices_loaded(
    prices: Result<Answer<FetchedPrices>, Vec<SourceFailure>>,
    read_at: SystemTime,
) -> AgentsIntent {
    AgentsIntent::PricesLoaded(LoadedPrices { prices, read_at })
}

/// Estimate the money in `state` at `prices`, as if LiteLLM gave them
/// just now.
pub(crate) fn set_prices(state: &mut AgentsState, prices: PriceTable) {
    let fetched = FetchedPrices::new(prices, SystemTime::now());
    let answer = std::iter::once((SourceId::LiteLlmPrices, Ok(fetched)));

    state.profiles_mut().apply_prices(LoadedPrices {
        prices: otty_agents::fixtures::walked(answer),
        read_at: SystemTime::now(),
    });
}

/// What the folder field of `asker` asks about the typed `folder`: the
/// folders in `list_in` whose name starts with `prefix`.
pub(crate) fn folder_query(
    asker: FolderAsker,
    folder: &str,
    list_in: &str,
    prefix: &str,
) -> FolderQuery {
    FolderQuery {
        asker,
        folder: PathBuf::from(folder),
        list_in: PathBuf::from(list_in),
        prefix: String::from(prefix),
    }
}

/// The answer about `query`: whether its folder is there, and the folders
/// it suggests.
pub(crate) fn loaded_folder(
    query: FolderQuery,
    is_folder: bool,
    suggestions: &[&str],
) -> LoadedFolder {
    LoadedFolder {
        query,
        is_folder,
        suggestions: suggestions.iter().map(PathBuf::from).collect(),
    }
}

/// The answer about `query` as it reaches Agents mode.
pub(crate) fn folder_looked_up(
    query: FolderQuery,
    is_folder: bool,
    suggestions: &[&str],
) -> AgentsIntent {
    AgentsIntent::FolderLookedUp(loaded_folder(query, is_folder, suggestions))
}

/// Select the project of `folder`.
pub(crate) fn select_project(state: &mut AgentsState, folder: &str) {
    let _ = reduce(state, AgentsIntent::SelectProject(PathBuf::from(folder)));
}

/// The menu that is open, without where it opened.
pub(crate) fn open_menu(state: &AgentsState) -> Option<AgentsMenu> {
    state.menu().map(|open| open.menu().clone())
}

/// The terminal tab titled `title` that runs `command` in `folder`.
pub(crate) fn open_tab(
    title: &str,
    folder: &str,
    command: AgentCommand,
) -> AgentsOutcome {
    AgentsOutcome::OpenTab {
        title: String::from(title),
        folder: PathBuf::from(folder),
        command,
    }
}

/// A press and a release of the right mouse button where the pointer is.
pub(crate) fn right_click() -> [iced::Event; 2] {
    use iced::mouse::{Button, Event};

    [
        iced::Event::Mouse(Event::ButtonPressed(Button::Right)),
        iced::Event::Mouse(Event::ButtonReleased(Button::Right)),
    ]
}

/// What a profiles read long ago found: `profiles`, each with the version
/// [`SESSION_VERSION`] from its sessions, and `saved` in `agents.json`.
fn loaded_profiles(
    profiles: &[(Profile, ProfileStatus)],
    saved: SavedProfiles,
) -> LoadedProfiles {
    LoadedProfiles {
        profiles: profiles
            .iter()
            .map(|(profile, status)| LoadedProfile {
                profile: profile.clone(),
                status: status.clone(),
                accounts: Vec::new(),
                version: Some(String::from(SESSION_VERSION)),
            })
            .collect(),
        hidden_profiles: Vec::new(),
        saved: Some(saved),
        problem: None,
        time_zone: Some(TimeZone::UTC),
        read_at: at(0),
    }
}

/// The pointer where the menus of the tests open.
pub(crate) fn menu_place() -> MenuPlace {
    MenuPlace::new(
        iced::Point::new(40.0, 200.0),
        iced::Size::new(1000.0, 700.0),
    )
}

/// The menu of `account` opened at [`menu_place`], as a right click on
/// its row or the "⋯" of its page opens it.
pub(crate) fn open_account_menu(state: &mut AgentsState, account: Account) {
    state.set_pointer(menu_place());
    let _ = reduce(state, AgentsIntent::OpenAccountMenu(account));
}

/// The menu of `profile` opened at [`menu_place`].
pub(crate) fn open_profile_menu(state: &mut AgentsState, profile: Profile) {
    state.set_pointer(menu_place());
    let _ = reduce(state, AgentsIntent::OpenProfileMenu(profile));
}

/// The outcome that writes `saved` to `agents.json`.
pub(crate) fn save(saved: SavedProfiles) -> AgentsOutcome {
    AgentsOutcome::SaveProfiles {
        saved,
        create_dir: None,
    }
}

/// A usage block of 7 days by model with two models, as a page shows it.
pub(crate) fn usage_block() -> UsageBlockViewModel {
    UsageBlockViewModel {
        line: Some(String::from("191 in 7 days")),
        period: UsagePeriod::SevenDays,
        measure: UsageMeasure::Tokens,
        tab: UsageTab::ByModel,
        tabs: UsageTab::PROFILE_PAGE.to_vec(),
        account_reason: None,
        bars: (10..=16)
            .map(|day| UsageBarViewModel {
                value: day,
                lit: None,
                tip: format!("Nov {day}: {day}"),
            })
            .collect(),
        current: Some(6),
        axis: Some((String::from("Nov 10"), String::from("Today"))),
        rows: vec![
            UsageTableRowViewModel {
                name: String::from("gpt-5.5-codex"),
                value: String::from("100"),
                share: String::from("52 %"),
            },
            UsageTableRowViewModel {
                name: String::from("gpt-5.4"),
                value: String::from("91"),
                share: String::from("48 %"),
            },
        ],
        picked: None,
        is_stale: false,
        notes: Vec::new(),
        failure: None,
        lists: None,
    }
}

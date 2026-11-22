//! What OTTY knows about coding agents and the providers they pay
//! through, with no agent or provider named in its code: the parts of
//! plugins read them (spec 2026-11-19-agent-plugins, Decision 1).
//!
//! Each agent implements [`Agent`]: it says how to find its default
//! [`Profile`], whether a profile is connected, and how to start or continue
//! a session. The data an agent can read comes from small sources:
//! [`SessionSource`] and [`SpendCounter`]. An agent gives its sources of
//! sessions best first, and [`read_sessions`] tries them in order until
//! one answers. A source that asks the agent's own program starts it with
//! the [`StartProgram`] the app gives, and a source that reads the
//! program's screen starts it with the [`StartTerminal`] the app gives.
//! [`projects`] groups the sessions of all profiles by their folder.
//!
//! [`profiles`] gives the list of profiles to show, from the default
//! profile of each agent, the [`SavedProfiles`] of `agents.json` and the
//! extra folders of each agent found in the home folder. An agent tells
//! its version from its session files with [`Agent::version`], else the app
//! asks its program and reads the answer with [`version_in_output`]. The
//! dialog that adds a profile gets its folder from [`proposed_config_dir`]
//! and [`create_config_dir`], and checks what the user typed with
//! [`check_profile_name`] and [`check_profile_config_dir`].
//!
//! A [`SpendCounter`] counts the tokens a profile spent from its session
//! files into [`Spend`]: by day, project and model, by the days of the
//! user's time zone that the app gives as a [`LocalNow`]. For a profile
//! billed by an API key, [`MoneyEstimate`] turns those tokens into an
//! estimate of the money, at the list prices of a [`PriceTable`] that a
//! [`PriceSource`] reads from the LiteLLM price table, fetched at most once
//! a day, or from its kept copy: each model at the price of who served it,
//! else at its maker's ([`PriceTable::price_of`]). Each reader counts the records it saw and
//! understood; when the newest session files are not understood, the count
//! fails with [`SourceError::FormatChanged`], whose [`FormatChange`] names
//! the agent's version and gives a report with no value of a record.
//!
//! An agent's [`StatsSource`] makes the [`SessionStats`] of one session and
//! the [`ProjectStats`] of one project folder over the whole history: a
//! plugin part gives the answers as rows, each naming its session and
//! project, and OTTY sums them with the same dedupe as the totals;
//! [`MoneyTotal::of_models`] prices their tokens.
//!
//! Limits, credits and bills belong to an [`Account`]: a login or a key at
//! a provider, named by a [`ProviderRef`], found in the files of each
//! profile by
//! [`Agent::accounts`] and made one per account by [`merge_accounts`]. The
//! app reads them once per account, with the files of one profile it was
//! found in. [`read_bill`] reads the real [`Bill`] of an organization
//! through the provider part that serves it. [`AccountMoney`] is the money
//! of an account: its bill, else an estimate over the tokens of the
//! profiles that pay through it.
//!
//! Every account is read by the provider part of each plugin that serves
//! it, and each plugin's reading is an account of its own
//! ([`Account::read_by`]; issue 29): [`read_account_numbers`] reads the
//! [`Balance`] of a key, the [`Limits`] of a subscription login or the
//! [`Bill`] of an account the user added, with a key the user added read
//! again from its [`KeyPlace`] and dropped right after the request, or
//! the key of a login an agent part found, and tells the part every
//! profile the account was found in, so it may also ask the agent there.
//! Where a part reads the numbers from is its business; with no way that
//! works it answers nothing. An account the user added by hand is an
//! [`Account::added`], saved in [`SavedProfiles`] by the file or the
//! variable of its key only; with no part that serves its provider it is
//! listed and not read.
//!
//! What the user must do for an agent or a provider comes from its part
//! as a [`PartNeed`], in the language the host tells the part: a profile
//! nobody is logged in with ([`ProfileStatus::NotLoggedIn`]), or a read
//! that waits for the user ([`SourceError::NeedsUser`]). A login a
//! provider part refused is the need of the agent part that found it in
//! the profile; when that part gives none, the read fails with
//! [`SourceError::LoginRefused`]. A key a part asks
//! for is a [`GivenKey`]: OTTY keeps only its [`KeyRef`], the file or the
//! variable that holds it, saved for the account in [`SavedProfiles`],
//! and gives it to the part's next reads under its slot.
//!
//! [`read_bill`], [`read_session_stats`] and [`read_project_stats`] find
//! the source of an agent with [`agent_of`] and read through it;
//! [`profile_status`] tells the state of a profile.
//!
//! The [`Registry`] is the one list of the agents and providers OTTY knows
//! right now: names, icons, usage pages and hosts come only from it (spec
//! 2026-11-19-agent-plugins, "Links in the interface"). The parts of the
//! plugins that are on fill it ([`Registry::of_readers`]). An account whose
//! provider no entry serves stays known by the host of its key.
//!
//! A [`PluginAgent`] is an agent part of a plugin as an [`Agent`]: OTTY
//! walks the spend places it names, reads only the files that changed,
//! counts an answer once and judges "format changed" from what the part
//! saw. A [`PluginProvider`] reads the numbers of the accounts of its own
//! provider id or hosts, with a handle to the key an agent part found. A
//! trap or a missed deadline fails that read only, as a `PluginFailure`
//! of `otty-plugin-host`, which calls the parts of any domain; three in a
//! row pause the part (`PartHealth`). This crate turns only what the
//! `agents.agent` and `agents.provider` parts answer into its own types.
//!
//! The crate knows nothing about the window and reads no environment
//! itself: the app passes in an [`AgentEnv`], built from the
//! [`ConfigFolder`] of each agent, and runs the blocking calls on a
//! background thread.
//!
//! The `fixtures` feature adds the module `fixtures`: values for the tests
//! of the crates that use this one.

mod account;
mod account_money;
mod account_numbers;
mod agent;
mod balance;
mod bill;
mod command;
mod config_dir;
mod dollars;
mod env;
mod error;
mod file_memory;
mod file_stamp;
#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
mod given_key;
mod hidden_terminal;
mod https;
mod key_fingerprint;
mod limits;
mod litellm;
mod local_time;
mod money_estimate;
mod part_need;
mod percent;
mod plugin_agent;
mod plugin_bill;
mod plugin_places;
mod plugin_provider;
mod plugin_sources;
mod plugin_spend;
mod plugin_values;
mod price_source;
mod price_table;
mod profile;
mod profile_check;
mod profile_list;
mod program;
mod program_version;
mod project;
mod provider;
mod read_failure;
mod record_format;
mod refused_login;
mod registry;
mod saved_profiles;
mod session;
mod session_files;
mod source_id;
mod spend;
mod spend_files;
mod spend_tally;
mod stats;
mod stats_tally;
#[cfg(test)]
mod testing;
mod timestamp;
mod token_count;
mod walk;

pub use crate::account::{
    Account, AccountIdentity, AccountKind, AccountLogin, FoundIn,
    merge_accounts,
};
pub use crate::account_money::AccountMoney;
pub use crate::account_numbers::{
    AccountNumbers, KeyPlace, own_reader_of, read_account_numbers,
};
pub use crate::agent::{Agent, AgentEntry, AgentId, ConfigFolder, agent_of};
pub use crate::balance::{Amount, Balance, KeyLimit};
pub use crate::bill::{Bill, DayCost, read_bill};
pub use crate::command::AgentCommand;
pub use crate::config_dir::{
    CreateConfigDirError, create_config_dir, proposed_config_dir,
};
pub use crate::dollars::Dollars;
pub use crate::env::AgentEnv;
pub use crate::error::{SourceError, Unavailable, Unexpected};
pub use crate::given_key::{GivenKey, KeyRef, KeySecret};
pub use crate::hidden_terminal::{
    RunningTerminal, ScreenOutput, StartTerminal,
};
pub use crate::key_fingerprint::KeyFingerprint;
pub use crate::limits::{
    Billing, BillingKind, Credits, ExtraUsage, LimitWindow, Limits,
    UsageWindows, WindowId,
};
pub use crate::local_time::LocalNow;
pub use crate::money_estimate::{MoneyEstimate, MoneyTotal};
pub use crate::part_need::{KeyAsk, PartNeed, PartNeedAction};
pub use crate::percent::Percent;
pub use crate::plugin_agent::PluginAgent;
pub use crate::plugin_provider::PluginProvider;
pub use crate::price_source::{FetchedPrices, PriceSource};
pub use crate::price_table::{ModelPrice, PriceTable, TokenPrices};
pub use crate::profile::{Profile, ProfileStatus};
pub use crate::profile_check::{
    ProfileInputError, check_profile_config_dir, check_profile_name,
};
pub use crate::profile_list::{profile_status, profiles};
pub use crate::program::{
    ProgramError, ProgramOutput, RunningProgram, StartProgram,
};
pub use crate::program_version::version_in_output;
pub use crate::project::{Project, ProjectSession, projects};
pub use crate::provider::{ProviderEntry, ProviderId, ProviderRef};
pub use crate::record_format::FormatChange;
pub use crate::refused_login::RefusedLogin;
pub use crate::registry::Registry;
pub use crate::saved_profiles::{
    DefaultProfileChanges, SavedProfiles, SavedProfilesError,
};
pub use crate::session::{AgentSession, SessionId, SessionSource};
pub use crate::source_id::SourceId;
pub use crate::spend::{
    DaySpend, HourSpend, ModelTokens, ProjectModelTokens, ProjectTokens, Spend,
    SpendCounter,
};
pub use crate::stats::{
    LimitChange, ProjectStats, SessionStats, StatsSource, read_project_stats,
    read_session_stats,
};
pub use crate::token_count::TokenCount;
pub use crate::walk::{Answer, SourceFailure, read_sessions};

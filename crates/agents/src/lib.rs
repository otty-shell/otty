//! What OTTY knows about coding agents such as Claude Code, Codex, Kilo
//! and OpenCode.
//!
//! Each agent implements [`Agent`]: it says how to find its default
//! [`Profile`], whether a profile is connected, and how to start or continue
//! a session. The data an agent can read comes from small sources:
//! [`SessionSource`], [`LimitSource`] and [`SpendCounter`]. An agent gives
//! its sources best first, and [`read_sessions`] and [`read_limits`] try them
//! in order until one answers. A source that asks the agent's own program
//! starts it with the [`StartProgram`] the app gives, and a source that
//! reads the program's screen starts it with the [`StartTerminal`] the app
//! gives. [`projects`] groups the sessions of all
//! profiles by their folder.
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
//! a day, or from its kept copy. Each reader counts the records it saw and
//! understood; when the newest session files are not understood, the count
//! fails with [`SourceError::FormatChanged`], whose [`FormatChange`] names
//! the agent's version and gives a report with no value of a record.
//!
//! An agent's [`StatsSource`] reads the [`SessionStats`] of one session and
//! the [`ProjectStats`] of one project folder over the whole history;
//! [`MoneyTotal::of_models`] prices their tokens.
//!
//! Limits, credits and bills belong to an [`Account`]: a login or a key at
//! a provider, named by a [`ProviderRef`], found in the files of each
//! profile by
//! [`Agent::accounts`] and made one per account by [`merge_accounts`]. The
//! app reads them once per account, with the files of one profile it was
//! found in. With an admin key, an agent's [`BillSource`] reads the real
//! [`Bill`] of an organization; the key is kept only as an
//! [`AdminKeyRef`], the file or the variable that holds it, saved for the
//! organization in [`SavedProfiles`]. [`AccountMoney`] is the money of an
//! account: its bill, else an estimate over the tokens of the profiles
//! that pay through it.
//!
//! Some accounts tell their own numbers with their own key:
//! [`read_account_numbers`] reads the [`Balance`] of a DeepSeek,
//! OpenRouter, Moonshot or Kilo key, the [`Bill`] of an Anthropic or
//! OpenAI organization the user added, or the [`Limits`] of a ChatGPT
//! login only Kilo or OpenCode keeps, with the key read again from its
//! [`KeyPlace`] and dropped right after the request. An account the user
//! added by hand is an [`Account::added`], saved in [`SavedProfiles`] by
//! the file or the variable of its key only.
//!
//! Some agents keep their keys in the system keyring. [`keyring_entries`]
//! lists the [`KeyringEntry`] of each known agent found on this computer,
//! such as the one of Goose, and [`read_keyring_accounts`] reads them, once
//! the user allowed it, into accounts found in the system keyring; it only
//! reads, and fails with a [`KeyringError`] that holds nothing of the
//! keyring.
//!
//! [`read_bill`], [`read_session_stats`] and [`read_project_stats`] find
//! the source of an agent with [`agent_of`] and read through it;
//! [`profile_status`] tells the state of a profile.
//!
//! The [`Registry`] is the one list of the agents and providers OTTY knows
//! right now: names, icons, usage pages and hosts come only from it (spec
//! 2026-11-19-agent-plugins, "Links in the interface"). The parts of the
//! plugins that are on fill it ([`Registry::of_readers`]), and the readers
//! of this crate put their entries in it as built-in ones for what no part
//! serves. An account whose provider no entry serves stays known by the
//! host of its key.
//!
//! A [`PluginAgent`] is an agent part of a plugin as an [`Agent`]: OTTY
//! walks the spend places it names, reads only the files that changed,
//! counts an answer once and judges "format changed" from what the part
//! saw. A [`PluginProvider`] reads the numbers of the accounts of its own
//! provider id or hosts, with a handle to the key an agent part found. A
//! trap or a missed deadline fails that read only, as a
//! [`PluginFailure`]; three in a row pause the part ([`PartHealth`]).
//!
//! The crate knows nothing about the window and reads no environment
//! itself: the app passes in an [`AgentEnv`], built from the
//! [`ConfigFolder`] of each agent, and runs the blocking calls on a
//! background thread.
//!
//! The `fixtures` feature adds the module `fixtures`: values for the tests
//! of the crates that use this one.

mod account;
mod account_key;
mod account_money;
mod account_numbers;
mod admin_key;
mod agent;
mod balance;
mod bill;
mod built_in;
mod called_part;
mod codex;
mod command;
mod config_dir;
mod dollars;
mod env;
mod error;
mod file_memory;
mod file_stamp;
mod file_tree;
#[cfg(feature = "fixtures")]
pub mod fixtures;
mod hidden_terminal;
mod https;
mod json_file;
mod jsonl;
mod key_fingerprint;
mod keyring_entry;
mod limits;
mod litellm;
mod local_time;
mod money_estimate;
mod open_code;
mod part_health;
mod percent;
mod plugin_agent;
mod plugin_failure;
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
mod record_format;
mod registry;
mod saved_profiles;
mod session;
mod source_id;
mod spend;
mod spend_files;
mod spend_tally;
mod stats;
mod system_keyring;
#[cfg(test)]
mod testing;
mod timestamp;
mod title;
mod token_count;
mod walk;

pub use crate::account::{
    Account, AccountIdentity, AccountKind, AccountLogin, FoundIn,
    merge_accounts,
};
pub use crate::account_key::AccountKey;
pub use crate::account_money::AccountMoney;
pub use crate::account_numbers::{
    AccountNumbers, KeyPlace, own_source_of, read_account_numbers,
};
pub use crate::admin_key::{AdminKey, AdminKeyRef};
pub use crate::agent::{Agent, AgentEntry, AgentId, ConfigFolder, agent_of};
pub use crate::balance::{Amount, Balance, KeyLimit};
pub use crate::bill::{Bill, BillSource, DayCost, read_bill};
pub use crate::codex::Codex;
pub use crate::command::AgentCommand;
pub use crate::config_dir::{
    CreateConfigDirError, create_config_dir, proposed_config_dir,
};
pub use crate::dollars::Dollars;
pub use crate::env::AgentEnv;
pub use crate::error::{ClaudeQuestion, SourceError, Unavailable, Unexpected};
pub use crate::hidden_terminal::{
    RunningTerminal, ScreenOutput, StartTerminal,
};
pub use crate::key_fingerprint::KeyFingerprint;
pub use crate::keyring_entry::{
    KeyringEntry, keyring_entries, read_keyring_accounts,
};
pub use crate::limits::{
    Billing, BillingKind, Credits, ExtraUsage, LimitSource, LimitWindow,
    Limits, UsageWindows, WindowId,
};
pub use crate::local_time::LocalNow;
pub use crate::money_estimate::{MoneyEstimate, MoneyTotal};
pub use crate::open_code::OpenCodeAgent;
pub use crate::part_health::{PAUSE_AFTER, PartHealth};
pub use crate::percent::Percent;
pub use crate::plugin_agent::PluginAgent;
pub use crate::plugin_failure::{PluginFailure, PluginFailureKind};
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
pub use crate::provider::{
    AddAccountKind, ProviderEntry, ProviderId, ProviderRef,
};
pub use crate::record_format::FormatChange;
pub use crate::registry::{PluginPart, Reader, Registry};
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
pub use crate::system_keyring::KeyringError;
pub use crate::token_count::TokenCount;
pub use crate::walk::{Answer, SourceFailure, read_limits, read_sessions};

/// The agents OTTY knows, in the order they are shown.
///
/// Adding an agent means implementing [`Agent`] and adding it here.
///
/// An agent may remember what it read between two refreshes, so the app
/// calls this once and keeps the list for the whole run.
///
/// ```
/// for agent in otty_agents::known_agents() {
///     println!("{}", agent.entry().name());
/// }
/// ```
pub fn known_agents() -> Vec<Box<dyn Agent>> {
    vec![
        Box::new(Codex::default()),
        Box::new(OpenCodeAgent::kilo()),
        Box::new(OpenCodeAgent::opencode()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codex_kilo_and_opencode_are_the_known_agents() {
        let agents = known_agents();
        let ids: Vec<_> = agents
            .iter()
            .map(|agent| agent.entry().id().as_str())
            .collect();

        assert_eq!(ids, ["codex", "kilo", "opencode"]);
    }

    #[test]
    fn each_known_agent_names_the_provider_of_its_models() {
        let agents = known_agents();
        let providers: Vec<_> = agents
            .iter()
            .map(|agent| agent.entry().provider_name())
            .collect();

        assert_eq!(providers, ["OpenAI", "Kilo", "OpenCode"]);
    }

    #[test]
    fn only_kilo_and_opencode_write_the_cost_and_keep_a_database() {
        let found: Vec<_> = known_agents()
            .iter()
            .map(|agent| {
                (
                    agent.writes_cost(),
                    agent.config_folder().database_variable(),
                )
            })
            .collect();

        assert_eq!(
            found,
            [
                (false, None),
                (true, Some("KILO_DB")),
                (true, Some("OPENCODE_DB"))
            ]
        );
    }

    #[test]
    fn every_known_agent_reads_statistics() {
        for agent in known_agents() {
            assert!(agent.stats_source().is_some(), "{}", agent.entry().name());
        }
    }

    /// The bill of an organization comes from an admin key the user adds,
    /// not from an agent: Claude Code's reader went to its plugin.
    #[test]
    fn no_known_agent_reads_a_bill() {
        let has_bill: Vec<_> = known_agents()
            .iter()
            .map(|agent| agent.bill_source().is_some())
            .collect();

        assert_eq!(has_bill, [false, false, false]);
    }

    #[test]
    fn each_known_agent_names_its_config_folder_variable() {
        let variables: Vec<_> = known_agents()
            .iter()
            .map(|agent| agent.config_folder().variable().to_string())
            .collect();

        assert_eq!(variables, ["CODEX_HOME", "XDG_DATA_HOME", "XDG_DATA_HOME"]);
    }
}

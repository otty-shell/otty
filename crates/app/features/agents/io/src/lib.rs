//! The I/O of Agents mode: reading the profiles, the sessions and the
//! limits, and writing `agents.json`.
//!
//! The agents feature says what to read; this crate reads it on a thread of
//! its own and answers with an [`AgentsIntent`](otty_app_agents::AgentsIntent).
//! [`load_profiles`] reads `agents.json` next to `modes.json`, finds the
//! default profile of each agent and checks each profile: its program is
//! looked up through the user's login shell, the way the agent tab starts
//! it. With the profiles it asks each provider part for the accounts it
//! finds in the shown profiles of its own plugin and in the folders of its
//! own logins. [`load_sessions`] reads
//! the sessions of one profile and finds the project folders that no
//! longer exist. [`load_limits`] reads the limits of one profile through
//! the provider part that found the account whose limits it shows, in the
//! folders it found it in, which may ask the provider's server, or start
//! the agent's program and read its screen in a hidden terminal;
//! [`load_limits_in_background`] does the same while Agents mode is away,
//! without starting the agent's program. [`load_spend`] counts the tokens
//! one profile spent from its session files, by the days of the user's
//! time zone, which it asks from the system. [`load_bill`] reads the real
//! bill of an organization through the part that found it, with the admin
//! key the user gave, read from its file or from a variable as the user's
//! login shell sees it, and dropped right after the request.
//! [`load_account_numbers`] reads the numbers of one account through the
//! part that found it: the money left, a key limit or the limits of a
//! subscription no profile reads.
//! [`load_prices`] reads the list prices of the money at API prices: the
//! LiteLLM price table, fetched at most once a day and kept in OTTY's cache
//! folder.
//! [`load_session_stats`] and [`load_project_stats`] read the statistics of
//! a session or of a project folder from the agents' session files.
//! [`load_version`] asks an agent's program for its version with
//! `--version`, which the feature asks for only while the mode is on
//! screen. [`save_profiles`] creates the folder of a new profile and writes
//! `agents.json`; it never deletes a folder. [`copy_command`] and [`copy_text`] put the command that
//! continues a session or its id on the clipboard. [`look_up_folder`]
//! tells the folder field of the New session and profile dialogs whether
//! its typed folder is there and which folders it may stand for.
//! [`open_page`] opens a page, such as the usage page of an account, in
//! the user's browser.
//! [`prepare_own_login`], [`make_own_login_folder`], [`check_own_login`]
//! and [`remove_own_login_folder`] make OTTY's own login of a provider part
//! in its data folder, `accounts/<id>/`, ask the part about it once its
//! login tab closed, and remove that folder when it holds no login or the
//! user removed its account; [`log_in_again`] asks the part for the login
//! command of a folder made before.
//! [`PluginParts`] loads the parts that are on of the installed plugins
//! ([`Readers`]): the agents, the provider parts and the registry. [`plugin_engine`] makes
//! the one engine of the plugins, and [`readers_now`] and
//! [`load_readers`] give what Agents mode reads with once the installed
//! plugins are loaded ([`LoadedReaders`]), at start and on a thread of its
//! own when they change. [`tell_language`] gives the plugins a language
//! the user picked and tells the ones with the language receiver.
//! The binary only calls them and wraps the intent into its message.

mod account_numbers;
mod agent_program;
mod background;
mod bill;
mod browser;
mod clipboard;
mod folder_lookup;
mod hidden_terminal;
mod limits;
mod load;
mod login_shell;
mod own_login;
mod plugin_language;
mod plugin_parts;
mod plugin_programs;
mod plugin_readers;
mod prices;
mod profiles_file;
mod program_lookup;
mod save;
mod shell_variable;
mod spend;
mod stats;
#[cfg(test)]
mod testing;
mod time_zone;
mod version;

pub use account_numbers::load_account_numbers;
pub use bill::load_bill;
pub use browser::open_page;
pub use clipboard::{copy_command, copy_text};
pub use folder_lookup::look_up_folder;
pub use limits::{load_limits, load_limits_in_background};
pub use load::{load_profiles, load_sessions};
pub use own_login::{
    check_own_login, log_in_again, make_own_login_folder, prepare_own_login,
    remove_own_login_folder,
};
pub use plugin_language::{LanguageReceivers, tell_language};
pub use plugin_parts::{
    InstalledParts, PluginParts, ProviderList, Readers, RunningParts,
};
pub use plugin_readers::{
    LoadedReaders, ReadVariable, agent_env, load_readers, plugin_engine,
    readers_now,
};
pub use prices::load_prices;
pub use save::save_profiles;
pub use spend::load_spend;
pub use stats::{load_project_stats, load_session_stats};
pub use version::load_version;

//! The I/O of Agents mode: reading the profiles, the sessions and the
//! limits, and writing `agents.json`.
//!
//! The agents feature says what to read; this crate reads it on a thread of
//! its own and answers with an [`AgentsIntent`](otty_app_agents::AgentsIntent).
//! [`load_profiles`] reads `agents.json` next to `modes.json`, finds the
//! default profile of each agent and checks each profile: its program is
//! looked up through the user's login shell, the way the agent tab starts
//! it. [`load_sessions`] reads the sessions of one profile and finds the
//! project folders that no longer exist. [`load_limits`] reads the limits
//! of one profile, which may ask the agent's server, or start the agent's
//! program and read its screen in a hidden terminal;
//! [`load_limits_in_background`] does the same while Agents mode is away,
//! without starting the agent's program. [`load_spend`] counts
//! the tokens one profile spent from its session files, by the days of the
//! user's time zone, which it asks from the system. [`load_bill`] reads the real bill of an API key
//! profile with its admin key, read from its file or from a variable as
//! the user's login shell sees it, and dropped right after the request.
//! [`load_account_numbers`] reads the numbers of one account with its own
//! key, read again from where it was found and dropped right after the
//! request: the money left, a key limit, the bill of an organization the
//! user added or the limits of a ChatGPT login no profile reads.
//! [`load_prices`] reads the list prices of the money estimate: the
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
//! the user's browser. [`load_keyring`] reads the entries of the known
//! agents in the system keyring, once the user allowed it.
//! The binary only calls them and wraps the intent into its message.

mod account_numbers;
mod agent_program;
mod background;
mod bill;
mod browser;
mod clipboard;
mod folder_lookup;
mod hidden_terminal;
mod keyring;
mod limits;
mod load;
mod login_shell;
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
pub use keyring::load_keyring;
pub use limits::{load_limits, load_limits_in_background};
pub use load::{load_profiles, load_sessions};
pub use prices::load_prices;
pub use save::save_profiles;
pub use spend::load_spend;
pub use stats::{load_project_stats, load_session_stats};
pub use version::load_version;

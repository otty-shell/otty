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
//! of one profile, which may ask the agent's server. [`load_spend`] counts
//! the tokens one profile spent from its session files, by the days of the
//! user's time zone. [`load_bill`] reads the real bill of an API key
//! profile with its admin key, read from its file or from the environment
//! and dropped right after the request. [`load_candidates`]
//! finds the folders the profile dialog offers. [`save_profiles`] creates
//! the folder of a new profile and writes `agents.json`; it never deletes a
//! folder. [`copy_command`] and [`copy_text`] put the command that
//! continues a session or its id on the clipboard.
//! The binary only calls them and wraps the intent into its message.

mod agent_program;
mod bill;
mod clipboard;
mod limits;
mod load;
mod profiles_file;
mod program_lookup;
mod save;
mod spend;
#[cfg(all(test, unix))]
mod testing;
mod utc_offset;

pub use bill::load_bill;
pub use clipboard::{copy_command, copy_text};
pub use limits::load_limits;
pub use load::{load_candidates, load_profiles, load_sessions};
pub use save::save_profiles;
pub use spend::load_spend;

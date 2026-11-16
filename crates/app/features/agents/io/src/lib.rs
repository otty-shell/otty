//! The I/O of Agents mode: reading the profiles and the sessions.
//!
//! The agents feature says what to read; this crate reads it on a thread of
//! its own and answers with an [`AgentsIntent`](otty_app_agents::AgentsIntent).
//! [`load_profiles`] reads `agents.json` next to `modes.json`, finds the
//! default profile of each agent and checks each profile: its program is
//! looked up through the user's login shell, the way the agent tab starts
//! it. [`load_sessions`] reads the sessions of one profile and finds the
//! project folders that no longer exist. The binary only calls them and
//! wraps the intent into its message.

mod load;
mod profiles_file;
mod program_lookup;

pub use load::{load_profiles, load_sessions};

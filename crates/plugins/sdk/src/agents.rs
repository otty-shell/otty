//! The `agents` domain, Agents mode in OTTY: coding agents and the
//! providers they pay through (issue 21). Its agent parts are served by
//! one [`Agent`] implementation and its provider parts by one
//! [`Provider`] implementation; OTTY names the part in each call.
//!
//! Each trait maps to its own versioned WIT contract:
//! `otty:agents-agent@0.1.0` and `otty:agents-provider@0.1.0`. The values
//! both share, such as [`ReadError`], are `otty:agents-agent/types`. A part names them in the manifest as
//! `[[agents.agent]]` and `[[agents.provider]]`, and the plugin exports
//! them with
//! `export!(Example with agents::agent, agents::provider)`.
//!
//! An agent part prices the tokens of its answers itself (spec
//! 2026-11-23-accounts-apart-from-profiles, Decision 3): the cost the
//! agent wrote as it is, else the tokens at the API price of the model
//! that OTTY's price list gives, [`price_of`]; [`answer_cost`] does both.
//! A provider part may ask the same list while it reads numbers, such as
//! to count what the agent's own sessions spent through a key this month
//! ([`MonthSpend`], issue 22).
//!
//! The host operations and the values every domain shares, such as a
//! [`Need`](crate::Need) and a [`Key`](crate::Key), are at the crate
//! root.

mod agent;
mod month_spend;
mod prices;
mod provider;
mod record_counts;

pub use crate::agents::agent::Agent;
#[doc(hidden)]
pub use crate::agents::agent::AgentExport;
pub use crate::agents::month_spend::MonthSpend;
pub use crate::agents::prices::{answer_cost, cost_at, price_of};
#[doc(hidden)]
pub use crate::agents::provider::ProviderExport;
pub use crate::agents::provider::Provider;
/// The types of the agent contract.
pub use crate::bindings::agents_agent::exports::otty::agents_agent::agent::{
    Answer, AnswerCost, Answers, Place, PlaceItem, PlaceKind, Profile,
    ProfilePlace, ProfileStatus, RecordCounts, Session, TokenCount,
};
/// OTTY's price list as a part asks it: who served an answer, the API
/// price of a model, and money at API prices kind by kind.
pub use crate::bindings::agents_agent::otty::agents_agent::prices::{
    LongPromptPrice, ModelPrice, PricedCost, ProviderRef, TokenPrices,
};
/// The values the agent and provider contracts share: why a part could
/// not read.
pub use crate::bindings::agents_agent::otty::agents_agent::types::ReadError;
/// The types of the provider contract.
pub use crate::bindings::agents_provider::exports::otty::agents_provider::provider::{
    Account, AccountKind, AccountNumbers, Bill, Credits, DayCost, DayRange,
    ExtraUsage, Found, KeyLimit, LimitWindow, LoginFolder, Money,
};

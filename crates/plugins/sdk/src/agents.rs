//! The `agents` domain, Agents mode in OTTY: coding agents and the
//! providers they pay through (issue 21). Its agent parts are served by
//! one [`Agent`] implementation and its provider parts by one
//! [`Provider`] implementation; OTTY names the part in each call.
//!
//! Each trait maps to its own versioned WIT contract:
//! `otty:agents-agent@0.1.0` and `otty:agents-provider@0.1.0`. The values
//! both share, such as [`ReadError`], [`Identity`] and [`Limits`], are
//! `otty:agents-agent/types`. A part names them in the manifest as
//! `[[agents.agent]]` and `[[agents.provider]]`, with
//! `[api] agents.agent = "0.1"` and `agents.provider = "0.1"`, and the
//! plugin exports them with
//! `export!(Example with agents::agent, agents::provider)`.
//!
//! The host operations and the values every domain shares, such as a
//! [`Need`](crate::Need) and a [`Key`](crate::Key), are at the crate
//! root.

mod agent;
mod provider;
mod record_counts;

pub use crate::agents::agent::Agent;
#[doc(hidden)]
pub use crate::agents::agent::AgentExport;
#[doc(hidden)]
pub use crate::agents::provider::ProviderExport;
pub use crate::agents::provider::Provider;
/// The types of the agent contract.
pub use crate::bindings::agents_agent::exports::otty::agents_agent::agent::{
    Answer, Answers, Billing, Connection, Login, LoginRef, ModelTokens,
    Profile, ProfilePlace, ProfileStatus, ProjectStats, ProviderRef,
    RecordCounts, Session, SessionStats, SpendItem, SpendKind, SpendPlace,
    TokenCount,
};
/// The values the agent and provider contracts share: whom a login is
/// for, what an account may still use, and why a part could not read.
pub use crate::bindings::agents_agent::otty::agents_agent::types::{
    Credits, Identity, LimitWindow, Limits, LoginKind, ReadError,
};
/// The types of the provider contract.
pub use crate::bindings::agents_provider::exports::otty::agents_provider::provider::{
    AccountFacts, AccountNumbers, Bill, DayCost, DayRange, ExtraUsage,
    KeyLimit, LoginFacts, Money,
};

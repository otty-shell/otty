//! The extension points of the `agents` domain, Agents mode (issue 21):
//! the [`agent`] and [`provider`] contracts, the bindings of each version
//! the host keeps, and the calls into their parts. The domain's shared
//! values, such as [`agent::ReadError`], are
//! `otty:agents-agent/types`; the provider contract's bindings use them.
//! An agent part prices the tokens of its answers itself, asking OTTY's
//! [`prices::PriceList`] that the app gives each read of answers.
//!
//! Calling a part is no domain's business: the batch, the deadlines, the
//! health of a part and the reports of its failures are at the crate
//! root.

pub mod agent;
pub mod prices;
pub mod provider;

//! OTTY's price list as the host gives it to an agent part (spec
//! 2026-11-23-accounts-apart-from-profiles, Decision 3): the part asks
//! `prices.price-of` for the API price of a model while it reads its
//! answers, and prices their tokens itself. The host keeps no prices: the
//! app gives a [`PriceList`] to each read of answers.

use wasmtime::component::{HasSelf, Linker};

/// The types of OTTY's price list, in the newest agent contract.
pub use crate::agents::agent::v0_1::otty::agents_agent::prices::{
    LongPromptPrice, ModelPrice, ProviderRef, TokenPrices,
};
use crate::calls::CallState;

/// OTTY's price list of models at API prices, which the app keeps and
/// renews, such as the LiteLLM price table. It is a trait because the
/// host does not know where OTTY keeps its prices: the app's price list
/// lives in a crate that uses this one.
#[cfg_attr(test, mockall::automock)]
pub trait PriceList: Send + Sync {
    /// The API price of `model`, as the provider names it, served by
    /// `provider`: that provider's price of the model, else its maker's.
    /// `None` when the list has no price for it.
    fn price_of(
        &self,
        model: &str,
        provider: Option<ProviderRef>,
    ) -> Option<ModelPrice>;
}

/// `prices.price-of`: the price list the running read of answers was
/// given; no price outside such a read.
impl crate::agents::agent::v0_1::otty::agents_agent::prices::Host
    for CallState
{
    fn price_of(
        &mut self,
        model: String,
        provider: Option<ProviderRef>,
    ) -> Option<ModelPrice> {
        self.prices()?.price_of(&model, provider)
    }
}

/// Adds OTTY's price list, `otty:agents-agent/prices`, to `linker`: a
/// component with agent parts imports it.
pub(crate) fn add_to_linker(
    linker: &mut Linker<CallState>,
) -> wasmtime::Result<()> {
    crate::agents::agent::v0_1::otty::agents_agent::prices::add_to_linker::<
        CallState,
        HasSelf<CallState>,
    >(linker, |state| state)
}

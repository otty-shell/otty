use super::limits::UsageRowViewModel;

/// The Money block of an account page (spec 2026-11-19, Decision 5): when
/// the money was read, then its rows like the usage windows of a
/// subscription, and a muted line. There is no budget: OTTY only shows.
/// The last good money stays, greyed, when a later count fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MoneyViewModel {
    /// When the money was read, such as "Updated 2 min ago".
    pub(crate) updated: Option<String>,
    /// Today, the last 7 days and this month; empty before a good count.
    pub(crate) rows: Vec<UsageRowViewModel>,
    /// The muted line under the rows: how many requests of this month
    /// have no price, or that no prices could be read.
    pub(crate) note: Option<String>,
    /// Whether the money is an older estimate, drawn greyed, because the
    /// latest count failed.
    pub(crate) is_stale: bool,
    /// Why the latest count failed, or why the bill could not be read.
    pub(crate) failure: Option<String>,
    /// The mark next to the money spent this month when it is an estimate
    /// at the LiteLLM list prices, not the provider's bill.
    pub(crate) estimate: Option<EstimateMarkViewModel>,
}

/// The short mark next to money that is an estimate at the LiteLLM list
/// prices (issue 19 of spec 2026-11-19-agent-plugins): "estimate, list
/// prices", with a tooltip that says when the prices are from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EstimateMarkViewModel {
    /// The mark: "estimate, list prices".
    pub(crate) label: &'static str,
    /// Its tooltip: the tokens of the session files at the LiteLLM prices
    /// of a time, an estimate.
    pub(crate) tip: String,
}

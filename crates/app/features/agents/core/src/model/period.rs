//! The pieces of the tokens block of a dashboard: the totals of today, 7
//! days and 30 days, and one bar per day.

/// One total of a stretch of time, such as the tokens of today: its label,
/// the number and a muted line under it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TotalViewModel {
    /// What the total counts, such as "Today".
    pub(crate) label: &'static str,
    /// The number, such as "1.2M".
    pub(crate) value: String,
    /// The line under it, such as "1M in · 0 cached · 0.2M out".
    pub(crate) note: String,
}

/// One bar per day, oldest first and today last.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DayBarsViewModel {
    /// The bars, today last.
    pub(crate) bars: Vec<DayBarViewModel>,
    /// The date of the first bar, written under it.
    pub(crate) first_day: String,
}

/// The bar of one day: its tokens and its tooltip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DayBarViewModel {
    /// The value of the day.
    pub(crate) value: u64,
    /// What the tooltip says, such as "Nov 13: 2.9M tokens".
    pub(crate) tip: String,
}

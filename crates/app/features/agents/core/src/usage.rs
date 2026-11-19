//! What the usage block of a profile shows (spec 2026-11-19, Decision 6):
//! the period, the measure and the tab the user picked, and the row of its
//! table picked or under the pointer, whose part of each bar is lit. The
//! choices are the same for every profile and kept while OTTY runs, not
//! saved.

use std::fmt;

use otty_app_i18n::{self as i18n, Key};

/// How far back the usage block looks.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum UsagePeriod {
    /// Today, one bar per hour.
    Today,
    /// The last 7 days, today included, one bar per day; chosen at start.
    #[default]
    SevenDays,
    /// The last 30 days, today included, one bar per day.
    ThirtyDays,
}

impl UsagePeriod {
    /// Every period, in the order the menu lists them.
    pub(crate) const ALL: [Self; 3] =
        [Self::Today, Self::SevenDays, Self::ThirtyDays];

    /// How the period is named in its menu, such as "7 days".
    pub(crate) fn label(self) -> &'static str {
        let key = match self {
            Self::Today => Key::AgentsToday,
            Self::SevenDays => Key::AgentsSevenDays,
            Self::ThirtyDays => Key::AgentsThirtyDays,
        };

        i18n::t(key)
    }
}

/// What the usage block counts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum UsageMeasure {
    /// The tokens; chosen at start.
    #[default]
    Tokens,
    /// The money the tokens cost at the list prices.
    Money,
}

impl UsageMeasure {
    /// Both measures, in the order the menu lists them.
    pub(crate) const ALL: [Self; 2] = [Self::Tokens, Self::Money];

    /// How the measure is named in its menu and over its column.
    pub(crate) fn label(self) -> &'static str {
        let key = match self {
            Self::Tokens => Key::AgentsTokens,
            Self::Money => Key::AgentsNumberMoney,
        };

        i18n::t(key)
    }
}

/// What the table of the usage block lists, one row each.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum UsageTab {
    /// The projects the tokens were spent in.
    ByProject,
    /// The models that spent the tokens; chosen at start.
    #[default]
    ByModel,
    /// The accounts that paid for the tokens.
    ByAccount,
}

impl UsageTab {
    /// Every tab, in the order they show.
    pub(crate) const ALL: [Self; 3] =
        [Self::ByProject, Self::ByModel, Self::ByAccount];
}

impl fmt::Display for UsageTab {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let key = match self {
            Self::ByProject => Key::AgentsTopByProject,
            Self::ByModel => Key::AgentsTopByModel,
            Self::ByAccount => Key::AgentsByAccount,
        };

        formatter.write_str(i18n::t(key))
    }
}

/// The choices of the usage block, and the row of its table that is
/// picked or under the pointer, by its name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct UsageChoice {
    period: UsagePeriod,
    measure: UsageMeasure,
    tab: UsageTab,
    picked: Option<String>,
    hovered: Option<String>,
}

impl UsageChoice {
    /// The period picked.
    pub(crate) fn period(&self) -> UsagePeriod {
        self.period
    }

    /// The measure picked.
    pub(crate) fn measure(&self) -> UsageMeasure {
        self.measure
    }

    /// The tab picked.
    pub(crate) fn tab(&self) -> UsageTab {
        self.tab
    }

    /// The name of the row picked in the table.
    pub(crate) fn picked(&self) -> Option<&str> {
        self.picked.as_deref()
    }

    /// The name of the row under the pointer.
    pub(crate) fn hovered(&self) -> Option<&str> {
        self.hovered.as_deref()
    }

    /// Look back over `period`.
    pub(crate) fn pick_period(&mut self, period: UsagePeriod) {
        self.period = period;
    }

    /// Count in `measure`.
    pub(crate) fn pick_measure(&mut self, measure: UsageMeasure) {
        self.measure = measure;
    }

    /// List the rows of `tab`; a row picked in another tab is forgotten.
    pub(crate) fn show_tab(&mut self, tab: UsageTab) {
        self.tab = tab;
        self.forget_rows();
    }

    /// Pick the row `name`, or let it go when it is the picked one.
    pub(crate) fn pick_row(&mut self, name: String) {
        if self.picked.as_deref() == Some(name.as_str()) {
            self.picked = None;
        } else {
            self.picked = Some(name);
        }
    }

    /// The pointer is over the row `name`, or over no row.
    pub(crate) fn hover_row(&mut self, name: Option<String>) {
        self.hovered = name;
    }

    /// Forget the picked row and the row under the pointer, such as when
    /// another profile shows.
    pub(crate) fn forget_rows(&mut self) {
        self.picked = None;
        self.hovered = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_block_opens_on_seven_days_of_tokens_by_model() {
        let choice = UsageChoice::default();

        assert_eq!(choice.period(), UsagePeriod::SevenDays);
        assert_eq!(choice.measure(), UsageMeasure::Tokens);
        assert_eq!(choice.tab(), UsageTab::ByModel);
        assert_eq!(choice.picked(), None);
    }

    #[test]
    fn a_second_press_on_the_picked_row_lets_it_go() {
        let mut choice = UsageChoice::default();

        choice.pick_row(String::from("opus"));
        let picked = choice.picked().map(String::from);
        choice.pick_row(String::from("sonnet"));
        let other = choice.picked().map(String::from);
        choice.pick_row(String::from("sonnet"));

        assert_eq!(picked.as_deref(), Some("opus"));
        assert_eq!(other.as_deref(), Some("sonnet"));
        assert_eq!(choice.picked(), None);
    }

    #[test]
    fn another_tab_forgets_the_rows_but_keeps_the_period_and_measure() {
        let mut choice = UsageChoice::default();
        choice.pick_period(UsagePeriod::Today);
        choice.pick_measure(UsageMeasure::Money);
        choice.pick_row(String::from("opus"));
        choice.hover_row(Some(String::from("sonnet")));

        choice.show_tab(UsageTab::ByProject);

        assert_eq!(choice.tab(), UsageTab::ByProject);
        assert_eq!(choice.picked(), None);
        assert_eq!(choice.hovered(), None);
        assert_eq!(choice.period(), UsagePeriod::Today);
        assert_eq!(choice.measure(), UsageMeasure::Money);
    }
}

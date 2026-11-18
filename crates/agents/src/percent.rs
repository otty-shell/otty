//! A share of a whole in percent, such as how much of a usage window is
//! used.

/// A share of a whole in percent, kept as the source told it: it may be
/// below 0 or above 100, such as a usage window used past its limit.
///
/// ```
/// use otty_agents::Percent;
///
/// let used = Percent::new(103.4);
///
/// assert_eq!(used.value(), 103.4);
/// assert_eq!(used.rounded(), 100);
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, PartialOrd)]
pub struct Percent {
    value: f64,
}

impl Percent {
    /// The percent as the source told it.
    pub fn value(&self) -> f64 {
        self.value
    }

    /// The percent rounded to a whole one and kept between 0 and 100, as
    /// a bar or a label shows it.
    pub fn rounded(&self) -> u8 {
        self.value.clamp(0.0, 100.0).round() as u8
    }

    /// `value` percent.
    pub const fn new(value: f64) -> Self {
        Self { value }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_percent_is_rounded_and_kept_between_0_and_100() {
        let cases = [
            (41.6, 42),
            (41.4, 41),
            (0.5, 1),
            (103.4, 100),
            (-3.0, 0),
            (f64::NAN, 0),
        ];

        for (value, rounded) in cases {
            assert_eq!(Percent::new(value).rounded(), rounded, "{value}");
        }
    }
}

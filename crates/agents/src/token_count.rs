//! A number of tokens: sent to the model, read from its cache and received
//! from it.

use std::ops::{Add, AddAssign, Sub};

/// A number of tokens sent to the model, read from its cache, and received
/// from it.
///
/// The three main numbers do not overlap: [`input`](Self::input) is what
/// was sent and not read from the cache, [`cached`](Self::cached) what was
/// read from the cache, [`output`](Self::output) what the model answered.
/// [`cache_written`](Self::cache_written) is the part of the input the
/// model also wrote to its cache, which some providers price apart.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TokenCount {
    input: u64,
    cached: u64,
    cache_written: u64,
    output: u64,
}

impl TokenCount {
    /// The tokens sent to the model that were not read from its cache.
    pub fn input(&self) -> u64 {
        self.input
    }

    /// The tokens read from the model's cache.
    pub fn cached(&self) -> u64 {
        self.cached
    }

    /// The part of [`input`](Self::input) the model wrote to its cache.
    /// Only Claude Code tells it; it is 0 for Codex.
    pub fn cache_written(&self) -> u64 {
        self.cache_written
    }

    /// The tokens the model answered with.
    pub fn output(&self) -> u64 {
        self.output
    }

    /// All the tokens: input, cached and output.
    pub fn total(&self) -> u64 {
        self.input
            .saturating_add(self.cached)
            .saturating_add(self.output)
    }

    /// A count from its parts, with nothing written to the cache.
    ///
    /// ```
    /// use otty_agents::TokenCount;
    ///
    /// let count = TokenCount::new(10, 300, 2);
    ///
    /// assert_eq!(count.total(), 312);
    /// ```
    pub fn new(input: u64, cached: u64, output: u64) -> Self {
        Self {
            input,
            cached,
            cache_written: 0,
            output,
        }
    }

    /// The same count, with `cache_written` of its input written to the
    /// cache.
    ///
    /// ```
    /// use otty_agents::TokenCount;
    ///
    /// let count = TokenCount::new(100, 0, 5).with_cache_written(80);
    ///
    /// assert_eq!(count.cache_written(), 80);
    /// ```
    pub fn with_cache_written(self, cache_written: u64) -> Self {
        Self {
            cache_written,
            ..self
        }
    }

    /// Whether every number is 0.
    pub fn is_zero(&self) -> bool {
        *self == Self::default()
    }
}

impl Add for TokenCount {
    type Output = Self;

    /// The sum of two counts. A number too large to keep stays at the
    /// largest one.
    fn add(self, other: Self) -> Self {
        Self {
            input: self.input.saturating_add(other.input),
            cached: self.cached.saturating_add(other.cached),
            cache_written: self
                .cache_written
                .saturating_add(other.cache_written),
            output: self.output.saturating_add(other.output),
        }
    }
}

impl AddAssign for TokenCount {
    fn add_assign(&mut self, other: Self) {
        *self = *self + other;
    }
}

impl Sub for TokenCount {
    type Output = Self;

    /// What is left of this count when `other` is taken from it, part by
    /// part. A part never goes below 0.
    fn sub(self, other: Self) -> Self {
        Self {
            input: self.input.saturating_sub(other.input),
            cached: self.cached.saturating_sub(other.cached),
            cache_written: self
                .cache_written
                .saturating_sub(other.cache_written),
            output: self.output.saturating_sub(other.output),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_add_up_part_by_part() {
        let mut count = TokenCount::new(1, 2, 3).with_cache_written(1);

        count += TokenCount::new(10, 20, 30);

        assert_eq!(count, TokenCount::new(11, 22, 33).with_cache_written(1));
        assert_eq!(count.total(), 66);
    }

    #[test]
    fn a_count_taken_from_another_stops_at_zero() {
        let count = TokenCount::new(10, 20, 30).with_cache_written(5)
            - TokenCount::new(4, 25, 30).with_cache_written(1);

        assert_eq!(count, TokenCount::new(6, 0, 0).with_cache_written(4));
    }

    #[test]
    fn a_sum_too_large_stays_at_the_largest_number() {
        let count = TokenCount::new(u64::MAX, 0, 0) + TokenCount::new(1, 0, 0);

        assert_eq!(count.input(), u64::MAX);
        assert_eq!(TokenCount::new(u64::MAX, 1, 0).total(), u64::MAX);
    }

    #[test]
    fn only_a_count_with_no_tokens_is_zero() {
        assert!(TokenCount::default().is_zero());
        assert!(!TokenCount::new(0, 0, 1).is_zero());
        assert!(!TokenCount::default().with_cache_written(1).is_zero());
    }
}

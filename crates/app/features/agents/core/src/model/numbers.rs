//! How the numbers of Agents mode are written: tokens short, money with
//! cents, credits in groups of three digits.

use otty_agents::Dollars;

/// A number of tokens written short: as it is below a thousand, else in
/// thousands, millions or billions with one decimal below 100 of them,
/// such as "999", "12.3K", "456K" and "1.2M".
pub(crate) fn short_count(number: u64) -> String {
    const UNITS: [(u64, &str); 3] =
        [(1_000_000_000, "B"), (1_000_000, "M"), (1_000, "K")];

    let Some((size, unit)) =
        UNITS.into_iter().find(|(size, _)| number >= *size)
    else {
        return format!("{number}");
    };

    let tenths = number / (size / 10);
    let whole = tenths / 10;
    let decimal = tenths % 10;
    if whole >= 100 || decimal == 0 {
        format!("{whole}{unit}")
    } else {
        format!("{whole}.{decimal}{unit}")
    }
}

/// `amount` in dollars with cents, such as "$12.40"; half a cent and more
/// rounds up.
pub(crate) fn dollars_text(amount: Dollars) -> String {
    let cents = amount.cents();
    let dollars = digit_groups(cents / 100);
    let cents = cents % 100;

    format!("${dollars}.{cents:02}")
}

/// An amount of money a source told in whole units of `currency`, with
/// cents: "$4.20" for US dollars or no currency, else the amount and the
/// code, such as "4.20 EUR".
pub(crate) fn money_text(amount: f64, currency: Option<&str>) -> String {
    let cents = (amount.abs() * 100.0).round() as u64;
    let sign = if amount < 0.0 && cents > 0 { "-" } else { "" };
    let number = format!("{}.{:02}", digit_groups(cents / 100), cents % 100);

    match currency {
        None | Some("USD") => format!("{sign}${number}"),
        Some(code) => format!("{sign}{number} {code}"),
    }
}

/// A number of credits, rounded to a whole one and written in groups of
/// three digits, such as "3 760" or "-12".
pub(crate) fn credits_text(credits: f64) -> String {
    let rounded = credits.round();
    let digits = digit_groups(rounded.abs() as u64);

    if rounded < 0.0 {
        format!("-{digits}")
    } else {
        digits
    }
}

/// `number` with a space between groups of three digits, such as
/// "1 234 567".
pub(crate) fn digit_groups(number: u64) -> String {
    let digits = number.to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);

    for (index, digit) in digits.chars().enumerate() {
        let left = digits.len() - index;
        if index > 0 && left.is_multiple_of(3) {
            grouped.push(' ');
        }
        grouped.push(digit);
    }

    grouped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_are_written_short() {
        let cases = [
            (0, "0"),
            (999, "999"),
            (1_000, "1K"),
            (12_345, "12.3K"),
            (99_960, "99.9K"),
            (456_789, "456K"),
            (1_250_000, "1.2M"),
            (3_000_000_000, "3B"),
        ];

        for (number, text) in cases {
            assert_eq!(short_count(number), text, "{number}");
        }
    }

    #[test]
    fn dollars_are_written_with_cents() {
        let cases = [
            (0, "$0.00"),
            (4_999, "$0.00"),
            (5_000, "$0.01"),
            (12_400_000, "$12.40"),
            (1_234_567_890, "$1 234.57"),
        ];

        for (micro_dollars, text) in cases {
            assert_eq!(
                dollars_text(Dollars::from_micro_dollars(micro_dollars)),
                text
            );
        }
    }

    #[test]
    fn money_of_a_source_is_written_in_its_currency() {
        assert_eq!(money_text(4.2, Some("USD")), "$4.20");
        assert_eq!(money_text(50.0, None), "$50.00");
        assert_eq!(money_text(1234.5, Some("EUR")), "1 234.50 EUR");
    }

    #[test]
    fn credits_are_whole_and_in_groups_of_three_digits() {
        assert_eq!(credits_text(3760.4), "3 760");
        assert_eq!(credits_text(1_240.6), "1 241");
        assert_eq!(credits_text(-12.0), "-12");
    }
}
